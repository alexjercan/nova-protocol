//! The docked assembly: every hull held together by docks, measured and
//! flown as ONE rigid body.
//!
//! An assembly is either a pair of ships on one connection, or a station
//! hub and every hull docked at its ports ([`StationDocks`]). Hulls on fixed
//! joints turn and burn as one body, so everything that plans or applies
//! motion for a docked root reads the assembly, not the root: the controller
//! stack and the PD use its inertia, the autopilot and RCS its mass, centre of
//! mass, velocity and reach. The attitude loop's torque is spread over every
//! root as the wrench a rigid body would feel ([`docked_helm_wrench`]). Torque
//! on the driver's root alone leaves the joints to drag the others round,
//! which chatters across them from a 1:1 pair up and never settles a heavy
//! one.
//!
//! Thrust is NOT split: each engine pushes its own root and the joint carries
//! the load, which the fixed joint holds within a millimetre on every pair
//! measured, up to a partner 20 times the driver's mass.

use avian3d::{math::SymmetricMatrix, prelude::*};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{
    DockedHelmType, DockedShip, DockedStation, DockingConnection, StationDock, StationDocks,
};
use crate::prelude::*;

/// A docked assembly measured as one body, on every one of its roots.
///
/// Re-measured every fixed tick from the roots' live avian mass properties,
/// so a section lost or gained mid-dock, or a hull docking or leaving a
/// station, moves it at once. Absent on an assembly that cannot be measured:
/// no root then drives, and the failure is logged, rather than any root
/// flying on its own numbers.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct DockedAssembly {
    /// Every root's mass together.
    pub mass: f32,
    /// The assembly's centre of mass, world space.
    pub center_of_mass: Vec3,
    /// The velocity of [`center_of_mass`](Self::center_of_mass).
    pub linear_velocity: Vec3,
    /// The assembly's inertia about [`center_of_mass`](Self::center_of_mass),
    /// in THIS root's body frame - the frame a root's own
    /// [`ComputedAngularInertia`] is in, so a reader can use either one the
    /// same way.
    pub inertia: ComputedAngularInertia,
    /// The furthest any hull's face reaches from the assembly's
    /// [`center_of_mass`](Self::center_of_mass): the assembly's
    /// [`HullRadius`], and the arm its attitude envelope is sized by.
    pub reach: f32,
}

/// One root of an assembly as [`measure_docked_assembly`] reads it: `(root,
/// mass, world centre of mass, centre-of-mass velocity, world inertia about
/// its own centre of mass, HullRadius)`.
pub(crate) type AssemblyRoot = (Entity, f32, Vec3, Vec3, Mat3, f32);

/// Measure an assembly from its roots, with the inertia in WORLD space.
///
/// The parallel axis term is written out: avian 0.7's `shifted_tensor` adds
/// the outer product instead of subtracting it.
///
/// Every quantity is checked, and the first bad one is the error, naming the
/// root. An assembly that cannot be measured is not flown on a guess.
pub(crate) fn measure_docked_assembly(roots: &[AssemblyRoot]) -> Result<DockedAssembly, String> {
    if roots.len() < 2 {
        return Err(format!("an assembly of {} roots is no dock", roots.len()));
    }
    for &(root, mass, center, velocity, inertia, radius) in roots {
        if !(mass.is_finite() && mass > 0.0) {
            return Err(format!(
                "root {root:?} mass {mass} is not finite and positive"
            ));
        }
        if !(center.is_finite() && velocity.is_finite()) {
            return Err(format!(
                "root {root:?} centre of mass {center} or velocity {velocity} is not finite"
            ));
        }
        if !inertia.is_finite() {
            return Err(format!("root {root:?} inertia {inertia} is not finite"));
        }
        if !(radius.is_finite() && radius >= 0.0) {
            return Err(format!("root {root:?} HullRadius {radius} is not finite"));
        }
    }
    let mass: f32 = roots.iter().map(|root| root.1).sum();
    let center_of_mass = roots.iter().map(|root| root.2 * root.1).sum::<Vec3>() / mass;
    let linear_velocity = roots.iter().map(|root| root.3 * root.1).sum::<Vec3>() / mass;
    let mut tensor = Mat3::ZERO;
    let mut reach = 0.0f32;
    for &(_, root_mass, center, _, inertia, radius) in roots {
        let offset = center - center_of_mass;
        let outer = Mat3::from_cols(offset * offset.x, offset * offset.y, offset * offset.z);
        tensor += inertia
            + (Mat3::from_diagonal(Vec3::splat(offset.length_squared())) - outer) * root_mass;
        reach = reach.max(offset.length() + radius);
    }
    let inertia = ComputedAngularInertia::from_tensor(SymmetricMatrix::from_mat3_unchecked(tensor));
    let (principal, _) = inertia.principal_angular_inertia_with_local_frame();
    if !(principal.is_finite() && principal.min_element() > 0.0) {
        return Err(format!(
            "the assembly's principal inertia {principal} is not finite and positive"
        ));
    }
    Ok(DockedAssembly {
        mass,
        center_of_mass,
        linear_velocity,
        inertia,
        reach,
    })
}

