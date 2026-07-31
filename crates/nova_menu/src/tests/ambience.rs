use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::support::{
    app, dummy_backdrop, dummy_scenario, dummy_scenarios, observe_load_scenario,
    spawn_planetoid_well, LoadedScenario, TEST_BACKDROP_ID, TEST_START_ID,
};

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

/// Review R1.1: the camera staging that carried dev bug 1 (pose written in
/// the same frame as the controller removal gets overwritten by the
/// controller). Frame 1 must deactivate + strip the controller; frame 2
/// must stage the runtime-derived pose and reactivate.
#[test]
fn menu_camera_is_staged_from_the_wells_geometry() {
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
    spawn_planetoid_well(&mut app);

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
    // r_orbit = body_radius 80 + clearance 40 = 120 -> pose (0, 90, 300).
    let staged = app.world().get::<Transform>(cam).unwrap().translation;
    assert!(
        (staged - Vec3::new(0.0, 90.0, 300.0)).length() < 1e-3,
        "pose must derive from the well's runtime geometry, got {staged:?}"
    );
    assert!(app.world().get::<Camera>(cam).unwrap().is_active);
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
