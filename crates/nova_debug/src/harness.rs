//! Nova adapter layer over the [`mod@nova_autopilot`] driver.
//!
//! The driver itself (the scripted autopilot) lives
//! in `nova_autopilot`, which depends on `bevy` alone and knows nothing about
//! Nova. Everything Nova-shaped they need is a caller hook, and this module is
//! what fills those hooks: the [`GameStates`] presets, the [`ScenarioLoaded`]
//! smoke assertion, camera posing, body freezing and overlay hiding. The
//! example fleet talks to this module, not to the crate.
//!
//! ## One capture idiom
//!
//! A capturing example is an ordinary autopilot script whose steps call
//! [`shoot`] from an `on_enter` hook - act, frame, shoot, in step order. The
//! same file is also the smoke path: [`shoot`] captures only when
//! [`capturing`] says the run is armed, so an unarmed run walks the identical
//! steps and writes nothing. [`force_capture_resolution`], [`hide_dev_overlays`],
//! [`hide_hud`] and [`freeze_bodies`] are the scene dressing every capturing
//! example needs; none of them is a driver.
//!
//! Every preset here is inert unless `NOVA_AUTOPILOT` is set, and every shot
//! is inert unless `NOVA_CAPTURE` is, so an example wires them permanently and
//! pays nothing in a normal run.
//!
//! ## Why the autopilot does not force `Playing`
//!
//! Nova's `Loading -> Playing` transition is *asset-gated*: the loader flips it
//! in `OnEnter(GameAssetsStates::Loaded)`, not on any input. If the autopilot
//! force-set `Playing` on its own timeline it would either fire before the
//! `GameAssets` resource exists (panicking scene setup that reads it) or re-enter
//! `Playing` after the loader already did (double-running `OnEnter(Playing)`
//! setup). So [`nova_autopilot`](nova_autopilot()) holds `Loading` on a single
//! generous step instead of forcing anything: the loader reaches `Playing` on its
//! own within that window, the run exercises gameplay (and any
//! [`input`](AutopilotPlugin::input) closure) there, and the autopilot reports
//! done when the step ends. The `nova harness: reached Playing` line (emitted by
//! [`DebugPlugin`](crate::DebugPlugin) under the autopilot) confirms the loader
//! got there before the exit, so a run that silently never leaves `Loading`
//! fails probe's `reached_playing` check instead of passing.
//!
//! ## Usage
//!
//! Add the preset under the `debug` feature (the harness lives there); it is a
//! no-op unless `NOVA_AUTOPILOT` is set, so leaving it in costs nothing:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use nova_debug::harness::nova_autopilot;
//! # fn add(app: &mut App) {
//! // No input needed -- just drive to Playing and exit without panic:
//! app.add_plugins(nova_autopilot());
//!
//! // Poke fire/thrust while in Playing (gate input to the gameplay state so it
//! // does not run during Loading):
//! app.add_plugins(nova_autopilot().input(|world, _elapsed| {
//!     use nova_gameplay::GameStates;
//!     if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
//!         return;
//!     }
//!     world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
//! }));
//! # }
//! ```
//!
//! Run it headless:
//!
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_scenario_grammar --features debug
//! # look for: `nova harness: reached Playing`
//! #           `autopilot: cycle complete, no panic`
//! ```

// Re-export the underlying drivers so examples can name/extend them (e.g. build
// a bespoke timeline) without reaching into nova_autopilot themselves.
use std::sync::Arc;

use avian3d::prelude::RigidBody;
use bevy::{prelude::*, window::PrimaryWindow};
use nova_autopilot::predicate::{any_entity, frames, resource_where, shot_written};
pub use nova_autopilot::{
    autopilot::{AutopilotLoop, AutopilotPlugin},
    capture::{
        capture_window, capturing, CaptureLog, CAPTURE_DIR_ENV, CAPTURE_ENV, CAPTURE_RESOLUTION,
    },
    // The self-ending examples (menu_scenarios) report the autopilot collector
    // done early rather than idling out the runway. They must reach the SAME
    // protocol instance the drivers register with, so it is re-exported here
    // alongside them.
    completion::{HarnessCompletion, AUTOPILOT},
    // The loop idiom rides the same route as the shot one: loop_start /
    // loop_end are the step calls, LoopCapturePlugin is the recorder an
    // example adds next to its autopilot, and the knobs are what the capture
    // script sizes its budget against.
    loops::{loop_end, loop_start, LoopCapturePlugin, LOOP_CRF, LOOP_FPS, LOOP_FRAME_CAP},
    // `not` reaches the examples HERE rather than through the prelude: the
    // name collides with `bevy::prelude::not`, which every example globs, so
    // it must stay a qualified call. The examples' package does not depend on
    // `nova_autopilot` directly (only on `nova_debug` and `nova_probe`), which
    // is why re-exporting it is the route rather than a third path dependency.
    predicate::{not, Predicate},
};
use nova_events::prelude::EntityId;
use nova_gameplay::{
    prelude::{PlayerSpaceshipMarker, SectionMarker, SpaceshipRootMarker},
    GameStates,
};
use nova_hud::prelude::HudVisibility;
use nova_scenario::prelude::{
    NovaEventWorld, ScenarioCameraMarker, ScenarioId, ScenarioLoaded, ScriptedCameraPose,
    VariableLiteral,
};
use nova_ship::prelude::WASDCameraController;

