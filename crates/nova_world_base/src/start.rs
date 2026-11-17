//! The start, and the station berth a fresh or rescued player ship takes.
//!
//! New Game picks the station once per seed ([`starting_station`]): the
//! living station nearest the origin whose civilization's seeded stance is
//! Ally, of the civilizations in the 3x3x3 lattice block round the origin.
//! A seed with no such station is refused at generation, before anything
//! streams; nothing falls back to a Neutral station or to open space. The
//! session and the world save carry the pick; `SpawnStartRole` spawns the
//! fresh player ship in the station's open space with a [`StationBerth`],
//! and [`berth_at_station`] puts it on the station's outward port and docks
//! it once the station stands. An answered rescue call berths the rescued
//! ship the same way (`crate::rescue`).

#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeSet;

#[cfg(not(target_arch = "wasm32"))]
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_assets::prelude::LoadedSectionPacks;
#[cfg(not(target_arch = "wasm32"))]
use nova_events::prelude::{EntityId, GeneratedBodyId};
use nova_events::prelude::{Meters3, SectorCoord};
use nova_gameplay::prelude::{
    CivilizationId, ClusterType, GameConsumables, GameItems, GameTradeProfiles, StanceType,
};
#[cfg(not(target_arch = "wasm32"))]
use nova_gameplay::prelude::{
    PointRotation, PointRotationOutput, SectionInactiveMarker, SectionMarker,
};
#[cfg(not(target_arch = "wasm32"))]
use nova_scenario::prelude::{
    parked_envelopes, PendingParkedHulls, StationBerth, StationBerthType,
};
use nova_scenario::prelude::{OpenWorldSession, StartRoleDesignId, StartingStation};
use nova_ship::prelude::GameStyles;
#[cfg(not(target_arch = "wasm32"))]
use nova_ship::prelude::{
    outward_berth, shift_chase_cameras, DesignSection, DockedPort, DockedShip,
    DockingConnectionRequest, DockingSectionMarker, ParkedHullType, ParkedHulls, RescueCallEnded,
    RescueEndType, ShipDesignSections, ShipSkin,
};
use nova_world::prelude::{
    sector_id, SectorFault, SectorGenerationInput, SectorGenerator, WorldGeometry,
};

use crate::{
    civilizations::{Civilization, CivilizationField, CivilizationStatusType},
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

/// Every station the civilizations of the 3x3x3 lattice block round lattice
/// node `centre` keep, of those `keep` accepts, each with its keeper. The
/// stations are the ones `plan_sector` places: each node is one its
/// civilization claims, with the pose the plan gives it. Each spawn point
/// stands past the station on its open side, two clearance margins clear of
/// it.
///
/// # Errors
///
/// Whatever the station claims or the clusters refuse for one of the kept
/// civilizations.
pub(crate) fn block_stations(
    seed: u32,
    generator: &NovaLayeredWorld,
    centre: [i32; 3],
    mut keep: impl FnMut(&Civilization) -> bool,
) -> Result<Vec<(StartingStation, CivilizationId)>, SectorFault> {
    let edge = OPEN_WORLD_SECTOR_EDGE;
    let fields = EnvironmentFields::new(seed);
    let civilizations = CivilizationField::new(seed);
    let mut claims = StationClaims::new(
        generator.parts(),
        generator.items(),
        generator.consumables(),
    );
    let mut hosts = |clearance, node| {
        Ok(cluster_at(&fields, seed, edge, node)?
            .is_some_and(|cluster| station_pose(&cluster, edge, clearance, seed).is_some()))
    };
    let mut stations = Vec::new();
    let span = -START_CIVILIZATION_SEARCH..=START_CIVILIZATION_SEARCH;
    for x in span.clone() {
        for y in span.clone() {
            for z in span.clone() {
                let civilization =
                    civilizations.civilization([centre[0] + x, centre[1] + y, centre[2] + z]);
                if !keep(&civilization) {
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
                    let standoff = (clearance + CLEARANCE_MARGIN + CLEARANCE_MARGIN).get();
                    let spawn = Meters3(pose.position.get() + pose.open * standoff);
                    stations.push((
                        StartingStation {
                            id,
                            position: pose.position,
                            spawn,
                        },
                        civilization.id,
                    ));
                }
            }
        }
    }
    Ok(stations)
}

/// The station of `stations` nearest `from`, ties by station id.
pub(crate) fn nearest_station(
    stations: impl IntoIterator<Item = (StartingStation, CivilizationId)>,
    from: Meters3,
) -> Option<(StartingStation, CivilizationId)> {
    stations.into_iter().min_by(|(a, _), (b, _)| {
        a.position
            .distance(from)
            .get()
            .total_cmp(&b.position.distance(from).get())
            .then_with(|| a.id.cmp(&b.id))
    })
}

/// The station the player starts docked at for `seed`: the station nearest
/// the origin among those the living Ally civilizations of the 3x3x3 lattice
/// block round the origin keep, ties by station id ([`block_stations`]).
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
    let stations = block_stations(seed, generator, [0, 0, 0], |civilization| {
        civilization.status == CivilizationStatusType::Living
            && civilization.stance == StanceType::Ally
    })?;
    nearest_station(stations, Meters3::ZERO)
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
    consumables: &GameConsumables,
    profiles: &GameTradeProfiles,
) -> Result<OpenWorldSession, String> {
    let generator = NovaLayeredWorld::from_loaded(loaded, styles, items, consumables, profiles)
        .map_err(|faults| {
            faults
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        })?;
    let start = starting_station(seed, &generator).map_err(|fault| fault.to_string())?;
    Ok(OpenWorldSession { seed, role, start })
}

