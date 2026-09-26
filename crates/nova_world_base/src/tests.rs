use std::collections::BTreeMap;

use bevy::ecs::system::RunSystemOnce;
use nova_gameplay::prelude::Fnv64;
use nova_scenario::prelude::ScenarioConfig;

use super::*;

const SEED: u32 = 20_260_923;

fn scenario(role: ScenarioRole) -> CurrentScenario {
    CurrentScenario(Some(ScenarioConfig {
        role,
        ..ScenarioConfig::new("under_test", "Under Test", "sky.png".into())
    }))
}

/// A world holding a live scenario of `role` and the New Game session.
fn world(role: ScenarioRole) -> World {
    let mut world = World::new();
    world.insert_resource(scenario(role));
    world.insert_resource(OpenWorldSession { seed: SEED });
    world
}

fn sync(world: &mut World) {
    world
        .run_system_once(sync_open_world)
        .expect("sync_open_world runs");
}

fn armed(world: &World) -> Option<&WorldConfig<NovaLayeredWorld>> {
    world.get_resource::<WorldConfig<NovaLayeredWorld>>()
}

fn session_config() -> WorldConfig<NovaLayeredWorld> {
    WorldConfig {
        seed: SEED,
        sector_edge: OPEN_WORLD_SECTOR_EDGE,
        active_radius: OPEN_WORLD_ACTIVE_RADIUS,
        generator: NovaLayeredWorld,
    }
}

/// The promise a world seed makes: the whole live window around the start
/// describes the same pristine sectors whichever order it is walked in.
#[test]
fn the_open_world_describes_the_same_sectors_in_any_visit_order() {
    let config = session_config();
    let cells: Vec<SectorCoord> = desired_sectors(SectorCoord::ORIGIN, config.active_radius)
        .into_iter()
        .collect();
    let describe_all = |order: &[SectorCoord]| {
        order
            .iter()
            .map(|coord| {
                let description = generate_sector(&config, *coord)
                    .unwrap_or_else(|fault| panic!("sector {coord:?}: {fault}"));
                (*coord, description.canonical())
            })
            .collect::<BTreeMap<_, _>>()
    };

    let forward = describe_all(&cells);
    let reverse = describe_all(&cells.iter().rev().copied().collect::<Vec<_>>());
    let strided = describe_all(
        &(0..cells.len())
            .map(|step| cells[step * 7 % cells.len()])
            .collect::<Vec<_>>(),
    );

    assert_eq!(forward.len(), 125, "radius 2 keeps a 5x5x5 window live");
    assert_eq!(forward, reverse, "a reverse walk changed a sector");
    assert_eq!(forward, strided, "a strided walk changed a sector");
}

/// A pinned window generates the bodies it was recorded with: every rock,
/// planetoid and ship, to the canonical text, in the window the examples fly.
///
/// The digest is FNV-1a 64 over the concatenated canonical descriptions of
/// the 125 cells around the origin, walked in `desired_sectors` order, for
/// seed 20,260,922 at a 32 km edge. Every input is written out here rather
/// than read from the session constants, so retuning New Game cannot move the
/// recorded window. A deliberate change to the generator's output changes
/// this digest; record the new value from the left side of this test's
/// failure message.
#[test]
fn a_pinned_window_generates_the_recorded_bodies() {
    let config = WorldConfig {
        seed: 20_260_922,
        sector_edge: Meters(32_000.0),
        active_radius: 2,
        generator: NovaLayeredWorld,
    };
    let canonical: String = desired_sectors(SectorCoord::ORIGIN, config.active_radius)
        .into_iter()
        .map(|coord| {
            generate_sector(&config, coord)
                .unwrap_or_else(|fault| panic!("sector {coord:?}: {fault}"))
                .canonical()
        })
        .collect();
    assert_eq!(
        Fnv64::new().write(canonical.as_bytes()).finish(),
        0xb333_e632_077c_a8bc,
        "the pinned window's bodies changed"
    );
}

#[test]
fn an_open_world_scenario_with_one_player_arms_the_world_around_it() {
    let mut world = world(ScenarioRole::OpenWorld);
    let player = world.spawn(PlayerSpaceshipMarker).id();

    sync(&mut world);

    assert_eq!(armed(&world), Some(&session_config()));
    assert!(world.entity(player).contains::<WorldObserver>());
}

/// `nova_world` clears every sector when the config changes, so a frame that
/// re-inserted an identical config would tear the world down every frame.
#[test]
fn an_armed_world_is_not_rewritten_by_the_next_sync() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    sync(&mut world);
    world.clear_trackers();

    sync(&mut world);

    assert!(!world.is_resource_changed::<WorldConfig<NovaLayeredWorld>>());
}

#[test]
fn a_scenario_of_another_role_or_none_disarms_the_world() {
    for current in [
        scenario(ScenarioRole::Backdrop),
        scenario(ScenarioRole::Chapter),
        CurrentScenario(None),
    ] {
        let mut world = world(ScenarioRole::OpenWorld);
        world.spawn(PlayerSpaceshipMarker);
        sync(&mut world);
        world.insert_resource(current.clone());

        sync(&mut world);

        assert!(
            armed(&world).is_none(),
            "{:?} left the world armed",
            current.0.map(|s| s.role)
        );
    }
}

/// A player ship that is still spawning, or already destroyed, leaves no
/// observer: the world stands down instead of tripping `nova_world`'s
/// one-observer rule.
#[test]
fn an_open_world_with_no_player_ship_disarms_the_world() {
    let mut world = world(ScenarioRole::OpenWorld);
    let player = world.spawn(PlayerSpaceshipMarker).id();
    sync(&mut world);
    world.despawn(player);

    sync(&mut world);

    assert!(armed(&world).is_none());
}

#[test]
#[should_panic(expected = "no OpenWorldSession")]
fn an_open_world_scenario_with_no_session_is_refused() {
    let mut world = World::new();
    world.insert_resource(scenario(ScenarioRole::OpenWorld));
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}

#[test]
#[should_panic(expected = "holds 2 player ships")]
fn an_open_world_with_two_player_ships_is_refused() {
    let mut world = world(ScenarioRole::OpenWorld);
    world.spawn(PlayerSpaceshipMarker);
    world.spawn(PlayerSpaceshipMarker);

    sync(&mut world);
}