/// Seconds the [`nova_autopilot()`] preset holds `Loading` before exiting. Must
/// comfortably outlast asset loading (the loader drives `Loading -> Playing` on
/// its own) so the run spends real time in `Playing` before the clean exit.
pub const NOVA_AUTOPILOT_SECS: f32 = 6.0;

/// Frames a beat settles before its shot: long enough for the scene to come to
/// rest, the UI to lay out and the render to reach the frame the shot wants.
///
/// ONE value for every capture example and BOTH paths. The per-example splits
/// this replaced (90/6, 40/6, 30/2, 20/2) were not disagreements about
/// stillness - they were each absorbing the PNG write latency on top of it,
/// with a `capturing` branch to keep the smoke walk short. [`shoot`] acks now
/// ([`shot_written`](nova_autopilot::predicate::shot_written)), so what is left
/// is the stillness figure alone, and it is the same on both paths.
pub const SETTLE_FRAMES: u32 = 30;

/// In-step seconds a shot gets to land before the run ABORTS naming the step.
///
/// The ack turns a lost capture into a step that never advances, and a step
/// with no deadline hangs the whole run silently. Generous against a
/// software-rendered GPU reading back a 1920x1080 frame, far under the
/// completion deadline, and never reached by a capture that works.
pub const SHOT_DEADLINE_SECS: f32 = 20.0;

/// The name of the single step [`nova_autopilot()`] builds, so a caller can
/// [`loop_from`](AutopilotPlugin::loop_from) it without repeating the string.
pub const NOVA_AUTOPILOT_STEP: &str = "nova: play the loading-gated window";

/// The line a harnessed run logs once its app has reached gameplay state, and
/// the line probe's `reached_playing` check greps a run log for.
///
/// [`DebugPlugin`](crate::DebugPlugin) emits it for every app that carries the
/// nova debug layer; an app that does NOT carry it (the `widget_zoo` showcase
/// runs a bare `App` on the widget library alone) emits it itself on entering
/// its own `Playing`, and means by it "the library is up", not "gameplay".
/// Several callers across two crates are exactly why the string is a const: a
/// literal duplicated that widely drifts.
pub const REACHED_PLAYING: &str = "nova harness: reached Playing";

/// Env-gated autopilot preset for nova examples.
///
/// One step, holding `Loading` for [`NOVA_AUTOPILOT_SECS`] (the asset loader
/// reaches `Playing` within that window on its own -- see the module docs on
/// why this does not force the transition), after which the driver reports done
/// to the completion protocol. Chain [`input`](AutopilotPlugin::input) to poke
/// fire/thrust while in `Playing`. Inert unless `NOVA_AUTOPILOT` is set.
///
/// It is a WALL-CLOCK preset, which the step model makes the exception rather
/// than the rule: an example with something observable to wait on should build
/// its own beats on [`player_ship_present`] / [`scenario_variable_is`] instead.
pub fn nova_autopilot() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::new()
        .step(NOVA_AUTOPILOT_STEP)
        .enter(GameStates::Loading)
        .until(nova_autopilot::predicate::elapsed(NOVA_AUTOPILOT_SECS))
        .add()
}

