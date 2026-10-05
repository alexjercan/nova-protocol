//! The main menu's live backdrop: that entering the menu loads the ambience
//! scenario, that the camera activates on the backdrop's own scripted pose,
//! that the menu's rotation cuts to the next backdrop when one reports its act
//! finished, and that a missing or broken backdrop degrades to a bare camera
//! instead of failing.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_rand::prelude::WyRand;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;
use rand::SeedableRng as _;

use super::support::{
    app, dummy_backdrop, dummy_scenario, dummy_scenarios, observe_load_scenario,
    observe_unload_scenario, script_backdrop_pose, LoadedScenario, Unloaded, TEST_BACKDROP_ID,
    TEST_START_ID,
};
use crate::ambience::MenuBackdropRotation;

/// Entering MainMenu loads the ambience backdrop through the real OnEnter systems.
#[test]
fn entering_main_menu_loads_the_ambience_scenario() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    observe_load_scenario(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_BACKDROP_ID)
    );
    // The menu is a cinematic shot: entering drives the HUD level to None (the absorbed
    // status-bar hide).
    assert_eq!(
        *app.world().resource::<HudVisibility>(),
        HudVisibility::Cinematic
    );
}

/// The camera contract: each backdrop poses its OWN camera (a SetCamera in
/// its OnStart). The menu blanks + strips the loader's flyable camera and
/// only activates it once the backdrop's scripted pose is pinned - entry
/// never flashes the loader's default pose, and the menu never derives a
/// pose of its own.
#[test]
fn menu_camera_activates_on_the_backdrops_scripted_pose() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    let cam = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            WASDCameraController,
            Transform::from_xyz(0.0, 10.0, 20.0),
        ))
        .id();

    app.update();
    assert!(
        !app.world().get::<Camera>(cam).unwrap().is_active,
        "camera must be blanked while the controller is still attached"
    );
    app.update();
    assert!(
        app.world().get::<WASDCameraController>(cam).is_none(),
        "controller must be stripped"
    );
    assert!(
        !app.world().get::<Camera>(cam).unwrap().is_active,
        "no scripted pose yet - the camera stays blank, it never invents a pose"
    );

    script_backdrop_pose(&mut app, cam);
    app.update();
    assert!(
        app.world().get::<Camera>(cam).unwrap().is_active,
        "the backdrop's own pose is what turns the picture on"
    );
}

/// A MID-MENU backdrop cut (the menu loading its next backdrop) tears down
/// the posed camera and spawns a fresh flyable one, whose own SetCamera only lands a frame later. The
/// remembered pose bridges the gap: the fresh camera stays ACTIVE at the
/// last scripted pose instead of blinking through the loader's default.
#[test]
fn a_mid_menu_reload_holds_the_last_scripted_pose() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    // First load: classic blank-then-pose.
    let cam = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            WASDCameraController,
            Transform::from_xyz(0.0, 10.0, 20.0),
        ))
        .id();
    app.update();
    app.update();
    script_backdrop_pose(&mut app, cam);
    app.update();
    assert!(app.world().get::<Camera>(cam).unwrap().is_active);

    // The cut: scoped teardown takes the posed camera; the loader spawns
    // a fresh flyable one; its SetCamera has not landed yet.
    app.world_mut().entity_mut(cam).despawn();
    let fresh = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            WASDCameraController,
            Transform::from_xyz(0.0, 10.0, 20.0),
        ))
        .id();

    app.update();
    assert!(
        app.world().get::<Camera>(fresh).unwrap().is_active,
        "the reload camera must stay active on the remembered pose, not blank"
    );
    app.update();
    assert!(
        app.world().get::<WASDCameraController>(fresh).is_none(),
        "controller must still be stripped"
    );
    assert!(app.world().get::<Camera>(fresh).unwrap().is_active);
    let held = app.world().get::<Transform>(fresh).unwrap().translation;
    assert!(
        (held - Vec3::new(0.0, 90.0, 300.0)).length() < 1e-3,
        "the held pose is the last scripted pose, got {held:?}"
    );

    // The reloading backdrop's own SetCamera lands; the camera stays on.
    script_backdrop_pose(&mut app, fresh);
    app.update();
    assert!(app.world().get::<Camera>(fresh).unwrap().is_active);
}

