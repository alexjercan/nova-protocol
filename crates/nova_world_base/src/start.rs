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

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_assets::prelude::LoadedSectionPacks;
use nova_events::prelude::{GeneratedBodyId, Meters3, SectorCoord};
use nova_gameplay::prelude::{
    GameItems, PointRotation, PointRotationOutput, SectionInactiveMarker, StanceType,
};
use nova_scenario::prelude::{
    OpenWorldSession, PendingParkedHulls, StartBerthMarker, StartRoleDesignId, StartingStation,
};
use nova_ship::prelude::{
    parked_hull_pose, DockedPort, DockedShip, DockingConnectionRequest, DockingSectionMarker,
    GameStyles, ParkedHullType, ParkedHulls,
};
use nova_world::prelude::{sector_id, SectorFault};

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
/// docked so the berth takes the port left free, the ship is put on the
/// station's free port that faces most squarely away from it, moving with
/// the station, the mouse rigs are pointed where the berth points the hull,
/// and the dock is asked for. The marker comes off once the ship is docked.
/// A station with no free port, or a ship with none, is logged as an error
/// and the ship stays where it spawned, undocked: the charter then asks for
/// an undock the player cannot give, which is the fault to read.
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
        let (Some(station_port), Some(own_port)) = (
            outward_port(world, station),
            free_ports(world, ship).into_iter().next(),
        ) else {
            error!(
                "berth_at_start: starting station {start_id} or the player ship has no free \
                 docking port; the ship stays undocked where it spawned"
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
        let (at, turn) = parked_hull_pose(
            (position, rotation),
            local_pose(world, station_port, station),
            local_pose(world, own_port, ship),
        );
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

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let Some(transform) = world.get::<Transform>(at) else {
            break;
        };
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        let Some(parent) = world.get::<ChildOf>(at) else {
            break;
        };
        at = parent.parent();
    }
    (position, rotation)
}

/// The free, live docking ports mounted under `root`, lowest first.
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

/// The free port of `station` that faces most squarely away from its root,
/// so the berth sets the ship outside the hull: the lowest port can face into
/// the station, where a berth crushes both hulls.
fn outward_port(world: &mut World, station: Entity) -> Option<Entity> {
    free_ports(world, station)
        .into_iter()
        .map(|port| {
            let (at, rotation) = local_pose(world, port, station);
            let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
            (port, facing)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(port, _)| port)
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
}
