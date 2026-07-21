use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::{
        render_resource::{TextureViewDescriptor, TextureViewDimension},
        view::screenshot::{save_to_disk, Screenshot},
    },
};
use bevy_common_systems::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Glob-import surface: `use nova_scenario::actions::prelude::*` brings the
/// action config vocabulary and scenario-object types into scope.
pub mod prelude {
    pub use super::{
        apply_pending_skybox_swaps, base_scenario_object, BaseScenarioObjectConfig, CurrentOutcome,
        DebugMessageActionConfig, DespawnScenarioObjectActionConfig, EventActionConfig,
        HintEmphasisClearActionConfig, HintEmphasisSetActionConfig, HudReadoutActionConfig,
        HudReadoutFormat, NextScenarioActionConfig,
        ObjectiveActionConfig, ObjectiveCompleteActionConfig, ObjectiveMarkerAttachActionConfig,
        ObjectiveMarkerDetachActionConfig, OutcomeActionConfig, PendingSkyboxSwap,
        ScatterObjectsConfig, ScatterRegion, ScenarioAreaConfig, ScenarioObjectConfig,
        ScenarioObjectKind, ScenarioOutcomeKind, ScreenshotActionConfig, SetCameraActionConfig,
        SetControllerVerbActionConfig, SetSkyboxActionConfig, SetSpeedCapActionConfig,
        StoryMessageActionConfig, VariableSetActionConfig, NEXT_SCENARIO_DELAY_MAX_SECS,
        NEXT_SCENARIO_DELAY_WARN_SECS, OUTCOME_AUTO_ADVANCE_MAX_SECS,
    };
}

/// What a handler does when it fires: one entry in the RON `actions` list,
/// run in order after every filter passes.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EventActionConfig {
    /// Log a message.
    DebugMessage(DebugMessageActionConfig),
    /// Evaluate an expression into a scenario variable.
    VariableSet(VariableSetActionConfig),
    /// Add a HUD objective by id.
    Objective(ObjectiveActionConfig),
    /// Complete a HUD objective by id.
    ObjectiveComplete(ObjectiveCompleteActionConfig),
    /// Attach the gold objective marker chip to the scoped object by id.
    ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig),
    /// Remove the objective marker chip from the scoped object by id.
    ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig),
    /// Pulse one keybind-hint row gold.
    HintEmphasisSet(HintEmphasisSetActionConfig),
    /// Clear a keybind-hint row's gold emphasis.
    HintEmphasisClear(HintEmphasisClearActionConfig),
    /// Spawn a scenario object.
    SpawnScenarioObject(ScenarioObjectConfig),
    /// Spawn many scenario objects across a region (id-prefixed).
    ScatterObjects(ScatterObjectsConfig),
    /// Despawn the scoped object whose id matches.
    DespawnScenarioObject(DespawnScenarioObjectActionConfig),
    /// Install or remove the manual flight speed cap on a scoped ship by id.
    SetSpeedCap(SetSpeedCapActionConfig),
    /// Enable or disable one flight verb on a scoped ship's controller by id.
    SetControllerVerb(SetControllerVerbActionConfig),
    /// Spawn a spherical sensor zone that drives `OnEnter`/`OnExit`.
    CreateScenarioArea(ScenarioAreaConfig),
    /// Queue a switch to another scenario by id.
    NextScenario(NextScenarioActionConfig),
    /// Pose the scenario camera for a scripted shot (photo mode).
    SetCamera(SetCameraActionConfig),
    /// Capture the primary window to a PNG (photo mode).
    Screenshot(ScreenshotActionConfig),
    /// Swap the scenario's skybox cubemap mid-scenario (modding hook).
    SetSkybox(SetSkyboxActionConfig),
    /// Declare the scenario's win/lose outcome (drives the outcome overlay).
    Outcome(OutcomeActionConfig),
    /// Speaker-attributed story text, rendered by the HUD comms panel (the
    /// story-campaign vocabulary; task 20260716-183220).
    StoryMessage(StoryMessageActionConfig),
    /// Show (or clear) a named HUD readout bound to a scenario variable - the
    /// display half of the scenario-variable vocabulary (task 20260716-174729).
    HudReadout(HudReadoutActionConfig),
}

impl EventAction<NovaEventWorld> for EventActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        match self {
            EventActionConfig::DebugMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::VariableSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::Objective(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveComplete(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerAttach(config) => {
                config.action(world, info);
            }
            EventActionConfig::ObjectiveMarkerDetach(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisSet(config) => {
                config.action(world, info);
            }
            EventActionConfig::HintEmphasisClear(config) => {
                config.action(world, info);
            }
            EventActionConfig::SpawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::ScatterObjects(config) => {
                config.action(world, info);
            }
            EventActionConfig::DespawnScenarioObject(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetSpeedCap(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetControllerVerb(config) => {
                config.action(world, info);
            }
            EventActionConfig::CreateScenarioArea(config) => {
                config.action(world, info);
            }
            EventActionConfig::NextScenario(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetCamera(config) => {
                config.action(world, info);
            }
            EventActionConfig::Screenshot(config) => {
                config.action(world, info);
            }
            EventActionConfig::SetSkybox(config) => {
                config.action(world, info);
            }
            EventActionConfig::Outcome(config) => {
                config.action(world, info);
            }
            EventActionConfig::StoryMessage(config) => {
                config.action(world, info);
            }
            EventActionConfig::HudReadout(config) => {
                config.action(world, info);
            }
        }
    }
}

/// Pose the scenario camera (the [`ScenarioCameraMarker`] entity) at `position`
/// looking at `look_at` by pinning a [`ScriptedCameraPose`] on it (and dropping
/// [`WASDCameraController`] so free-fly input stops). The pose is enforced every
/// frame after the WASD sync, so it holds even though the controller's state
/// machine keeps writing the Transform - a one-shot set would be overwritten,
/// and removing the controller does not stop it (its private state components
/// survive). A no-op with a warning when no scenario camera is present (e.g. a
/// headless rig without the loader's camera).
///
/// Part of the in-engine photo-mode surface, paired with
/// [`ScreenshotActionConfig`]: a beat poses the camera, settles, then captures.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetCameraActionConfig {
    /// World-space camera position.
    pub position: Vec3,
    /// World-space point the camera looks at (up is +Y).
    pub look_at: Vec3,
}

impl EventAction<NovaEventWorld> for SetCameraActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let position = self.position;
        let look_at = self.look_at;
        debug!("SetCamera: position {:?} look_at {:?}", position, look_at);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // Resolve the camera before taking a mutable borrow (the query's
                // immutable borrow of `world` ends with this block).
                let camera = {
                    let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
                    query.iter(world).next()
                };
                let Some(camera) = camera else {
                    warn!("SetCamera: no scenario camera present; nothing to pose");
                    return;
                };

                if let Ok(mut entity) = world.get_entity_mut(camera) {
                    // Drop free-fly input and pin the scripted pose; the loader's
                    // enforcer applies it after the WASD sync every frame.
                    entity.remove::<WASDCameraController>();
                    entity.insert(ScriptedCameraPose { position, look_at });
                }
            });
        });
    }
}

/// Resolve a screenshot output path. Absolute paths are used as-is; a relative
/// path is joined under the `NOVA_SHOT_DIR` env var when set (so an example or a
/// packaging script can redirect all captures to a staging folder), else it is
/// relative to the process working directory.
fn resolve_capture_path(path: &str) -> std::path::PathBuf {
    let dir = std::env::var("NOVA_SHOT_DIR")
        .ok()
        .filter(|dir| !dir.is_empty());
    resolve_capture_path_in(path, dir.as_deref())
}

/// Pure core of [`resolve_capture_path`], with the capture dir passed in so it
/// is testable without mutating the process environment.
fn resolve_capture_path_in(path: &str, capture_dir: Option<&str>) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    match capture_dir {
        Some(dir) if !path.is_absolute() => std::path::Path::new(dir).join(path),
        _ => path.to_path_buf(),
    }
}

/// Capture the primary window to a PNG at `path` (photo mode). Relative paths
/// resolve under `NOVA_SHOT_DIR` (see `resolve_capture_path`). Built on Bevy's
/// built-in `Screenshot::primary_window()` + `save_to_disk` observer - the same
/// primitive the screenshot harness uses - so no capture dependency is added.
/// The parent directory is created if missing; a capture on a build without a
/// render backend simply never lands, which is acceptable for a dev/marketing
/// tool.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScreenshotActionConfig {
    /// Output PNG path (relative paths resolve under `NOVA_SHOT_DIR`).
    pub path: String,
}

impl ScreenshotActionConfig {
    /// Construct from a string slice.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ScreenshotActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let path = self.path.clone();
        debug!("Screenshot: capturing to '{}'", path);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let resolved = resolve_capture_path(&path);
                if let Some(parent) = resolved.parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(error) = std::fs::create_dir_all(parent) {
                            warn!(
                                "Screenshot: could not create capture dir {:?}: {error}",
                                parent
                            );
                        }
                    }
                }
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(resolved));
            });
        });
    }
}

/// Fallback skybox brightness, matching the value the loader spawns the scenario
/// camera with (`loader.rs`). Only used if a swap targets a camera that somehow
/// has no current `SkyboxConfig` to inherit brightness from.
const DEFAULT_SKYBOX_BRIGHTNESS: f32 = 1000.0;

/// Swap the scenario's skybox cubemap mid-scenario. A modding hook (task
/// 20260525-133017): a beat can change the sky by authoring a new cubemap path,
/// resolved through the same [`AssetRef`] path-or-handle layer the RON format
/// uses for the initial `cubemap`.
///
/// The cubemap cannot be applied synchronously: the skybox setup observer in
/// `bevy_common_systems` reads the image out of `Assets<Image>` the instant a
/// `SkyboxConfig` is inserted and panics if it is not loaded yet - and a
/// freshly-referenced modder path is not. So the action only *tags* the scenario
/// camera with a [`PendingSkyboxSwap`]; [`apply_pending_skybox_swaps`] inserts the
/// real `SkyboxConfig` once the image has finished loading.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetSkyboxActionConfig {
    /// The new cubemap image, authored as an asset path (e.g.
    /// `"scenarios/space.cube.png"`) or a live handle in code-built configs.
    pub cubemap: AssetRef<Image>,
    /// Optional brightness multiplier. `None` keeps the current skybox brightness.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub brightness: Option<f32>,
}