/// Spread one world-space attitude `torque` over an assembly as the wrench
/// each root would feel if they were one rigid body.
///
/// `assembly` is the assembly's world inertia about its centre of mass; each
/// root is `(mass, world inertia about its own centre of mass, its centre of
/// mass minus the assembly's)`. The angular acceleration is
/// `a = assembly^-1 * torque`; each root gets `own * a` as torque and
/// `mass * (a x offset)` as force at its centre of mass. The forces cancel,
/// and the torques plus their moments sum back to `torque`, so the assembly
/// turns as commanded and nothing is left for the joints to transmit. Returns
/// `(force, torque)` per root, in the order given.
pub(crate) fn docked_helm_wrench(
    torque: Vec3,
    assembly: Mat3,
    roots: &[(f32, Mat3, Vec3)],
) -> Vec<(Vec3, Vec3)> {
    let acceleration = assembly.inverse() * torque;
    roots
        .iter()
        .map(|&(mass, own, offset)| (acceleration.cross(offset) * mass, own * acceleration))
        .collect()
}

/// One docked assembly: its roots, the connections that hold it, and the
/// station hub at its centre, if it has one.
struct DockedGroup {
    /// The station every connection of the group is held by. `None` for a
    /// pair of ships.
    hub: Option<Entity>,
    /// Every root, the hub first.
    roots: Vec<Entity>,
    /// Every connection holding the group.
    connections: Vec<Entity>,
}

/// Every docked assembly in the world: each ship pair on its own, and each
/// station hub with every hull docked at it.
///
/// A connection names at most one station (`on_docking_connection_request`
/// refuses a dock between two), so the groups never overlap.
fn docked_groups<'a>(
    pairs: impl Iterator<Item = (Entity, &'a DockingConnection, Option<&'a StationDock>)>,
    hubs: impl Iterator<Item = (Entity, &'a StationDocks)>,
    connection: impl Fn(Entity) -> Option<DockingConnection>,
) -> Vec<DockedGroup> {
    let mut groups: Vec<DockedGroup> = pairs
        .filter(|(.., hub)| hub.is_none())
        .map(|(entity, connection, _)| DockedGroup {
            hub: None,
            roots: vec![connection.first_ship, connection.second_ship],
            connections: vec![entity],
        })
        .collect();
    for (station, docks) in hubs {
        let mut group = DockedGroup {
            hub: Some(station),
            roots: vec![station],
            connections: Vec::new(),
        };
        for entity in docks.iter() {
            let Some(record) = connection(entity) else {
                continue;
            };
            group.connections.push(entity);
            group.roots.push(if record.first_ship == station {
                record.second_ship
            } else {
                record.first_ship
            });
        }
        groups.push(group);
    }
    groups
}

