//! The docked assembly: a pair measured and flown as ONE rigid body.
//!
//! Two hulls on a fixed joint turn and burn as one body, so everything that
//! plans or applies motion for a docked root reads the pair, not the root:
//! the controller stack and the PD use the pair's inertia, the autopilot and
//! RCS the pair's mass, centre of mass, velocity and reach. The attitude
//! loop's torque is spread over both roots as the wrench a rigid body would
//! feel ([`docked_helm_wrench`]). Torque on the driver's root alone leaves the
//! joint to drag the partner round, which chatters across it from a 1:1 pair
//! up and never settles a heavy one.
//!
//! Thrust is NOT split: each engine pushes its own root and the joint carries
//! the load, which the fixed joint holds within a millimetre on every pair
//! measured, up to a partner 20 times the driver's mass.

use avian3d::{math::SymmetricMatrix, prelude::*};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::{DockedHelmType, DockedShip, DockingConnection};
use crate::prelude::*;

/// A docked pair measured as one body, on both of its roots.
///
/// Re-measured every fixed tick from the two roots' live avian mass
/// properties, so a section lost or gained mid-dock moves it at once. Absent
/// on a pair that cannot be measured: neither root then drives, and the
/// failure is logged, rather than either root flying on its own numbers.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct DockedAssembly {
    /// Both roots' mass together.
    pub mass: f32,
    /// The pair's centre of mass, world space.
    pub center_of_mass: Vec3,
    /// The velocity of [`center_of_mass`](Self::center_of_mass).
    pub linear_velocity: Vec3,
    /// The pair's inertia about [`center_of_mass`](Self::center_of_mass), in
    /// THIS root's body frame - the frame a root's own
    /// [`ComputedAngularInertia`] is in, so a reader can use either one the
    /// same way.
    pub inertia: ComputedAngularInertia,
    /// The furthest either hull's face reaches from the pair's
    /// [`center_of_mass`](Self::center_of_mass): the assembly's
    /// [`HullRadius`], and the arm its attitude envelope is sized by.
    pub reach: f32,
}