impl SetSkyboxActionConfig {
    /// Construct a swap to `cubemap`, keeping the current brightness.
    pub fn new(cubemap: impl Into<AssetRef<Image>>) -> Self {
        Self {
            cubemap: cubemap.into(),
            brightness: None,
        }
    }
}

impl EventAction<NovaEventWorld> for SetSkyboxActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let cubemap = self.cubemap.clone();
        let brightness = self.brightness;
        debug!("SetSkybox: cubemap {:?}", cubemap.path());

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // Start the load (idempotent for an already-resolved handle).
                let handle = {
                    let asset_server = world.resource::<AssetServer>();
                    cubemap.resolve(asset_server)
                };

                // Resolve the camera before taking a mutable borrow.
                let camera = {
                    let mut query = world.query_filtered::<Entity, With<ScenarioCameraMarker>>();
                    query.iter(world).next()
                };
                let Some(camera) = camera else {
                    warn!("SetSkybox: no scenario camera present; nothing to swap");
                    return;
                };

                if let Ok(mut entity) = world.get_entity_mut(camera) {
                    // Do NOT insert SkyboxConfig here - the setup observer would
                    // read the not-yet-loaded image and panic. Tag for the
                    // deferred applier instead.
                    entity.insert(PendingSkyboxSwap {
                        cubemap: handle,
                        brightness,
                    });
                }
            });
        });
    }
}

/// A requested skybox swap waiting on its cubemap image to finish loading. Set by
/// [`SetSkyboxActionConfig`], consumed by [`apply_pending_skybox_swaps`].
#[derive(Component, Clone, Debug, Reflect)]
pub struct PendingSkyboxSwap {
    /// The (loading) cubemap to install once it is present in `Assets<Image>`.
    pub cubemap: Handle<Image>,
    /// Brightness override, or `None` to keep the camera's current brightness.
    pub brightness: Option<f32>,
}

/// Applies a [`PendingSkyboxSwap`] once its cubemap image is available.
///
/// Readiness is "present in `Assets<Image>`" rather than the asset server's load
/// state, because that is exactly what the skybox setup observer needs to read -
/// and it also lets code-built swaps (a handle added straight to `Assets`) apply
/// without a server round-trip. A load the *server* reports as failed is dropped
/// with a warning so a bad modder path leaves the sky unchanged instead of
/// waiting forever; the action always resolves through a server load, so that
/// covers every real swap (a bare code-built handle that is never added would
/// wait indefinitely, but nothing constructs one).
///
/// A cubemap that arrives ALREADY multi-layer (its `.meta` `array_layout`
/// applied at load time - now every cubemap with a sidecar, base or mod, since
/// `assets_plugin()` reads metas with `AssetMetaCheck::Always`) skips the bcs
/// setup observer's single-layer fallback branch, which is also where the Cube
/// texture view was set. Without the
/// view, bevy's skybox sanity check (`sanity_check_skybox_image_and_warn` in
/// bevy_core_pipeline's skybox module) refuses the non-Cube view with a
/// `warn_once` and withholds the skybox bind group - the sky silently
/// disappears. So the applier sets the view itself before installing the
/// config (task 20260717-013440). The write happens only when the view is
/// actually missing: writing through the `AssetMut` guard queues
/// `AssetEvent::Modified` (a full re-upload of the hundreds-of-MB cubemap
/// texture), so the no-change path must provably not write.
pub fn apply_pending_skybox_swaps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    q_pending: Query<(Entity, &PendingSkyboxSwap, Option<&SkyboxConfig>)>,
) {
    for (entity, pending, current) in &q_pending {
        if images.contains(&pending.cubemap) {
            let needs_cube_view = images.get(&pending.cubemap).is_some_and(|image| {
                image.texture_descriptor.array_layer_count() > 1
                    && image.texture_view_descriptor.is_none()
            });
            if needs_cube_view {
                if let Some(mut image) = images.get_mut(&pending.cubemap) {
                    image.texture_view_descriptor = Some(TextureViewDescriptor {
                        dimension: Some(TextureViewDimension::Cube),
                        ..default()
                    });
                }
            }
            let brightness = pending
                .brightness
                .or_else(|| current.map(|config| config.brightness))
                .unwrap_or(DEFAULT_SKYBOX_BRIGHTNESS);
            debug!("SetSkybox: cubemap loaded, installing (brightness {brightness})");
            commands
                .entity(entity)
                .remove::<PendingSkyboxSwap>()
                .insert(SkyboxConfig {
                    cubemap: pending.cubemap.clone(),
                    brightness,
                });
        } else if asset_server.load_state(&pending.cubemap).is_failed() {
            warn!("SetSkybox: cubemap failed to load; leaving the skybox unchanged");
            commands.entity(entity).remove::<PendingSkyboxSwap>();
        }
        // else: still loading - keep the tag and check again next frame.
    }
}

/// Action that evaluates an expression and stores the result in a scenario
/// variable.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableSetActionConfig {
    /// The scenario variable to write.
    pub key: String,
    /// The expression evaluated (against the current variables) into that key.
    pub expression: VariableExpressionNode,
}

impl EventAction<NovaEventWorld> for VariableSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        match self.expression.evaluate(world) {
            Ok(literal) => {
                world.insert_variable(self.key.clone(), literal);
            }
            Err(e) => {
                error!(
                    "VariableSetActionConfig: failed to evaluate expression for key '{}': {:?}",
                    self.key, e
                );
            }
        }
    }
}

/// Action that logs a message; an authoring/debugging aid with no game effect.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DebugMessageActionConfig {
    /// The text to log.
    pub message: String,
}

impl EventAction<NovaEventWorld> for DebugMessageActionConfig {
    fn action(&self, _: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!("Event Action Message: {}", self.message);
    }
}

/// Which way a scenario ended. The variant picks the overlay's banner and
/// styling (gold VICTORY / red DEFEAT); everything else about the ending
/// stays the author's composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioOutcomeKind {
    /// The player won: the overlay shows the gold VICTORY banner.
    Victory,
    /// The player lost: the overlay shows the red DEFEAT banner.
    Defeat,
}

/// Declare the scenario's outcome: show the win/lose overlay. Presentation
/// only - what happens NEXT stays composed from the existing vocabulary: pair
/// with `NextScenario(linger: true)` so [Enter] continues (Victory) or
/// retries (Defeat); with nothing queued, [Enter] returns to the main menu.
/// In strict RON the optional message is written with its variant:
/// `Outcome((outcome: Defeat, message: Some("...")))`, never a bare string.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutcomeActionConfig {
    /// Victory or Defeat.
    pub outcome: ScenarioOutcomeKind,
    /// Optional flavor line under the banner.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub message: Option<String>,
    /// Timed overlay (task 20260717-163050): after this many REAL seconds
    /// (the overlay pauses virtual time) the banner advances the queued
    /// LINGERING chain exactly as if Continue were pressed. Strict RON:
    /// `auto_advance_secs: Some(6.0)`. Absent = wait for the player;
    /// meaningless without a queued lingering NextScenario.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub auto_advance_secs: Option<f64>,
}

impl OutcomeActionConfig {
    /// Construct with a message.
    pub fn new(outcome: ScenarioOutcomeKind, message: &str) -> Self {
        Self {
            outcome,
            message: Some(message.to_string()),
            auto_advance_secs: None,
        }
    }
}

/// The currently-declared scenario outcome, `None` while a scenario is in
/// play. Written by [`OutcomeActionConfig`], cleared by scenario teardown
/// (both the load and unload paths), read by the outcome overlay in
/// `nova_menu` and by the Enter handler's return-to-menu fallback.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct CurrentOutcome(pub Option<OutcomeActionConfig>);

impl EventAction<NovaEventWorld> for OutcomeActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let outcome = self.clone();
        debug!("Outcome: declaring {:?}", outcome.outcome);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // get_resource_mut, not resource_mut: headless rigs that
                // exercise scenario scripts without the loader plugin have no
                // outcome resource, and the action must not panic there.
                let Some(mut current) = world.get_resource_mut::<CurrentOutcome>() else {
                    warn!("Outcome: no CurrentOutcome resource (scenario loader not loaded)");
                    return;
                };
                current.0 = Some(outcome);
            });
        });
    }
}

/// Runtime cap on the delayed cut (panic-proofing Timer construction);
/// content_lint warns above [`NEXT_SCENARIO_DELAY_WARN_SECS`] already.
pub const NEXT_SCENARIO_DELAY_MAX_SECS: f32 = 300.0;
/// The authored range content_lint considers sane for a delayed cut.
pub const NEXT_SCENARIO_DELAY_WARN_SECS: f32 = 60.0;
/// Runtime cap on the timed banner (same panic-proofing).
pub const OUTCOME_AUTO_ADVANCE_MAX_SECS: f64 = 300.0;

/// Action that queues a switch to another scenario.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NextScenarioActionConfig {
    /// The id of the scenario to switch to.
    pub scenario_id: String,
    /// When true, defer the switch until something releases it (the
    /// scenario-advance input or the outcome overlay's Continue/Retry).
    pub linger: bool,
    /// Delayed non-lingering switch (task 20260717-163050, user-directed):
    /// with `linger: false`, hold the cut for this many seconds while the
    /// world keeps playing - the middle gear between the hard cut and the
    /// modal overlay. Strict RON: `delay: Some(4.0)`. Ticks on virtual
    /// (pause-frozen) time; non-positive or absent = instant.
    /// Meaningless with `linger: true` (the overlay's Continue is the
    /// release; see `OutcomeActionConfig::auto_advance_secs` for a timed
    /// overlay).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub delay: Option<f32>,
}

