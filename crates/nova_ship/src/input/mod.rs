//! What decides where a ship goes and when it shoots. Four producers feed the
//! same section inputs: [`player`] (human keybinds, flight verbs, weapon fire),
//! [`ai`] (the enemy behavior state machine), [`targeting`] (the player's
//! lock/radar system that also derives weapons-safety) and [`point_defense`]
//! (the autonomous answer to inbound ordnance, which both controllers share).
//! [`keybind_reference`](prelude::keybind_reference)
//! exposes the keybind table for the HUD. [`SpaceshipInputPlugin`] adds all
//! four.
//!
//! Touch this module when adding a new way to command a ship. The intents these
//! produce are consumed by the section plugins ([`sections`](crate::sections))
//! and the flight controller ([`flight`](crate::flight)).

use bevy::prelude::*;

pub mod ai;
pub mod player;
pub mod point_defense;
mod reference;
pub mod targeting;

/// The AI, player, targeting and point-defence preludes, the keybind reference,
/// and `SpaceshipInputPlugin` with `SpaceshipInputSystems`.
pub mod prelude {
    pub use super::{
        ai::prelude::*,
        player::prelude::*,
        point_defense::prelude::*,
        reference::{keybind_reference, KeybindEntry},
        targeting::prelude::*,
        SpaceshipInputPlugin, SpaceshipInputSystems,
    };
}

/// System set holding all input production (player, AI, targeting), ordered
/// first among the gameplay sets so downstream sections/flight read fresh intent.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;

/// Adds the player, targeting, point-defence and AI input plugins - everything
/// that commands a ship. Added by
/// [`NovaGameplayPlugin`](nova_gameplay::plugin::NovaGameplayPlugin).
#[derive(Default)]
pub struct SpaceshipInputPlugin {
    /// Whether the render-side half is added: the point-defence lines. Mirrors
    /// `NovaShipPlugin::render`.
    pub render: bool,
}

impl Plugin for SpaceshipInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipInputPlugin: build");

        app.add_plugins(player::SpaceshipPlayerInputPlugin);
        app.add_plugins(targeting::SpaceshipTargetingPlugin);
        app.add_plugins(point_defense::SpaceshipPointDefensePlugin {
            render: self.render,
        });
        app.add_plugins(ai::SpaceshipAIInputPlugin);
    }
}