/// Measure every docked assembly, publish its [`DockedAssembly`] on every
/// root, and decide its one driver.
///
/// For a pair of ships the driver is the helm holder while
/// [`DockedHelmType::Held`], and the endpoint that is not the player while
/// neutral - `second_ship` if neither is, which only happens once the player
/// has moved to another hull. For a station hub it is the player holding the
/// helm of its own connection (only at a dead station, where `HELM` is
/// admitted), and the station otherwise: a living station owns every
/// connection ([`DockedHelmType::Station`]), and a dead one is the partner a
/// neutral pair hands the helm to. A holder that is no longer the player, or
/// no longer in its connection, hands the helm back to neutral here.
///
/// An assembly that cannot be measured drives no root and sets
/// [`DockingConnection::measurement_fault`] on every connection of it; the
/// first pass that measures it clears the fault.
///
/// A held root's throttles are cut every tick: its writers are all gated, so
/// this only matters on the tick it is held, and it is what puts out its
/// plume and its engine sound.
#[expect(clippy::type_complexity, reason = "one query per role")]
pub(super) fn measure_docked_assemblies(
    // Assemblies whose measurement failure is already logged, keyed by the
    // hub or the pair's connection, so an assembly that stays unmeasurable
    // says so once, not at the fixed rate.
    mut reported: Local<EntityHashSet>,
    mut held: Local<Vec<Entity>>,
    mut commands: Commands,
    mut q_connections: Query<(Entity, &mut DockingConnection, Option<&StationDock>)>,
    q_hubs: Query<(Entity, &StationDocks)>,
    q_bodies: Query<(
        &ComputedMass,
        &ComputedCenterOfMass,
        &ComputedAngularInertia,
        &Position,
        &Rotation,
        &LinearVelocity,
        Option<&HullRadius>,
        Has<PlayerSpaceshipMarker>,
    )>,
    mut q_roots: Query<(
        Option<&mut DockedShip>,
        Option<&mut DockedStation>,
        Option<&mut DockedAssembly>,
    )>,
    mut q_thrusters: Query<(&ChildOf, &mut ThrusterSectionInput), With<ThrusterSectionMarker>>,
) {
    reported.retain(|key| q_connections.contains(*key) || q_hubs.contains(*key));
    held.clear();

    let groups = docked_groups(
        q_connections
            .iter()
            .map(|(entity, connection, hub)| (entity, &*connection, hub)),
        q_hubs.iter(),
        |entity| q_connections.get(entity).ok().map(|(_, record, _)| *record),
    );
    let is_player = |ship: Entity| q_bodies.get(ship).is_ok_and(|body| body.7);

    for group in groups {
        let key = group.hub.unwrap_or(group.connections[0]);
        let mut holder = None;
        for &entity in &group.connections {
            let Ok((_, mut connection, _)) = q_connections.get_mut(entity) else {
                continue;
            };
            if let DockedHelmType::Held(ship) = connection.helm {
                if connection.joins(ship) && is_player(ship) {
                    holder = holder.or(Some(ship));
                } else {
                    debug!(
                        "measure_docked_assemblies: {ship:?} no longer holds {entity:?}, \
                         helm back to neutral"
                    );
                    connection.helm = DockedHelmType::Neutral;
                }
            }
        }
        let driver = match (group.hub, holder) {
            (_, Some(holder)) => holder,
            (Some(station), None) => station,
            (None, None) => {
                let Ok((_, connection, _)) = q_connections.get(group.connections[0]) else {
                    continue;
                };
                let ships = [connection.first_ship, connection.second_ship];
                match connection.helm {
                    DockedHelmType::Station(station) => station,
                    _ if is_player(ships[1]) && !is_player(ships[0]) => ships[0],
                    _ => ships[1],
                }
            }
        };

        let readings: Option<Vec<AssemblyRoot>> = group
            .roots
            .iter()
            .map(|&root| {
                let (mass, center, inertia, position, rotation, velocity, radius, _) =
                    q_bodies.get(root).ok()?;
                Some((
                    root,
                    mass.value(),
                    position.0 + rotation.0 * center.0,
                    velocity.0,
                    Mat3::from(inertia.rotated(rotation.0).tensor()),
                    radius.map_or(f32::NAN, |radius| **radius),
                ))
            })
            .collect();
        let measured = match readings {
            Some(readings) => measure_docked_assembly(&readings),
            None => Err("a root has no avian mass properties".to_string()),
        };
        let fault = measured.is_err();
        for &entity in &group.connections {
            if let Ok((_, mut connection, _)) = q_connections.get_mut(entity) {
                if connection.measurement_fault != fault {
                    connection.measurement_fault = fault;
                }
            }
        }

        match measured {
            Ok(assembly) => {
                reported.remove(&key);
                for &root in &group.roots {
                    let drives = root == driver;
                    if !drives {
                        held.push(root);
                    }
                    let Ok((ship, station, published)) = q_roots.get_mut(root) else {
                        continue;
                    };
                    set_drives(ship, station, drives);
                    let Ok((.., rotation, _, _, _)) = q_bodies.get(root) else {
                        continue;
                    };
                    let local = DockedAssembly {
                        inertia: assembly.inertia.rotated(rotation.0.inverse()),
                        ..assembly
                    };
                    match published {
                        Some(mut published) => {
                            published.set_if_neq(local);
                        }
                        None => {
                            commands.entity(root).try_insert(local);
                        }
                    }
                }
            }
            Err(reason) => {
                if reported.insert(key) {
                    error!(
                        "measure_docked_assemblies: {key:?} cannot be measured ({reason}); \
                         no hull drives until it can"
                    );
                }
                for &root in &group.roots {
                    if let Ok((ship, station, published)) = q_roots.get_mut(root) {
                        set_drives(ship, station, false);
                        if published.is_some() {
                            commands.entity(root).try_remove::<DockedAssembly>();
                        }
                    }
                    held.push(root);
                }
            }
        }
    }

    if held.is_empty() {
        return;
    }
    for (&ChildOf(root), mut input) in &mut q_thrusters {
        if **input != 0.0 && held.contains(&root) {
            **input = 0.0;
        }
    }
}