impl EventAction<NovaEventWorld> for NextScenarioActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        debug!(
            "NextScenario: queuing scenario '{}' (linger: {}, delay: {:?})",
            self.scenario_id, self.linger, self.delay
        );
        world.next_scenario = Some(self.clone());
        // Arm the delayed cut only for the non-lingering shape; a fresh
        // queue always resets the clock (last request wins wholesale).
        // Finite-check and cap before Timer::from_seconds - an authored
        // 1e30 parses fine and would PANIC Duration::from_secs_f32
        // (review R1.1); content_lint warns outside (0, 60].
        world.next_scenario_delay = match self.delay {
            Some(delay) if !self.linger && delay > 0.0 && delay.is_finite() => Some(
                Timer::from_seconds(delay.min(NEXT_SCENARIO_DELAY_MAX_SECS), TimerMode::Once),
            ),
            _ => None,
        };
    }
}

/// A scenario action that adds an objective to the HUD.
///
/// The objective *data* (id + message) is the generic `bevy_common_systems` `Objective`, but
/// this scenario-action wrapper stays nova-local because it implements the (foreign)
/// `EventAction` trait - which the orphan rule forbids implementing on the foreign
/// `Objective` type directly.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveActionConfig {
    /// Opaque identifier, used to complete/remove the objective later.
    pub id: String,
    /// The text shown in the objectives HUD.
    pub message: String,
}

impl ObjectiveActionConfig {
    /// Construct from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_objective(self.clone());
    }
}

/// One speaker-attributed story line for the HUD comms panel (task
/// 20260716-183220). Appends to the event world's story log; the log is
/// scenario-scoped (cleared at teardown with the rest of the event world),
/// so a line can never leak into the next scenario or the menu. RON:
/// `StoryMessage((speaker: "Foreman Okono", text: "Strip it clean."))`,
/// optionally `dwell: Some(12.0)` for a longer hold (strict RON `Some`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoryMessageActionConfig {
    /// Who says it (the panel renders it as the line's prefix).
    pub speaker: String,
    /// The line itself.
    pub text: String,
    /// Optional on-screen hold override in seconds (task 20260717-163033).
    /// Strict RON: `dwell: Some(12.0)`, never a bare number; omit the field
    /// for the default (8s). The panel clamps to [3, 30] at use;
    /// content_lint warns on an authored value outside that range.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dwell: Option<f32>,
}

impl EventAction<NovaEventWorld> for StoryMessageActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.push_story_message(self.clone());
    }
}

/// How a [`HudReadoutActionConfig`] renders its bound variable on the HUD. Maps
/// one-to-one onto nova_gameplay's `HudReadoutFormat` at sync time (the HUD
/// cannot depend on nova_scenario, so the enum is mirrored, the same split as
/// `StoryMessageActionConfig` -> `StoryLine`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HudReadoutFormat {
    /// One decimal place, e.g. `12.3`.
    Number,
    /// No decimals (rounded), e.g. `12`.
    Integer,
    /// Minutes and seconds, `mm:ss.s` (e.g. `01:23.4`) - the time-trial clock.
    Time,
}

impl Default for HudReadoutFormat {
    fn default() -> Self {
        HudReadoutFormat::Number
    }
}

/// Show, update, or clear a named HUD readout bound to a scenario variable (task
/// 20260716-174729) - the DISPLAY half of the scenario-variable vocabulary. The
/// timekeeping half already exists: `scenario_elapsed` (and any authored
/// variable) lives on the event world; this action is what finally puts one on
/// the HUD. Generic on purpose (per the spike): any mod can surface any variable
/// (a score, a countdown, a lap time), not just the gauntlet clock.
///
/// A readout is identified by its `slot`. Firing the action with `visible: true`
/// shows or updates that slot; the HUD then tracks the bound variable's CURRENT
/// value every frame (read at sync time), so a single fire from the start gate
/// is enough for a live clock. `visible: false` clears just that slot. Every
/// readout also clears automatically at scenario teardown, exactly like the
/// comms panel, so one cannot leak into the next scenario or the menu.
///
/// The value freezes on pause and behind the outcome overlay because
/// `scenario_elapsed` freezes there - a time-trial's FINAL time simply holds,
/// frozen, on the HUD through the Victory banner.
///
/// RON: `HudReadout((slot: "timer", variable: "scenario_elapsed",
/// format: Time, label: Some("TIME")))`; clear with
/// `HudReadout((slot: "timer", variable: "scenario_elapsed", visible: false))`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HudReadoutActionConfig {
    /// The readout's stable id: shows/updates/clears this one slot, and lets a
    /// scenario run several readouts side by side.
    pub slot: String,
    /// The scenario variable whose value the readout shows (e.g.
    /// `"scenario_elapsed"`). Read live off the event world every frame.
    pub variable: String,
    /// How the value renders. Omit for the default ([`HudReadoutFormat::Number`]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub format: HudReadoutFormat,
    /// Optional caption shown before the value (e.g. `"TIME"`).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label: Option<String>,
    /// `true` (the default) shows/updates the slot; `false` clears it.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub visible: bool,
}

/// Serde default for [`HudReadoutActionConfig::visible`]: a readout with the
/// field omitted is shown, not hidden.
#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

impl HudReadoutActionConfig {
    /// Construct a shown readout (`visible: true`) with the default format and
    /// no label.
    pub fn new(slot: &str, variable: &str) -> Self {
        Self {
            slot: slot.to_string(),
            variable: variable.to_string(),
            format: HudReadoutFormat::default(),
            label: None,
            visible: true,
        }
    }
}

impl EventAction<NovaEventWorld> for HudReadoutActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.set_hud_readout(self.clone());
    }
}

/// Action that completes (removes) the HUD objective with the given id.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveCompleteActionConfig {
    /// The id of the objective to complete.
    pub id: String,
}

impl EventAction<NovaEventWorld> for ObjectiveCompleteActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        world.remove_objective(&self.id);
    }
}

/// Despawn the scenario object whose [`EntityId`] matches `id` (recursive,
/// so the object's whole child hierarchy goes with it). The complement of
/// `SpawnScenarioObject`, e.g. a salvage crate the script removes on pickup.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DespawnScenarioObjectActionConfig {
    /// The `EntityId` of the scoped object to despawn.
    pub id: String,
}

impl DespawnScenarioObjectActionConfig {
    /// Construct from a string slice.
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

impl EventAction<NovaEventWorld> for DespawnScenarioObjectActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        debug!("DespawnScenarioObject: despawning '{}'", id);

        // The id -> Entity lookup needs world access, which push_command's
        // `&mut Commands` does not have - so the command queues a Command
        // closure that resolves and despawns in one step. The lookup is
        // gated on ScenarioScopedMarker: spaceship SECTIONS also carry
        // EntityId (their per-ship section ids like "controller"), and an
        // unscoped match on such an id would rip that section out of every
        // ship in the scene.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "DespawnScenarioObject: no entity with id '{}'; check the scenario \
                         for a typo or a double despawn",
                        id
                    );
                }
                for entity in matches {
                    // get_entity_mut, not entity_mut: an earlier recursive
                    // despawn in this loop may have taken a matched
                    // descendant with it (review R1.1).
                    if let Ok(entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.despawn();
                    }
                }
            });
        });
    }
}

/// Attach the gold objective marker (task 20260712-093831) to the scenario
/// object whose [`EntityId`] matches `target_id`: inserts
/// [`ObjectiveMarkerTarget`] with `label`, and the HUD's objective-markers
/// observer grows the chip. Scoped-only lookup, same rule as
/// DespawnScenarioObject. Attaching to an already-marked entity just
/// updates the label.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveMarkerAttachActionConfig {
    /// The `EntityId` of the scoped object the marker chip attaches to.
    pub target_id: String,
    /// The short name the marker chip shows next to the distance.
    pub label: String,
}

impl ObjectiveMarkerAttachActionConfig {
    /// Construct from string slices.
    pub fn new(target_id: &str, label: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
            label: label.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerAttachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        let label = self.label.clone();
        debug!("ObjectiveMarkerAttach: '{}' <- '{}'", id, label);

        // Same shape as DespawnScenarioObject: the id lookup needs world
        // access, so the queued command resolves and inserts in one step -
        // which also means an attach ordered after a spawn in the same
        // handler sees the freshly spawned entity.
        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    warn!(
                        "ObjectiveMarkerAttach: no scoped entity with id '{}'; check the \
                         scenario for a typo or an attach before the spawn",
                        id
                    );
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.insert(ObjectiveMarkerTarget::new(&label));
                    }
                }
            });
        });
    }
}

/// Detach the objective marker from the scenario object whose [`EntityId`]
/// matches `target_id` (no-op with a warning when nothing matches; a
/// marker whose entity despawned is already detached - the chip died with
/// it).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectiveMarkerDetachActionConfig {
    /// The `EntityId` of the scoped object to detach the marker chip from.
    pub target_id: String,
}

impl ObjectiveMarkerDetachActionConfig {
    /// Construct from a string slice.
    pub fn new(target_id: &str) -> Self {
        Self {
            target_id: target_id.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for ObjectiveMarkerDetachActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.target_id.clone();
        debug!("ObjectiveMarkerDetach: '{}'", id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query =
                    world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
                let matches: Vec<Entity> = query
                    .iter(world)
                    .filter(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                    .collect();
                if matches.is_empty() {
                    // Quieter than attach: detaching an entity that already
                    // despawned (crate picked up) is a legitimate script
                    // shape, not necessarily a typo.
                    debug!("ObjectiveMarkerDetach: no scoped entity with id '{}'", id);
                }
                for entity in matches {
                    if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                        entity_mut.remove::<ObjectiveMarkerTarget>();
                    }
                }
            });
        });
    }
}

/// Emphasize one keybind-hint row (task 20260712-093831): pushes `verb`
/// into nova_gameplay's [`HintEmphasis`] resource, so the cluster pulses
/// that row toward objective gold until a `HintEmphasisClear` (or scenario
/// teardown) drops it. Only `ROW_VERBS` names are valid; the resource
/// refuses unknown verbs with a warning.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HintEmphasisSetActionConfig {
    /// The keybind-hint row to emphasize (one of `ROW_VERBS`).
    pub verb: String,
}

