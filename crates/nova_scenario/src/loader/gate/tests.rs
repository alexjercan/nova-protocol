use std::time::Duration;

use avian3d::{prelude::Physics, schedule::PhysicsTime};
use bevy::{ecs::system::RunSystemOnce, time::TimeUpdateStrategy};
use nova_events::prelude::*;
use nova_ship::prelude::*;

use super::*;
use crate::loader::{clock::tick_scenario_clock, fixtures::scenario_with};

/// One frame of real time. Short enough to sit under `Time<Virtual>`'s own
/// clamp, so a released frame advances by exactly this much.
const FRAME: Duration = Duration::from_millis(100);

/// A rig with the production gate wiring, the production settle gate on the
/// scenario clock, and both clocks the hold acts on.
fn loading_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Real time by the frame rather than by the wall clock: the hold is a
    // claim about what the world does while REAL time keeps running.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    app.init_resource::<Time<Physics>>();
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<ScenarioPreload>();
    app.init_resource::<ScenarioStartFailure>();
    app.insert_resource(CurrentScenario(Some(scenario_with("held", vec![]))));
    register_scenario_load_gate(&mut app);
    app.add_systems(Update, tick_scenario_clock.run_if(scenario_has_settled));
    app
}

/// The load announcement the gate listens for.
fn announce_load(app: &mut App) {
    app.world_mut().trigger(ScenarioLoaded {
        scenario_id: "held".to_string(),
        handler_count: 0,
        object_count: 1,
    });
}

/// Give the world something to spawn, so it reports as settling the way a
/// scenario whose objects are still landing does.
fn queue_a_spawn(app: &mut App) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .push_command(|commands| {
            commands.queue(|_: &mut World| {});
        });
}

/// Land everything the world has queued.
fn settle(app: &mut App) {
    <NovaEventWorld as EventWorld>::state_to_world_system(app.world_mut());
}

fn is_frozen(app: &App) -> bool {
    let held = app
        .world()
        .resource::<ClockFreeze>()
        .is_held_by(FreezeOwner::ScenarioLoad);
    held && app.world().resource::<Time<Virtual>>().is_paused()
        && app.world().resource::<Time<Physics>>().is_paused()
}

fn scenario_elapsed(app: &App) -> f64 {
    app.world().resource::<NovaEventWorld>().scenario_elapsed()
}

/// The whole contract in one pass: the hold is taken on the load, it survives
/// every frame the world is still building, and it is dropped on the frame the
/// world finishes.
#[test]
fn a_scenario_load_holds_the_world_until_it_is_built() {
    let mut app = loading_app();
    queue_a_spawn(&mut app);

    announce_load(&mut app);
    assert_eq!(
        *app.world().resource::<ScenarioLoadGate>(),
        ScenarioLoadGate::Loading
    );
    assert!(is_frozen(&app), "the load takes the hold as it starts");

    app.update();
    app.update();
    assert!(
        is_frozen(&app),
        "a world with spawns still landing stays held"
    );

    settle(&mut app);
    app.update();
    assert_eq!(
        *app.world().resource::<ScenarioLoadGate>(),
        ScenarioLoadGate::Idle
    );
    assert!(
        !app.world().resource::<ClockFreeze>().is_held(),
        "a built scenario gets its world back"
    );
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(!app.world().resource::<Time<Physics>>().is_paused());
}

/// What the hold is FOR. Real time runs through the whole load - the loading
/// panel animates on it - and the scenario clock does not move at all, so the
/// first frame the player is given is the scenario's own frame zero.
#[test]
fn the_first_playable_frame_is_scenario_time_zero() {
    let mut app = loading_app();
    queue_a_spawn(&mut app);
    announce_load(&mut app);

    for _ in 0..5 {
        app.update();
    }
    assert!(
        app.world().resource::<Time<Real>>().elapsed_secs_f64() >= 0.4,
        "real time must keep running for the loading panel"
    );
    assert_eq!(scenario_elapsed(&app), 0.0, "no mission time passes");

    // The frame that releases is still a frozen frame: its delta was taken
    // while the clocks were held.
    settle(&mut app);
    app.update();
    assert_eq!(
        scenario_elapsed(&app),
        0.0,
        "the release frame is the last frozen one"
    );

    app.update();
    assert!(
        (scenario_elapsed(&app) - FRAME.as_secs_f64()).abs() < 1e-6,
        "the first playable frame starts the clock at zero (got {})",
        scenario_elapsed(&app)
    );
}

