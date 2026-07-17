//! Gameplay related functionality for Nova Protocol.
//!
//! Nova Protocol specific systems and components.

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
/// assert on the log itself (tasks 20260712-115902, 20260713-175352).
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

pub mod prelude {
    // Re-export bevy_common_systems prelude
    pub use bevy_common_systems::prelude::*;

    pub use super::{
        asset_ref::prelude::*,
        audio::{
            load_world_sfx_bank, NovaAudioPlugin, SfxListenerMarker, UiSfx, WorldSfx,
            MENU_SELECT_VOLUME, SALVAGE_PICKUP_VOLUME, UI_SFX_FILES, UI_TOGGLE_VOLUME,
            WORLD_SFX_FILES,
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
        settings::{GraphicsBudget, GraphicsQuality, MasterVolume, NovaSettingsPlugin},
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
    Loading,
    MainMenu,
    Playing,
}

/// Whether gameplay is frozen behind the pause overlay (task
/// 20260711-185156). Owned UI-wise by `nova_menu` (ESC toggle + overlay);
/// `nova_gameplay` gates the spaceship input/section system sets on
/// `Unpaused`, and the clocks (`Time<Virtual>` + `Time<Physics>`) pause on
/// entering `Paused`. Init'd by `AppBuilder` next to [`GameStates`]. Only
/// meaningful inside `GameStates::Playing`; leaving Playing must reset it.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum PauseStates {
    #[default]
    Unpaused,
    Paused,
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
