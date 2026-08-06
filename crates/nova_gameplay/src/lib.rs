//! `nova_gameplay` is the game itself: the systems and components that make a
//! ship fly, fight, and take damage. `NovaGameplayPlugin` composes it and owns
//! the top-level [`GameStates`] state machine. The modules span the whole
//! playable surface - `sections` (the modular ship parts), `integrity` and
//! `damage` (health, disable, destroy), `flight` and `gravity` (the diegetic
//! controller, autopilot verbs, and gravity wells), `input` (player, AI, and
//! radar targeting), `hud`, `camera_controller`, `audio`, `juice` (camera
//! shake and hit feedback), and `settings` (volume + graphics presets). It
//! builds on the reusable `bevy_common_systems` layer for integrity, health,
//! and blast mechanics.
#![warn(missing_docs)]

use bevy::prelude::*;

pub mod asset_ref;
pub mod audio;
pub mod beacon;
pub mod camera_controller;
pub mod damage;
pub mod flight;
pub mod gravity;
pub mod hud;
pub mod input;
pub mod integrity;
pub mod juice;
pub mod objective_marker;
pub mod plugin;
pub mod relations;
pub mod sections;
pub mod settings;

pub use bevy_common_systems;

/// Test-only helper for asserting on log output: a shared in-memory sink
/// installed as the thread's tracing subscriber. `EntityCommands::remove` and
/// `despawn` bake in the WARN handler at queue time (bevy_ecs
/// commands/mod.rs `queue_handled(_, warn)`), so a `FallbackErrorHandler`
/// swap can never see them - a "no stale command" regression test must
/// assert on the log itself.
#[cfg(test)]
pub(crate) mod test_log {
    /// Cloneable in-memory log sink; every clone shares the same buffer.
    #[derive(Clone, Default)]
    pub(crate) struct CapturedLog(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl CapturedLog {
        pub(crate) fn contents(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
        pub(crate) fn clear(&self) {
            self.0.lock().unwrap().clear();
        }
    }

    impl std::io::Write for CapturedLog {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

/// Glob-import surface: `use nova_gameplay::prelude::*` re-exports the public API
/// of this crate's submodules plus the top-level game-state enums.
pub mod prelude {
    // The `bevy_common_systems` names nova's own gameplay code is written in,
    // re-exported BY NAME. A glob (`pub use bevy_common_systems::prelude::*`)
    // used to stand here, and it also dragged in the retired bcs harness twins
    // - `AutopilotPlugin`, `AutopilotLoop`, `ScreenshotPlugin`,
    // `ScreenshotReelPlugin`, `HarnessCompletion` - which shadow nova's
    // harness at every example's `use nova_protocol::prelude::*` and boot the
    // example INERT (task 20260802-183403). Keep this list explicit: adding a
    // name here is a decision, and the twins are never on it.
    pub use bevy_common_systems::prelude::{
        destructible_body, status_bar, status_bar_item, status_fps_color_fn, status_fps_value_fn,
        status_version_color_fn, status_version_value_fn, CameraShake, CameraShakeInput,
        CameraShakePlugin, ChaseCamera, ChaseCameraInput, Cooldown, DirectionalSphereOrbitOutput,
        GameObjectives, Health, HealthApplyDamage, HealthZeroMarker, IntegrityDestroyMarker,
        Objective, ObjectivesPlugin, PDController, PDControllerInput, PDControllerPlugin,
        PDControllerSystems, PDControllerTarget, PlaySfx, PointRotation, PointRotationOutput,
        PostProcessingCamera, SfxCommandsExt, SfxPlugin, SkyboxConfig, SoundBank,
        StatusBarItemConfig, StatusBarRootConfig, WASDCameraController,
    };

    pub use super::{
        asset_ref::prelude::*,
        audio::{
            NovaAudioPlugin, SfxListenerMarker, UiSfx, MENU_SELECT_VOLUME, SALVAGE_PICKUP_VOLUME,
            UI_SFX_FILES, UI_TOGGLE_VOLUME,
        },
        beacon::prelude::*,
        camera_controller::prelude::*,
        damage::prelude::*,
        flight::prelude::*,
        gravity::prelude::*,
        hud::prelude::*,
        input::prelude::*,
        integrity::prelude::*,
        juice::prelude::*,
        objective_marker::prelude::*,
        plugin::{NovaGameplayPlugin, SpaceshipSystems},
        relations::prelude::*,
        sections::prelude::*,
        settings::{
            GraphicsBudget, GraphicsQuality, HarnessMute, MasterVolume, NovaSettingsPlugin,
        },
        GameMode, GameStates, PauseStates,
    };
}

/// Top-level game lifecycle state.
///
/// `Loading` while assets load, `MainMenu` while the main menu (owned by `nova_menu`)
/// is up, `Playing` once the game is running. Apps without the menu (examples that
/// supply their own game plugins) go straight `Loading -> Playing`. Lives here (the
/// foundational gameplay crate) so the wiring layer (`nova_core`), the editor
/// (`nova_editor`) and the menu (`nova_menu`) can gate on it without depending on
/// each other.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameStates {
    #[default]
    /// Assets are still loading; no menu or gameplay yet.
    Loading,
    /// The `nova_menu` main menu is up.
    MainMenu,
    /// The game is running.
    Playing,
}

/// Whether gameplay is frozen behind a modal overlay. Owned UI-wise by
/// `nova_menu` (ESC toggle + overlay) and `nova_gameplay`'s Tab ship-computer
/// NOVA OS; `nova_gameplay` gates the spaceship input/section system sets on
/// `Unpaused`, and the clocks (`Time<Virtual>` + `Time<Physics>`) pause on
/// entering any frozen variant. Init'd by `AppBuilder` next to [`GameStates`].
/// Only meaningful inside `GameStates::Playing`; leaving Playing must reset it.
///
/// Both frozen variants ([`PauseStates::Paused`] and [`PauseStates::NovaOs`])
/// are entered ONLY from [`PauseStates::Unpaused`] and exit back to it - never
/// one directly into the other - so the freeze/cursor hooks never
/// double-fire.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum PauseStates {
    #[default]
    /// Gameplay is running; input and section systems tick.
    Unpaused,
    /// Gameplay is frozen behind the pause overlay; the clocks are stopped.
    Paused,
    /// Gameplay is frozen behind the Tab ship-computer NOVA OS; the clocks are
    /// stopped and the cursor is freed, exactly like [`PauseStates::Paused`]
    /// but without the pause menu.
    NovaOs,
}

impl PauseStates {
    /// True when gameplay is frozen (any
    /// non-[`Unpaused`](PauseStates::Unpaused) variant): the clocks are stopped
    /// and input observers must suppress. This is the predicate the ~18
    /// observer self-guards use instead of comparing against a single variant,
    /// so a new frozen overlay is covered without re-auditing every guard.
    pub fn is_frozen(&self) -> bool {
        *self != PauseStates::Unpaused
    }
}

/// Which game the main menu handed off to when it set [`GameStates::Playing`].
///
/// `Sandbox` is the default so apps that skip the menu keep the pre-menu behavior
/// (the editor comes up on entering `Playing`). Initialized by `NovaGameplayPlugin`;
/// written by the menu buttons, read on `OnEnter(GameStates::Playing)` by the editor
/// (enter only in `Sandbox`) and the menu's New Game loader (only in `NewGame`).
#[derive(Resource, Clone, Copy, Eq, PartialEq, Debug, Hash, Default, Reflect)]
#[reflect(Resource)]
pub enum GameMode {
    /// Ship editor plus its sandbox scenario (the default game).
    #[default]
    Sandbox,
    /// Jump straight into a ready-to-play scenario.
    NewGame,
}
