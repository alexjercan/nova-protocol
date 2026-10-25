//! The hulls a station holds: every hull it sells and every ship the player
//! owns and left there, parked at its docking ports, and every escort it hires
//! out, coasting beside it.
//!
//! A parked hull is a ship root of its own and inert: nobody drives it, it
//! is Neutral, every capability is off and it is not lootable
//! ([`FrozenShip::parked`]). It is no body of any cell: it freezes into the
//! station's record ([`freeze_parked_hulls`]) and despawns with the station
//! ([`ParkedHulls`]).
//!
//! A station's hulls arrive as [`PendingParkedHulls`]: the offers of a
//! generated station's first spawn, or the records of a thaw.
//! [`park_pending_hulls`] spawns them once the station's ports are live: a
//! hull for sale at the first free station port where one of its own ports
//! lets it stand clear of the station, plating included
//! ([`parked_hull_clear_pose`]), one port always kept free for the player
//! (at the starting station, the berth the fresh player ship takes,
//! [`outward_berth`]), a thawed docked hull where it froze relative to the
//! station, and every escort for hire on a line through the station
//! ([`hire_offer_offsets`]).
//! [`dock_parked_hulls`] then docks each hull for sale or owned with the port
//! it lies on.
//!
//! An escort for hire coasts: in a gravity well the line is the station's
//! orbit, each escort on it ahead or behind with the station's speed, so it
//! keeps its distance; in flat space the line is the station's +X. A thawed
//! escort is placed again the same way, so it never comes back off the
//! orbit it froze beside.
//!
//! A hull for sale that fits at no free port is not parked, with a warning:
//! the catalogue offers what the station's ports hold. A hull for sale with
//! no docking port, or one whose design does not resolve, is a generator bug
//! and panics.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, MetersPerSecond3};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::objects::{
    ship_design::prelude::{resolve_ship_design, GameShipDesigns, ShipDesignSource},
    spaceship::{
        freeze_ship, parked_capabilities,
        prelude::{FrozenShip, SpaceshipConfig, SpaceshipController},
        spaceship_scenario_object, thaw_ship,
    },
    start_role::prelude::{GameStartRoles, OpenWorldSession},
    station::station_well,
    station_hulls::prelude::StationHullOffer,
};

/// The parked hull records, the pending list and the plugin.
pub mod prelude {
    pub use super::{
        freeze_parked_hulls, parked_envelopes, FrozenParkedHull, ParkedHullsPlugin,
        PendingParkedHull, PendingParkedHulls,
    };
}

/// How many frames a parked hull asks to dock before it gives up and drifts
/// inert, with a warning: a pair the capture gates refuse that long will not
/// pass them by waiting.
const BERTH_FRAMES: u32 = 30;

/// A parked hull frozen with its station: its name, where it lay in the
/// station root's frame, how it is listed and its ship record, already
/// parked ([`FrozenShip::parked`]).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenParkedHull {
    name: String,
    pose: Transform,
    terms: ParkedHullType,
    ship: FrozenShip,
}

impl FrozenParkedHull {
    /// The name the list shows.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How the hull is listed.
    pub fn terms(&self) -> ParkedHullType {
        self.terms
    }

    /// The hull's ship record.
    pub fn ship(&self) -> &FrozenShip {
        &self.ship
    }
}

/// One hull a station has yet to park.
#[derive(Clone, Debug)]
pub enum PendingParkedHull {
    /// A new offer: a hull for sale, parked at the next free port in design
    /// order, or an escort for hire, coasting beside the station.
    Offer(Box<StationHullOffer>),
    /// A hull thawed with its station: a docked one parks where it froze,
    /// an escort for hire coasts where a new one would.
    Frozen(Box<FrozenParkedHull>),
}

/// The hulls a station root has yet to park, inserted by the world's spawn
/// and by a thaw. [`park_pending_hulls`] takes it once the station's ports
/// are live.
#[derive(Component, Clone, Debug)]
pub struct PendingParkedHulls(pub Vec<PendingParkedHull>);

/// A parked hull that has yet to dock, and how many frames it has asked.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct ParkedHullBerth {
    frames: u32,
}

/// Parks and docks the hulls a station holds.
pub struct ParkedHullsPlugin;