/// Berth each [`StationBerth`] ship at its station and dock it.
///
/// Each frame the ship's station stands, with every hull it parks for sale
/// or owned docked round the berth the parking reserved for a start, the
/// ship is put on the berth [`outward_berth`] picks for its live sections at
/// the station's free ports, clear of the station and the parked hulls,
/// plating included ([`clear_berth`]), moving with the station, the mouse
/// rigs are pointed where the berth points the hull, and the dock is asked
/// for. The berth comes off once the ship is docked.
///
/// A start ship that stands clear at no free port is logged as an error and
/// stays where it spawned, undocked: never berthed into a hull. A rescued
/// ship gives up, undocked beside the station, when it stands clear at no
/// free port or is not docked within its frame budget, counted only in
/// frames the game clock runs: a full or refusing station is a runtime
/// condition, so it is a notice ([`RescueEndType::NoBerth`]), not an error.
/// The answer already took the bill and rebuilt the chain, so that ship is
/// fixed and docks by itself. A rescued ship jumps to its berth with the
/// chase camera ([`shift_chase_cameras`]).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn berth_at_station(world: &mut World) {
    let ships: Vec<(Entity, StationBerth)> = world
        .query::<(Entity, &StationBerth)>()
        .iter(world)
        .map(|(entity, berth)| (entity, berth.clone()))
        .collect();
    for (ship, berth) in ships {
        let rescue = matches!(berth.purpose, StationBerthType::Rescue { .. });
        if world.get::<DockedShip>(ship).is_some() {
            world.entity_mut(ship).remove::<StationBerth>();
            if rescue {
                info!(
                    "berth_at_station: the rescued ship docked at {}",
                    berth.station
                );
            }
            continue;
        }
        if let StationBerthType::Rescue { frames_left } = berth.purpose {
            if frames_left == 0 {
                give_up_rescue_berth(world, ship, &berth.station, "is not docked in time");
                continue;
            }
            // TAB and the pause menu stop the game clock; a frozen frame
            // spends none of the budget.
            if world.resource::<Time<Virtual>>().delta_secs() > 0.0 {
                world.entity_mut(ship).insert(StationBerth {
                    station: berth.station.clone(),
                    purpose: StationBerthType::Rescue {
                        frames_left: frames_left - 1,
                    },
                });
            }
        }
        let Some(station) = world
            .query::<(Entity, &GeneratedBodyId)>()
            .iter(world)
            .find(|(_, id)| **id == berth.station)
            .map(|(entity, _)| entity)
        else {
            continue;
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
            continue;
        }
        let free = free_ports(world, station);
        let Some((at, turn)) = clear_berth(world, station, &free, ship, &berth) else {
            if rescue {
                give_up_rescue_berth(world, ship, &berth.station, "stands clear at no free port");
            } else {
                error!(
                    "berth_at_station: the player ship stands clear of starting station {} \
                     and its parked hulls at no free port and roll; the ship stays undocked \
                     where it spawned",
                    berth.station
                );
                world.entity_mut(ship).remove::<StationBerth>();
            }
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
        if rescue {
            let from = world.get::<Position>(ship).map_or(at, |p| p.0);
            shift_chase_cameras(world, at - from);
        }
        world.entity_mut(ship).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Left on the spawn heading, the rig turns the hull back through the
        // station the moment it undocks.
        point_rigs(world, turn);
        world.trigger(DockingConnectionRequest {
            entity: ship,
            target: station,
        });
    }
}

