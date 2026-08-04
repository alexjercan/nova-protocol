//! Nova adapter layer over the [`nova_autopilot`] drivers.
//!
//! The drivers themselves (scripted autopilot, settled-frame screenshot,
//! screenshot reel) live in `nova_autopilot`, which depends on `bevy` alone and
//! knows nothing about Nova. Everything Nova-shaped they need is a caller hook,
//! and this module is what fills those hooks: the [`GameStates`] presets, the
//! [`ScenarioLoaded`] smoke assertion, camera posing, body freezing and overlay
//! hiding. The example fleet talks to this module, not to the crate.
//!
//! Both presets are inert unless their env var is set (`NOVA_AUTOPILOT` /
//! `NOVA_SHOT`), so an example adds them permanently and pays nothing in a
//! normal run.
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
//! fails the smoke test instead of passing.
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
//! NOVA_AUTOPILOT=1 cargo run --example scenario_grammar --features debug
//! # look for: `nova harness: reached Playing`
//! #           `autopilot: cycle complete, no panic`
//! ```

// Re-export the underlying drivers so examples can name/extend them (e.g. build
// a bespoke timeline) without reaching into nova_autopilot themselves.
use std::sync::Arc;

use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use bevy_common_systems::prelude::WASDCameraController;
pub use nova_autopilot::{
    autopilot::{AutopilotLoop, AutopilotPlugin},
    // The self-ending examples (broadside, lifeline, menu_scenarios,
    // screenshot_nova_os) report the autopilot collector done early rather than
    // idling out the runway. They must reach the SAME protocol instance the
    // drivers register with, so it is re-exported here alongside them.
    completion::{HarnessCompletion, AUTOPILOT},
    predicate::Predicate,
    reel::{capture_window, ReelBeat, REEL_ENV},
    screenshot::ScreenshotPlugin,
};
use nova_autopilot::{
    predicate::{any_entity, resource_where},
    reel::ScreenshotReelPlugin,
};
use nova_events::prelude::EntityId;
use nova_gameplay::{
    prelude::{HudVisibility, PlayerSpaceshipMarker, SectionMarker, SpaceshipRootMarker},
    GameStates,
};
use nova_scenario::prelude::{
    NovaEventWorld, ScenarioCameraMarker, ScenarioId, ScenarioLoaded, ScriptedCameraPose,
    VariableLiteral,
};

/// Seconds the [`nova_autopilot`] preset holds `Loading` before exiting. Must
/// comfortably outlast asset loading (the loader drives `Loading -> Playing` on
/// its own) so the run spends real time in `Playing` before the clean exit.
pub const NOVA_AUTOPILOT_SECS: f32 = 6.0;

/// Settle frames the [`nova_screenshot`] preset waits after `Playing` is
/// reached, so the scene and UI have a few frames to render before the capture.
pub const NOVA_SCREENSHOT_SETTLE_FRAMES: u32 = 30;

/// The name of the single step [`nova_autopilot`] builds, so a caller can
/// [`loop_from`](AutopilotPlugin::loop_from) it without repeating the string.
pub const NOVA_AUTOPILOT_STEP: &str = "nova: play the loading-gated window";

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

