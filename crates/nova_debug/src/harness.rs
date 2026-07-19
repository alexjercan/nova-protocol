//! Headless smoke-test harness for nova examples.
//!
//! Thin nova-specific presets over the `bevy_common_systems` env-gated harness
//! plugins ([`AutopilotPlugin`] / [`ScreenshotPlugin`]), pinned to nova's
//! [`GameStates`] lifecycle. Both are inert unless their env var is set
//! (`BCS_AUTOPILOT` / `BCS_SHOT`), so an example adds them permanently and pays
//! nothing in a normal run.
//!
//! ## Why the autopilot does not force `Playing`
//!
//! Nova's `Loading -> Playing` transition is *asset-gated*: the loader flips it
//! in `OnEnter(GameAssetsStates::Loaded)`, not on any input. If the autopilot
//! force-set `Playing` on its own timeline it would either fire before the
//! `GameAssets` resource exists (panicking scene setup that reads it) or re-enter
//! `Playing` after the loader already did (double-running `OnEnter(Playing)`
//! setup). So [`nova_autopilot`] holds `Loading` on a single generous step
//! instead of forcing anything: the loader reaches `Playing` on its own within
//! that window, the run exercises gameplay (and any
//! [`input`](AutopilotPlugin::input) closure) there, and the autopilot exits
//! cleanly with `AppExit::Success` when the step ends. The
//! `nova harness: reached Playing` line (emitted by [`DebugPlugin`](crate::DebugPlugin)
//! under the autopilot) confirms the loader got there before the exit, so a run
//! that silently never leaves `Loading` fails the smoke test instead of passing.
//!
//! ## Usage
//!
//! Add the preset under the `debug` feature (the harness lives there); it is a
//! no-op unless `BCS_AUTOPILOT` is set, so leaving it in costs nothing:
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
//! BCS_AUTOPILOT=1 cargo run --example scenario --features debug
//! # look for: `nova harness: reached Playing`
//! #           `autopilot: cycle complete, no panic`
//! ```

// Re-export the underlying harness plugins so examples can name/extend them
// (e.g. build a bespoke timeline) without reaching into bevy_common_systems.
use avian3d::prelude::RigidBody;
use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured},
};
pub use bevy_common_systems::debug::harness::prelude::{
    AutopilotLoop, AutopilotPlugin, ScreenshotPlugin,
};
use bevy_common_systems::prelude::WASDCameraController;
use nova_gameplay::{prelude::HudVisibility, GameStates};
use nova_scenario::prelude::{
    ScenarioCameraMarker, ScenarioId, ScenarioLoaded, ScriptedCameraPose,
};

/// Seconds the [`nova_autopilot`] preset holds `Loading` before exiting. Must
/// comfortably outlast asset loading (the loader drives `Loading -> Playing` on
/// its own) so the run spends real time in `Playing` before the clean exit.
pub const NOVA_AUTOPILOT_SECS: f32 = 6.0;

/// Settle frames the [`nova_screenshot`] preset waits after `Playing` is
/// reached, so the scene and UI have a few frames to render before the capture.
pub const NOVA_SCREENSHOT_SETTLE_FRAMES: u32 = 30;

/// Env-gated autopilot preset for nova examples.
///
/// Holds `Loading` for [`NOVA_AUTOPILOT_SECS`] (the asset loader reaches
/// `Playing` within that window on its own -- see the module docs on why this
/// does not force the transition), then exits with `AppExit::Success`. Chain
/// [`input`](AutopilotPlugin::input) to poke fire/thrust while in `Playing`.
/// Inert unless `BCS_AUTOPILOT` is set.
pub fn nova_autopilot() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::new().hold(GameStates::Loading, NOVA_AUTOPILOT_SECS)
}