impl HintEmphasisSetActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisSetActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisSet: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                // get_resource_mut, not resource_mut: headless rigs that
                // exercise scenario scripts without the HUD plugins have no
                // emphasis resource, and the action must not panic there.
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    warn!("HintEmphasisSet: no HintEmphasis resource (HUD not loaded)");
                    return;
                };
                emphasis.set(&verb);
            });
        });
    }
}

/// Drop the emphasis on one keybind-hint row (see [`HintEmphasisSetActionConfig`]).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HintEmphasisClearActionConfig {
    /// The keybind-hint row to clear (one of `ROW_VERBS`).
    pub verb: String,
}

impl HintEmphasisClearActionConfig {
    /// Construct from a string slice.
    pub fn new(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
        }
    }
}

impl EventAction<NovaEventWorld> for HintEmphasisClearActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let verb = self.verb.clone();
        debug!("HintEmphasisClear: '{}'", verb);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(mut emphasis) = world.get_resource_mut::<HintEmphasis>() else {
                    return;
                };
                emphasis.clear(&verb);
            });
        });
    }
}

/// Set or clear the manual [`FlightSpeedCap`] on a scenario ship by id
/// (the shakedown training governor releases at beacon 1; playtest round
/// 2 finding 3). Scoped-only lookup, same rule as DespawnScenarioObject.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetSpeedCapActionConfig {
    /// The `EntityId` of the scoped ship to cap.
    pub id: String,
    /// `Some(cap)` installs/updates the cap (u/s); `None` removes it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cap: Option<f32>,
}

impl EventAction<NovaEventWorld> for SetSpeedCapActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let cap = self.cap;
        debug!("SetSpeedCap: '{}' -> {:?}", id, cap);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = query
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetSpeedCap: no scoped ship with id '{}'", id);
                    return;
                };
                match cap {
                    Some(cap) => {
                        world.entity_mut(ship).insert(FlightSpeedCap(cap));
                    }
                    None => {
                        world.entity_mut(ship).remove::<FlightSpeedCap>();
                    }
                }
            });
        });
    }
}

/// Enable or disable one flight verb on a scenario ship's controller
/// section(s) by id. Flight verbs (STOP/GOTO/ORBIT) are a capability the
/// controller grants; this flips a single verb at runtime - the shakedown
/// withholds GOTO until the first objective is complete
/// (spike docs/spikes/20260712-143551-controller-provided-verb-flags.md).
/// Scoped-only lookup, same rule as SetSpeedCap; writes every controller
/// section on the ship so the union the input layer reads matches.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetControllerVerbActionConfig {
    /// The `EntityId` of the scoped ship whose controller sections to edit.
    pub id: String,
    /// The flight verb (STOP/GOTO/ORBIT/LOCK/RCS) to toggle.
    pub verb: FlightVerb,
    /// Whether the verb is enabled (true) or disabled (false).
    pub enabled: bool,
}

impl EventAction<NovaEventWorld> for SetControllerVerbActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let verb = self.verb;
        let enabled = self.enabled;
        debug!("SetControllerVerb: '{}' {:?} -> {}", id, verb, enabled);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut ships = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = ships
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetControllerVerb: no scoped ship with id '{}'", id);
                    return;
                };

                // Every controller section on this ship (active or not - the
                // flag persists across (de)activation), so the union the hint
                // pass and observers read reflects the change.
                let mut controllers =
                    world.query_filtered::<(Entity, &ChildOf), With<ControllerSectionMarker>>();
                let targets: Vec<Entity> = controllers
                    .iter(world)
                    .filter(|(_, &ChildOf(parent))| parent == ship)
                    .map(|(entity, _)| entity)
                    .collect();
                if targets.is_empty() {
                    warn!("SetControllerVerb: ship '{}' has no controller section", id);
                    return;
                }
                for controller in targets {
                    // `WithheldVerbs` is absent on a fresh controller (all
                    // granted); a disable must materialize it first. An enable
                    // on an absent component is already a no-op (nothing is
                    // withheld), so only insert-if-absent when disabling.
                    if world.get::<WithheldVerbs>(controller).is_none() {
                        if !enabled {
                            world
                                .entity_mut(controller)
                                .insert(WithheldVerbs::default());
                        } else {
                            continue;
                        }
                    }
                    let mut withheld = world
                        .get_mut::<WithheldVerbs>(controller)
                        .expect("WithheldVerbs present: it was just inserted or already existed");
                    if enabled {
                        withheld.grant(verb);
                    } else {
                        withheld.withhold(verb);
                    }
                }
            });
        });
    }
}

/// A spawnable scenario object: the shared base (id, name, transform) plus the
/// kind-specific config that picks what to spawn.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioObjectConfig {
    /// The shared base fields every scenario object carries.
    pub base: BaseScenarioObjectConfig,
    /// Which kind of object to spawn and its per-kind config.
    pub kind: ScenarioObjectKind,
}

/// The fields every scenario object shares, regardless of kind: identity and
/// initial pose.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseScenarioObjectConfig {
    /// The object's scenario `EntityId`.
    pub id: String,
    /// The object's display name.
    pub name: String,
    /// The object's initial world position.
    pub position: Vec3,
    /// The object's initial world rotation.
    pub rotation: Quat,
}

