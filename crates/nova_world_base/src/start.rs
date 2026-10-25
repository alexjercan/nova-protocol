//! The start: the allied station the player begins docked at.
//!
//! New Game picks the station once per seed ([`starting_station`]): the
//! living station nearest the origin whose civilization's seeded stance is
//! Ally, of the civilizations in the 3x3x3 lattice block round the origin.
//! A seed with no such station is refused at generation, before anything
//! streams; nothing falls back to a Neutral station or to open space. The
//! session and the world save carry the pick; `SpawnStartRole` spawns the
//! fresh player ship in the station's open space with a
//! [`StartBerthMarker`], and [`berth_at_start`] puts it on the station's
//! outward port and docks it once the station stands.

#[cfg(not(target_arch = "wasm32"))]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_assets::prelude::LoadedSectionPacks;
#[cfg(not(target_arch = "wasm32"))]
use nova_events::prelude::{EntityId, GeneratedBodyId};
use nova_events::prelude::{Meters3, SectorCoord};
use nova_gameplay::prelude::{ClusterType, GameItems, StanceType};
#[cfg(not(target_arch = "wasm32"))]
use nova_gameplay::prelude::{PointRotation, PointRotationOutput, SectionInactiveMarker};
#[cfg(not(target_arch = "wasm32"))]
use nova_scenario::prelude::{parked_envelopes, PendingParkedHulls, StartBerthMarker};
use nova_scenario::prelude::{OpenWorldSession, StartRoleDesignId, StartingStation};
use nova_ship::prelude::GameStyles;
#[cfg(not(target_arch = "wasm32"))]
use nova_ship::prelude::{
    outward_berth, DockedPort, DockedShip, DockingConnectionRequest, DockingSectionMarker,
    ParkedHullType, ParkedHulls, ShipDesignSections, ShipSkin,
};
use nova_world::prelude::{
    sector_id, SectorFault, SectorGenerationInput, SectorGenerator, WorldGeometry,
};

use crate::{
    civilizations::{CivilizationField, CivilizationStatusType},
    clusters::{cluster_at, station_pose, Cluster},
    environment::EnvironmentFields,
    layered::{NovaLayeredWorld, CLEARANCE_MARGIN},
    sector_stations::StationClaims,
    OPEN_WORLD_SECTOR_EDGE,
};

/// How many civilization lattice steps out from the origin the start looks
/// for an allied civilization: the 3x3x3 block, 27 civilizations, of which
/// about six are living allies.
const START_CIVILIZATION_SEARCH: i32 = 1;

/// The station the player starts docked at for `seed`: the station nearest
/// the origin among those the living Ally civilizations of the 3x3x3 lattice
/// block round the origin keep, ties by station id. The station is the one
/// `plan_sector` places: its node is one the civilization claims, with the
/// pose the plan gives it. The spawn point stands past the station on its
/// open side, two clearance margins clear of it.
///
/// # Errors
///
/// [`SectorFault::Generation`] with the seed when no living Ally civilization
/// of the block keeps a station, and whatever the station claims or the
/// clusters refuse for one of its civilizations.
pub fn starting_station(
    seed: u32,
    generator: &NovaLayeredWorld,
) -> Result<StartingStation, SectorFault> {
    let edge = OPEN_WORLD_SECTOR_EDGE;
    let fields = EnvironmentFields::new(seed);
    let civilizations = CivilizationField::new(seed);
    let mut claims = StationClaims::new(generator.parts(), generator.items());
    let mut hosts = |clearance, node| {
        Ok(cluster_at(&fields, seed, edge, node)?
            .is_some_and(|cluster| station_pose(&cluster, edge, clearance, seed).is_some()))
    };
    let mut nearest: Option<(StartingStation, f32)> = None;
    let span = -START_CIVILIZATION_SEARCH..=START_CIVILIZATION_SEARCH;
    for x in span.clone() {
        for y in span.clone() {
            for z in span.clone() {
                let civilization = civilizations.civilization([x, y, z]);
                if civilization.status != CivilizationStatusType::Living
                    || civilization.stance != StanceType::Ally
                {
                    continue;
                }
                let clearance = claims.station(&civilization)?.clearance;
                for node in claims.nodes(&civilization, &mut hosts)?.to_vec() {
                    let Some(cluster) = cluster_at(&fields, seed, edge, node)? else {
                        continue;
                    };
                    let claimed = claims.claim(&civilizations, cluster.anchor, node, &mut hosts)?;
                    if claimed.is_none_or(|owner| owner.id != civilization.id) {
                        continue;
                    }
                    let Some(pose) = station_pose(&cluster, edge, clearance, seed) else {
                        continue;
                    };
                    let coord = SectorCoord::containing(pose.position, edge);
                    let id = sector_id(coord, &format!("{}_station", Cluster::slug(node)), 0);
                    let distance = pose.position.length().get();
                    let closer = nearest.as_ref().is_none_or(|(best, best_distance)| {
                        distance < *best_distance || (distance == *best_distance && id < best.id)
                    });
                    if closer {
                        let standoff = (clearance + CLEARANCE_MARGIN + CLEARANCE_MARGIN).get();
                        let spawn = Meters3(pose.position.get() + pose.open * standoff);
                        let start = StartingStation {
                            id,
                            position: pose.position,
                            spawn,
                        };
                        nearest = Some((start, distance));
                    }
                }
            }
        }
    }
    nearest
        .map(|(start, _)| start)
        .ok_or_else(|| SectorFault::Generation {
            id: format!("seed {seed}"),
            field: "start",
            value: "no living Ally civilization of the 3x3x3 lattice block round the origin \
                    keeps a station"
                .to_string(),
        })
}