/// Advance once the scenario's own [`NovaEventWorld`] holds `key` as the number
/// `value`.
///
/// The scenario's variables are what its event handlers actually wrote, so a
/// script that waits on one waits on the GAME agreeing the beat happened -
/// "the target is down" rather than "three seconds have passed". They are
/// latches (`0 -> 1`), which is why an exact comparison is the right one here.
///
/// False while the variable is absent or holds another type, so it doubles as
/// "wait for the scenario to seed its state" - the gate a reloading scene
/// needs, since the variables outlive the load that is replacing them.
pub fn scenario_variable_is(key: impl Into<String>, value: f64) -> Arc<Predicate> {
    let key = key.into();
    resource_where::<NovaEventWorld>(
        move |events| matches!(events.get_variable(&key), Some(VariableLiteral::Number(live)) if *live == value),
    )
}

/// Advance once no live section carries the scenario id `id`.
///
/// The end of the real disable -> destroy -> despawn pipeline: a script that
/// kills a section waits for THIS rather than for however long the pipeline
/// usually takes. Matches on [`SectionMarker`], so an editor preview of the
/// same id does not hold the step open.
pub fn section_gone(id: impl Into<String>) -> Arc<Predicate> {
    let id = id.into();
    Arc::new(move |world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SectionMarker>>()
            .is_none_or(|mut query| query.iter(world).all(|live| live.0 != id))
    })
}

/// Advance once the script has reported the autopilot collector done itself.
///
/// The advance condition of a SELF-ENDING step: an example whose closure walks
/// its own beat list and calls
/// [`HarnessCompletion::done`](HarnessCompletion::done) on its last one wants a
/// step that ends exactly there and nowhere else. Pair it with a
/// [`deadline`](nova_autopilot::autopilot::StepBuilder::deadline) sized as the
/// old runway, so a walk that never finishes error-exits naming the step
/// instead of idling out and passing.
///
/// Not in `nova_autopilot`'s own vocabulary on purpose: it reads a collector's
/// state to decide a step, which is a knot only this migration needs. A script
/// written fresh ends its last step on a world predicate and lets the driver
/// report done.
pub fn script_reports_done() -> Arc<Predicate> {
    resource_where::<HarnessCompletion>(|completion| !completion.is_pending(AUTOPILOT))
}

/// Advance once the player's ship root exists.
///
/// The scenario-is-live gate for a script that drives the player: the ship is
/// spawned by an `OnStart` handler after the asset load, so this is the honest
/// end of "loading", where a wall-clock runway is only a guess at it.
pub fn player_ship_present() -> Arc<Predicate> {
    any_entity::<(With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>()
}

/// The PNG a [`nova_screenshot`] beat writes. Relative, so it stages under
/// `NOVA_CAPTURE_DIR` like every other shot.
pub const NOVA_SCREENSHOT_PATH: &str = "screenshot.png";

/// Append the ONE capture beat to `script`: drop the dev overlays, settle
/// [`SETTLE_FRAMES`] frames, shoot [`NOVA_SCREENSHOT_PATH`], and hold until the
/// PNG is on disk.
///
/// The picture idiom for a range that already drives itself. A range takes its
/// own script, wraps it here, and adds ONE plugin:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use nova_debug::harness::{nova_autopilot, nova_screenshot};
/// # fn add(app: &mut App) {
/// app.add_plugins(nova_screenshot(nova_autopilot()));
/// # }
/// ```
///
/// It EXTENDS the run's one driver rather than adding a second: two
/// [`AutopilotPlugin`]s in an app is a duplicate-plugin panic, and two drivers
/// writing `NextState` was what the old stand-down rule existed to referee.
/// The shot itself is armed by `NOVA_CAPTURE` like every other shot in the
/// fleet - [`shoot`] is a logged no-op otherwise and [`shot_written`] already
/// holds, so an unarmed run walks the identical beats and writes nothing.
pub fn nova_screenshot(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
    script
        .step("nova: settle for the screenshot")
        .on_enter(hide_dev_overlays)
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("nova: shoot the screenshot")
        .on_enter(|world: &mut World| shoot(world, NOVA_SCREENSHOT_PATH))
        .until(shot_written(NOVA_SCREENSHOT_PATH))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

/// Smoke-test assertion preset: fail a headless run if scenario init is broken.
///
/// A scenario-loading example passes `autopilot: cycle complete, no panic` even
/// if the scenario silently came up empty. This preset closes that gap: it
/// observes the [`ScenarioLoaded`] init-status payload and panics (which fails
/// the `NOVA_AUTOPILOT` run with a non-zero exit) when init is trivial -- the
/// wrong scenario id, zero event handlers, or zero objects -- and, via a `fired`
/// flag checked on entering `Playing`, when the event never fires at all.
///
/// Add it under the `debug` feature next to [`nova_autopilot()`], passing the id
/// the example expects to load:
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use nova_debug::harness::assert_scenario_loaded;
/// # fn add(app: &mut App) {
/// app.add_plugins(assert_scenario_loaded("asteroid_field"));
/// # }
/// ```
///
/// The assertion is an invariant every scenario-loading example already holds,
/// so it is harmless (a single observer) in a normal `cargo run` too. It expects
/// exactly one scenario to load, which fits the examples that load once and do
/// not switch scenarios within the autopilot window.
pub fn assert_scenario_loaded(expected_id: impl Into<ScenarioId>) -> ScenarioLoadedAssertPlugin {
    ScenarioLoadedAssertPlugin {
        expected_id: expected_id.into(),
    }
}

/// Plugin returned by [`assert_scenario_loaded`]. Construct it through that
/// preset rather than directly.
pub struct ScenarioLoadedAssertPlugin {
    expected_id: ScenarioId,
}

impl Plugin for ScenarioLoadedAssertPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScenarioLoadAssertion {
            expected_id: self.expected_id.clone(),
            fired: false,
        });
        app.add_observer(assert_scenario_loaded_payload);
        app.add_systems(OnEnter(GameStates::Playing), assert_scenario_loaded_fired);
    }
}

