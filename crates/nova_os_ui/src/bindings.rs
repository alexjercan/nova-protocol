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
/// on the subject are per-app - and `map_goto` and `ship_mates` may share a key
/// for the same reason, because only one app owns the screen at a time.
///
/// The viewer actions carry no pad binding. They never had one: the monitor is
/// a keyboard surface, and inventing pad buttons here would reserve them
/// against the flight rig for a gesture nobody asked for.
///
/// Three firing contexts, and the split is what makes the shared keys legal.
/// `novaos_toggle` is `Always` - it has to open the monitor from wherever the
/// player is. The orbit and cycle verbs are `Viewer`: live while ANY app owns
/// the screen, quiet at the prompt, where the keyboard is typing. The per-app
/// verbs are `ViewerApp`, so `map_goto` and `ship_mates` can both hold `G`
/// without colliding, and `novaos_next` and a future `map_next` could not.
///
/// # What the pad reaches
///
/// `Viewer` does not overlap `Flight`, so the viewer verbs REUSE the flight
/// rig's buttons rather than hunting for spare ones: the D-pad orbits, the
/// triggers dolly, the bumpers step the selection, the left stick press
/// reframes. Only the two `Always` actions (the NOVA OS toggle on Right Thumb,
/// the HUD cycle on View) are off limits, plus the fixed Menu pause chord.
///
/// A pad runs out before the app verbs do. `ViewerApp` overlaps `Viewer`, so
/// only the two face buttons the camera does not use are left - A and Y, which
/// go to what each app does MOST. `ship_repair` and `ship_rebind` stay
/// keyboard-only rather than take a camera verb's button away.
pub fn novaos_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard};
    vec![
        // RightThumb, and only RightThumb: this is `Always`, so it collides
        // with every context at once.
        ActionBinding::new("novaos_toggle", "SYSTEM", "NOVA OS")
            .context(ActionContext::Always)
            .keyboard([Keyboard(KeyCode::Tab)])
            .gamepad([Gamepad(GamepadButton::RightThumb)]),
        // The shared viewer controls. `map` and `ship` both drive an orbit
        // camera over a schematic, so they answer the same verbs.
        ActionBinding::new("novaos_orbit_left", "NOVA OS", "Turn Left")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyQ)])
            .gamepad([Gamepad(GamepadButton::DPadLeft)]),
        ActionBinding::new("novaos_orbit_right", "NOVA OS", "Turn Right")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyE)])
            .gamepad([Gamepad(GamepadButton::DPadRight)]),
        ActionBinding::new("novaos_orbit_up", "NOVA OS", "Tilt Up")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyR)])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("novaos_orbit_down", "NOVA OS", "Tilt Down")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyF)])
            .gamepad([Gamepad(GamepadButton::DPadDown)]),
        ActionBinding::new("novaos_pan_forward", "NOVA OS", "Pan Forward")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyW)])
            .gamepad([Gamepad(GamepadButton::RightTrigger2)]),
        ActionBinding::new("novaos_pan_back", "NOVA OS", "Pan Back")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyS)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger2)]),
        ActionBinding::new("novaos_pan_left", "NOVA OS", "Pan Left")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyA)])
            .gamepad([Gamepad(GamepadButton::West)]),
        ActionBinding::new("novaos_pan_right", "NOVA OS", "Pan Right")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyD)])
            .gamepad([Gamepad(GamepadButton::East)]),
        ActionBinding::new("novaos_reframe", "NOVA OS", "Reset View")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyT)])
            .gamepad([Gamepad(GamepadButton::LeftThumb)]),
        ActionBinding::new("novaos_next", "NOVA OS", "Select Next")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::BracketRight)])
            .gamepad([Gamepad(GamepadButton::RightTrigger)]),
        ActionBinding::new("novaos_prev", "NOVA OS", "Select Previous")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::BracketLeft)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger)]),
        // What each app does to the thing it has selected.
        ActionBinding::new("map_goto", "MAP", "Set GOTO")
            .context(ActionContext::ViewerApp("map"))
            .keyboard([Keyboard(KeyCode::KeyG)])
            .gamepad([Gamepad(GamepadButton::South)]),
        ActionBinding::new("ship_mates", "SHIP", "Mates Overlay")
            .context(ActionContext::ViewerApp("ship"))
            .keyboard([Keyboard(KeyCode::KeyG)])
            .gamepad([Gamepad(GamepadButton::South)]),
        ActionBinding::new("ship_reload", "SHIP", "Reload Section")
            .context(ActionContext::ViewerApp("ship"))
            .keyboard([Keyboard(KeyCode::KeyL)])
            .gamepad([Gamepad(GamepadButton::North)]),
        ActionBinding::new("ship_repair", "SHIP", "Repair Section")
            .context(ActionContext::ViewerApp("ship"))
            .keyboard([Keyboard(KeyCode::KeyP)]),
        ActionBinding::new("ship_rebind", "SHIP", "Rebind Section Key")
            .context(ActionContext::ViewerApp("ship"))
            .keyboard([Keyboard(KeyCode::KeyB)]),
    ]
}

/// One footer hint: the first key of each action in `actions`, joined with `/`,
/// then the verb the group performs - `"Q/E: TURN"`.
///
/// Reads the LIVE table, so the monitor's footer follows a rebind instead of
/// printing the key the game shipped with. An action with nothing bound
/// contributes no key, and a hint left with no keys at all is dropped rather
/// than shown as a bare verb.
pub fn hint(bindings: &InputBindings, actions: &[&str], verb: &str) -> Option<String> {
    let keys: Vec<String> = actions
        .iter()
        .filter_map(|name| bindings.get(name))
        .filter_map(|action| action.keyboard.first())
        .map(InputSource::readout_label)
        .collect();
    (!keys.is_empty()).then(|| format!("{}: {verb}", keys.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hint_names_the_key_each_action_is_bound_to_now() {
        let mut bindings = InputBindings::from_actions(novaos_bindings());
        assert_eq!(
            hint(
                &bindings,
                &["novaos_orbit_left", "novaos_orbit_right"],
                "TURN"
            ),
            Some("Q/E: TURN".to_string())
        );
        assert_eq!(
            hint(&bindings, &["novaos_next", "novaos_prev"], "CYCLE"),
            Some("]/[: CYCLE".to_string()),
            "the punctuation keys print as their symbols, not their KeyCode names"
        );

        bindings.rebind(
            "novaos_orbit_left",
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
                gamepad: Vec::new(),
            },
        );
        assert_eq!(
            hint(
                &bindings,
                &["novaos_orbit_left", "novaos_orbit_right"],
                "TURN"
            ),
            Some("J/E: TURN".to_string()),
            "the footer follows the move"
        );
    }

    #[test]
    fn a_hint_with_nothing_bound_is_no_hint_at_all() {
        let bindings = InputBindings::default();
        assert_eq!(hint(&bindings, &["novaos_reframe"], "RESET"), None);
    }
}