/// How many cells out from the start's cell on each horizontal axis New
/// Game's seed preview describes: a 5 by 5 grid, a quarter of the chart
/// view's 7 by 7 at a glance.
pub const START_PREVIEW_REACH: i32 = 2;

/// What New Game's seed preview draws for a seed: the start and the cells
/// round it, as the generator describes them without streaming anything.
#[derive(Clone, Debug, PartialEq)]
pub struct StartPreview {
    /// The station the player starts docked at.
    pub start: StartingStation,
    /// Every cell within [`START_PREVIEW_REACH`] of the start's cell on its
    /// layer, in row order, each with the type of every cluster with a body
    /// in it, as the chart records a visited cell.
    pub cells: Vec<(SectorCoord, Vec<ClusterType>)>,
    /// Every station those cells place, in cell order: the cell and whether
    /// it is dead. The start is among them.
    pub stations: Vec<(SectorCoord, bool)>,
}

impl StartPreview {
    /// The cell the start stands in.
    pub fn cell(&self) -> SectorCoord {
        self.start.id.origin
    }

    /// Where the starting station stands in its cell, as a share of the
    /// cell's edge on each axis, `-0.5` to `0.5`: a chart drawn at any cell
    /// scale puts the start there.
    pub fn start_within_cell(&self) -> Vec3 {
        (self.start.position - self.cell().centre(OPEN_WORLD_SECTOR_EDGE)).0
            / OPEN_WORLD_SECTOR_EDGE.0
    }
}

/// The seed preview of `seed`: the starting station and the generator's
/// description of the cells round it, the same plan `plan_sector` streams.
///
/// # Errors
///
/// What [`starting_station`] refuses, and whatever the generator refuses one
/// of the cells for.
pub fn start_preview(seed: u32, generator: &NovaLayeredWorld) -> Result<StartPreview, SectorFault> {
    let start = starting_station(seed, generator)?;
    let centre = start.id.origin;
    let mut cells = Vec::new();
    let mut stations = Vec::new();
    for dz in -START_PREVIEW_REACH..=START_PREVIEW_REACH {
        for dx in -START_PREVIEW_REACH..=START_PREVIEW_REACH {
            let coord = SectorCoord::new(centre.x + dx, centre.y, centre.z + dz);
            let manifest = generator.generate(SectorGenerationInput {
                seed,
                geometry: WorldGeometry {
                    sector_edge: OPEN_WORLD_SECTOR_EDGE,
                },
                coord,
            })?;
            cells.push((coord, manifest.cluster_types));
            stations.extend(
                manifest
                    .stations
                    .iter()
                    .map(|station| (coord, station.spawn.derelict)),
            );
        }
    }
    Ok(StartPreview {
        start,
        cells,
        stations,
    })
}