/// Point every mouse flight rig at `turn`, so a hull put on that heading is
/// not turned back the moment it flies.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn point_rigs(world: &mut World, turn: Quat) {
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
}

/// End the berth of rescued `ship` at `station` undocked, as
/// [`RescueEndType::NoBerth`]: the ship stays where it is, beside the
/// station, and flies on.
#[cfg(not(target_arch = "wasm32"))]
fn give_up_rescue_berth(world: &mut World, ship: Entity, station: &GeneratedBodyId, why: &str) {
    warn!(
        "berth_at_station: the rescued ship {why} at station {station}; it stays undocked \
         beside the station, fixed, to dock by itself"
    );
    world.entity_mut(ship).remove::<StationBerth>();
    world.write_message(RescueCallEnded {
        ship,
        end: RescueEndType::NoBerth,
    });
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
    berth: &StationBerth,
) -> Option<(Vec3, Quat)> {
    let design = |root: Entity| {
        let (Some(design), Some(&ShipSkin(skin))) = (
            world.get::<ShipDesignSections>(root),
            world.get::<ShipSkin>(root),
        ) else {
            panic!("berth_at_station: {root:?} has no design sections or skin");
        };
        (&design.0, skin)
    };
    let ((station_sections, station_skin), (ship_design, ship_skin)) =
        (design(station), design(ship));
    // Only the ship's live sections: a lost port cannot dock, and a lost
    // hull plate takes no room.
    let live: BTreeSet<&str> = world
        .get::<Children>(ship)
        .into_iter()
        .flatten()
        .filter(|child| world.get::<SectionMarker>(**child).is_some())
        .filter_map(|child| world.get::<EntityId>(*child))
        .map(|id| id.0.as_str())
        .collect();
    let ship_sections: Vec<DesignSection> = ship_design
        .iter()
        .filter(|section| live.contains(section.id.as_str()))
        .cloned()
        .collect();
    let ship_sections = ship_sections.as_slice();
    // A port that holds a dock or does not work cannot take this one; it
    // still takes room.
    let ship_taken: Vec<usize> = world
        .get::<Children>(ship)
        .into_iter()
        .flatten()
        .filter(|child| {
            world.get::<DockingSectionMarker>(**child).is_some()
                && (world.get::<DockedPort>(**child).is_some()
                    || world.get::<SectionInactiveMarker>(**child).is_some())
        })
        .filter_map(|child| world.get::<EntityId>(*child))
        .filter_map(|id| ship_sections.iter().position(|section| section.id == id.0))
        .collect();
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
                .unwrap_or_else(|| panic!("berth_at_station: port {port:?} has no design section"))
        })
        .collect();
    let taken: Vec<usize> = (0..station_sections.len())
        .filter(|index| !free.contains(index))
        .collect();
    let placed = outward_berth(
        station_sections,
        station_skin,
        ship_sections,
        ship_skin,
        &taken,
        &ship_taken,
        &parked_envelopes(world, station),
    );
    // Only a start has a berth reserved: the station parked its sale hulls
    // round it before the ship came. A rescue takes what is free.
    let reserved = match berth.purpose {
        StationBerthType::Start => outward_berth(
            station_sections,
            station_skin,
            ship_sections,
            ship_skin,
            &[],
            &ship_taken,
            &[],
        ),
        StationBerthType::Rescue { .. } => None,
    };
    if let Some((reserved_port, reserved_own, reserved_at, reserved_turn)) = reserved {
        let same = placed.is_some_and(|(port, own, at, turn)| {
            port == reserved_port
                && own == reserved_own
                && at.abs_diff_eq(reserved_at, 1e-4)
                && turn.abs_diff_eq(reserved_turn, 1e-4)
        });
        if !same {
            error!(
                "berth_at_station: the player ship does not take the berth reserved for it at \
                 port '{}' of starting station {}; it takes {}",
                station_sections[reserved_port].id,
                berth.station,
                placed.map_or("none".to_string(), |(port, ..)| format!(
                    "port '{}'",
                    station_sections[port].id
                ))
            );
        }
    }
    placed.map(|(_, _, at, turn)| (at, turn))
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