/// Backs [`ScenarioLoadedAssertPlugin`]: the id the smoke run expects to load and
/// whether [`ScenarioLoaded`] has fired for it yet.
#[derive(Resource)]
struct ScenarioLoadAssertion {
    expected_id: ScenarioId,
    fired: bool,
}

/// Assert the [`ScenarioLoaded`] payload is non-trivial, right where the data is
/// known good. A panic here fails the smoke run, so a regression that loads the
/// wrong scenario or spawns nothing is caught instead of passing on
/// `autopilot: cycle complete` alone.
fn assert_scenario_loaded_payload(
    loaded: On<ScenarioLoaded>,
    mut assertion: ResMut<ScenarioLoadAssertion>,
) {
    info!(
        "smoke: ScenarioLoaded id={:?} handlers={} objects={}",
        loaded.scenario_id, loaded.handler_count, loaded.object_count
    );

    // NOTE: the smoke contract covers the FIRST load only - the app must boot
    // into the expected scenario with real content. LATER loads are
    // legitimate gameplay (completing an objective advances to the next
    // scenario, which may be an object-less epilogue) - an assertion that
    // stays armed panics the whole app on the player's first success
    // (playtest 2026-07-13: finishing asteroid_field crashed on the
    // asteroid_next transition).
    if assertion.fired {
        return;
    }

    assert_eq!(
        loaded.scenario_id, assertion.expected_id,
        "smoke: ScenarioLoaded reported scenario id {:?}, expected {:?}",
        loaded.scenario_id, assertion.expected_id
    );
    assert!(
        loaded.handler_count > 0,
        "smoke: ScenarioLoaded for {:?} reported zero event handlers -- scenario init registered no handlers",
        loaded.scenario_id
    );
    assert!(
        loaded.object_count > 0,
        "smoke: ScenarioLoaded for {:?} reported zero objects -- scenario init spawned nothing",
        loaded.scenario_id
    );

    assertion.fired = true;
}

/// By the time gameplay starts, the scenario must have loaded. If [`ScenarioLoaded`]
/// never fired, the payload assertion never ran, so guard the silent-empty case
/// here: reaching `Playing` with no load is itself a failure.
fn assert_scenario_loaded_fired(assertion: Res<ScenarioLoadAssertion>) {
    assert!(
        assertion.fired,
        "smoke: reached Playing but ScenarioLoaded for {:?} never fired -- scenario init silently failed",
        assertion.expected_id
    );
}

/// Capture the primary window to `path`, but only when this run is on the
/// capture path ([`capturing`]).
///
/// THE capture idiom: a script's shot step is `on_enter(|world| shoot(world,
/// "wiki-gravity.png"))`, so the same file drives both the capture run and the
/// smoke run - unarmed, this is a logged no-op and the walk carries on.
///
/// Asynchronous, like the [`capture_window`] it wraps, but it ACKS: the shot
/// step holds on `until(shot_written(path))` rather than on a guessed number of
/// frames, and that predicate is already satisfied on the smoke path.
pub fn shoot(world: &mut World, path: &str) {
    if !capturing() {
        return;
    }
    capture_window(world, path);
    info!("nova capture: {path}");
}