/// The session New Game's Create inserts for `seed` and `role`: the world
/// built from the loaded content and the starting station it picks.
///
/// # Errors
///
/// One line naming every fault the loaded content leaves the generator
/// with, or the refusal of [`starting_station`].
pub fn start_session(
    seed: u32,
    role: StartRoleDesignId,
    loaded: &LoadedSectionPacks,
    styles: &GameStyles,
    items: &GameItems,
) -> Result<OpenWorldSession, String> {
    let generator = NovaLayeredWorld::from_loaded(loaded, styles, items).map_err(|faults| {
        faults
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let start = starting_station(seed, &generator).map_err(|fault| fault.to_string())?;
    Ok(OpenWorldSession { seed, role, start })
}

/// Berth the fresh player ship at the session's starting station and dock
/// it.
///
/// Each frame the ship carries a [`StartBerthMarker`] and the station with
/// the session's start id stands, with every hull it parks for sale or owned
/// docked round the berth the parking reserved for the ship, the ship is put
/// on the berth [`outward_berth`] picks at the station's free ports, clear
/// of the station and the parked hulls, plating included ([`clear_berth`]),
/// moving with the station, the mouse rigs are pointed where the berth
/// points the hull, and the dock is asked for. The marker comes off once
/// the ship is docked. A ship that stands clear at no free port is logged
/// as an error and stays where it spawned, undocked: never berthed into a
/// hull.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn berth_at_start(world: &mut World) {
    let Some(start_id) = world
        .get_resource::<OpenWorldSession>()
        .map(|session| session.start.id.clone())
    else {
        return;
    };
    let ships: Vec<Entity> = world
        .query_filtered::<Entity, With<StartBerthMarker>>()
        .iter(world)
        .collect();
    if ships.is_empty() {
        return;
    }
    let Some(station) = world
        .query::<(Entity, &GeneratedBodyId)>()
        .iter(world)
        .find(|(_, id)| **id == start_id)
        .map(|(entity, _)| entity)
    else {
        return;
    };
    let stands = world.get::<Position>(station).is_some()
        && world.get::<PendingParkedHulls>(station).is_none()
        && world.get::<ParkedHulls>(station).is_none_or(|parked| {
            parked.iter().all(|hull| {
                matches!(
                    world.get::<ParkedHullType>(hull),
                    Some(ParkedHullType::Hire { .. })
                ) || world.get::<DockedShip>(hull).is_some()
            })
        });
    if !stands {
        return;
    }
    for ship in ships {
        if world.get::<DockedShip>(ship).is_some() {
            world.entity_mut(ship).remove::<StartBerthMarker>();
            continue;
        }
        let free = free_ports(world, station);
        let Some((at, turn)) = clear_berth(world, station, &free, ship, &start_id) else {
            error!(
                "berth_at_start: the player ship stands clear of starting station {start_id} \
                 and its parked hulls at no free port and roll; the ship stays undocked where \
                 it spawned"
            );
            world.entity_mut(ship).remove::<StartBerthMarker>();
            continue;
        };
        let position = world.get::<Position>(station).map_or(Vec3::ZERO, |p| p.0);
        let rotation = world
            .get::<Rotation>(station)
            .map_or(Quat::IDENTITY, |r| r.0);
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (at, turn) = (position + rotation * at, (rotation * turn).normalize());
        world.entity_mut(ship).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Left on the spawn heading, the rig turns the hull back through the
        // station the moment it undocks.
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        world.trigger(DockingConnectionRequest {
            entity: ship,
            target: station,
        });
    }
}

/// The free, live docking ports mounted under `root`, lowest first.
#[cfg(not(target_arch = "wasm32"))]
fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
    let ports: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .collect();
    let mut ports: Vec<Entity> = ports
        .into_iter()
        .filter(|port| {
            let mut at = *port;
            while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                if parent == root {
                    return true;
                }
                at = parent;
            }
            false
        })
        .collect();
    ports.sort();
    ports
}