impl Plugin for ParkedHullsPlugin {
    fn build(&self, app: &mut App) {
        // The well match reads the gravity tunables; the gravity plugin owns
        // the same resource and init_resource is idempotent.
        app.init_resource::<GravitySettings>();
        app.add_systems(Update, (park_pending_hulls, dock_parked_hulls).chain());
    }
}

/// A body's pose as avian holds it, or its `Transform` before avian has
/// seen it.
fn body_pose(world: &World, entity: Entity) -> (Vec3, Quat) {
    let body = world.entity(entity);
    match (body.get::<Position>(), body.get::<Rotation>()) {
        (Some(position), Some(rotation)) => (position.0, rotation.0),
        _ => {
            let transform = body
                .get::<Transform>()
                .unwrap_or_else(|| panic!("parked hulls: {entity} has no pose"));
            (transform.translation, transform.rotation)
        }
    }
}

/// Freeze every hull parked at `station`, each where it lies in the
/// station root's frame.
///
/// # Errors
///
/// [`UnsettledBody`] while the station still has hulls to park, or while a
/// parked hull is mid-process ([`freeze_ship`]).
///
/// # Panics
///
/// On a parked hull with no name or no [`ParkedHullType`]: the park spawns
/// both.
pub fn freeze_parked_hulls(
    world: &World,
    station: Entity,
) -> Result<Vec<FrozenParkedHull>, UnsettledBody> {
    let root = world.entity(station);
    if root.contains::<PendingParkedHulls>() {
        return Err(UnsettledBody {
            reason: "the station has hulls still to park",
        });
    }
    let Some(hulls) = root.get::<ParkedHulls>() else {
        return Ok(Vec::new());
    };
    let (at, turn) = body_pose(world, station);
    let inverse = turn.inverse();
    hulls
        .iter()
        .map(|hull| {
            let body = world.entity(hull);
            let (position, rotation) = body_pose(world, hull);
            Ok(FrozenParkedHull {
                name: body
                    .get::<Name>()
                    .unwrap_or_else(|| panic!("freeze_parked_hulls: hull {hull} has no name"))
                    .to_string(),
                pose: Transform::from_translation(inverse * (position - at))
                    .with_rotation((inverse * rotation).normalize()),
                terms: *body.get::<ParkedHullType>().unwrap_or_else(|| {
                    panic!("freeze_parked_hulls: hull {hull} has no ParkedHullType")
                }),
                ship: freeze_ship(world, hull)?.parked(),
            })
        })
        .collect()
}

/// Spawn one parked hull of `station` at `pose`, with the berth that docks
/// it unless it is for hire. The caller builds the ship and sets its motion
/// on the returned commands.
pub(crate) fn spawn_parked_hull<'a>(
    commands: &'a mut Commands,
    station: Entity,
    name: &str,
    terms: ParkedHullType,
    pose: Transform,
) -> EntityCommands<'a> {
    let mut hull = commands.spawn((
        Name::new(name.to_string()),
        pose,
        GlobalTransform::from(pose),
        Visibility::Visible,
        ParkedHull(station),
        terms,
    ));
    match terms {
        ParkedHullType::Sale => {
            hull.insert(ParkedHullBerth::default());
        }
        ParkedHullType::Owned => {
            hull.insert((ParkedHullBerth::default(), OwnedShipMarker));
        }
        ParkedHullType::Hire { .. } => {}
    }
    hull
}

/// The ship a new offer spawns as: its design, inline, with no driver,
/// Neutral, every capability off, an empty hold and no credits.
fn offer_hull(offer: &StationHullOffer) -> impl Bundle {
    spaceship_scenario_object(SpaceshipConfig {
        design: ShipDesignSource::Inline(offer.design.clone()),
        controller: SpaceshipController::None,
        initial_velocity: MetersPerSecond3::ZERO,
        allegiance: Some(Allegiance::Neutral),
        capabilities: parked_capabilities(),
        inventory: ShipInventoryStock::default(),
        lootable: false,
        credits: 0,
    })
}

/// The resolved sections of the design `source`, for the hull `name`.
///
/// # Panics
///
/// On a design that does not resolve: the manifest check and the catalog
/// digest refuse one before a station carries it.
/// The sections of hull `name`'s design `source`, and whether the design
/// wears the derived skin (its `presentation.skin`, the root's [`ShipSkin`]).
///
/// # Panics
///
/// When the design does not resolve.
fn resolve_hull(
    source: &ShipDesignSource,
    name: &str,
    game_designs: &GameShipDesigns,
    game_sections: &GameSections,
) -> (Vec<DesignSection>, bool) {
    let (resolved, errors) = resolve_ship_design(source, game_designs, game_sections);
    assert!(
        errors.is_empty(),
        "park_pending_hulls: hull '{name}' does not resolve: {errors:?}"
    );
    let sections = resolved
        .sections
        .into_iter()
        .map(|section| DesignSection {
            id: section.id,
            position: section.position,
            rotation: section.rotation,
            config: section.config,
        })
        .collect();
    (sections, resolved.presentation.skin)
}