/// Env-gated screenshot preset for nova examples: advance to `Playing`, settle
/// [`NOVA_SCREENSHOT_SETTLE_FRAMES`] frames, hide the dev overlays, capture the
/// primary window to a PNG, and report done. Inert unless `NOVA_SHOT` is set (a
/// `WxH` value also overrides the window resolution). See [`ScreenshotPlugin`].
///
/// Unlike [`nova_autopilot`], this force-advances to `Playing` on the first
/// frame, so it is best used with examples that set their scene up in
/// `OnEnter(GameAssetsStates::Loaded)` (the nova scenario convention, e.g.
/// `scenario`) rather than `OnEnter(GameStates::Playing)`, which the early
/// forced transition would run before `GameAssets` is ready.
pub fn nova_screenshot() -> ScreenshotPlugin<GameStates> {
    ScreenshotPlugin::new(GameStates::Playing)
        .settle_frames(NOVA_SCREENSHOT_SETTLE_FRAMES)
        .hide_overlay(hide_dev_overlays)
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

/// A camera pose for a reel beat: where the camera sits and what it looks at
/// (up is +Y), the same framing the `SetCamera` scenario action takes.
///
/// Nova-only, which is why it never moved into `nova_autopilot`: position +
/// look-at means nothing there without [`ScenarioCameraMarker`] and
/// [`ScriptedCameraPose`] to apply it to.
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

/// A reel beat that poses the scenario camera and captures to `path`: the Nova
/// filling of [`ReelBeat::apply`], which the driver runs on beat entry.
///
/// A beat that keeps the previous framing (two shots of the same view) is a
/// plain [`ReelBeat::new`] with no `apply` hook.
pub fn reel_beat(camera: ReelCamera, path: impl Into<String>) -> ReelBeat {
    ReelBeat::new(path).apply(move |world: &mut World| {
        reel_pose_camera(world, camera.position, camera.look_at);
    })
}

/// Env-gated reel-capture preset for nova examples: once the scenario is live
/// (its camera exists), step an ordered list of [`ReelBeat`]s - pose, settle,
/// capture - then report done. Inert unless `NOVA_REEL` is set.
///
/// This is the Nova wiring of [`ScreenshotReelPlugin`]: the `ready` predicate is
/// "a scenario camera exists", the `hide_overlay` hook clears the dev overlays
/// and drops the HUD to [`HudVisibility::Cinematic`], and [`reel_freeze_bodies`]
/// pins the scene still for the whole reel. Build the beats with [`reel_beat`].
///
/// UI/state-dependent shots (menu, editor, HUD, combat) are NOT expressible as a
/// [`ReelBeat`] (they need button clicks / state changes); those are driven by
/// the example's own autopilot script, reusing [`reel_pose_camera`] and
/// [`capture_window`].
pub fn nova_reel(beats: Vec<ReelBeat>) -> NovaReelPlugin {
    NovaReelPlugin { beats }
}

/// Plugin returned by [`nova_reel`]. Construct it through that preset rather
/// than directly.
pub struct NovaReelPlugin {
    beats: Vec<ReelBeat>,
}

impl Plugin for NovaReelPlugin {
    fn build(&self, app: &mut App) {
        // The driver is inert without the env var on its own, but the freeze
        // system is ours: gate it here so a normal run keeps its physics.
        if std::env::var(REEL_ENV).is_err() {
            return;
        }
        app.add_plugins(
            ScreenshotReelPlugin::new(self.beats.clone())
                .ready(scenario_camera_present)
                .hide_overlay(hide_reel_chrome),
        );
        app.add_systems(Update, reel_freeze_bodies);
    }
}

/// The reel's readiness gate: the scenario is live once its camera has spawned.
/// A `&World` predicate polled every frame, so it stays a cheap query.
fn scenario_camera_present(world: &World) -> bool {
    world
        .iter_entities()
        .any(|entity| entity.contains::<ScenarioCameraMarker>())
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
        // NOTE: RigidBody is an immutable component, so swap it via a command
        // insert.
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
///
/// Exclusive rather than a parameterised system so ONE function satisfies both
/// the `Startup` registration the screenshot examples use and the drivers'
/// `Fn(&mut World)` `hide_overlay` hook.
pub fn hide_dev_overlays(world: &mut World) {
    if let Some(mut debug) = world.get_resource_mut::<crate::DebugEnabled>() {
        debug.0 = false;
    }
    if let Some(mut debug) =
        world.get_resource_mut::<bevy_common_systems::debug::inspector::DebugEnabled>()
    {
        debug.0 = false;
    }
    if let Some(mut debug) =
        world.get_resource_mut::<bevy_common_systems::debug::wireframe::DebugEnabled>()
    {
        debug.0 = false;
    }
}

/// The reel's `hide_overlay` hook: [`hide_dev_overlays`] plus the HUD chrome
/// (the reel scenes carry no player HUD, so the fps/version bar is just
/// clutter). Kept out of [`hide_dev_overlays`] so a HUD-showcase capture can
/// keep the HUD up.
fn hide_reel_chrome(world: &mut World) {
    hide_dev_overlays(world);
    if let Some(mut hud) = world.get_resource_mut::<HudVisibility>() {
        *hud = HudVisibility::Cinematic;
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

    /// The reel's readiness gate is exactly "the scenario camera spawned": false
    /// on an empty world, true once the marker exists. The driver polls this
    /// every frame to hold the first beat until the scene is live.
    #[test]
    fn scenario_camera_present_gates_on_the_marker() {
        let mut world = World::new();
        world.spawn(Transform::default());
        assert!(!scenario_camera_present(&world));

        world.spawn((ScenarioCameraMarker, Transform::default()));
        assert!(scenario_camera_present(&world));
    }

    /// `reel_beat` carries the path through to the driver. The pose it wires
    /// into the beat's `apply` hook is `reel_pose_camera`, pinned by its own
    /// tests above; the crate's `apply` field is private, so the wiring itself
    /// is proved end to end by the reel run in the task's manual proof.
    #[test]
    fn reel_beat_carries_the_output_path() {
        let beat = reel_beat(ReelCamera::new(Vec3::ONE, Vec3::ZERO), "shot.png");
        assert_eq!(beat.path, "shot.png");
        assert_eq!(beat.settle_frames, NOVA_SCREENSHOT_SETTLE_FRAMES);
    }
}