/// Env-gated screenshot preset for nova examples: advance to `Playing`, settle
/// [`NOVA_SCREENSHOT_SETTLE_FRAMES`] frames, capture the primary window to a
/// PNG, and exit. Inert unless `BCS_SHOT` is set (a `WxH` value also overrides
/// the window resolution). See [`ScreenshotPlugin`].
///
/// Unlike [`nova_autopilot`], this force-advances to `Playing` on the first
/// frame, so it is best used with examples that set their scene up in
/// `OnEnter(GameAssetsStates::Loaded)` (the nova scenario convention, e.g.
/// `scenario`) rather than `OnEnter(GameStates::Playing)`, which the early
/// forced transition would run before `GameAssets` is ready.
pub fn nova_screenshot() -> ScreenshotPlugin<GameStates> {
    ScreenshotPlugin::new(GameStates::Playing).settle_frames(NOVA_SCREENSHOT_SETTLE_FRAMES)
}

/// Smoke-test assertion preset: fail a headless run if scenario init is broken.
///
/// A scenario-loading example passes `autopilot: cycle complete, no panic` even
/// if the scenario silently came up empty. This preset closes that gap: it
/// observes the [`ScenarioLoaded`] init-status payload and panics (which fails
/// the `BCS_AUTOPILOT` run with a non-zero exit) when init is trivial -- the
/// wrong scenario id, zero event handlers, or zero objects -- and, via a `fired`
/// flag checked on entering `Playing`, when the event never fires at all.
///
/// Add it under the `debug` feature next to [`nova_autopilot`], passing the id
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

    // The smoke contract covers the FIRST load only: the app must boot
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

/// Environment variable that arms [`ScreenshotReelPlugin`]. Distinct from
/// `BCS_SHOT` (the single-shot [`ScreenshotPlugin`]) so a reel run and a
/// one-off capture never fight over the window/exit.
pub const SCREENSHOT_REEL_ENV: &str = "BCS_REEL";

/// A camera pose for a reel beat: where the camera sits and what it looks at
/// (up is +Y), the same framing the `SetCamera` scenario action takes.
#[derive(Clone, Copy, Debug)]
pub struct ReelCamera {
    /// World-space camera position.
    pub position: Vec3,
    /// World-space point the camera looks at.
    pub look_at: Vec3,
}

impl ReelCamera {
    /// Construct a pose from a position looking at a target.
    pub fn new(position: Vec3, look_at: Vec3) -> Self {
        Self { position, look_at }
    }
}

/// One beat of a screenshot reel: optionally re-pose the scenario camera, wait
/// `settle_frames` for the scene to render, then capture the primary window to
/// `path`. A `None` camera keeps the previous beat's framing (e.g. two shots of
/// the same view). Relative `path`s resolve under `NOVA_SHOT_DIR`, matching the
/// `Screenshot` scenario action, so a whole reel stages into one folder.
#[derive(Clone, Debug)]
pub struct ReelBeat {
    /// Camera framing for this beat, or `None` to keep the current pose.
    pub camera: Option<ReelCamera>,
    /// Frames to let the scene render after posing before capturing.
    pub settle_frames: u32,
    /// Output PNG path (relative paths resolve under `NOVA_SHOT_DIR`).
    pub path: String,
}

impl ReelBeat {
    /// A beat that poses the camera, settles [`NOVA_SCREENSHOT_SETTLE_FRAMES`]
    /// frames, and captures to `path`.
    pub fn new(camera: ReelCamera, path: impl Into<String>) -> Self {
        Self {
            camera: Some(camera),
            settle_frames: NOVA_SCREENSHOT_SETTLE_FRAMES,
            path: path.into(),
        }
    }

    /// Override the settle-frame count for this beat.
    pub fn settle_frames(mut self, frames: u32) -> Self {
        self.settle_frames = frames;
        self
    }
}