/// Measure a pair from its two roots, with the inertia in WORLD space.
///
/// Each root is `(root, mass, world centre of mass, centre-of-mass velocity,
/// world inertia about its own centre of mass, HullRadius)`. The parallel
/// axis term is written out: avian 0.7's `shifted_tensor` adds the outer
/// product instead of subtracting it.
///
/// Every quantity is checked, and the first bad one is the error, naming the
/// root. A pair that cannot be measured is not flown on a guess.
pub(crate) fn measure_docked_assembly(
    roots: [(Entity, f32, Vec3, Vec3, Mat3, f32); 2],
) -> Result<DockedAssembly, String> {
    for (root, mass, center, velocity, inertia, radius) in roots {
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
    for (_, root_mass, center, _, inertia, radius) in roots {
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
            "the pair's principal inertia {principal} is not finite and positive"
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

/// Spread one world-space attitude `torque` over a pair as the wrench each
/// root would feel if the two were one rigid body.
///
/// `assembly` is the pair's world inertia about its centre of mass; each root
/// is `(mass, world inertia about its own centre of mass, its centre of mass
/// minus the pair's)`. The pair's angular acceleration is
/// `a = assembly^-1 * torque`; each root gets `own * a` as torque and
/// `mass * (a x offset)` as force at its centre of mass. The forces cancel,
/// and the torques plus their moments sum back to `torque`, so the pair turns
/// as commanded and nothing is left for the joint to transmit. Returns
/// `(force, torque)` per root, in the order given.
pub(crate) fn docked_helm_wrench(
    torque: Vec3,
    assembly: Mat3,
    roots: [(f32, Mat3, Vec3); 2],
) -> [(Vec3, Vec3); 2] {
    let acceleration = assembly.inverse() * torque;
    roots.map(|(mass, own, offset)| (acceleration.cross(offset) * mass, own * acceleration))
}

/// Measure every docked pair, publish its [`DockedAssembly`] on both roots,
/// and decide its one driver.
///
/// The driver is the helm holder while [`DockedHelmType::Held`], and the
/// endpoint that is not the player while neutral - `second_ship` if neither
/// is, which only happens once the player has moved to another hull. A
/// holder that is no longer the player, or no longer in the pair, hands the
/// helm back to neutral here.
///
/// A pair that cannot be measured drives neither root and sets
/// [`DockingConnection::measurement_fault`]; the first pass that measures it
/// clears the fault.
///
/// A held root's throttles are cut every tick: its writers are all gated, so
/// this only matters on the tick it is held, and it is what puts out its
/// plume and its engine sound.
#[expect(clippy::type_complexity, reason = "one query per role")]
pub(super) fn measure_docked_assemblies(
    // Connections whose measurement failure is already logged, so a pair
    // that stays unmeasurable says so once, not at the fixed rate.
    mut reported: Local<EntityHashSet>,
    mut held: Local<Vec<Entity>>,
    mut commands: Commands,
    mut q_connections: Query<(Entity, &mut DockingConnection)>,
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
    mut q_ships: Query<(&mut DockedShip, Option<&mut DockedAssembly>)>,
    mut q_thrusters: Query<(&ChildOf, &mut ThrusterSectionInput), With<ThrusterSectionMarker>>,
) {
    reported.retain(|connection| q_connections.contains(*connection));
    held.clear();

    for (entity, mut connection) in &mut q_connections {
        let ships = [connection.first_ship, connection.second_ship];
        let is_player = |ship: Entity| q_bodies.get(ship).is_ok_and(|body| body.7);

        if let DockedHelmType::Held(holder) = connection.helm {
            if !(ships.contains(&holder) && is_player(holder)) {
                debug!(
                    "measure_docked_assemblies: {holder:?} no longer holds {entity:?}, \
                     helm back to neutral"
                );
                connection.helm = DockedHelmType::Neutral;
            }
        }
        let driver = match connection.helm {
            DockedHelmType::Held(holder) => holder,
            DockedHelmType::Neutral if is_player(ships[1]) && !is_player(ships[0]) => ships[0],
            DockedHelmType::Neutral => ships[1],
        };

        let readings = ships.map(|ship| {
            let (mass, center, inertia, position, rotation, velocity, radius, _) =
                q_bodies.get(ship).ok()?;
            Some((
                ship,
                mass.value(),
                position.0 + rotation.0 * center.0,
                velocity.0,
                Mat3::from(inertia.rotated(rotation.0).tensor()),
                radius.map_or(f32::NAN, |radius| **radius),
            ))
        });
        let measured = match readings {
            [Some(first), Some(second)] => measure_docked_assembly([first, second]),
            _ => Err("a root has no avian mass properties".to_string()),
        };

        match measured {
            Ok(assembly) => {
                reported.remove(&entity);
                if connection.measurement_fault {
                    connection.measurement_fault = false;
                }
                for ship in ships {
                    let Ok((mut docked, published)) = q_ships.get_mut(ship) else {
                        continue;
                    };
                    let drives = ship == driver;
                    if docked.drives != drives {
                        docked.drives = drives;
                    }
                    if !drives {
                        held.push(ship);
                    }
                    let Ok((.., rotation, _, _, _)) = q_bodies.get(ship) else {
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
                            commands.entity(ship).try_insert(local);
                        }
                    }
                }
            }
            Err(reason) => {
                if reported.insert(entity) {
                    error!(
                        "measure_docked_assemblies: {entity:?} cannot be measured ({reason}); \
                         neither hull drives until it can"
                    );
                }
                if !connection.measurement_fault {
                    connection.measurement_fault = true;
                }
                for ship in ships {
                    if let Ok((mut docked, published)) = q_ships.get_mut(ship) {
                        if docked.drives {
                            docked.drives = false;
                        }
                        if published.is_some() {
                            commands.entity(ship).try_remove::<DockedAssembly>();
                        }
                    }
                    held.push(ship);
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

/// Apply the driver's attitude loop to a docked pair as the rigid-body wrench
/// over both roots.
///
/// The torque is the sum of the driver's live controllers' outputs - the same
/// set `sync_controller_section_forces` applies to an undocked hull, which
/// skips both docked roots, so the loop is applied once. A held root's loop
/// is ignored, and a driver with no live computer turns nothing: its pilot
/// still has the drive.
#[expect(clippy::type_complexity, reason = "one query per role")]
pub(super) fn apply_docked_helm_wrench(
    q_connections: Query<&DockingConnection>,
    q_roots: Query<(
        &DockedShip,
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
    for connection in &q_connections {
        let ships = [connection.first_ship, connection.second_ship];
        let Some(driver) = ships
            .into_iter()
            .find(|ship| q_roots.get(*ship).is_ok_and(|root| root.0.drives))
        else {
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
        let Ok((_, assembly, .., rotation)) = q_roots.get(driver) else {
            continue;
        };
        let world = Mat3::from(assembly.inertia.rotated(rotation.0).tensor());
        let parts = ships.map(|ship| {
            let (_, _, mass, inertia, center, position, rotation) = q_roots.get(ship).ok()?;
            Some((
                mass.value(),
                Mat3::from(inertia.rotated(rotation.0).tensor()),
                position.0 + rotation.0 * center.0 - assembly.center_of_mass,
            ))
        });
        let [Some(first), Some(second)] = parts else {
            continue;
        };
        let wrench = docked_helm_wrench(torque, world, [first, second]);
        for (ship, (force, torque)) in ships.into_iter().zip(wrench) {
            if let Ok(mut forces) = q_forces.get_mut(ship) {
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
        let assembly = measure_docked_assembly([
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
            let assembly = measure_docked_assembly([
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
                [
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
            let error = measure_docked_assembly([good, root]).unwrap_err();
            assert!(error.contains(&format!("{bad:?}")), "{label}: {error}");
        }
        let flat = measure_docked_assembly([
            (bad, 1.0, Vec3::ZERO, Vec3::ZERO, Mat3::ZERO, 0.0),
            (bad, 1.0, Vec3::ZERO, Vec3::ZERO, Mat3::ZERO, 0.0),
        ]);
        assert!(
            flat.is_err(),
            "two point masses on one spot have no inertia"
        );
    }
}
