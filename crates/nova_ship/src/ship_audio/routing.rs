//! Which ship a cue belongs to, and therefore which side of the hull the pilot
//! hears it from. The one place the game answers "is this mine or theirs?".

use bevy::prelude::*;
use nova_gameplay::prelude::*;

/// The [`SpaceshipRootMarker`] ancestor of `entity`, or `entity` itself when the
/// walk leaves the tree without finding one - torpedo thrusters hang off the
/// projectile rather than a ship root, and bare rigs have no parent at all, so
/// a rootless source is heard at its own pose.
pub(super) fn owning_root(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_root: &Query<(), With<SpaceshipRootMarker>>,
) -> Entity {
    let mut current = entity;
    loop {
        if q_is_root.contains(current) {
            return current;
        }
        match q_child_of.get(current) {
            Ok(&ChildOf(parent)) => current = parent,
            Err(_) => return entity,
        }
    }
}

/// [`AudioRoute::Hull`] when `root` is the player's own ship, otherwise
/// [`AudioRoute::Exterior`].
///
/// THE routing decision for every world cue in the game: your own ship is the
/// room you are sitting in, so its sounds reach you through the structure -
/// undimmed by how far the camera has pulled back, and with no bearing to pan
/// them to. Everything else is out there.
pub(super) fn route_from(
    root: Entity,
    q_is_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> AudioRoute {
    if q_is_player.contains(root) {
        AudioRoute::Hull
    } else {
        AudioRoute::Exterior
    }
}

/// [`owning_root`] then [`route_from`], for the fire sites that hold only the
/// section that made the noise.
pub(super) fn route_for(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_is_root: &Query<(), With<SpaceshipRootMarker>>,
    q_is_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> AudioRoute {
    route_from(owning_root(entity, q_child_of, q_is_root), q_is_player)
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;

    type RoutingQueries<'w, 's> = (
        Query<'w, 's, &'static ChildOf>,
        Query<'w, 's, (), With<SpaceshipRootMarker>>,
        Query<'w, 's, (), With<PlayerSpaceshipMarker>>,
    );

    #[test]
    fn a_section_of_the_players_ship_routes_to_the_hull() {
        let mut world = World::new();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let gun = world.spawn(ChildOf(player)).id();
        let mount = world.spawn(ChildOf(gun)).id();

        let mut state: SystemState<RoutingQueries> = SystemState::new(&mut world);
        let (child_of, is_root, is_player) = state.get(&world).unwrap();
        assert_eq!(
            route_for(mount, &child_of, &is_root, &is_player),
            AudioRoute::Hull,
            "the walk must climb past the mount to the ship root"
        );
    }

    #[test]
    fn another_ships_section_routes_to_the_exterior() {
        let mut world = World::new();
        let raider = world.spawn(SpaceshipRootMarker).id();
        let gun = world.spawn(ChildOf(raider)).id();

        let mut state: SystemState<RoutingQueries> = SystemState::new(&mut world);
        let (child_of, is_root, is_player) = state.get(&world).unwrap();
        assert_eq!(
            route_for(gun, &child_of, &is_root, &is_player),
            AudioRoute::Exterior
        );
    }

    #[test]
    fn something_with_no_ship_above_it_is_heard_at_its_own_pose() {
        // An asteroid, or a torpedo's own thruster: no ship root, so the walk
        // stops at the entity itself and the cue is exterior.
        let mut world = World::new();
        let rock = world.spawn_empty().id();
        let chunk = world.spawn(ChildOf(rock)).id();

        let mut state: SystemState<RoutingQueries> = SystemState::new(&mut world);
        let (child_of, is_root, is_player) = state.get(&world).unwrap();
        assert_eq!(owning_root(chunk, &child_of, &is_root), chunk);
        assert_eq!(
            route_for(chunk, &child_of, &is_root, &is_player),
            AudioRoute::Exterior
        );
    }
}