/// A load that fails is over. The hold is never dropped, because the only
/// thing dropping it could offer is a half-built scene to fly around in.
#[test]
fn a_failed_load_never_gives_the_world_back() {
    let mut app = loading_app();
    announce_load(&mut app);

    app.world_mut()
        .run_system_once(
            |mut gate: ResMut<ScenarioLoadGate>, mut failure: ResMut<ScenarioStartFailure>| {
                fail_scenario_load(
                    &mut gate,
                    Some(&mut failure),
                    "Held".to_string(),
                    vec!["'art/hull.glb#Scene0' stopped loading".to_string()],
                );
            },
        )
        .expect("the failure path runs");

    // Everything the release reads is now satisfied: nothing is settling and
    // no art is pending. The gate still refuses.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<ScenarioLoadGate>(),
        ScenarioLoadGate::Failed
    );
    assert!(is_frozen(&app), "a failed load keeps the world still");
    assert_eq!(scenario_elapsed(&app), 0.0, "and never starts the mission");

    let report = app
        .world()
        .resource::<ScenarioStartFailure>()
        .0
        .clone()
        .expect("the failure is reported to FAILED TO START");
    assert_eq!(report.scenario_name, "Held");
    assert_eq!(report.messages.len(), 1);
}

/// Main Menu is the way out of a failed load, and it leaves through
/// [`UnloadScenario`]. The menu behind it has a backdrop that has to move.
#[test]
fn leaving_a_failed_scenario_starts_the_world_again() {
    let mut app = loading_app();
    announce_load(&mut app);
    app.world_mut()
        .run_system_once(|mut gate: ResMut<ScenarioLoadGate>| {
            fail_scenario_load(&mut gate, None, "Held".to_string(), vec![]);
        })
        .expect("the failure path runs");

    app.world_mut().trigger(UnloadScenario);

    assert_eq!(
        *app.world().resource::<ScenarioLoadGate>(),
        ScenarioLoadGate::Idle
    );
    assert!(
        !app.world().resource::<ClockFreeze>().is_held(),
        "the menu does not inherit the dead run's hold"
    );
}

/// Counted ticks per gated set: freezing the CLOCKS is not enough on its own,
/// because a camera rig and an input reader both run on real time.
#[derive(Resource, Default)]
struct Ticks {
    input: u32,
    input_fixed: u32,
    sections: u32,
    camera: u32,
    camera_sync: u32,
}

/// The production gating (`configure_scenario_gating`) with one probe in each
/// set it holds, and a scenario live so only the load gate can stop them.
fn probed_app() -> App {
    let mut app = App::new();
    app.insert_resource(CurrentScenario(Some(scenario_with("held", vec![]))));
    app.init_resource::<Ticks>();
    crate::loader::lifecycle::configure_scenario_gating(&mut app);
    app.add_systems(
        Update,
        (
            (|mut t: ResMut<Ticks>| t.input += 1).in_set(SpaceshipInputSystems),
            (|mut t: ResMut<Ticks>| t.sections += 1).in_set(SpaceshipSectionSystems),
            (|mut t: ResMut<Ticks>| t.camera += 1).in_set(NovaCameraSystems),
        ),
    );
    app.add_systems(
        FixedUpdate,
        (|mut t: ResMut<Ticks>| t.input_fixed += 1).in_set(SpaceshipInputSystems),
    );
    app.add_systems(
        PostUpdate,
        (|mut t: ResMut<Ticks>| t.camera_sync += 1).in_set(WASDCameraSystems::Sync),
    );
    app
}

fn ticks(app: &App) -> (u32, u32, u32, u32, u32) {
    let t = app.world().resource::<Ticks>();
    (t.input, t.input_fixed, t.sections, t.camera, t.camera_sync)
}

fn step(app: &mut App) {
    app.update();
    app.world_mut().run_schedule(FixedUpdate);
}

/// Nothing the player drives runs behind the loading panel - not the ship,
/// not its sections, and neither camera rig. The live phase in the middle is
/// the delivery guard: the same probes demonstrably CAN run in this app.
#[test]
fn nothing_the_player_drives_runs_while_a_load_is_held() {
    let mut app = probed_app();

    app.insert_resource(ScenarioLoadGate::Loading);
    step(&mut app);
    assert_eq!(ticks(&app), (0, 0, 0, 0, 0), "loading: everything held");

    app.insert_resource(ScenarioLoadGate::Failed);
    step(&mut app);
    assert_eq!(ticks(&app), (0, 0, 0, 0, 0), "failed: still held");

    app.insert_resource(ScenarioLoadGate::Idle);
    step(&mut app);
    assert_eq!(
        ticks(&app),
        (1, 1, 1, 1, 1),
        "built: the player has the helm"
    );
}