/// Park every station's pending hulls once its sections are live: a hull
/// for sale, in catalogue order, at the first free station port in design
/// order where one of its docking ports, tried in design order, lets it
/// stand clear of the station, of the hulls parked before it and of the
/// other free ports' approach lanes, plating included
/// ([`parked_hull_clear_pose`]), while two ports are still free, so one
/// stays free for the player; a hull that fits nowhere is dropped with a
/// warning. At the session's starting station new offers park round the
/// fresh player ship: the berth it will take ([`outward_berth`], with
/// nothing parked) is that port, out of the free ones, and its envelope
/// stands in every sale's way, so `berth_at_start` finds the same berth
/// once the sales are docked. A thawed docked hull parks where it froze relative to the
/// station, moving as the station moves. Every escort for hire, new or
/// thawed, coasts where [`hire_offer_place`] puts it.
///
/// The station's pose is its `Transform`: a station spawned this frame has
/// no avian pose yet, and the spawn and the thaw both set the `Transform`.
///
/// # Panics
///
/// On new offers beside thawed hulls (a thawed station carries no new
/// offers), on an offer whose design does not resolve or a hull for sale
/// with no docking port, and at the starting station on a session start role
/// whose hull is not in the loaded content.
#[expect(
    clippy::type_complexity,
    reason = "the station's pending list, id, pose, motion, design and skin in one query"
)]
fn park_pending_hulls(
    mut commands: Commands,
    game_designs: Res<GameShipDesigns>,
    game_sections: Res<GameSections>,
    session: Option<Res<OpenWorldSession>>,
    game_roles: Option<Res<GameStartRoles>>,
    q_stations: Query<(
        Entity,
        &PendingParkedHulls,
        Option<&GeneratedBodyId>,
        &Transform,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
        Option<&ShipDesignSections>,
        Option<&ShipSkin>,
    )>,
    q_wells: Query<(&Transform, &GravityWell)>,
    gravity: Res<GravitySettings>,
) {
    // The well a station is held by, by the rule its post uses, so a hull
    // parked beside it rides the same orbit.
    let wells: Vec<(Vec3, GravityWell)> = q_wells
        .iter()
        .map(|(transform, well)| (transform.translation, well.clone()))
        .collect();
    for (station, pending, id, transform, velocity, spin, design, skin) in &q_stations {
        // The section spawn has yet to run.
        let Some(design) = design else {
            continue;
        };
        let ShipSkin(station_skin) = *skin.unwrap_or_else(|| {
            panic!(
                "park_pending_hulls: station {station:?} has its sections but no ShipSkin; the \
                 section spawn inserts both"
            )
        });
        let velocity = velocity.copied().unwrap_or_default();
        let spin = spin.copied().unwrap_or_default();
        let mut free: Vec<usize> = (0..design.0.len())
            .filter(|&index| design.0[index].config.kind.class() == SectionClass::Docking)
            .collect();
        let offers = pending
            .0
            .iter()
            .filter(|hull| matches!(hull, PendingParkedHull::Offer(_)))
            .count();
        assert!(
            offers == 0 || offers == pending.0.len(),
            "park_pending_hulls: station {station:?} parks new offers beside thawed hulls"
        );
        let mut places = hire_places(
            pending,
            transform,
            velocity.0,
            spin.0,
            station_well(transform.translation, &wells, &gravity).map(|(centre, _)| centre),
            design_reach(&design.0),
            (&game_designs, &game_sections),
        )
        .into_iter();
        // What a sale must stand clear of besides the station: the hulls
        // parked before it and, at the starting station, the player ship.
        let mut parked: Vec<PlacedSectionCollider> = Vec::new();
        let mut reserved = 0;
        // Derived on the first sale: a clad station's envelope is its whole
        // skin, and every sale tries it at every free port.
        let mut station_envelope: Option<Vec<PlacedSectionCollider>> = None;
        let starting = id
            .zip(session.as_deref())
            .is_some_and(|(id, session)| session.is_starting_station(id));
        if offers > 0 && starting {
            let role = session.as_deref().map(|session| &session.role);
            let hull = game_roles
                .as_deref()
                .zip(role)
                .and_then(|(roles, role)| roles.get_role(role))
                .map(|role| role.hull.clone())
                .unwrap_or_else(|| {
                    panic!("park_pending_hulls: the session's start role {role:?} is not loaded")
                });
            let (ship, ship_skin) = resolve_hull(
                &ShipDesignSource::prototype(hull.clone()),
                &hull.to_string(),
                &game_designs,
                &game_sections,
            );
            match outward_berth(&design.0, station_skin, &ship, ship_skin, &[], &[]) {
                Some((berth, _, at, turn)) => {
                    free.retain(|&port| port != berth);
                    reserved = 1;
                    parked.extend(design_envelope(&ship, ship_skin, (at, turn)));
                }
                None => error!(
                    "park_pending_hulls: the player ship '{hull}' stands clear of starting \
                     station {station:?} at no port; its hulls park with no berth kept for it"
                ),
            }
        }
        for hull in &pending.0 {
            match hull {
                PendingParkedHull::Offer(offer) if offer.terms != HullOfferType::Sale => {
                    let HullOfferType::Hire { slot } = offer.terms else {
                        unreachable!("the guard takes every offer but a sale");
                    };
                    let (pose, motion) = places.next().expect("one place per escort for hire");
                    spawn_parked_hull(
                        &mut commands,
                        station,
                        &offer.name,
                        ParkedHullType::Hire { slot },
                        pose,
                    )
                    .insert(offer_hull(offer))
                    .insert((LinearVelocity(motion), AngularVelocity::ZERO));
                }
                PendingParkedHull::Frozen(frozen)
                    if matches!(frozen.terms, ParkedHullType::Hire { .. }) =>
                {
                    let (pose, motion) = places.next().expect("one place per escort for hire");
                    let mut hull =
                        spawn_parked_hull(&mut commands, station, &frozen.name, frozen.terms, pose);
                    thaw_ship(&mut hull, frozen.ship.clone());
                    hull.insert((LinearVelocity(motion), AngularVelocity::ZERO));
                }
                PendingParkedHull::Offer(offer) => {
                    // The player ship's berth is the port the rule keeps.
                    if free.len() + reserved < 2 {
                        warn!(
                            "park_pending_hulls: station {station:?} keeps its last free port; \
                             '{}' is not parked",
                            offer.name
                        );
                        continue;
                    }
                    let (sections, hull_skin) = resolve_hull(
                        &ShipDesignSource::Inline(offer.design.clone()),
                        &offer.name,
                        &game_designs,
                        &game_sections,
                    );
                    let ports: Vec<usize> = (0..sections.len())
                        .filter(|&index| {
                            sections[index].config.kind.class() == SectionClass::Docking
                        })
                        .collect();
                    assert!(
                        !ports.is_empty(),
                        "park_pending_hulls: hull '{}' has no docking port",
                        offer.name
                    );
                    let station_envelope = station_envelope.get_or_insert_with(|| {
                        design_envelope(&design.0, station_skin, (Vec3::ZERO, Quat::IDENTITY))
                    });
                    let hull_envelope =
                        design_envelope(&sections, hull_skin, (Vec3::ZERO, Quat::IDENTITY));
                    let fit = free.iter().enumerate().find_map(|(slot, &berth)| {
                        let mut obstacles = parked.clone();
                        obstacles.extend(
                            free.iter()
                                .filter(|&&other| other != berth)
                                .map(|&other| approach_lane(&design.0[other])),
                        );
                        ports.iter().find_map(|&port| {
                            parked_hull_clear_pose(
                                &design.0,
                                station_envelope,
                                berth,
                                &sections,
                                &hull_envelope,
                                port,
                                &obstacles,
                            )
                            .map(|pose| (slot, pose))
                        })
                    });
                    let Some((slot, (at, turn))) = fit else {
                        warn!(
                            "park_pending_hulls: hull '{}' fits at no free port of station \
                             {station:?}; it is not parked",
                            offer.name
                        );
                        continue;
                    };
                    let berth = free.remove(slot);
                    parked.extend(hull_envelope.iter().map(|boxed| PlacedSectionCollider {
                        position: at + turn * boxed.position,
                        rotation: turn * boxed.rotation,
                        collider: boxed.collider,
                    }));
                    debug!(
                        "park_pending_hulls: station {station:?} parks '{}' at port '{}'",
                        offer.name, design.0[berth].id
                    );
                    let pose = *transform * Transform::from_translation(at).with_rotation(turn);
                    spawn_parked_hull(
                        &mut commands,
                        station,
                        &offer.name,
                        ParkedHullType::Sale,
                        pose,
                    )
                    .insert(offer_hull(offer))
                    // After the spawn bundle, which holds a zero velocity.
                    .insert((velocity, spin));
                }
                PendingParkedHull::Frozen(frozen) => {
                    let pose = *transform * frozen.pose;
                    let mut hull =
                        spawn_parked_hull(&mut commands, station, &frozen.name, frozen.terms, pose);
                    thaw_ship(&mut hull, frozen.ship.clone());
                    hull.insert((velocity, spin));
                }
            }
        }
        commands.entity(station).remove::<PendingParkedHulls>();
    }
}