/// Build the shared bundle every scenario object spawns with: scoped marker,
/// identity, interpolated transform, dynamic body, and visibility.
pub fn base_scenario_object(config: &BaseScenarioObjectConfig) -> impl Bundle {
    (
        ScenarioScopedMarker,
        Name::new(config.name.clone()),
        EntityId::new(config.id.clone()),
        Transform::from_translation(config.position).with_rotation(config.rotation),
        RigidBody::Dynamic,
        // Physics advances Transform only on fixed ticks (64 Hz by
        // default); everything
        // watched by the render-rate camera must interpolate between them or
        // it stair-steps. Invisible while the chase camera was bolted rigidly
        // to the ship (both stepped together), but the camera smoothing from
        // the flight-feel retune eases at render rate and exposed the steps
        // as twitch (task 20260709-160753).
        TransformInterpolation,
        Visibility::Visible,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transition-pacing fields parse in their documented strict-RON
    /// shapes and default when omitted (task 20260717-163050).
    #[cfg(feature = "serde")]
    #[test]
    fn transition_pacing_ron_parses_and_defaults() {
        let delayed = r#"NextScenario((scenario_id: "x", linger: false, delay: Some(4.0)))"#;
        let parsed: EventActionConfig = ron::from_str(delayed).expect("delay syntax parses");
        let EventActionConfig::NextScenario(next) = &parsed else {
            panic!("NextScenario variant");
        };
        assert_eq!(next.delay, Some(4.0));

        let plain = r#"NextScenario((scenario_id: "x", linger: true))"#;
        let parsed: EventActionConfig = ron::from_str(plain).expect("omitted delay parses");
        let EventActionConfig::NextScenario(next) = &parsed else {
            panic!("NextScenario variant");
        };
        assert_eq!(next.delay, None);

        let timed = r#"Outcome((outcome: Victory, auto_advance_secs: Some(6.0)))"#;
        let parsed: EventActionConfig = ron::from_str(timed).expect("auto_advance parses");
        let EventActionConfig::Outcome(outcome) = &parsed else {
            panic!("Outcome variant");
        };
        assert_eq!(outcome.auto_advance_secs, Some(6.0));
        assert_eq!(outcome.message, None);
    }

    /// The authored RON shape parses and round-trips - the exact syntax the
    /// authoring guide documents: `StoryMessage((speaker: ..., text: ...))`,
    /// with `dwell` OMITTED defaulting to None and the documented strict-RON
    /// `dwell: Some(12.0)` parsing (review 20260717-163033 R1.2: the
    /// authored dwell syntax was documented but never parsed in a test).
    #[cfg(feature = "serde")]
    #[test]
    fn story_message_ron_round_trips() {
        let authored = r#"StoryMessage((speaker: "Foreman Okono", text: "Quota's quota."))"#;
        let parsed: EventActionConfig = ron::from_str(authored).expect("authored RON parses");
        let EventActionConfig::StoryMessage(config) = &parsed else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(config.speaker, "Foreman Okono");
        assert_eq!(config.text, "Quota's quota.");
        assert_eq!(config.dwell, None, "omitted dwell defaults to None");

        let with_dwell = r#"StoryMessage((speaker: "Okono", text: "Slowly.", dwell: Some(12.0)))"#;
        let parsed_dwell: EventActionConfig =
            ron::from_str(with_dwell).expect("the documented dwell syntax parses");
        let EventActionConfig::StoryMessage(config_dwell) = &parsed_dwell else {
            panic!("parsed the StoryMessage variant");
        };
        assert_eq!(config_dwell.dwell, Some(12.0));

        let ron = ron::to_string(&parsed).expect("serializes");
        let back: EventActionConfig = ron::from_str(&ron).expect("round-trips");
        let EventActionConfig::StoryMessage(again) = back else {
            panic!("round-tripped the StoryMessage variant");
        };
        assert_eq!(&again, config);
    }

    /// The authored `HudReadout` RON shapes parse and round-trip (task
    /// 20260716-174729): the shown form with a format + label, the omitted
    /// `format`/`label`/`visible` defaults (Number / None / true), and the
    /// clear form (`visible: false`).
    #[cfg(feature = "serde")]
    #[test]
    fn hud_readout_ron_round_trips() {
        let shown = r#"HudReadout((slot: "timer", variable: "scenario_elapsed", format: Time, label: Some("TIME")))"#;
        let parsed: EventActionConfig = ron::from_str(shown).expect("shown RON parses");
        let EventActionConfig::HudReadout(config) = &parsed else {
            panic!("parsed the HudReadout variant");
        };
        assert_eq!(config.slot, "timer");
        assert_eq!(config.variable, "scenario_elapsed");
        assert_eq!(config.format, HudReadoutFormat::Time);
        assert_eq!(config.label.as_deref(), Some("TIME"));
        assert!(config.visible, "visible defaults to true when omitted");

        let minimal = r#"HudReadout((slot: "score", variable: "score"))"#;
        let parsed_min: EventActionConfig = ron::from_str(minimal).expect("minimal RON parses");
        let EventActionConfig::HudReadout(config_min) = &parsed_min else {
            panic!("parsed the HudReadout variant");
        };
        assert_eq!(
            config_min.format,
            HudReadoutFormat::Number,
            "omitted format defaults to Number"
        );
        assert_eq!(config_min.label, None);
        assert!(config_min.visible);

        let cleared = r#"HudReadout((slot: "timer", variable: "scenario_elapsed", visible: false))"#;
        let parsed_clear: EventActionConfig = ron::from_str(cleared).expect("clear RON parses");
        let EventActionConfig::HudReadout(config_clear) = &parsed_clear else {
            panic!("parsed the HudReadout variant");
        };
        assert!(!config_clear.visible, "the clear form parses visible: false");

        let ron = ron::to_string(&parsed).expect("serializes");
        let back: EventActionConfig = ron::from_str(&ron).expect("round-trips");
        let EventActionConfig::HudReadout(again) = back else {
            panic!("round-tripped the HudReadout variant");
        };
        assert_eq!(&again, config);
    }

    /// The `HudReadout` action's EFFECT through the production drain: the
    /// action upserts a readout on the event world, and the sync mirrors it -
    /// with the bound variable's CURRENT value - into the HUD's `HudReadouts`
    /// resource. A `visible: false` fire clears the slot.
    #[test]
    fn hud_readout_action_syncs_and_clears_through_the_drain() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<nova_gameplay::hud::readout::HudReadouts>();

        // Show a Time readout bound to a variable, and set that variable.
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            world.insert_variable("scenario_elapsed".to_string(), VariableLiteral::Number(83.4));
            let show = EventActionConfig::HudReadout(HudReadoutActionConfig {
                slot: "timer".to_string(),
                variable: "scenario_elapsed".to_string(),
                format: HudReadoutFormat::Time,
                label: Some("TIME".to_string()),
                visible: true,
            });
            show.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());

        let readouts = app
            .world()
            .resource::<nova_gameplay::hud::readout::HudReadouts>();
        assert_eq!(readouts.0.len(), 1, "the shown readout synced");
        assert_eq!(readouts.0[0].slot, "timer");
        assert_eq!(readouts.0[0].value, 83.4, "the live variable value synced");

        // Clear it.
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            let clear = EventActionConfig::HudReadout(HudReadoutActionConfig {
                slot: "timer".to_string(),
                variable: "scenario_elapsed".to_string(),
                format: HudReadoutFormat::Time,
                label: None,
                visible: false,
            });
            clear.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());
        assert!(
            app.world()
                .resource::<nova_gameplay::hud::readout::HudReadouts>()
                .0
                .is_empty(),
            "the clear fire dropped the slot"
        );
    }

    /// The Outcome action's EFFECT through the production drain (task
    /// 20260716-125856): the action queues a command on the event world, and
    /// the state sync applies it to the `CurrentOutcome` resource - fire ->
    /// drain -> assert on the world, not just the config struct.
    #[test]
    fn outcome_action_sets_current_outcome_through_the_drain() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<CurrentOutcome>();
        // state_to_world_system mirrors objectives unconditionally.
        app.init_resource::<GameObjectives>();

        let action = EventActionConfig::Outcome(OutcomeActionConfig::new(
            ScenarioOutcomeKind::Victory,
            "The belt is quiet again.",
        ));
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            action.action(&mut world, &GameEventInfo { data: None });
        }
        // Nothing lands before the drain: the action only queues.
        assert_eq!(app.world().resource::<CurrentOutcome>().0, None);

        NovaEventWorld::state_to_world_system(app.world_mut());

        let current = app.world().resource::<CurrentOutcome>();
        assert_eq!(
            current.0.as_ref().map(|outcome| outcome.outcome),
            Some(ScenarioOutcomeKind::Victory),
            "the drained command writes the declared outcome"
        );
        assert_eq!(
            current
                .0
                .as_ref()
                .and_then(|outcome| outcome.message.as_deref()),
            Some("The belt is quiet again."),
        );
    }

    /// Graceful degradation: a headless rig without the loader's
    /// `CurrentOutcome` resource must not panic when a script declares an
    /// outcome - the command drops (with a warn; the log side is not
    /// asserted here - review R1.6 renamed the test to what it pins).
    #[test]
    fn outcome_action_without_the_resource_does_not_panic_or_conjure_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        // Deliberately NO CurrentOutcome resource.

        let action = EventActionConfig::Outcome(OutcomeActionConfig {
            outcome: ScenarioOutcomeKind::Defeat,
            message: None,
            auto_advance_secs: None,
        });
        {
            let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
            action.action(&mut world, &GameEventInfo { data: None });
        }
        NovaEventWorld::state_to_world_system(app.world_mut());
        assert!(
            app.world().get_resource::<CurrentOutcome>().is_none(),
            "the action must not conjure the resource it warns about"
        );
    }

    /// The authored `SpaceshipConfig.allegiance` override, through the
    /// production spawn path (task 20260708-203659): a NEUTRAL AI ship ends
    /// NEUTRAL even though `AISpaceshipMarker` requires
    /// `Allegiance = Enemy` - the spawn action's explicit insert wins over
    /// the requirement default regardless of command ordering (observer
    /// commands apply before the queue's remaining commands, and a plain
    /// insert overwrites). Companion delivery guard: the same spawn WITHOUT
    /// the override ends Enemy, so the Neutral assert cannot pass vacuously.
    #[test]
    fn authored_allegiance_overrides_the_controller_default() {
        fn spawn_ship(allegiance: Option<Allegiance>) -> Option<Allegiance> {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.add_plugins(crate::objects::spaceship::SpaceshipPlugin);
            app.init_resource::<NovaEventWorld>();
            app.init_resource::<GameObjectives>();

            let config = ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "ship".to_string(),
                    name: "Ship".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                    controller: SpaceshipController::AI(AIControllerConfig::default()),
                    allegiance,
                    sections: vec![],
                }),
            };
            {
                let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
                EventActionConfig::SpawnScenarioObject(config)
                    .action(&mut world, &GameEventInfo { data: None });
            }
            NovaEventWorld::state_to_world_system(app.world_mut());
            app.update();

            let mut q = app
                .world_mut()
                .query_filtered::<&Allegiance, With<SpaceshipRootMarker>>();
            q.iter(app.world()).next().copied()
        }

        assert_eq!(
            spawn_ship(Some(Allegiance::Neutral)),
            Some(Allegiance::Neutral),
            "the authored override survives the AI marker's Enemy default"
        );
        assert_eq!(
            spawn_ship(None),
            Some(Allegiance::Enemy),
            "delivery guard: without the override the AI default applies"
        );
    }

    /// The behavior the component buys (task 20260709-160753): a moving
    /// scenario body's Transform advances on EVERY render frame, not just on
    /// fixed physics ticks. 4 ms frames against the 15.6 ms tick mean at
    /// most one tick lands inside any 3-frame span - without easing at
    /// least two consecutive frames would show identical translations.
    #[test]
    fn scenario_bodies_move_between_fixed_ticks() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        // Mirrors the integrity physics harness: MeshPlugin because avian's
        // collider-from-mesh backend reads AssetEvent<Mesh> at startup.
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.004,
        )));
        app.finish();

        let body = app
            .world_mut()
            .spawn((
                base_scenario_object(&BaseScenarioObjectConfig {
                    id: "mover".to_string(),
                    name: "Mover".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                }),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                LinearVelocity(Vec3::X * 10.0),
            ))
            .id();

        // Warm up past two fixed ticks so the easing has start+end states.
        for _ in 0..10 {
            app.update();
        }

        // Four consecutive 4 ms frames: with easing every frame advances the
        // translation; stair-stepping would repeat a value.
        let mut positions = Vec::new();
        for _ in 0..4 {
            app.update();
            positions.push(app.world().get::<Transform>(body).unwrap().translation.x);
        }
        for pair in positions.windows(2) {
            assert!(
                pair[1] > pair[0],
                "translation must advance every render frame, got {positions:?}"
            );
        }
    }

    /// The skybox swap (task 20260525-133017) is two-step on purpose: the bcs
    /// skybox setup observer reads the cubemap out of `Assets<Image>` the instant
    /// a `SkyboxConfig` is inserted and panics on an unloaded handle, so
    /// `apply_pending_skybox_swaps` holds the `PendingSkyboxSwap` until the image
    /// is present, then installs the config - inheriting the camera's current
    /// brightness unless the swap overrides it.
    #[test]
    fn skybox_swap_waits_for_load_then_installs() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_systems(Update, apply_pending_skybox_swaps);
        app.finish();

        // A scenario camera already showing a skybox at brightness 500.
        let initial = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                SkyboxConfig {
                    cubemap: initial.clone(),
                    brightness: 500.0,
                },
            ))
            .id();

        // Swap to a cubemap that has NOT loaded yet: reserve an id with no asset.
        let loading = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .reserve_handle();
        app.world_mut()
            .entity_mut(camera)
            .insert(PendingSkyboxSwap {
                cubemap: loading.clone(),
                brightness: None,
            });

        // While the image is absent, the swap stays pending and the sky is unchanged.
        app.update();
        assert!(
            app.world().get::<PendingSkyboxSwap>(camera).is_some(),
            "swap must stay pending until the cubemap loads"
        );
        assert_eq!(
            app.world().get::<SkyboxConfig>(camera).unwrap().cubemap,
            initial,
            "skybox must not change while the new cubemap is still loading"
        );

        // The image arrives (load finishes) -> the applier installs it and clears
        // the tag, inheriting brightness 500 because the swap did not override it.
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(loading.id(), Image::default())
            .expect("inserting the loaded cubemap asset");
        app.update();
        assert!(
            app.world().get::<PendingSkyboxSwap>(camera).is_none(),
            "swap must be consumed once the cubemap is present"
        );
        let config = app.world().get::<SkyboxConfig>(camera).unwrap();
        assert_eq!(
            config.cubemap, loading,
            "cubemap must swap to the new handle"
        );
        assert_eq!(
            config.brightness, 500.0,
            "brightness must be inherited when the swap does not set it"
        );

        // An explicit brightness overrides the inherited one.
        let bright = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.world_mut()
            .entity_mut(camera)
            .insert(PendingSkyboxSwap {
                cubemap: bright.clone(),
                brightness: Some(250.0),
            });
        app.update();
        let config = app.world().get::<SkyboxConfig>(camera).unwrap();
        assert_eq!(config.cubemap, bright);
        assert_eq!(
            config.brightness, 250.0,
            "an explicit brightness must override the inherited one"
        );
    }

    /// Builds the applier's minimal rig: assets + the applier, no bcs observer
    /// (its behavior is pinned by the skybox_swap_e2e integration test).
    fn skybox_applier_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.init_asset::<Image>();
        app.add_systems(Update, apply_pending_skybox_swaps);
        app.finish();
        app
    }

    /// A 6 layer array image the way a meta'd cubemap comes out of the loader:
    /// stacked, then reinterpreted - `texture_view_descriptor` still `None`.
    fn six_layer_image() -> Image {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        };
        let mut image = Image::new_fill(
            Extent3d {
                width: 1,
                height: 6,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let _ = image.reinterpret_stacked_2d_as_array(6);
        assert_eq!(
            image.texture_descriptor.array_layer_count(),
            6,
            "rig sanity: the stacked reinterpret produced the 6 layer array"
        );
        image
    }

    /// A cubemap that arrives ALREADY 6-layer (its `.meta` `array_layout`
    /// applied at load time, e.g. `base/textures/cubemap_alt.png` through
    /// `assets_plugin()`) skips the bcs observer's single-layer fallback - the
    /// branch that also set the Cube texture view. The applier must set the
    /// view itself, or bevy's skybox sanity check refuses the non-Cube view
    /// (`warn_once`) and skips rendering - the sky silently disappears
    /// (task 20260717-013440).
    #[test]
    fn skybox_swap_sets_cube_view_on_a_preinterpreted_cubemap() {
        let mut app = skybox_applier_app();

        let cubemap = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(six_layer_image());
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                PendingSkyboxSwap {
                    cubemap: cubemap.clone(),
                    brightness: Some(700.0),
                },
            ))
            .id();

        app.update();

        // The swap landed...
        assert_eq!(
            app.world()
                .get::<SkyboxConfig>(camera)
                .expect("the applier installs the SkyboxConfig")
                .cubemap,
            cubemap
        );
        // ...and the applier readied the image for bevy's Cube skybox binding.
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&cubemap).expect("cubemap is in Assets");
        assert_eq!(
            image
                .texture_view_descriptor
                .as_ref()
                .and_then(|descriptor| descriptor.dimension),
            Some(TextureViewDimension::Cube),
            "an already-arrayed cubemap must get its Cube view from the applier"
        );
    }

    /// The applier must not WRITE to an image whose Cube view is already set
    /// (the preloaded `GameAssets` cubemap after `prepare_cubemap_view`): a
    /// write through the `AssetMut` guard queues `AssetEvent::Modified`, which
    /// re-uploads the hundreds-of-MB cubemap texture for nothing.
    #[test]
    fn skybox_swap_does_not_remodify_an_already_cubed_image() {
        let mut app = skybox_applier_app();

        let mut cubed = six_layer_image();
        cubed.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
        let cubemap = app.world_mut().resource_mut::<Assets<Image>>().add(cubed);
        let camera = app
            .world_mut()
            .spawn((
                ScenarioCameraMarker,
                PendingSkyboxSwap {
                    cubemap: cubemap.clone(),
                    brightness: None,
                },
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<SkyboxConfig>(camera).is_some(),
            "rig sanity: the applier consumed the swap"
        );
        let events: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<Image>>>()
            .drain()
            .collect();
        // Delivery guard: the `.add()` above must have produced an Added event
        // in the drained buffer, or the no-Modified assertion below would be
        // vacuously green whenever asset events stop reaching this resource.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssetEvent::Added { id } if *id == cubemap.id())),
            "rig sanity: the add's Added event reaches the drained messages: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, AssetEvent::Modified { id } if *id == cubemap.id())),
            "consuming a swap for an already-cubed image must not emit Modified \
             (a Modified re-uploads the whole cubemap texture): {events:?}"
        );
    }

    /// The despawn action removes exactly the scenario object whose id
    /// matches - and ONLY scenario-scoped entities: spaceship sections
    /// carry EntityId too (per-ship ids like "controller"), and an
    /// unscoped match would rip that section out of every ship.
    #[test]
    fn despawn_action_removes_the_scoped_object_by_id() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let crate_1 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_1".to_string())))
            .id();
        let crate_2 = world
            .spawn((ScenarioScopedMarker, EntityId::new("crate_2".to_string())))
            .id();
        // An unscoped entity with a colliding id - a stand-in for a ship
        // section - must survive.
        let section = world.spawn(EntityId::new("crate_1".to_string())).id();

        let action = DespawnScenarioObjectActionConfig::new("crate_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());

        // The action only queues; the drain in state_to_world applies it.
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            world.get_entity(crate_1).is_err(),
            "the matching scoped object despawns"
        );
        assert!(
            world.get_entity(crate_2).is_ok(),
            "other scoped objects survive"
        );
        assert!(
            world.get_entity(section).is_ok(),
            "an unscoped entity with the same id (a ship section) survives"
        );
    }

    /// A missing id is a warning, not a crash: the drain must complete and
    /// unrelated entities survive (double-despawn / typo path).
    #[test]
    fn despawn_action_with_missing_id_is_harmless() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        let action = DespawnScenarioObjectActionConfig::new("no_such_id");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(world.get_entity(bystander).is_ok());
    }

    /// Scatter is deterministic: the same seed yields the same layout every
    /// load (a data file must be reproducible), and samples stay in bounds.
    #[test]
    fn scatter_region_sampling_is_deterministic_and_bounded() {
        use rand::SeedableRng;

        let region = ScatterRegion::Box {
            min: Vec3::new(-10.0, -2.0, -10.0),
            max: Vec3::new(10.0, 2.0, 10.0),
        };

        let sample_10 = || {
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);
            (0..10).map(|_| region.sample(&mut rng)).collect::<Vec<_>>()
        };
        let a = sample_10();
        let b = sample_10();
        assert_eq!(a, b, "same seed must produce the same positions");

        for p in &a {
            assert!(p.x >= -10.0 && p.x <= 10.0, "x in box: {p:?}");
            assert!(p.y >= -2.0 && p.y <= 2.0, "y in box: {p:?}");
            assert!(p.z >= -10.0 && p.z <= 10.0, "z in box: {p:?}");
        }
    }

    /// A degenerate region (min == max on an axis) does not panic; it pins that
    /// axis to the value.
    #[test]
    fn scatter_region_degenerate_axis_does_not_panic() {
        use rand::SeedableRng;

        let region = ScatterRegion::Box {
            min: Vec3::new(5.0, 0.0, 5.0),
            max: Vec3::new(5.0, 0.0, 5.0),
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let p = region.sample(&mut rng);
        assert_eq!(p, Vec3::new(5.0, 0.0, 5.0));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn scatter_objects_config_round_trips_through_ron() {
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: 12,
            seed: 7,
            region: ScatterRegion::Ring {
                inner: 100.0,
                outer: 150.0,
                y_min: -20.0,
                y_max: 20.0,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::from("textures/asteroid.png"),
                    health: 100.0,
                    surface_gravity: None,
                    invulnerable: false,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
        };

        let ron = ron::to_string(&config).expect("serialize");
        let back: ScatterObjectsConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.id_prefix, "rock_");
        assert_eq!(back.count, 12);
        assert_eq!(back.seed, 7);
        assert_eq!(back.asteroid_radius, Some((1.0, 3.0)));
        // The nested enum fields most likely to regress in a serde change: the
        // region variant and the template's asset ref must survive intact.
        match back.region {
            ScatterRegion::Ring {
                inner,
                outer,
                y_min,
                y_max,
            } => assert_eq!((inner, outer, y_min, y_max), (100.0, 150.0, -20.0, 20.0)),
            other => panic!("region variant changed on round-trip: {other:?}"),
        }
        match &back.template.kind {
            ScenarioObjectKind::Asteroid(asteroid) => {
                assert_eq!(asteroid.texture.path(), Some("textures/asteroid.png"))
            }
            other => panic!("template kind changed on round-trip: {other:?}"),
        }
    }

    /// The scatter ACTION spawns exactly `count` scoped objects, each with an id
    /// under the prefix, a position inside the region, and a radius in range.
    /// Mirrors the despawn harness: fire into a `NovaEventWorld`, drain, assert on
    /// the world. Guards the spawn loop that only the windowed example exercised.
    #[test]
    fn scatter_action_spawns_count_objects_in_region() {
        let region_min = Vec3::new(-10.0, -5.0, -10.0);
        let region_max = Vec3::new(10.0, 5.0, 10.0);
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: 8,
            seed: 123,
            region: ScatterRegion::Box {
                min: region_min,
                max: region_max,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    health: 100.0,
                    surface_gravity: None,
                    invulnerable: false,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
        };

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            config.action(&mut event_world, &GameEventInfo::default());
        }
        // The action only queues; the drain in state_to_world applies the spawns.
        NovaEventWorld::state_to_world_system(&mut world);

        let mut query = world
            .query_filtered::<(&EntityId, &Transform, &AsteroidRadius), With<AsteroidMarker>>();
        let mut ids: Vec<String> = Vec::new();
        for (id, transform, radius) in query.iter(&world) {
            let p = transform.translation;
            assert!(
                p.x >= region_min.x && p.x <= region_max.x,
                "x in region: {p:?}"
            );
            assert!(
                p.y >= region_min.y && p.y <= region_max.y,
                "y in region: {p:?}"
            );
            assert!(
                p.z >= region_min.z && p.z <= region_max.z,
                "z in region: {p:?}"
            );
            assert!(
                radius.0 >= 1.0 && radius.0 <= 3.0,
                "radius in range: {}",
                radius.0
            );
            assert!(id.0.starts_with("rock_"), "id has the prefix: {}", id.0);
            ids.push(id.0.clone());
        }

        assert_eq!(ids.len(), 8, "scatter spawns exactly `count` objects");
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8, "scattered ids are unique (no collision)");
    }

    /// Scatter is gameplay content, so it spawns the full authored count on EVERY
    /// graphics tier - the density lever was removed in task 20260718-004834.
    /// Regression: even with the cheapest (Low) [`GraphicsBudget`] inserted and
    /// carried into the event world, the field is not thinned. Mirrors the
    /// full-count harness above with a Low budget inserted first, to prove the
    /// budget has no effect on scatter counts.
    #[test]
    fn scatter_action_ignores_graphics_budget() {
        use nova_gameplay::prelude::{GraphicsBudget, GraphicsQuality};

        let authored_count = 20u32;
        let config = ScatterObjectsConfig {
            id_prefix: "rock_".to_string(),
            count: authored_count,
            seed: 123,
            region: ScatterRegion::Box {
                min: Vec3::new(-10.0, -5.0, -10.0),
                max: Vec3::new(10.0, 5.0, 10.0),
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "rock".to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: None,
                    destroy_sound: None,
                    radius: 2.0,
                    texture: nova_gameplay::prelude::AssetRef::default(),
                    health: 100.0,
                    surface_gravity: None,
                    invulnerable: false,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some((1.0, 3.0)),
        };

        // The cheapest tier: if any preset were going to thin scatter, this is the
        // one that would. It must not.
        let low_budget = GraphicsBudget::for_quality(GraphicsQuality::Low);

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        world.insert_resource(low_budget);
        // Pulls the budget into the event world, exactly as the PostUpdate chain
        // does before the queue processes. This is a no-op now that scatter
        // ignores the budget - kept to prove that even a Low budget present in the
        // world does not thin the field.
        NovaEventWorld::world_to_state_system(&mut world);
        {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            config.action(&mut event_world, &GameEventInfo::default());
        }
        NovaEventWorld::state_to_world_system(&mut world);

        let mut query = world.query_filtered::<&EntityId, With<AsteroidMarker>>();
        let spawned = query.iter(&world).count();
        assert_eq!(
            spawned as u32, authored_count,
            "scatter spawns the full authored count ({authored_count}) even on Low - it is never thinned"
        );
    }

    /// The marker attach/detach pair drives the [`ObjectiveMarkerTarget`]
    /// component on exactly the scoped object with the id - unscoped
    /// entities with colliding ids (ship sections) are never marked, and a
    /// re-attach updates the label in place.
    #[test]
    fn objective_marker_attach_and_detach_drive_the_component() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let beacon = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();
        let section = world.spawn(EntityId::new("beacon_1".to_string())).id();

        let attach = ObjectiveMarkerAttachActionConfig::new("beacon_1", "BEACON 1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        attach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("BEACON 1"),
            "the scoped object is marked"
        );
        assert!(
            world.get::<ObjectiveMarkerTarget>(section).is_none(),
            "an unscoped entity with the same id (a ship section) is never marked"
        );

        // Re-attach updates the label in place (no detach needed between).
        let relabel = ObjectiveMarkerAttachActionConfig::new("beacon_1", "NEXT");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        relabel.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert_eq!(
            world
                .get::<ObjectiveMarkerTarget>(beacon)
                .map(|marker| marker.label.as_str()),
            Some("NEXT")
        );

        let detach = ObjectiveMarkerDetachActionConfig::new("beacon_1");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        detach.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world.get::<ObjectiveMarkerTarget>(beacon).is_none(),
            "detach removes the marker"
        );
    }

    /// Attach/detach against a missing id must warn and complete, not
    /// crash - the detach-after-despawn shape is legitimate script data
    /// (crate picked up before its detach action runs).
    #[test]
    fn objective_marker_actions_with_missing_id_are_harmless() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let bystander = world
            .spawn((ScenarioScopedMarker, EntityId::new("beacon_1".to_string())))
            .id();

        for action in [
            EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(
                "no_such_id",
                "GHOST",
            )),
            EventActionConfig::ObjectiveMarkerDetach(ObjectiveMarkerDetachActionConfig::new(
                "no_such_id",
            )),
        ] {
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            action.action(&mut event_world, &GameEventInfo::default());
            NovaEventWorld::state_to_world_system(&mut world);
        }

        assert!(world.get_entity(bystander).is_ok());
        assert!(world.get::<ObjectiveMarkerTarget>(bystander).is_none());
    }

    /// The emphasis pair mutates nova_gameplay's HintEmphasis resource
    /// through the queued-command drain; without the resource (headless
    /// scenario rigs) both are warn-and-continue no-ops.
    #[test]
    fn hint_emphasis_actions_drive_the_resource() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // Without the resource: harmless.
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        // With it: set lands, clear drops.
        world.init_resource::<HintEmphasis>();
        let set = HintEmphasisSetActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        set.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(world.resource::<HintEmphasis>().contains("GOTO"));

        let clear = HintEmphasisClearActionConfig::new("GOTO");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        clear.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(!world.resource::<HintEmphasis>().contains("GOTO"));
    }

    /// SetControllerVerb flips exactly the addressed ship's controller verb,
    /// leaving other verbs on that controller and other ships untouched; and
    /// re-enabling restores it. If the action did not scope by ship id, the
    /// bystander ship's controller would flip too and this test would fail.
    #[test]
    fn set_controller_verb_flips_only_the_scoped_ship() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // The target ship and a bystander ship, each a scoped root with a
        // controller section carrying no WithheldVerbs (all granted, the
        // production default - disabling must materialize the component).
        let player = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("player".to_string()),
            ))
            .id();
        let player_ctrl = world.spawn((ChildOf(player), ControllerSectionMarker)).id();
        let bystander = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("bystander".to_string()),
            ))
            .id();
        let bystander_ctrl = world
            .spawn((ChildOf(bystander), ControllerSectionMarker))
            .id();

        // Disable GOTO on the player only.
        let disable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let pv = world.get::<WithheldVerbs>(player_ctrl).unwrap();
        assert!(
            !pv.granted(FlightVerb::Goto),
            "GOTO disabled on the addressed ship"
        );
        assert!(
            pv.granted(FlightVerb::Stop) && pv.granted(FlightVerb::Orbit),
            "other verbs on that controller untouched"
        );
        assert!(
            world
                .get::<WithheldVerbs>(bystander_ctrl)
                .is_none_or(|w| w.granted(FlightVerb::Goto)),
            "the bystander ship's controller is untouched (still grants GOTO)"
        );

        // Re-enable restores it.
        let enable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: true,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        enable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world
                .get::<WithheldVerbs>(player_ctrl)
                .unwrap()
                .granted(FlightVerb::Goto),
            "GOTO re-enabled on the addressed ship"
        );
    }

    /// SetControllerVerb writes EVERY controller section on the ship, so the
    /// union the input layer reads (verb available if ANY live controller
    /// grants it) reflects the change no matter which controller it samples.
    #[test]
    fn set_controller_verb_writes_all_controllers_on_the_ship() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let ship = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("twin".to_string()),
            ))
            .id();
        let ctrl_a = world.spawn((ChildOf(ship), ControllerSectionMarker)).id();
        let ctrl_b = world.spawn((ChildOf(ship), ControllerSectionMarker)).id();

        let disable = SetControllerVerbActionConfig {
            id: "twin".to_string(),
            verb: FlightVerb::Stop,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            !world
                .get::<WithheldVerbs>(ctrl_a)
                .unwrap()
                .granted(FlightVerb::Stop),
            "first controller written"
        );
        assert!(
            !world
                .get::<WithheldVerbs>(ctrl_b)
                .unwrap()
                .granted(FlightVerb::Stop),
            "second controller written too"
        );
    }

    /// Every dynamic scenario body must interpolate its Transform between
    /// fixed physics ticks, or it stair-steps under the smoothed chase
    /// camera (task 20260709-160753).
    #[test]
    fn scenario_objects_interpolate_their_transforms() {
        let mut world = World::new();
        let entity = world
            .spawn(base_scenario_object(&BaseScenarioObjectConfig {
                id: "test".to_string(),
                name: "Test".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            }))
            .id();
        assert!(world.get::<TransformInterpolation>(entity).is_some());
    }

    /// SetCamera pins a `ScriptedCameraPose` on the scenario camera and drops
    /// WASD control (the loader's enforcer then applies the pose every frame, so
    /// it holds against the free-fly state machine). Mirrors the despawn harness:
    /// fire into a `NovaEventWorld`, drain, assert on the world.
    #[test]
    fn set_camera_pins_a_scripted_pose_and_drops_wasd() {
        use bevy_common_systems::prelude::{EventWorld, WASDCameraController};

        use crate::prelude::{ScenarioCameraMarker, ScriptedCameraPose};

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let camera = world
            .spawn((
                ScenarioCameraMarker,
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();

        let action = SetCameraActionConfig {
            position: Vec3::new(5.0, 6.0, 7.0),
            look_at: Vec3::ZERO,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let pose = world
            .get::<ScriptedCameraPose>(camera)
            .expect("the camera is pinned to a scripted pose");
        assert_eq!(pose.position, Vec3::new(5.0, 6.0, 7.0));
        assert_eq!(pose.look_at, Vec3::ZERO);
        assert!(
            world.get::<WASDCameraController>(camera).is_none(),
            "WASD control is dropped so free-fly input stops"
        );
    }

    /// SetCamera against a world with no scenario camera is a warn-and-continue
    /// no-op, not a panic (a headless rig without the loader's camera).
    #[test]
    fn set_camera_without_a_camera_is_harmless() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let bystander = world.spawn(Transform::default()).id();

        let action = SetCameraActionConfig {
            position: Vec3::ONE,
            look_at: Vec3::ZERO,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(world.get_entity(bystander).is_ok());
    }

    /// The Screenshot action queues a capture without panicking on a world with
    /// no render backend (the `save_to_disk` observer simply never fires): the
    /// drain must complete and a `Screenshot` request entity must exist. A bare
    /// filename has no parent dir, so the action writes nothing to disk here.
    #[test]
    fn screenshot_action_queues_a_capture_without_render() {
        use bevy_common_systems::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let action = ScreenshotActionConfig::new("nova_test_shot.png");
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let requests = world.query::<&Screenshot>().iter(&world).count();
        assert_eq!(requests, 1, "exactly one capture request is spawned");
    }

    /// `resolve_capture_path_in` joins relative paths under the capture dir,
    /// leaves absolute paths alone, and is a no-op without a dir. Tests the pure
    /// core so no process-wide env mutation is needed.
    #[test]
    fn resolve_capture_path_honors_the_capture_dir() {
        use std::path::Path;

        // A relative path is joined under the capture dir.
        assert_eq!(
            resolve_capture_path_in("feature-gravity.png", Some("/tmp/nova-shots")),
            Path::new("/tmp/nova-shots/feature-gravity.png")
        );
        // No capture dir: the relative path is used as-is.
        assert_eq!(
            resolve_capture_path_in("feature-gravity.png", None),
            Path::new("feature-gravity.png")
        );
        // An absolute path passes through even with a capture dir set.
        assert_eq!(
            resolve_capture_path_in("/shots/a.png", Some("/tmp/nova-shots")),
            Path::new("/shots/a.png")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn set_camera_config_round_trips_through_ron() {
        let config = SetCameraActionConfig {
            position: Vec3::new(1.0, 2.0, 3.0),
            look_at: Vec3::new(-1.0, 0.0, 5.0),
        };
        let ron = ron::to_string(&config).expect("serialize");
        let back: SetCameraActionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.position, config.position);
        assert_eq!(back.look_at, config.look_at);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn screenshot_config_round_trips_through_ron() {
        let config = ScreenshotActionConfig::new("shots/feature-gravity.png");
        let ron = ron::to_string(&config).expect("serialize");
        let back: ScreenshotActionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.path, config.path);
    }

    /// A modder authors `SetSkybox` in RON as a bare cubemap path (the `AssetRef`
    /// shape), so the whole action must round-trip through serde. Confirms the new
    /// hook is reachable from a data file, not just from code.
    #[cfg(feature = "serde")]
    #[test]
    fn set_skybox_action_round_trips_through_ron() {
        let action =
            EventActionConfig::SetSkybox(SetSkyboxActionConfig::new("scenarios/nebula.cube.png"));
        let ron = ron::to_string(&action).expect("serialize");
        let back: EventActionConfig = ron::from_str(&ron).expect("deserialize");
        match back {
            EventActionConfig::SetSkybox(config) => {
                assert_eq!(config.cubemap.path(), Some("scenarios/nebula.cube.png"));
                assert_eq!(config.brightness, None);
            }
            other => panic!("expected SetSkybox, got {other:?}"),
        }
    }
}

/// Which kind of scenario object to spawn, carrying that kind's config.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScenarioObjectKind {
    /// A destructible rock with a gravity well.
    Asteroid(AsteroidConfig),
    /// A ship built from sections, with a controller (None/Player/AI).
    Spaceship(SpaceshipConfig),
    /// A nav waypoint with an automatic HUD chip.
    Beacon(BeaconConfig),
    /// A proximity pickup crate that fires `OnEnter` when flown through.
    SalvageCrate(SalvageCrateConfig),
}

impl EventAction<NovaEventWorld> for ScenarioObjectConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!("SpawnScenarioObject: spawning '{}'", config.base.id);

        world.push_command(move |commands| {
            let mut entity_commands = commands.spawn(base_scenario_object(&config.base));

            match &config.kind {
                ScenarioObjectKind::Asteroid(config) => {
                    entity_commands.insert(asteroid_scenario_object(config.clone()));
                }
                ScenarioObjectKind::Spaceship(config) => {
                    entity_commands.insert(spaceship_scenario_object(config.clone()));
                    // The authored allegiance override. Ordering is safe
                    // either way: observer-queued commands (the controller
                    // marker whose requirement defaults Player/Enemy) apply
                    // BEFORE this queue's remaining commands (ledger:
                    // verify-engine-guarantees-in-source), and a plain
                    // insert overwrites the requirement default - so the
                    // authored side always wins.
                    if let Some(allegiance) = config.allegiance {
                        entity_commands.insert(allegiance);
                    }
                }
                ScenarioObjectKind::Beacon(config) => {
                    entity_commands.insert(beacon_scenario_object(config.clone()));
                }
                ScenarioObjectKind::SalvageCrate(config) => {
                    entity_commands.insert(salvage_crate_scenario_object(config.clone()));
                }
            }
        });
    }
}