/// The interface renders through the menu's OWN UI camera, spawned on menu
/// entry and independent of every scenario camera - a backdrop reload that
/// despawns the scenario camera can no longer yank the UI's render target
/// out from under the layout (the live BorderRadius::resolve crash on every
/// backdrop self-reset).
#[test]
fn the_menu_owns_a_ui_camera_independent_of_the_backdrop() {
    use bevy::ui::IsDefaultUiCamera;

    use crate::ambience::MenuUiCameraMarker;

    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    let mut q_ui_cam = app
        .world_mut()
        .query_filtered::<(&Camera, Option<&IsDefaultUiCamera>), With<MenuUiCameraMarker>>();
    let (camera, default_ui) = q_ui_cam
        .single(app.world())
        .expect("menu UI camera spawned");
    assert!(camera.is_active);
    assert!(
        default_ui.is_some(),
        "IsDefaultUiCamera is what routes every root Node to this camera"
    );

    // Leaving the menu removes it (DespawnOnExit), so gameplay HUD keeps
    // rendering through the gameplay camera.
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    app.update();
    let mut q_ui_cam = app
        .world_mut()
        .query_filtered::<Entity, With<MenuUiCameraMarker>>();
    assert!(q_ui_cam.iter(app.world()).next().is_none());
}

/// The backdrop draw stays inside the `menu_backdrop`-flagged set and,
/// over a seeded 8-entry rotation, reaches more than one backdrop -
/// the flag is a ROTATION, not a single hardcoded scene.
#[test]
fn menu_backdrop_pick_stays_flagged_and_rotates() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
        dummy_backdrop("backdrop_a"),
        dummy_backdrop("backdrop_b"),
    ])));
    observe_load_scenario(&mut app);
    app.update();

    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..8 {
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        let picked = app
            .world_mut()
            .resource_mut::<LoadedScenario>()
            .0
            .take()
            .expect("entering the menu loads a backdrop");
        assert!(
            picked == "backdrop_a" || picked == "backdrop_b",
            "the pick must be a flagged backdrop, got '{picked}'"
        );
        seen.insert(picked);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
    }
    assert_eq!(
        seen.len(),
        2,
        "a seeded 8-draw rotation reaches both backdrops"
    );
}

/// The runtime content gate on the menu side: a backdrop with Error-level issues is
/// filtered OUT of the draw (a refused menu load would leave no camera at all) - the
/// clean one is always picked; ALL broken degrades to the bare-camera path.
#[test]
fn broken_backdrops_are_skipped_in_the_draw() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_backdrop("backdrop_clean"),
        dummy_backdrop("backdrop_broken"),
    ])));
    let mut issues = ContentIssues::default();
    issues.0.insert(
        "backdrop_broken".to_string(),
        vec![LintIssue {
            severity: LintSeverity::Error,
            scenario: "backdrop_broken".to_string(),
            message: "unknown section prototype 'ghost'".to_string(),
        }],
    );
    app.insert_resource(issues);
    observe_load_scenario(&mut app);
    app.update();

    for _ in 0..6 {
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        let picked = app
            .world_mut()
            .resource_mut::<LoadedScenario>()
            .0
            .take()
            .expect("a clean backdrop still loads");
        assert_eq!(picked, "backdrop_clean", "the broken backdrop never draws");
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
    }
}

/// NOTHING flagged degrades to a bare camera (the UI must keep
/// rendering), never a panic - a mod set may deregister every backdrop.
#[test]
fn no_menu_backdrop_degrades_to_a_bare_camera() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
    ])));
    observe_load_scenario(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    assert_eq!(
        app.world().resource::<LoadedScenario>().0,
        None,
        "no backdrop scenario loads"
    );
    let cameras = app
        .world_mut()
        .query_filtered::<(), With<Camera3d>>()
        .iter(app.world())
        .count();
    assert_eq!(
        cameras, 1,
        "the fallback camera spawns so the menu UI still renders"
    );
}

/// The world as the unload found it: how many 3D cameras were standing when
/// the teardown landed. The fallback camera is spawned by the same system, so
/// a zero here is the ORDER - the teardown ran before the menu put a camera
/// of its own up, not after it.
#[derive(Resource, Default)]
struct UnloadWitness(Option<usize>);

fn witness_the_unload(app: &mut App) {
    app.init_resource::<UnloadWitness>();
    app.add_observer(
        |_: On<UnloadScenario>,
         q_cameras: Query<(), With<Camera3d>>,
         mut witness: ResMut<UnloadWitness>| {
            witness.0 = Some(q_cameras.iter().count());
        },
    );
}

fn enter_the_menu(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
}

fn cameras_3d(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<Camera3d>>()
        .iter(app.world())
        .count()
}

/// The fallback path is a TEARDOWN as much as the draw is: entering the menu
/// ends whatever was running before it goes looking for a backdrop, so a mod
/// set that leaves nothing to draw cannot leave gameplay simulating behind the
/// front door. The bare camera goes up after that, never over a live scene.
#[test]
fn the_fallback_path_ends_the_scenario_it_replaces() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
    ])));
    observe_load_scenario(&mut app);
    observe_unload_scenario(&mut app);
    witness_the_unload(&mut app);
    app.update();

    enter_the_menu(&mut app);

    assert!(
        app.world().resource::<Unloaded>().0,
        "a menu entry with nothing to draw must still end the scenario it replaces"
    );
    assert_eq!(
        app.world().resource::<UnloadWitness>().0,
        Some(0),
        "the teardown must land before the fallback camera, not after it"
    );
    assert_eq!(
        app.world().resource::<LoadedScenario>().0,
        None,
        "no backdrop scenario loads"
    );
    assert_eq!(cameras_3d(&mut app), 1, "exactly one fallback camera");
}