/// Write `drives` on whichever docked record a root carries: a ship's
/// [`DockedShip`] or a hub's [`DockedStation`].
fn set_drives(ship: Option<Mut<DockedShip>>, station: Option<Mut<DockedStation>>, drives: bool) {
    if let Some(mut ship) = ship {
        if ship.drives != drives {
            ship.drives = drives;
        }
    }
    if let Some(mut station) = station {
        if station.drives != drives {
            station.drives = drives;
        }
    }
}

/// Apply the driver's attitude loop to a docked assembly as the rigid-body
/// wrench over every root.
///
/// The torque is the sum of the driver's live controllers' outputs - the same
/// set `sync_controller_section_forces` applies to an undocked hull, which
/// skips every docked root, so the loop is applied once. A held root's loop
/// is ignored, and a driver with no live computer turns nothing: its pilot
/// still has the drive.
#[expect(clippy::type_complexity, reason = "one query per role")]
pub(super) fn apply_docked_helm_wrench(
    q_connections: Query<(Entity, &DockingConnection, Option<&StationDock>)>,
    q_hubs: Query<(Entity, &StationDocks)>,
    q_drives: Query<(Option<&DockedShip>, Option<&DockedStation>)>,
    q_roots: Query<(
        &DockedAssembly,
        &ComputedMass,
        &ComputedAngularInertia,
        &ComputedCenterOfMass,
        &Position,
        &Rotation,
    )>,
    q_controllers: Query<
        (&PDControllerOutput, &PDControllerTarget),
        Without<SectionInactiveMarker>,
    >,
    mut q_forces: Query<Forces>,
) {
    let groups = docked_groups(q_connections.iter(), q_hubs.iter(), |entity| {
        q_connections.get(entity).ok().map(|(_, record, _)| *record)
    });
    for group in groups {
        let Some(driver) = group.roots.iter().copied().find(|root| {
            q_drives.get(*root).is_ok_and(|(ship, station)| {
                ship.is_some_and(|ship| ship.drives) || station.is_some_and(|hub| hub.drives)
            })
        }) else {
            continue;
        };
        let torque: Vec3 = q_controllers
            .iter()
            .filter(|(_, target)| ***target == driver)
            .map(|(output, _)| output.0)
            .sum();
        if torque == Vec3::ZERO {
            continue;
        }
        let Ok((assembly, .., rotation)) = q_roots.get(driver) else {
            continue;
        };
        let world = Mat3::from(assembly.inertia.rotated(rotation.0).tensor());
        let parts: Option<Vec<(f32, Mat3, Vec3)>> = group
            .roots
            .iter()
            .map(|&root| {
                let (_, mass, inertia, center, position, rotation) = q_roots.get(root).ok()?;
                Some((
                    mass.value(),
                    Mat3::from(inertia.rotated(rotation.0).tensor()),
                    position.0 + rotation.0 * center.0 - assembly.center_of_mass,
                ))
            })
            .collect();
        let Some(parts) = parts else {
            continue;
        };
        let wrench = docked_helm_wrench(torque, world, &parts);
        for (&root, (force, torque)) in group.roots.iter().zip(wrench) {
            if let Ok(mut forces) = q_forces.get_mut(root) {
                forces.apply_force(force);
                forces.apply_torque(torque);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solid box's inertia about its own centre, world-aligned.
    fn box_inertia(mass: f32, size: Vec3) -> Mat3 {
        let s = size * size;
        Mat3::from_diagonal(Vec3::new(s.y + s.z, s.x + s.z, s.x + s.y) * (mass / 12.0))
    }

    /// Two boxes side by side along X, measured as a pair, equal the one box
    /// that is both of them.
    #[test]
    fn a_docked_assembly_matches_the_merged_rigid_body() {
        let size = Vec3::new(2.0, 1.0, 3.0);
        let assembly = measure_docked_assembly(&[
            (
                Entity::from_raw_u32(1).unwrap(),
                6.0,
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::X,
                box_inertia(6.0, size),
                1.5,
            ),
            (
                Entity::from_raw_u32(2).unwrap(),
                6.0,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::NEG_X,
                box_inertia(6.0, size),
                1.5,
            ),
        ])
        .unwrap();
        let merged = box_inertia(12.0, Vec3::new(4.0, 1.0, 3.0));
        assert_eq!(assembly.mass, 12.0);
        assert!(assembly.center_of_mass.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(assembly.linear_velocity.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(
            Mat3::from(assembly.inertia.tensor()).abs_diff_eq(merged, 1e-4),
            "pair {:?} vs merged {merged:?}",
            Mat3::from(assembly.inertia.tensor())
        );
        assert_eq!(assembly.reach, 2.5);
    }

    /// Four boxes in a row along X, measured as a station hub and three
    /// docked hulls, equal the one box that is all of them, and the helm
    /// wrench over the four turns it as commanded with no net force.
    #[test]
    fn a_hub_and_three_hulls_measure_and_turn_as_one_body() {
        let size = Vec3::new(2.0, 1.0, 3.0);
        let roots: Vec<AssemblyRoot> = (0..4)
            .map(|index| {
                (
                    Entity::from_raw_u32(index + 1).unwrap(),
                    6.0,
                    Vec3::new(-3.0 + 2.0 * index as f32, 0.0, 0.0),
                    Vec3::Y,
                    box_inertia(6.0, size),
                    1.5,
                )
            })
            .collect();
        let assembly = measure_docked_assembly(&roots).unwrap();
        let merged = box_inertia(24.0, Vec3::new(8.0, 1.0, 3.0));
        assert_eq!(assembly.mass, 24.0, "the mass is the sum");
        assert!(assembly.center_of_mass.abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(assembly.linear_velocity.abs_diff_eq(Vec3::Y, 1e-6));
        assert!(
            Mat3::from(assembly.inertia.tensor()).abs_diff_eq(merged, 1e-3),
            "hub {:?} vs merged {merged:?}",
            Mat3::from(assembly.inertia.tensor())
        );
        assert_eq!(assembly.reach, 4.5);

        let torque = Vec3::new(40.0, -900.0, 15.0);
        let parts: Vec<(f32, Mat3, Vec3)> = roots
            .iter()
            .map(|root| (root.1, root.4, root.2 - assembly.center_of_mass))
            .collect();
        let wrench = docked_helm_wrench(torque, Mat3::from(assembly.inertia.tensor()), &parts);
        let net_force: Vec3 = wrench.iter().map(|(force, _)| *force).sum();
        let net_torque: Vec3 = wrench
            .iter()
            .zip(&parts)
            .map(|((force, turn), part)| *turn + part.2.cross(*force))
            .sum();
        assert!(
            net_force.length() <= 1e-3 * torque.length(),
            "net force {net_force}"
        );
        assert!(
            net_torque.abs_diff_eq(torque, 1e-3 * torque.length()),
            "net torque {net_torque} vs {torque}"
        );
    }

    /// The wrench turns the pair exactly as commanded and pushes it nowhere,
    /// on a 1:1 pair and on a partner 20 times the driver.
    #[test]
    fn the_helm_wrench_sums_to_the_commanded_torque_with_no_net_force() {
        for ratio in [1.0, 20.0] {
            let size = Vec3::new(3.0, 2.0, 15.0);
            let driver = (185.0, Vec3::new(0.0, 0.0, 0.0), box_inertia(185.0, size));
            let partner = (
                185.0 * ratio,
                Vec3::new(9.0, 1.0, -4.0),
                box_inertia(185.0 * ratio, size * 2.0),
            );
            let assembly = measure_docked_assembly(&[
                (
                    Entity::from_raw_u32(1).unwrap(),
                    driver.0,
                    driver.1,
                    Vec3::ZERO,
                    driver.2,
                    8.0,
                ),
                (
                    Entity::from_raw_u32(2).unwrap(),
                    partner.0,
                    partner.1,
                    Vec3::ZERO,
                    partner.2,
                    16.0,
                ),
            ])
            .unwrap();
            let world = Mat3::from(assembly.inertia.tensor());
            let torque = Vec3::new(120.0, -9760.0, 35.0);
            let offsets = [
                driver.1 - assembly.center_of_mass,
                partner.1 - assembly.center_of_mass,
            ];
            let wrench = docked_helm_wrench(
                torque,
                world,
                &[
                    (driver.0, driver.2, offsets[0]),
                    (partner.0, partner.2, offsets[1]),
                ],
            );
            let net_force = wrench[0].0 + wrench[1].0;
            let net_torque = wrench[0].1
                + wrench[1].1
                + offsets[0].cross(wrench[0].0)
                + offsets[1].cross(wrench[1].0);
            assert!(
                net_force.length() <= 1e-3 * torque.length(),
                "ratio {ratio}: net force {net_force}"
            );
            assert!(
                net_torque.abs_diff_eq(torque, 1e-3 * torque.length()),
                "ratio {ratio}: net torque {net_torque} vs {torque}"
            );
        }
    }

    /// A root with no mass, a broken tensor or no measured radius is refused,
    /// and the error names the root.
    #[test]
    fn an_unmeasurable_assembly_is_refused() {
        let good = (
            Entity::from_raw_u32(1).unwrap(),
            1.0,
            Vec3::ZERO,
            Vec3::ZERO,
            Mat3::IDENTITY,
            1.0,
        );
        let bad = Entity::from_raw_u32(2).unwrap();
        for (label, root) in [
            (
                "massless",
                (bad, 0.0, Vec3::X, Vec3::ZERO, Mat3::IDENTITY, 1.0),
            ),
            (
                "nan mass",
                (bad, f32::NAN, Vec3::X, Vec3::ZERO, Mat3::IDENTITY, 1.0),
            ),
            (
                "nan inertia",
                (
                    bad,
                    1.0,
                    Vec3::X,
                    Vec3::ZERO,
                    Mat3::from_diagonal(Vec3::NAN),
                    1.0,
                ),
            ),
            (
                "no radius",
                (bad, 1.0, Vec3::X, Vec3::ZERO, Mat3::IDENTITY, f32::NAN),
            ),
        ] {
            let error = measure_docked_assembly(&[good, root]).unwrap_err();
            assert!(error.contains(&format!("{bad:?}")), "{label}: {error}");
        }
        let flat = measure_docked_assembly(&[
            (bad, 1.0, Vec3::ZERO, Vec3::ZERO, Mat3::ZERO, 0.0),
            (bad, 1.0, Vec3::ZERO, Vec3::ZERO, Mat3::ZERO, 0.0),
        ]);
        assert!(
            flat.is_err(),
            "two point masses on one spot have no inertia"
        );
        assert!(
            measure_docked_assembly(&[good]).is_err(),
            "one root is no dock"
        );
    }
}
