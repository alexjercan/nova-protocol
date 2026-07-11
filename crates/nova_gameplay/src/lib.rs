//! Gameplay related functionality for Nova Protocol.
//!
//! Nova Protocol specific systems and components.

use bevy::prelude::*;

pub mod audio;
pub mod camera_controller;
pub mod flight;
pub mod gravity;
pub mod hud;
pub mod input;
pub mod integrity;
pub mod juice;
pub mod plugin;
pub mod relations;
pub mod sections;

pub use bevy_common_systems;

pub mod prelude {
    // Re-export bevy_common_systems prelude
    pub use bevy_common_systems::prelude::*;

    pub use super::{
        audio::{NovaAudioPlugin, NovaSfx, SfxListenerMarker, NOVA_SFX_FILES},
        camera_controller::prelude::*,
        flight::prelude::*,
        gravity::prelude::*,
        hud::prelude::*,
        input::prelude::*,
        integrity::prelude::*,
        juice::prelude::*,
        plugin::{NovaGameplayPlugin, SpaceshipSystems},
        relations::prelude::*,
        sections::prelude::*,
        GameMode, GameStates,
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
