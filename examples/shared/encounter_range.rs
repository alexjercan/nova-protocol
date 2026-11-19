//! What the encounter examples share: find open space, put the player's
//! hull in it and dock with a body an encounter put in the world.
//!
//! Include with `#[path = "../shared/encounter_range.rs"] mod encounter_range;`.
//! Every function reads or writes the world the shipped app runs; none adds
//! a system.

use std::collections::BTreeSet;

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use nova_protocol::prelude::*;
use nova_world::prelude::{desired_sectors, PendingSectorStation, SectorRoot, WorldConfig};

/// How far the open-space point stands from where the player was.
const OPEN_SPACE_RUN: Meters = Meters(14_000.0);
/// How far ahead of the open-space point the way must stay clear: past the
/// nearest place the director puts an encounter, 6 km out.
const OPEN_SPACE_AHEAD: Meters = Meters(8_000.0);
/// How far every station stands from the open-space point and from the way
/// ahead of it: the director keeps 10 km.
const STATION_CLEAR: Meters = Meters(12_000.0);
/// How far every standing body stands from the open-space point.
const BODY_CLEAR: Meters = Meters(1_500.0);

/// The one player ship, if exactly one stands.
pub fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing the generated id `id`, if one stands.
pub fn body_wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .try_query::<(Entity, &GeneratedBodyId)>()?
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// Where `entity` stands.
fn place_of(world: &World, entity: Entity) -> Option<Meters3> {
    world
        .get::<Position>(entity)
        .map(|position| Meters3::from_engine(position.0))
}

/// Whether the world is armed around one player ship and the live cells
/// are exactly the window around the player's cell.
pub fn window_streamed(world: &World) -> bool {
    let Some(config) = world.get_resource::<WorldConfig<NovaLayeredWorld>>() else {
        return false;
    };
    let Some(at) = the_player(world).and_then(|player| place_of(world, player)) else {
        return false;
    };
    let centre = SectorCoord::containing(at, config.sector_edge);
    let live: BTreeSet<SectorCoord> = world
        .try_query::<&SectorRoot>()
        .map(|mut roots| roots.iter(world).map(|root| root.0).collect())
        .unwrap_or_default();
    live == desired_sectors(centre, config.active_radius)
}

/// A point in open space near the player and the way to fly on from it:
/// [`OPEN_SPACE_RUN`] out along a world axis, in a live cell, with every
/// station [`STATION_CLEAR`] off the point and off the way ahead and every
/// standing body [`BODY_CLEAR`] off the point. `None` when no axis is clear.
pub fn open_space(world: &World) -> Option<(Meters3, Vec3)> {
    let config = world.get_resource::<WorldConfig<NovaLayeredWorld>>()?;
    let from = place_of(world, the_player(world)?)?;
    let live: BTreeSet<SectorCoord> = world
        .try_query::<&SectorRoot>()?
        .iter(world)
        .map(|root| root.0)
        .collect();
    let mut stations: Vec<Meters3> = world
        .try_query_filtered::<&Position, With<StationMarker>>()?
        .iter(world)
        .map(|position| Meters3::from_engine(position.0))
        .collect();
    if let Some(mut held) = world.try_query::<&PendingSectorStation>() {
        stations.extend(held.iter(world).map(|held| held.station().position));
    }
    let bodies: Vec<Meters3> = world
        .try_query_filtered::<&Position, With<GeneratedBodyId>>()?
        .iter(world)
        .map(|position| Meters3::from_engine(position.0))
        .collect();
    [
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Z,
        Vec3::NEG_Z,
        Vec3::Y,
        Vec3::NEG_Y,
    ]
    .into_iter()
    .find_map(|heading| {
        let at = Meters3(from.get() + heading * OPEN_SPACE_RUN.get());
        let ahead = Meters3(at.get() + heading * OPEN_SPACE_AHEAD.get());
        let clear = [at, ahead].iter().all(|point| {
            live.contains(&SectorCoord::containing(*point, config.sector_edge))
                && stations
                    .iter()
                    .all(|station| station.distance(*point).get() > STATION_CLEAR.get())
        }) && bodies
            .iter()
            .all(|body| body.distance(at).get() > BODY_CLEAR.get());
        clear.then_some((at, heading))
    })
}

/// Put the player's hull at `at`, nose along `heading`, flying that way at
/// `speed` with no spin.
pub fn park_player(world: &mut World, at: Meters3, heading: Vec3, speed: MetersPerSecond) {
    let player = the_player(world).expect("encounter range: exactly one player ship");
    world.entity_mut(player).insert((
        Position(at.to_engine()),
        Rotation(Quat::from_rotation_arc(Vec3::NEG_Z, heading)),
        LinearVelocity(MetersPerSecond3(heading * speed.0).to_engine()),
        AngularVelocity::ZERO,
    ));
}

/// Put the player's hull `distance` from `body`, on the side the player is
/// on now, at rest with its nose at the body.
pub fn park_off(world: &mut World, body: Entity, distance: Meters) {
    let player = the_player(world).expect("encounter range: exactly one player ship");
    let at = place_of(world, body).expect("encounter range: the body stands");
    let own = place_of(world, player).expect("encounter range: the player stands");
    let away = (own.get() - at.get()).normalize_or(Vec3::X);
    park_player(
        world,
        Meters3(at.get() + away * distance.get()),
        -away,
        MetersPerSecond(0.0),
    );
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("encounter range: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("encounter range: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// The free, live docking ports mounted on `root`, in `Entity` order, each
/// with how squarely it faces away from the root.
fn free_ports(world: &mut World, root: Entity) -> Vec<(Entity, f32)> {
    let ports: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .collect();
    let mut ports: Vec<(Entity, f32)> = ports
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
        .map(|port| {
            let (at, rotation) = local_pose(world, port, root);
            (port, (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero()))
        })
        .collect();
    ports.sort_by_key(|(port, _)| *port);
    ports
}

/// Put the player on the berth at `target`'s most outward free collar and
/// ask to dock. A walk calls it every frame until the dock takes.
pub fn berth_and_dock(world: &mut World, target: Entity) {
    let player = the_player(world).expect("encounter range: exactly one player ship");
    if world.get::<DockedShip>(player).is_some() {
        return;
    }
    let target_port = free_ports(world, target)
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(port, _)| port)
        .unwrap_or_else(|| panic!("encounter range: {target:?} has no free docking port"));
    let player_port = free_ports(world, player)
        .first()
        .map(|(port, _)| *port)
        .expect("encounter range: the player has a free docking port");
    let pose = |entity: Entity| {
        (
            world.get::<Position>(entity).expect("a hull stands").0,
            world.get::<Rotation>(entity).expect("a hull is turned").0,
        )
    };
    let (position, rotation) = parked_hull_pose(
        pose(target),
        local_pose(world, target_port, target),
        local_pose(world, player_port, player),
    );
    let velocity = world
        .get::<LinearVelocity>(target)
        .map_or(Vec3::ZERO, |velocity| velocity.0);
    let spin = world
        .get::<AngularVelocity>(target)
        .map_or(Vec3::ZERO, |spin| spin.0);
    world.entity_mut(player).insert((
        Position(position),
        Rotation(rotation),
        LinearVelocity(velocity),
        AngularVelocity(spin),
    ));
    world.trigger(DockingConnectionRequest {
        entity: player,
        target,
    });
}
