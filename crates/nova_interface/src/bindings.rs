//! The TAB interface vocabulary: every named action the interface and its panes
//! answer to, as plain data.
//!
//! These are the DEFAULTS, not the live table - [`crate::InterfacePlugin`]
//! registers them into [`InputBindings`](nova_input::prelude::InputBindings) at
//! build, and every reader looks them up by name from there, so a rebind moves
//! the interface with it.
//!
//! None of these are `bevy_enhanced_input` rigs. A rig spawns with the player
//! ship, and the interface has to answer while the freeze axis holds the world
//! still.

use bevy::prelude::*;
use nova_input::prelude::*;

/// Opening and closing the interface, plus the shared controls its panes read.
///
/// The two viewers share the orbit, reframe and cycle verbs deliberately: `map`
/// and `ship` are the same instrument pointed at different subjects, and a
/// player who learns to fly one has learned the other. Only the verbs that act
/// on the subject are per-pane - and `map_goto` and `ship_mates` may share a key
/// for the same reason, because only one pane is on screen at a time.
///
/// Three firing contexts, and the split is what makes the shared keys legal.
/// `interface_toggle` is `Always` - it has to open the interface from wherever
/// the player is. The orbit, cycle and pane-switch verbs are `Viewer`: live
/// while the interface owns the screen, quiet under the command modal, where
/// the keyboard is typing. The per-pane verbs are `InterfacePane`, so `map_goto`
/// and `ship_mates` can both hold `G` without colliding, and `viewer_next` and
/// a future `map_next` could not.
///
/// # What the pad reaches
///
/// `Viewer` does not overlap `Flight`, so the viewer verbs REUSE the flight
/// rig's buttons rather than hunting for spare ones: the D-pad orbits, the
/// triggers dolly, the bumpers step the selection, the left stick press
/// reframes. Only the two `Always` actions (the interface toggle on Right Thumb,
/// the HUD cycle on View) are off limits, plus the fixed Menu pause chord.
///
/// A pad runs out before the pane verbs do. `InterfacePane` overlaps `Viewer`, so
/// only the two face buttons the camera does not use are left: Y switches the
/// pane, and A goes to what each pane does MOST. `ship_repair`,
/// `ship_rebind` and `inventory_details` stay keyboard-only rather than take a
/// camera verb's button away.
pub fn interface_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard};
    vec![
        // RightThumb, and only RightThumb: this is `Always`, so it collides
        // with every context at once.
        ActionBinding::new("interface_toggle", "SYSTEM", "Open Interface")
            .context(ActionContext::Always)
            .keyboard([Keyboard(KeyCode::Tab)])
            .gamepad([Gamepad(GamepadButton::RightThumb)]),
        ActionBinding::new("interface_next_tab", "INTERFACE", "Next Pane")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyM)])
            .gamepad([Gamepad(GamepadButton::North)]),
        // The shared viewer controls. `map` and `ship` both drive an orbit
        // camera over a schematic, so they answer the same verbs.
        ActionBinding::new("viewer_orbit_left", "INTERFACE", "Turn Left")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyQ)])
            .gamepad([Gamepad(GamepadButton::DPadLeft)]),
        ActionBinding::new("viewer_orbit_right", "INTERFACE", "Turn Right")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyE)])
            .gamepad([Gamepad(GamepadButton::DPadRight)]),
        ActionBinding::new("viewer_orbit_up", "INTERFACE", "Tilt Up")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyR)])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("viewer_orbit_down", "INTERFACE", "Tilt Down")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyF)])
            .gamepad([Gamepad(GamepadButton::DPadDown)]),
        ActionBinding::new("viewer_pan_forward", "INTERFACE", "Pan Forward")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyW)])
            .gamepad([Gamepad(GamepadButton::RightTrigger2)]),
        ActionBinding::new("viewer_pan_back", "INTERFACE", "Pan Back")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyS)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger2)]),
        ActionBinding::new("viewer_pan_left", "INTERFACE", "Pan Left")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyA)])
            .gamepad([Gamepad(GamepadButton::West)]),
        ActionBinding::new("viewer_pan_right", "INTERFACE", "Pan Right")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyD)])
            .gamepad([Gamepad(GamepadButton::East)]),
        ActionBinding::new("viewer_reframe", "INTERFACE", "Reset View")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::KeyT)])
            .gamepad([Gamepad(GamepadButton::LeftThumb)]),
        ActionBinding::new("viewer_next", "INTERFACE", "Select Next")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::BracketRight)])
            .gamepad([Gamepad(GamepadButton::RightTrigger)]),
        ActionBinding::new("viewer_prev", "INTERFACE", "Select Previous")
            .context(ActionContext::Viewer)
            .keyboard([Keyboard(KeyCode::BracketLeft)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger)]),
        // What each pane does to the thing it has selected.
        ActionBinding::new("map_goto", "MAP", "Set GOTO")
            .context(ActionContext::InterfacePane("map"))
            .keyboard([Keyboard(KeyCode::KeyG)])
            .gamepad([Gamepad(GamepadButton::South)]),
        ActionBinding::new("ship_mates", "SHIP", "Mates Overlay")
            .context(ActionContext::InterfacePane("ship"))
            .keyboard([Keyboard(KeyCode::KeyG)])
            .gamepad([Gamepad(GamepadButton::South)]),
        ActionBinding::new("ship_repair", "SHIP", "Repair Section")
            .context(ActionContext::InterfacePane("ship"))
            .keyboard([Keyboard(KeyCode::KeyP)]),
        ActionBinding::new("ship_rebind", "SHIP", "Rebind Section Key")
            .context(ActionContext::InterfacePane("ship"))
            .keyboard([Keyboard(KeyCode::KeyB)]),
        ActionBinding::new("inventory_details", "INVENTORY", "Item Details")
            .context(ActionContext::InterfacePane("inventory"))
            .keyboard([Keyboard(KeyCode::KeyI)]),
    ]
}
