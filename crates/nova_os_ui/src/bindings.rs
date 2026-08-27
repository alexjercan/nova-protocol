//! The NOVA OS vocabulary: every named action the monitor and its apps answer
//! to, as plain data.
//!
//! These are the DEFAULTS, not the live table - [`crate::terminal::NovaOsPlugin`]
//! registers them into [`InputBindings`](nova_input::prelude::InputBindings) at
//! build, and every reader looks them up by name from there, so a rebind moves
//! the monitor with it.
//!
//! None of these are `bevy_enhanced_input` rigs. A rig spawns with the player
//! ship, and the monitor has to answer with the ship's rig torn down (the
//! toggle) or while the freeze axis holds the world still (the apps).

use bevy::prelude::*;
use nova_input::prelude::*;

/// Opening and closing the monitor, plus the shared controls its apps read.
///
/// The two viewers share the orbit, reframe and cycle verbs deliberately: `map`
/// and `ship` are the same instrument pointed at different subjects, and a
/// player who learns to fly one has learned the other. Only the verbs that act
/// on the subject are per-app.
pub fn novaos_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard};
    vec![
        // RightThumb is the one free pad button, mirroring `nova_menu`'s
        // optional-gamepad guard.
        ActionBinding::new("novaos_toggle", "SYSTEM", "NOVA OS")
            .keyboard([Keyboard(KeyCode::Tab)])
            .gamepad([Gamepad(GamepadButton::RightThumb)]),
    ]
}