/// Env-gated reel-capture preset for nova examples: once a scenario is live
/// (its camera exists), step an ordered list of [`ReelBeat`]s - pose the camera,
/// settle, capture a PNG - then exit with `AppExit::Success`. Inert unless
/// `BCS_REEL` is set.
///
/// This is the multi-shot sibling of [`nova_screenshot`] (one shot, then exit):
/// the reel drives the *cadence* and camera framing for the pure-3D showcase
/// beats, while the scene itself comes from a loaded scenario (e.g. the embedded
/// reel scenario in `screenshot_reel`). Captures are serialized - each beat waits for its PNG
/// to land before advancing - so a shot is never taken mid-camera-move.
///
/// UI/state-dependent shots (menu, editor, HUD, combat) are NOT expressible as a
/// `ReelBeat` (they need button clicks / state changes); those are driven by the
/// example's own autopilot script, reusing [`reel_pose_camera`] and the same
/// capture primitive.
pub struct ScreenshotReelPlugin {
    beats: Vec<ReelBeat>,
    resolution: (f32, f32),
}

impl ScreenshotReelPlugin {
    /// Build a reel from an ordered list of beats, capturing at the default
    /// [`REEL_CAPTURE_RESOLUTION`] (1920x1080, the 16:9 the web figures want).
    pub fn new(beats: Vec<ReelBeat>) -> Self {
        Self {
            beats,
            resolution: REEL_CAPTURE_RESOLUTION,
        }
    }

    /// Override the capture resolution (the primary window is forced to this
    /// size at startup so every beat is captured at a known aspect).
    pub fn resolution(mut self, width: f32, height: f32) -> Self {
        self.resolution = (width, height);
        self
    }
}

/// Default reel capture resolution: 1920x1080, the 16:9 the web site's figures
/// and thumbnails use (thumbnails share this capture and the site sizes them
/// down with CSS at ~300px wide).
pub const REEL_CAPTURE_RESOLUTION: (f32, f32) = (1920.0, 1080.0);

/// The forced capture resolution; drives the startup window resize.
#[derive(Resource)]
struct ReelWindowSize(f32, f32);

/// Internal driver state; kept out of the prelude per the crate conventions.
#[derive(Resource)]
struct ReelState {
    beats: Vec<ReelBeat>,
    index: usize,
    settled: u32,
    posed: bool,
    capturing: bool,
    done: bool,
}

/// Flipped by the per-capture `ScreenshotCaptured` observer so the driver knows
/// the PNG has landed and it is safe to advance (or exit).
#[derive(Resource, Default)]
struct ReelCaptureDone(bool);

impl Plugin for ScreenshotReelPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var(SCREENSHOT_REEL_ENV).is_err() {
            return;
        }
        if self.beats.is_empty() {
            warn!("ScreenshotReelPlugin: {SCREENSHOT_REEL_ENV} set but no beats; doing nothing");
            return;
        }
        debug!(
            "ScreenshotReelPlugin: build ({SCREENSHOT_REEL_ENV} active, {} beats)",
            self.beats.len()
        );
        app.insert_resource(ReelState {
            beats: self.beats.clone(),
            index: 0,
            settled: 0,
            posed: false,
            capturing: false,
            done: false,
        });
        app.insert_resource(ReelWindowSize(self.resolution.0, self.resolution.1));
        app.init_resource::<ReelCaptureDone>();
        app.add_systems(
            Startup,
            (reel_resize_window, hide_dev_overlays, reel_hide_hud),
        );
        app.add_systems(Update, (reel_freeze_bodies, reel_drive));
    }
}

/// Freeze the scene so every beat is a deterministic still: make every dynamic
/// body static. Scenario props are dynamic rigidbodies, so a spawn impulse or an
/// idle thruster would drift them across the reel (in zero-g nothing damps the
/// motion) and a later beat would frame empty space. Pinning them static holds
/// every position for the whole reel while leaving visuals intact - the
/// photo-mode "freeze the scene" behaviour. Idempotent (only rewrites dynamic
/// bodies), so it costs nothing once the scene has settled.
fn reel_freeze_bodies(mut commands: Commands, bodies: Query<(Entity, &RigidBody)>) {
    for (entity, body) in &bodies {
        // RigidBody is an immutable component, so swap it via a command insert.
        if matches!(body, RigidBody::Dynamic) {
            commands.entity(entity).insert(RigidBody::Static);
        }
    }
}