/// Advance once the scenario is live - its camera has spawned.
///
/// The scene-is-dressed gate for a capturing script: the scenario camera is
/// spawned by the loader after the asset load, and a framing step that poses
/// before it exists poses nothing (see [`pose_camera`]).
pub fn scenario_camera_present() -> Arc<Predicate> {
    any_entity::<With<ScenarioCameraMarker>>()
}

/// Force the primary window to [`CAPTURE_RESOLUTION`] and pin it there, so
/// every shot in the fleet lands at the same known 16:9 the web figures use.
/// A `Startup` system; non-resizable so a tiling WM cannot reflow it mid-run.
pub fn force_capture_resolution(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window
            .resolution
            .set(CAPTURE_RESOLUTION.0, CAPTURE_RESOLUTION.1);
        window.resizable = false;
    }
}

/// Freeze the scene so every shot is a deterministic still: make every dynamic
/// body static. Scenario props are dynamic rigidbodies, so a spawn impulse or an
/// idle thruster would drift them between shots (in zero-g nothing damps the
/// motion) and a later framing would catch empty space. Pinning them static
/// holds every position for the whole run while leaving visuals intact - the
/// photo-mode "freeze the scene" behaviour. Idempotent (only rewrites dynamic
/// bodies), so it costs nothing once the scene has settled.
///
/// An `Update` system a capturing example adds behind its own [`capturing`]
/// check, so a normal run keeps its physics.
pub fn freeze_bodies(mut commands: Commands, bodies: Query<(Entity, &RigidBody)>) {
    for (entity, body) in &bodies {
        // NOTE: RigidBody is an immutable component, so swap it via a command
        // insert.
        if matches!(body, RigidBody::Dynamic) {
            commands.entity(entity).insert(RigidBody::Static);
        }
    }
}

/// Disable the dev overlays so a captured frame is clean game render: nova's
/// gizmos ([`crate::DebugEnabled`], the gravity/section overlays) and the
/// [`crate::inspector`] diagnostics panel and [`crate::wireframe`] pass (each a
/// separate `DebugEnabled`). All default on under `DebugPlugin`. This leaves the
/// HUD alone, so a capture example that wants the HUD in shot (the 3-tier HUD
/// showcase) can keep it - add [`hide_dev_overlays`] at `Startup` and manage
/// [`HudVisibility`] per beat.
///
/// Exclusive rather than a parameterised system so ONE function satisfies both
/// the `Startup` registration the screenshot examples use and the drivers'
/// `Fn(&mut World)` `hide_overlay` hook.
pub fn hide_dev_overlays(world: &mut World) {
    if let Some(mut debug) = world.get_resource_mut::<crate::DebugEnabled>() {
        debug.0 = false;
    }
    if let Some(mut debug) = world.get_resource_mut::<crate::inspector::DebugEnabled>() {
        debug.0 = false;
    }
    if let Some(mut debug) = world.get_resource_mut::<crate::wireframe::DebugEnabled>() {
        debug.0 = false;
    }
}

/// Drop the HUD to [`HudVisibility::Cinematic`] so the fps/version bar is out
/// of shot. Kept OUT of [`hide_dev_overlays`] so a HUD-showcase capture can
/// hide the dev chrome and keep the HUD up; a scene shot calls both.
///
/// Called right before a shot rather than once at `Startup` in the examples
/// that enter the editor or a new scenario, which re-raise the HUD.
pub fn hide_hud(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
    }
}

