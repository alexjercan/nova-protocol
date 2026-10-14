//! The hulls a station parks at its docking ports: every hull it sells, and
//! every hull the player traded in there.
//!
//! A parked hull is a ship root of its own, docked with the station through
//! one of its ports, and inert: nobody drives it, it is Neutral, every
//! capability is off and it is not lootable ([`FrozenShip::parked`]). It is
//! no body of any cell: it freezes into the station's record
//! ([`freeze_parked_hulls`]) and despawns with the station
//! ([`ParkedHulls`]).
//!
//! A station's hulls arrive as [`PendingParkedHulls`]: the hulls for sale of
//! a generated station's first spawn, or the records of a thaw.
//! [`park_pending_hulls`] spawns them once the station's ports are live: a
//! hull for sale at the station's ports in design order, the last port kept
//! free for the player, and a thawed hull where it froze relative to the
//! station. [`dock_parked_hulls`] then docks each with the port it lies on.
//!
//! FAIL LOUD. More hulls for sale than the station has ports less one, or a
//! hull for sale with no docking port, is a generator bug and panics.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond3;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::objects::{
    ship_design::prelude::{resolve_ship_design, GameShipDesigns, ShipDesignSource},
    spaceship::{
        freeze_ship, parked_capabilities,
        prelude::{FrozenShip, SpaceshipConfig, SpaceshipController},
        spaceship_scenario_object, thaw_ship,
    },
    station_hulls::prelude::StationHullOffer,
};

/// The parked hull records, the pending list and the plugin.
pub mod prelude {
    pub use super::{
        freeze_parked_hulls, FrozenParkedHull, ParkedHullsPlugin, PendingParkedHull,
        PendingParkedHulls,
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
    /// A new hull for sale, parked at the next free port in design order.
    Sale(StationHullOffer),
    /// A hull thawed with its station, parked where it froze.
    Frozen(FrozenParkedHull),
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

/// Spawn one parked hull of `station` at `pose`, moving as the station
/// moves, with the berth that docks it. The caller builds the ship on the
/// returned commands.
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
        ParkedHullBerth::default(),
    ));
    if matches!(terms, ParkedHullType::Rebuy { .. }) {
        hull.insert(OwnedShipMarker);
    }
    hull
}

/// The ship a new hull for sale spawns as: its design, inline, with no
/// driver, Neutral, every capability off, an empty hold and no credits.
fn sale_hull(offer: &StationHullOffer) -> impl Bundle {
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

/// Park every station's pending hulls once its sections are live: a hull
/// for sale on the station port it takes in design order, the last port
/// kept free and hulls past the other ports dropped with a warning, posed by [`parked_hull_pose`] on the hull's first docking
/// port; a thawed hull where it froze relative to the station. Each moves
/// as the station moves.
///
/// The station's pose is its `Transform`: a station spawned this frame has
/// no avian pose yet, and the spawn and the thaw both set the `Transform`.
///
/// # Panics
///
/// On hulls for sale beside thawed ones (a thawed station carries no new
/// hulls for sale), and on a hull for sale whose design does not
/// resolve or has no docking port.
#[expect(
    clippy::type_complexity,
    reason = "the station's pending list, pose, motion and design in one query"
)]
fn park_pending_hulls(
    mut commands: Commands,
    game_designs: Res<GameShipDesigns>,
    game_sections: Res<GameSections>,
    q_stations: Query<(
        Entity,
        &PendingParkedHulls,
        &Transform,
        Option<&LinearVelocity>,
        Option<&AngularVelocity>,
        Option<&ShipDesignSections>,
    )>,
) {
    for (station, pending, transform, velocity, spin, design) in &q_stations {
        // The section spawn has yet to run.
        let Some(design) = design else {
            continue;
        };
        let velocity = velocity.copied().unwrap_or_default();
        let spin = spin.copied().unwrap_or_default();
        let ports: Vec<&DesignSection> = design
            .0
            .iter()
            .filter(|section| section.config.kind.class() == SectionClass::Docking)
            .collect();
        let sales = pending
            .0
            .iter()
            .filter(|hull| matches!(hull, PendingParkedHull::Sale(_)))
            .count();
        assert!(
            sales == 0 || sales == pending.0.len(),
            "park_pending_hulls: station {station:?} parks hulls for sale beside thawed ones"
        );
        let berths = ports.len().saturating_sub(1);
        if sales > berths {
            warn!(
                "park_pending_hulls: station {station:?} has {} ports and {sales} hulls for \
                 sale; it keeps one port free and parks the first {berths}",
                ports.len()
            );
        }
        let mut berths = ports[..berths].iter();
        for hull in &pending.0 {
            match hull {
                PendingParkedHull::Sale(offer) => {
                    // Hulls past the station's ports less one are not parked.
                    let Some(berth) = berths.next() else {
                        continue;
                    };
                    let (resolved, errors) = resolve_ship_design(
                        &ShipDesignSource::Inline(offer.design.clone()),
                        &game_designs,
                        &game_sections,
                    );
                    assert!(
                        errors.is_empty(),
                        "park_pending_hulls: hull '{}' does not resolve: {errors:?}",
                        offer.name
                    );
                    let port = resolved
                        .sections
                        .iter()
                        .find(|section| section.config.kind.class() == SectionClass::Docking)
                        .unwrap_or_else(|| {
                            panic!(
                                "park_pending_hulls: hull '{}' has no docking port",
                                offer.name
                            )
                        });
                    let (at, turn) = parked_hull_pose(
                        (transform.translation, transform.rotation),
                        (berth.position, berth.rotation),
                        (port.position, port.rotation),
                    );
                    debug!(
                        "park_pending_hulls: station {station:?} parks '{}' at port '{}'",
                        offer.name, berth.id
                    );
                    spawn_parked_hull(
                        &mut commands,
                        station,
                        &offer.name,
                        ParkedHullType::Sale,
                        Transform::from_translation(at).with_rotation(turn),
                    )
                    .insert(sale_hull(offer))
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