/// A volume to scatter objects within, for [`ScatterObjectsConfig`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScatterRegion {
    /// An axis-aligned box; each object is placed uniformly per-axis in
    /// `[min, max]`.
    Box {
        /// The box's minimum corner.
        min: Vec3,
        /// The box's maximum corner.
        max: Vec3,
    },
    /// A horizontal annulus centred on the origin: uniform angle, radius in
    /// `[inner, outer]`, height in `[y_min, y_max]`.
    Ring {
        /// The annulus inner radius.
        inner: f32,
        /// The annulus outer radius.
        outer: f32,
        /// The lower bound of the vertical (y) spread.
        y_min: f32,
        /// The upper bound of the vertical (y) spread.
        y_max: f32,
    },
}

impl ScatterRegion {
    /// Sample a position in the region. `random_in` guards empty ranges
    /// (`a >= b` yields `a`) so a degenerate authored region cannot panic.
    fn sample(&self, rng: &mut impl rand::Rng) -> Vec3 {
        fn random_in(rng: &mut impl rand::Rng, a: f32, b: f32) -> f32 {
            use rand::RngExt;
            if a < b {
                rng.random_range(a..b)
            } else {
                a
            }
        }
        match self {
            ScatterRegion::Box { min, max } => Vec3::new(
                random_in(rng, min.x, max.x),
                random_in(rng, min.y, max.y),
                random_in(rng, min.z, max.z),
            ),
            ScatterRegion::Ring {
                inner,
                outer,
                y_min,
                y_max,
            } => {
                let angle = random_in(rng, 0.0, std::f32::consts::TAU);
                let dist = random_in(rng, *inner, *outer);
                Vec3::new(
                    angle.cos() * dist,
                    random_in(rng, *y_min, *y_max),
                    angle.sin() * dist,
                )
            }
        }
    }
}