/// Same when every backdrop is REGISTERED but erroring: the draw filters them
/// out and reaches the same bare-camera branch, which owes the same teardown.
#[test]
fn a_menu_whose_every_backdrop_errors_ends_the_scenario_too() {
    let mut app = app();
    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_backdrop("backdrop_broken"),
    ])));
    let mut issues = ContentIssues::default();
    issues.0.insert(
        "backdrop_broken".to_string(),
        vec![LintIssue {
            severity: LintSeverity::Error,
            scenario: "backdrop_broken".to_string(),
            message: "unknown section prototype 'ghost'".to_string(),
        }],
    );
    app.insert_resource(issues);
    observe_load_scenario(&mut app);
    observe_unload_scenario(&mut app);
    witness_the_unload(&mut app);
    app.update();

    enter_the_menu(&mut app);

    assert!(
        app.world().resource::<Unloaded>().0,
        "a backdrop the gate refuses leaves the same debt as no backdrop at all"
    );
    assert_eq!(app.world().resource::<UnloadWitness>().0, Some(0));
    assert_eq!(app.world().resource::<LoadedScenario>().0, None);
    assert_eq!(cameras_3d(&mut app), 1, "exactly one fallback camera");
}

/// The teardown belongs to the menu ENTRY, not to the fallback branch: an
/// entry that does draw a backdrop ends the previous scenario first as well,
/// so the ordering is one rule rather than one branch's good luck.
#[test]
fn drawing_a_backdrop_ends_the_scenario_it_replaces_first() {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    observe_load_scenario(&mut app);
    observe_unload_scenario(&mut app);
    witness_the_unload(&mut app);
    app.update();

    enter_the_menu(&mut app);

    assert!(app.world().resource::<Unloaded>().0);
    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some(TEST_BACKDROP_ID),
        "the backdrop still draws"
    );
}

/// The rotation's order, as pure logic: a shuffled CYCLE in which every
/// eligible backdrop plays once before any plays again, and no backdrop plays
/// twice in a row - not inside a cycle, and not across the reshuffle between
/// two. Two backdrops are the tight case: the boundary rule alone forces them
/// to alternate.
#[test]
fn the_rotation_plays_each_backdrop_once_per_cycle_and_never_twice_in_a_row() {
    let three: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
    let mut rng = WyRand::from_seed(7u64.to_ne_bytes());
    let mut rotation = MenuBackdropRotation::default();
    let draws: Vec<String> = (0..30)
        .map(|_| rotation.draw(&three, &mut rng).expect("three are eligible"))
        .collect();
    for cycle in draws.chunks(3) {
        let mut cycle = cycle.to_vec();
        cycle.sort();
        assert_eq!(
            cycle, three,
            "each cycle plays each backdrop once: {draws:?}"
        );
    }
    assert!(
        draws.windows(2).all(|pair| pair[0] != pair[1]),
        "no backdrop plays twice in a row: {draws:?}"
    );

    let two: Vec<String> = ["a", "b"].map(String::from).to_vec();
    let mut rotation = MenuBackdropRotation::default();
    let draws: Vec<String> = (0..20)
        .map(|_| rotation.draw(&two, &mut rng).expect("two are eligible"))
        .collect();
    assert!(
        draws.windows(2).all(|pair| pair[0] != pair[1]),
        "two backdrops alternate: {draws:?}"
    );
}

/// The rotation's edges: nothing eligible draws nothing, one eligible
/// backdrop is drawn every time, and a bagged id that stopped being eligible
/// mid-cycle (its mod disabled, its content now erroring) is never drawn.
#[test]
fn the_rotation_degrades_with_zero_one_or_missing_backdrops() {
    let mut rng = WyRand::from_seed(7u64.to_ne_bytes());

    let mut rotation = MenuBackdropRotation::default();
    assert_eq!(rotation.draw(&[], &mut rng), None);

    let one = vec!["solo".to_string()];
    let mut rotation = MenuBackdropRotation::default();
    for _ in 0..3 {
        assert_eq!(rotation.draw(&one, &mut rng).as_deref(), Some("solo"));
    }

    let three: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
    let mut rotation = MenuBackdropRotation::default();
    let first = rotation.draw(&three, &mut rng).expect("three are eligible");
    // Withdraw one of the two still in the bag.
    let gone = three
        .iter()
        .find(|id| **id != first)
        .expect("two remain")
        .clone();
    let left: Vec<String> = three.iter().filter(|id| **id != gone).cloned().collect();
    for _ in 0..6 {
        let drawn = rotation.draw(&left, &mut rng).expect("two are eligible");
        assert_ne!(drawn, gone, "a withdrawn backdrop is never drawn");
    }
}