/// Disable the dev overlays so a captured frame is clean game render: nova's
/// gizmos ([`crate::DebugEnabled`], the gravity/section overlays) and the
/// `bevy_common_systems` inspector diagnostics panel and wireframe pass (each a
/// separate `DebugEnabled`). All default on under `DebugPlugin`. This leaves the
/// HUD alone, so a capture example that wants the HUD in shot (the 3-tier HUD
/// showcase) can keep it - add [`hide_dev_overlays`] at `Startup` and manage
/// [`HudVisibility`] per beat.
pub fn hide_dev_overlays(
    nova: Option<ResMut<crate::DebugEnabled>>,
    inspector: Option<ResMut<bevy_common_systems::debug::inspector::DebugEnabled>>,
    wireframe: Option<ResMut<bevy_common_systems::debug::wireframe::DebugEnabled>>,
) {
    if let Some(mut debug) = nova {
        debug.0 = false;
    }
    if let Some(mut debug) = inspector {
        debug.0 = false;
    }
    if let Some(mut debug) = wireframe {
        debug.0 = false;
    }
}

/// Reel-only: also hide the HUD chrome (the reel scenes carry no player HUD, so
/// the fps/version bar is just clutter). Kept out of [`hide_dev_overlays`] so a
/// HUD-showcase capture can keep the HUD up.
fn reel_hide_hud(hud: Option<ResMut<HudVisibility>>) {
    if let Some(mut hud) = hud {
        *hud = HudVisibility::None;
    }
}

/// Force the primary window to the reel's capture resolution at startup, so
/// every beat lands at a known aspect (mirrors the single-shot harness's
/// resize). Non-resizable so a tiling WM cannot reflow it mid-reel.
fn reel_resize_window(
    size: Res<ReelWindowSize>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    if let Ok(mut window) = windows.single_mut() {
        window.resolution.set(size.0, size.1);
        window.resizable = false;
    }
}

/// Pose the scenario camera (the [`ScenarioCameraMarker`] entity) at `position`
/// looking at `look_at` by pinning a [`ScriptedCameraPose`] on it (and dropping
/// [`WASDCameraController`] so free-fly input stops). The loader's enforcer
/// applies the pose after the WASD sync every frame, so it holds. The world-level
/// twin of the `SetCamera` scenario action, for examples that script beats from
/// their own autopilot closure (the UI/combat shots). A no-op with a warning
/// when no scenario camera is present yet.
pub fn reel_pose_camera(world: &mut World, position: Vec3, look_at: Vec3) {
    let camera = {
        let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
        query.iter(world).next()
    };
    let Some(camera) = camera else {
        warn!("reel_pose_camera: no scenario camera present yet");
        return;
    };
    if let Ok(mut entity) = world.get_entity_mut(camera) {
        entity.remove::<WASDCameraController>();
        entity.insert(ScriptedCameraPose { position, look_at });
    }
}

/// Capture the primary window to `path` (relative paths resolve under
/// `NOVA_SHOT_DIR`), creating the parent directory if needed. For capture
/// examples that drive their own beat script from the autopilot closure (the
/// UI/menu/editor/combat shots) rather than the camera-posed [`ReelBeat`] list.
/// Built on Bevy's `Screenshot::primary_window()` + `save_to_disk`.
pub fn capture_window(world: &mut World, path: &str) {
    let resolved = reel_capture_path(path);
    if let Some(parent) = resolved.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                warn!(
                    "capture_window: could not create capture dir {:?}: {error}",
                    parent
                );
            }
        }
    }
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(resolved));
}

/// Resolve a reel output path under `NOVA_SHOT_DIR` (relative paths only),
/// matching the `Screenshot` scenario action so a reel and hand-authored shots
/// stage into the same folder.
fn reel_capture_path(path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    match std::env::var("NOVA_SHOT_DIR") {
        Ok(dir) if !dir.is_empty() && !path.is_absolute() => std::path::Path::new(&dir).join(path),
        _ => path.to_path_buf(),
    }
}