/// Spawn `count` copies of a template object scattered through a region, with a
/// deterministic seed so the layout is reproducible across loads. Each copy is a
/// clone of `template` with `base.id = "{id_prefix}{i}"` and a sampled position;
/// when `asteroid_radius` is set and the template is an asteroid, its radius is
/// randomized too. This is the declarative form of a procedural asteroid field.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScatterObjectsConfig {
    /// The id prefix each copy gets (`"{id_prefix}{i}"`).
    pub id_prefix: String,
    /// How many copies to spawn.
    pub count: u32,
    /// The RNG seed, so the layout is reproducible across loads.
    pub seed: u64,
    /// The region copies are scattered within.
    pub region: ScatterRegion,
    /// The template object each copy clones.
    pub template: ScenarioObjectConfig,
    /// If set and `template.kind` is an asteroid, randomize each rock's radius in
    /// this `[lo, hi]` range.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub asteroid_radius: Option<(f32, f32)>,
}

impl EventAction<NovaEventWorld> for ScatterObjectsConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        use rand::{RngExt, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(self.seed);
        // Always spawn the authored count. Scatter is gameplay content (asteroid /
        // debris fields), so no graphics-quality tier thins it - the density lever
        // was removed in task 20260718-004834.
        let count = self.count;
        debug!(
            "ScatterObjects: scattering {} '{}' objects (seed {})",
            count, self.id_prefix, self.seed
        );