/// Pose the scenario camera (the [`ScenarioCameraMarker`] entity) at `position`
/// looking at `look_at` by pinning a [`ScriptedCameraPose`] on it (and dropping
/// [`WASDCameraController`] so free-fly input stops). The loader's enforcer
/// applies the pose after the WASD sync every frame, so it holds. The world-level
/// twin of the `SetCamera` scenario action, and the framing half of every
/// capture step. A no-op with a warning when no scenario camera is present yet -
/// gate the script on [`scenario_camera_present`] to make that a stall the
/// harness names rather than an unframed shot.
pub fn pose_camera(world: &mut World, position: Vec3, look_at: Vec3) {
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    let Some(camera) = camera else {
        warn!("pose_camera: no scenario camera present yet");
        return;
    };
    if let Ok(mut entity) = world.get_entity_mut(camera) {
        entity.remove::<WASDCameraController>();
        entity.insert(ScriptedCameraPose { position, look_at });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smoke assertion covers the FIRST load only: a later legitimate
    /// scenario transition (completing an objective -> an object-less
    /// epilogue scenario) must NOT panic the app (playtest 2026-07-13:
    /// finishing asteroid_field crashed on the asteroid_next load).
    #[test]
    fn a_second_scenario_load_is_not_the_smoke_contract() {
        let mut world = World::new();
        world.insert_resource(ScenarioLoadAssertion {
            expected_id: "asteroid_field".to_string(),
            fired: false,
        });
        world.add_observer(assert_scenario_loaded_payload);

        // The boot load: matches the contract, arms `fired`.
        world.trigger(ScenarioLoaded {
            scenario_id: "asteroid_field".to_string(),
            handler_count: 5,
            object_count: 12,
        });
        assert!(world.resource::<ScenarioLoadAssertion>().fired);

        // The gameplay transition: different id, ZERO objects - both would
        // have tripped the old always-armed asserts. Must be a no-op.
        world.trigger(ScenarioLoaded {
            scenario_id: "asteroid_next".to_string(),
            handler_count: 1,
            object_count: 0,
        });
    }

    /// `pose_camera` moves the scenario camera onto the scripted pose and
    /// drops WASD control so the free-fly controller cannot overwrite it.
    #[test]
    fn pose_camera_pins_a_pose_and_drops_wasd() {
        let mut world = World::new();
        let camera = world
            .spawn((
                ScenarioCameraMarker,
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();

        pose_camera(&mut world, Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO);

        let pose = world
            .get::<ScriptedCameraPose>(camera)
            .expect("the camera is pinned to a scripted pose");
        assert_eq!(pose.position, Vec3::new(3.0, 4.0, 5.0));
        assert_eq!(pose.look_at, Vec3::ZERO);
        assert!(
            world.get::<WASDCameraController>(camera).is_none(),
            "WASD control is dropped so free-fly input stops"
        );
    }

    /// `pose_camera` with no scenario camera is a warn-and-continue no-op.
    #[test]
    fn pose_camera_without_a_camera_is_harmless() {
        let mut world = World::new();
        let bystander = world.spawn(Transform::default()).id();
        pose_camera(&mut world, Vec3::ONE, Vec3::ZERO);
        assert!(world.get_entity(bystander).is_ok());
    }

    /// The scene-is-dressed gate is exactly "the scenario camera spawned":
    /// false on a world without one, true once the marker exists. A capturing
    /// script holds its first framing step on it, because `pose_camera` before
    /// that point poses nothing.
    #[test]
    fn scenario_camera_present_gates_on_the_marker() {
        let mut world = World::new();
        world.spawn(Transform::default());
        let present = scenario_camera_present();
        assert!(!present(&world));

        world.spawn((ScenarioCameraMarker, Transform::default()));
        assert!(present(&world));
    }

    /// `shoot` on an UNARMED run captures nothing: no `Screenshot` request is
    /// spawned, so the smoke path of a capturing example walks the identical
    /// steps and writes no PNG. (The armed branch spawns a bare `Screenshot`
    /// that only a real render app resolves, so it is proved by the capture
    /// run in the task's manual proof.)
    #[test]
    fn shoot_captures_nothing_when_the_run_is_not_armed() {
        // The test binary never sets `NOVA_CAPTURE`, and `capturing()` reads
        // the process env - so this is the unarmed branch by construction.
        assert!(!capturing(), "the test binary must not arm {CAPTURE_ENV}");

        let mut world = World::new();
        shoot(&mut world, "never-written.png");

        let requests = world
            .query::<&bevy::render::view::screenshot::Screenshot>()
            .iter(&world)
            .count();
        assert_eq!(requests, 0, "an unarmed shoot spawns no capture request");
    }
}

/// The `nova_autopilot` script preset and the `nova_screenshot` capture beat,
/// the `shoot` idiom and its scene dressing, and the Nova-typed predicates the
/// beats compose from.
pub mod prelude {
    pub use super::{
        assert_scenario_loaded, force_capture_resolution, freeze_bodies, hide_dev_overlays,
        hide_hud, nova_autopilot, nova_screenshot, player_ship_present, pose_camera,
        scenario_camera_present, scenario_variable_is, script_reports_done, section_gone, shoot,
        ScenarioLoadedAssertPlugin, NOVA_AUTOPILOT_SECS, NOVA_AUTOPILOT_STEP, NOVA_SCREENSHOT_PATH,
        REACHED_PLAYING, SETTLE_FRAMES, SHOT_DEADLINE_SECS,
    };
}
