//! The main menu's live backdrop: that entering the menu loads the ambience
//! scenario, that the camera activates on the backdrop's own scripted pose,
//! and that a missing or broken backdrop degrades to a bare camera instead
//! of failing.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use super::support::{
    app, dummy_backdrop, dummy_scenario, dummy_scenarios, observe_load_scenario,
    script_backdrop_pose, LoadedScenario, TEST_BACKDROP_ID, TEST_START_ID,
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

/// A MID-MENU backdrop reload (the self-resetting backdrops fire
/// NextScenario at their own id) tears down the posed camera and spawns a
/// fresh flyable one, whose own SetCamera only lands a frame later. The
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

    // The reload: scoped teardown takes the posed camera; the loader spawns
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