        for i in 0..count {
            let mut object = self.template.clone();
            object.base.id = format!("{}{}", self.id_prefix, i);
            object.base.name = format!("{} {}", self.template.base.name, i);
            object.base.position = self.region.sample(&mut rng);

            if let (Some((lo, hi)), ScenarioObjectKind::Asteroid(asteroid)) =
                (self.asteroid_radius, &mut object.kind)
            {
                asteroid.radius = if lo < hi {
                    rng.random_range(lo..hi)
                } else {
                    lo
                };
            }

            // Reuse the ordinary spawn path so scatter and SpawnScenarioObject
            // stay identical in how they build an object.
            object.action(world, info);
        }
    }
}

/// A spherical sensor zone that drives `OnEnter`/`OnExit` when a body crosses
/// its boundary.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScenarioAreaConfig {
    /// The area's scenario `EntityId` (the `id` reported by `OnEnter`/`OnExit`).
    pub id: String,
    /// The area's display name.
    pub name: String,
    /// The area's world position (sphere centre).
    pub position: Vec3,
    /// The area's world rotation.
    pub rotation: Quat,
    /// The sphere radius.
    pub radius: f32,
}

impl EventAction<NovaEventWorld> for ScenarioAreaConfig {
    fn action(&self, world: &mut NovaEventWorld, _info: &GameEventInfo) {
        let config = self.clone();
        debug!(
            "CreateScenarioArea: creating area '{}' (radius: {})",
            config.id, config.radius
        );

        world.push_command(move |commands| {
            commands.spawn((
                ScenarioScopedMarker,
                ScenarioAreaMarker,
                Name::new(config.name.clone()),
                EntityId::new(config.id.clone()),
                Transform::from_translation(config.position).with_rotation(config.rotation),
                RigidBody::Static,
                Collider::sphere(config.radius),
                Sensor,
                Visibility::Visible,
            ));
        });
    }
}
