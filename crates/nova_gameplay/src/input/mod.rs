//! What decides where a ship goes and when it shoots. Three producers feed the
//! same section inputs: [`player`] (human keybinds, flight verbs, weapon fire),
//! [`ai`] (the enemy behavior state machine), and [`targeting`] (the player's
//! lock/radar system that also derives weapons-safety). [`reference`](mod@reference) exposes
//! the keybind table for the HUD. [`SpaceshipInputPlugin`] adds all three.
//!
//! Touch this module when adding a new way to command a ship. The intents these
//! produce are consumed by the section plugins ([`sections`](crate::sections))
//! and the flight controller ([`flight`](crate::flight)).

use bevy::prelude::*;

pub mod ai;
pub mod player;
pub mod reference;
pub mod targeting;

/// Glob-import surface: `use nova_gameplay::input::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        ai::prelude::*,
        player::prelude::*,
        reference::{keybind_reference, KeybindEntry},
        targeting::prelude::*,
        SpaceshipInputPlugin, SpaceshipInputSystems,
    };
}

/// System set holding all input production (player, AI, targeting), ordered
/// first among the gameplay sets so downstream sections/flight read fresh intent.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;

/// Adds the player, targeting and AI input plugins - everything that commands a
/// ship. Added by [`NovaGameplayPlugin`](crate::plugin::NovaGameplayPlugin).
pub struct SpaceshipInputPlugin;

impl Plugin for SpaceshipInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipInputPlugin: build");

        app.add_plugins(player::SpaceshipPlayerInputPlugin);
        app.add_plugins(targeting::SpaceshipTargetingPlugin);
        app.add_plugins(ai::SpaceshipAIInputPlugin);
    }
}