/// The berth `ship` takes at `station`, as `(position, rotation)` in the
/// station root's frame: [`outward_berth`] at the station's `free` ports,
/// clear of its parked hulls ([`parked_envelopes`]), or `None` when it
/// stands clear at none.
///
/// The sale hulls parked round the berth the same rule picked at every
/// port with nothing parked, so the two agree. When they do not, a hull
/// took the reserved port or stands in the way, and the disagreement is
/// logged as an error: the ship still takes the berth it stands clear at,
/// never the reserved one.
///
/// # Panics
///
/// When the station or the ship has no design sections or no [`ShipSkin`],
/// or a free port has no design section: the section spawn gives each root
/// both and each port its [`EntityId`].
#[cfg(not(target_arch = "wasm32"))]
fn clear_berth(
    world: &World,
    station: Entity,
    free: &[Entity],
    ship: Entity,
    start_id: &GeneratedBodyId,
) -> Option<(Vec3, Quat)> {
    let design = |root: Entity| {
        let (Some(design), Some(&ShipSkin(skin))) = (
            world.get::<ShipDesignSections>(root),
            world.get::<ShipSkin>(root),
        ) else {
            panic!("berth_at_start: {root:?} has no design sections or skin");
        };
        (&design.0, skin)
    };
    let ((station_sections, station_skin), (ship_sections, ship_skin)) =
        (design(station), design(ship));
    let free: Vec<usize> = free
        .iter()
        .map(|&port| {
            world
                .get::<EntityId>(port)
                .and_then(|id| {
                    station_sections
                        .iter()
                        .position(|section| section.id == id.0)
                })
                .unwrap_or_else(|| panic!("berth_at_start: port {port:?} has no design section"))
        })
        .collect();
    let taken: Vec<usize> = (0..station_sections.len())
        .filter(|index| !free.contains(index))
        .collect();
    let reserved = outward_berth(
        station_sections,
        station_skin,
        ship_sections,
        ship_skin,
        &[],
        &[],
    );
    let berth = outward_berth(
        station_sections,
        station_skin,
        ship_sections,
        ship_skin,
        &taken,
        &parked_envelopes(world, station),
    );
    if let Some((reserved_port, reserved_own, reserved_at, reserved_turn)) = reserved {
        let same = berth.is_some_and(|(port, own, at, turn)| {
            port == reserved_port
                && own == reserved_own
                && at.abs_diff_eq(reserved_at, 1e-4)
                && turn.abs_diff_eq(reserved_turn, 1e-4)
        });
        if !same {
            error!(
                "berth_at_start: the player ship does not take the berth reserved for it at \
                 port '{}' of starting station {start_id}; it takes {}",
                station_sections[reserved_port].id,
                berth.map_or("none".to_string(), |(port, ..)| format!(
                    "port '{}'",
                    station_sections[port].id
                ))
            );
        }
    }
    berth.map(|(_, _, at, turn)| (at, turn))
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::StanceType;

    use super::*;
    use crate::{civilizations::CivilizationStatusType, tests::fixture_world};

    const SEED: u32 = 20_260_923;

    #[test]
    fn the_start_is_the_living_ally_station_nearest_the_origin() {
        let start = starting_station(SEED, &fixture_world()).expect("the fixture seed starts");
        let civilizations = CivilizationField::new(SEED);
        let span = -START_CIVILIZATION_SEARCH..=START_CIVILIZATION_SEARCH;
        let mut allies = 0;
        for x in span.clone() {
            for y in span.clone() {
                for z in span.clone() {
                    let civilization = civilizations.civilization([x, y, z]);
                    if civilization.status == CivilizationStatusType::Living
                        && civilization.stance == StanceType::Ally
                    {
                        allies += 1;
                    }
                }
            }
        }
        assert!(
            allies > 0,
            "the fixture seed has a living ally near the origin"
        );
        assert!(
            start.id.to_string().ends_with("_station_0"),
            "a station id, got {}",
            start.id
        );
        assert!(start.position.get().is_finite() && start.spawn.get().is_finite());
        let standoff = start.spawn.distance(start.position).get();
        assert!(
            standoff > 2.0 * CLEARANCE_MARGIN.get(),
            "the spawn stands past the station's clearance, got {standoff} m"
        );
        assert_eq!(
            starting_station(SEED, &fixture_world()).expect("the pick is seeded"),
            start,
            "the same seed picks the same station"
        );
    }

    /// The seed preview describes the 5 by 5 cells round the start's cell on
    /// its layer, lists the start's cell among its stations, and is seeded.
    #[test]
    fn the_seed_preview_describes_the_cells_round_the_start_and_places_its_station() {
        let preview = start_preview(SEED, &fixture_world()).expect("the fixture seed previews");
        let span = (2 * START_PREVIEW_REACH + 1) as usize;
        assert_eq!(preview.cells.len(), span * span);
        let centre = preview.cell();
        assert!(preview.cells.iter().all(|(coord, _)| {
            coord.y == centre.y
                && (coord.x - centre.x).abs() <= START_PREVIEW_REACH
                && (coord.z - centre.z).abs() <= START_PREVIEW_REACH
        }));
        assert!(
            preview
                .stations
                .iter()
                .any(|(cell, dead)| *cell == centre && !dead),
            "the start's cell places its living station, got {:?}",
            preview.stations
        );
        assert_eq!(
            start_preview(SEED, &fixture_world()).expect("the preview is seeded"),
            preview
        );
    }
}