/// Step the reel: pose -> settle -> capture -> (await the PNG) -> advance ->
/// exit. Exclusive because posing the camera and spawning the capture need
/// `&mut World`; runs only once a scenario camera exists (scenario loaded).
fn reel_drive(world: &mut World) {
    if world.resource::<ReelState>().done {
        return;
    }
    // Wait until the scenario is live (its camera spawned) before the first beat.
    let has_camera = world
        .query_filtered::<(), With<ScenarioCameraMarker>>()
        .iter(world)
        .next()
        .is_some();
    if !has_camera {
        return;
    }

    world.resource_scope(|world, mut state: Mut<ReelState>| {
        let index = state.index;
        let beat = state.beats[index].clone();

        // Waiting on the previous capture to land before advancing.
        if state.capturing {
            if !world.resource::<ReelCaptureDone>().0 {
                return;
            }
            world.resource_mut::<ReelCaptureDone>().0 = false;
            state.capturing = false;
            state.posed = false;
            state.settled = 0;
            state.index += 1;
            if state.index >= state.beats.len() {
                info!("reel: {} beats captured, exiting", state.beats.len());
                world.write_message(AppExit::Success);
                state.done = true;
            }
            return;
        }

        // Pose the camera on beat entry, then give the transform a frame to apply.
        if !state.posed {
            if let Some(camera) = beat.camera {
                reel_pose_camera(world, camera.position, camera.look_at);
            }
            state.posed = true;
            state.settled = 0;
            return;
        }

        // Let the scene render before capturing.
        state.settled += 1;
        if state.settled < beat.settle_frames {
            return;
        }

        let path = reel_capture_path(&beat.path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    warn!("reel: could not create capture dir {:?}: {error}", parent);
                }
            }
        }
        info!(
            "reel: beat {}/{} capturing -> {}",
            index + 1,
            state.beats.len(),
            path.display()
        );
        world
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path))
            .observe(
                |_: On<ScreenshotCaptured>, mut done: ResMut<ReelCaptureDone>| {
                    done.0 = true;
                },
            );
        state.capturing = true;
    });
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

    /// `reel_pose_camera` moves the scenario camera onto the scripted pose and
    /// drops WASD control so the free-fly controller cannot overwrite it.
    #[test]
    fn reel_pose_camera_pins_a_pose_and_drops_wasd() {
        let mut world = World::new();
        let camera = world
            .spawn((
                ScenarioCameraMarker,
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();

        reel_pose_camera(&mut world, Vec3::new(3.0, 4.0, 5.0), Vec3::ZERO);

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

    /// `reel_pose_camera` with no scenario camera is a warn-and-continue no-op.
    #[test]
    fn reel_pose_camera_without_a_camera_is_harmless() {
        let mut world = World::new();
        let bystander = world.spawn(Transform::default()).id();
        reel_pose_camera(&mut world, Vec3::ONE, Vec3::ZERO);
        assert!(world.get_entity(bystander).is_ok());
    }

    /// A relative reel path joins under `NOVA_SHOT_DIR`-less default (cwd) and an
    /// absolute path passes through. The env-driven join is exercised by the
    /// scenario action's own test; here we pin the no-env and absolute cases so
    /// the helper does not accidentally rewrite them.
    #[test]
    fn reel_capture_path_leaves_bare_and_absolute_paths_alone() {
        use std::path::Path;

        // No NOVA_SHOT_DIR in the test env: a relative path is used as-is.
        if std::env::var("NOVA_SHOT_DIR").is_err() {
            assert_eq!(
                reel_capture_path("feature-gravity.png"),
                Path::new("feature-gravity.png")
            );
        }
        // Absolute paths pass through regardless of the env.
        assert_eq!(reel_capture_path("/shots/a.png"), Path::new("/shots/a.png"));
    }
}