/// Every load the menu asked for, in order. Also stands in for the loader's
/// teardown, which clears the event world on every load and unload: that is
/// what retires a backdrop's done report once the cut lands.
#[derive(Resource, Default)]
struct Loads(Vec<String>);

/// A menu rig with a moving clock - 100 ms of real time per update through
/// `TimePlugin`, so a paused virtual clock really holds - entered into the
/// menu on `scenarios`.
fn rotation_app(scenarios: GameScenarios) -> App {
    let mut app = app();
    app.add_plugins(bevy::time::TimePlugin);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.insert_resource(scenarios);
    app.init_resource::<Loads>();
    app.add_observer(
        |load: On<LoadScenario>, mut loads: ResMut<Loads>, mut world: ResMut<NovaEventWorld>| {
            loads.0.push(load.0.id.clone());
            world.clear();
        },
    );
    app.add_observer(|_: On<UnloadScenario>, mut world: ResMut<NovaEventWorld>| {
        world.clear();
    });
    // The first update's clock delta is zero; burn it before the menu opens.
    app.update();
    enter_the_menu(&mut app);
    assert_eq!(
        app.world().resource::<Loads>().0.len(),
        1,
        "entry loads one"
    );
    app
}

fn report_done(app: &mut App) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .mark_backdrop_done();
}

fn loads(app: &App) -> Vec<String> {
    app.world().resource::<Loads>().0.clone()
}

fn run_frames(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

/// The hand-off: a backdrop that reports its act finished is cut to a
/// DIFFERENT backdrop after the one-second beat, and only once. A repeated
/// report inside the beat neither restarts nor doubles the cut, and once the
/// successor's load retires the report nothing cuts again.
#[test]
fn a_finished_backdrop_is_cut_once_to_the_next_after_its_beat() {
    let mut app = rotation_app(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_backdrop("backdrop_a"),
        dummy_backdrop("backdrop_b"),
    ])));

    report_done(&mut app);
    run_frames(&mut app, 5);
    report_done(&mut app);
    run_frames(&mut app, 4);
    assert_eq!(loads(&app).len(), 1, "0.9 s in, the beat still plays");

    app.update();
    let after_cut = loads(&app);
    assert_eq!(after_cut.len(), 2, "1.0 s in, the menu cuts: {after_cut:?}");
    assert_ne!(
        after_cut[0], after_cut[1],
        "the cut moves to another backdrop"
    );

    run_frames(&mut app, 30);
    assert_eq!(loads(&app), after_cut, "one report makes one cut");
}

/// The beat runs on VIRTUAL time: a paused clock holds the cut however much
/// real time passes, and resuming finishes it. Leaving the menu drops a
/// pending cut, so it can never load a backdrop into the game.
#[test]
fn a_pause_holds_the_cut_and_leaving_the_menu_drops_it() {
    let mut app = rotation_app(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
        dummy_backdrop("backdrop_a"),
        dummy_backdrop("backdrop_b"),
    ])));

    report_done(&mut app);
    run_frames(&mut app, 5);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    run_frames(&mut app, 30);
    assert_eq!(loads(&app).len(), 1, "3 s of paused real time hold the cut");
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    run_frames(&mut app, 5);
    assert_eq!(loads(&app).len(), 2, "the rest of the beat cuts on resume");

    report_done(&mut app);
    run_frames(&mut app, 5);
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    run_frames(&mut app, 30);
    assert_eq!(
        loads(&app).len(),
        2,
        "leaving the menu drops the pending cut"
    );
}

/// The cut's edges: with one backdrop the menu replays it, and with none
/// left the finished backdrop ends and the bare fallback camera goes up.
#[test]
fn the_cut_replays_a_lone_backdrop_and_falls_back_with_none() {
    let mut app = rotation_app(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
        dummy_backdrop("backdrop_solo"),
    ])));

    report_done(&mut app);
    run_frames(&mut app, 10);
    assert_eq!(loads(&app), ["backdrop_solo", "backdrop_solo"]);

    app.insert_resource(GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
    ])));
    report_done(&mut app);
    run_frames(&mut app, 10);
    assert_eq!(loads(&app).len(), 2, "nothing is left to load");
    assert!(
        !app.world().resource::<NovaEventWorld>().backdrop_done(),
        "the finished backdrop was ended"
    );
    let fallback = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|name| name.as_str() == "Menu Fallback Camera")
        .count();
    assert_eq!(fallback, 1, "the bare camera keeps the menu rendering");
}