/// The envelope ([`design_envelope`]) of every hull parked at a port of
/// `station`, for sale or owned, in the station root's frame: what a ship
/// berthed at a free port must stand clear of. An escort for hire coasts
/// away from the ports and is left out.
///
/// # Panics
///
/// On a station or a parked hull at a port with no pose, no design sections
/// or no [`ShipSkin`]: a parked hull's spawn and its section spawn give it
/// all three.
pub fn parked_envelopes(world: &World, station: Entity) -> Vec<PlacedSectionCollider> {
    let pose = |entity: Entity| -> (Vec3, Quat) {
        match (world.get::<Position>(entity), world.get::<Rotation>(entity)) {
            (Some(position), Some(rotation)) => (position.0, rotation.0),
            _ => panic!("parked_envelopes: {entity:?} has no pose"),
        }
    };
    let (station_at, station_turn) = pose(station);
    let into_station = station_turn.inverse();
    world
        .get::<ParkedHulls>(station)
        .into_iter()
        .flat_map(|parked| parked.iter())
        .filter(|&hull| {
            !matches!(
                world.get::<ParkedHullType>(hull),
                Some(ParkedHullType::Hire { .. })
            )
        })
        .flat_map(|hull| {
            let (Some(design), Some(&ShipSkin(skin))) = (
                world.get::<ShipDesignSections>(hull),
                world.get::<ShipSkin>(hull),
            ) else {
                panic!("parked_envelopes: parked hull {hull:?} has no design sections or skin");
            };
            let (at, turn) = pose(hull);
            design_envelope(
                &design.0,
                skin,
                (into_station * (at - station_at), into_station * turn),
            )
        })
        .collect()
}

