//! Gameplay related functionality for Nova Protocol.
//!
//! Nova Protocol specific systems and components.

use bevy::prelude::*;

pub mod audio;
pub mod beacon;
pub mod camera_controller;
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

pub use bevy_common_systems;

pub mod prelude {
    // Re-export bevy_common_systems prelude
    pub use bevy_common_systems::prelude::*;

    pub use super::{
        audio::{NovaAudioPlugin, NovaSfx, SfxListenerMarker, NOVA_SFX_FILES},
        beacon::prelude::*,
        camera_controller::prelude::*,
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
