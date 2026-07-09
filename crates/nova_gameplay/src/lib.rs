//! Gameplay related functionality for Nova Protocol.
//!
//! Nova Protocol specific systems and components.

use bevy::prelude::*;

pub mod audio;
pub mod camera_controller;
pub mod flight;
pub mod hud;
pub mod input;
pub mod integrity;
pub mod juice;
pub mod plugin;
pub mod sections;

pub use bevy_common_systems;

pub mod prelude {
    // Re-export bevy_common_systems prelude
    pub use bevy_common_systems::prelude::*;

    pub use super::{
        audio::{NovaAudioPlugin, NovaSfx, NOVA_SFX_FILES},
        camera_controller::prelude::*,
        flight::prelude::*,
        hud::prelude::*,
        input::prelude::*,
        integrity::prelude::*,
        juice::prelude::*,
        plugin::{NovaGameplayPlugin, SpaceshipSystems},
        sections::prelude::*,
        GameStates,
    };
}

/// Top-level game lifecycle state.
///
/// `Loading` while assets load, `Playing` once the game is running. Lives here (the
/// foundational gameplay crate) so both the thin wiring layer (`nova_core`) and the
/// editor (`nova_editor`) can gate on it without depending on each other.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameStates {
    #[default]
    Loading,
    Playing,
}