/// Where each escort for hire of `pending` coasts, in list order, and its
/// velocity: the station at `station` moving at `velocity` and `spin`, in
/// the well centred on `well` if any, with reach `station_reach`. Each
/// stands at its [`hire_offer_offsets`] place on the line
/// ([`hire_offer_place`]).
fn hire_places(
    pending: &PendingParkedHulls,
    station: &Transform,
    velocity: Vec3,
    spin: Vec3,
    well: Option<Vec3>,
    station_reach: f32,
    (game_designs, game_sections): (&GameShipDesigns, &GameSections),
) -> Vec<(Transform, Vec3)> {
    let reaches: Vec<f32> = pending
        .0
        .iter()
        .filter_map(|hull| match hull {
            PendingParkedHull::Offer(offer) if offer.terms != HullOfferType::Sale => {
                Some((ShipDesignSource::Inline(offer.design.clone()), &offer.name))
            }
            PendingParkedHull::Frozen(frozen)
                if matches!(frozen.terms, ParkedHullType::Hire { .. }) =>
            {
                Some((frozen.ship.design().clone(), &frozen.name))
            }
            _ => None,
        })
        .map(|(source, name)| {
            design_reach(&resolve_hull(&source, name, game_designs, game_sections).0)
        })
        .collect();
    hire_offer_offsets(station_reach, &reaches)
        .into_iter()
        .map(|offset| hire_offer_place(station, velocity, spin, well, offset))
        .collect()
}

/// Where an escort for hire `offset` world units along its station's line
/// coasts, and its velocity.
///
/// In the well centred on `well`, the station's orbit turned about its pole
/// by the angle `offset` spans on it carries the station's pose and
/// velocity to the escort's: the escort flies the station's orbit, shifted
/// along it, and keeps its distance. In flat space, or for a station at rest
/// in the well, the escort stands `offset` along the station's +X, at the
/// station's velocity plus its spin about the station.
fn hire_offer_place(
    station: &Transform,
    velocity: Vec3,
    spin: Vec3,
    well: Option<Vec3>,
    offset: f32,
) -> (Transform, Vec3) {
    let orbit = well.and_then(|centre| {
        let radius = station.translation - centre;
        let pole = radius.cross(velocity).try_normalize()?;
        Some((centre, radius, pole))
    });
    match orbit {
        Some((centre, radius, pole)) => {
            let turn = Quat::from_axis_angle(pole, offset / radius.length());
            (
                Transform::from_translation(centre + turn * radius)
                    .with_rotation((turn * station.rotation).normalize()),
                turn * velocity,
            )
        }
        None => {
            let arm = station.rotation * Vec3::X * offset;
            (
                Transform::from_translation(station.translation + arm)
                    .with_rotation(station.rotation),
                velocity + spin.cross(arm),
            )
        }
    }
}

/// The lane a ship docks along at a free station port: the three cells
/// straight out from its face, one cell wide, in the station root's frame.
fn approach_lane(port: &DesignSection) -> PlacedSectionCollider {
    PlacedSectionCollider {
        position: port.position + port.rotation * Vec3::new(0.0, 0.0, -2.0),
        rotation: port.rotation,
        collider: SectionCollider::Cuboid {
            size: Vec3::new(1.0, 1.0, 3.0),
        },
    }
}

/// Dock every parked hull that has yet to with its station, once its port
/// is live, for at most [`BERTH_FRAMES`] frames.
fn dock_parked_hulls(
    mut commands: Commands,
    mut q_hulls: Query<(
        Entity,
        &ParkedHull,
        &mut ParkedHullBerth,
        Has<DockedShip>,
        Option<&Children>,
    )>,
    q_ports: Query<
        (),
        (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (hull, parked, mut berth, docked, children) in &mut q_hulls {
        if docked {
            commands.entity(hull).remove::<ParkedHullBerth>();
            continue;
        }
        if !children
            .into_iter()
            .flatten()
            .any(|child| q_ports.contains(*child))
        {
            continue;
        }
        if berth.frames >= BERTH_FRAMES {
            warn!(
                "dock_parked_hulls: hull {hull:?} could not dock with station {:?}; it drifts \
                 inert",
                parked.0
            );
            commands.entity(hull).remove::<ParkedHullBerth>();
            continue;
        }
        berth.frames += 1;
        commands.trigger(DockingConnectionRequest {
            entity: hull,
            target: parked.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_escort_for_hire_shares_the_station_orbit_a_phase_ahead() {
        let centre = Vec3::new(5.0, -3.0, 2.0);
        let station = Transform::from_translation(centre + Vec3::new(400.0, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(0.3));
        let velocity = Vec3::new(0.0, 0.0, -2.0);
        let (pose, motion) = hire_offer_place(&station, velocity, Vec3::ZERO, Some(centre), 40.0);
        let radius = pose.translation - centre;
        assert!((radius.length() - 400.0).abs() < 1e-3, "same orbit radius");
        assert!((motion.length() - 2.0).abs() < 1e-5, "same orbit speed");
        assert!(radius.dot(motion).abs() < 1e-3, "still a circular orbit");
        let arc = (station.translation - centre).angle_between(radius) * 400.0;
        assert!(
            (arc - 40.0).abs() < 1e-2,
            "40 units along the orbit, got {arc}"
        );
        assert!(
            (pose.translation - station.translation).dot(velocity) > 0.0,
            "a positive offset stands ahead of the station"
        );
    }

    #[test]
    fn an_escort_for_hire_in_flat_space_stands_on_the_station_x_axis_and_turns_with_it() {
        let station = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let spin = Vec3::new(0.0, 0.0, 0.1);
        let (pose, motion) = hire_offer_place(&station, velocity, spin, None, -20.0);
        assert!(pose
            .translation
            .abs_diff_eq(Vec3::new(1.0, -18.0, 3.0), 1e-4));
        assert_eq!(pose.rotation, station.rotation);
        assert!(motion.abs_diff_eq(Vec3::new(3.0, 0.0, 0.0), 1e-4));
    }
}
