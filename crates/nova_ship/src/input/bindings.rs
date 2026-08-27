//! The default bindings for the ship's fixed rigs, as plain data.
//!
//! These are the DEFAULTS, not the live table: `nova_ship`'s plugin registers
//! them into [`InputBindings`] at build, and the rigs are spawned from that
//! resource. Kept as pure functions because the content lint has no world and
//! still has to know which sources flight reserves.
//!
//! Per-section weapon actions are absent on purpose: they are built from
//! content `input_mapping` at spawn, keyed by section entity, and their names
//! are derived rather than fixed.

use bevy::prelude::*;
use nova_input::prelude::*;

/// The always-on flight and targeting actions, in reading order.
///
/// Every action here runs with `consume_input: false`, so a content
/// `input_mapping` that reuses one of these sources SILENTLY double-drives
/// flight (lesson `input-mapping-overlays-flight-rig`). That is what
/// [`flight_rig_reserved_sources`] exists to prevent.
pub fn flight_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard};
    vec![
        ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
            .keyboard([Keyboard(KeyCode::KeyW), Keyboard(KeyCode::Space)])
            .gamepad([Gamepad(GamepadButton::RightTrigger)]),
        ActionBinding::new("autopilot_stop", "FLIGHT", "Autopilot: Stop")
            .keyboard([Keyboard(KeyCode::KeyX)])
            .gamepad([Gamepad(GamepadButton::East)]),
        ActionBinding::new("autopilot_goto", "FLIGHT", "Autopilot: Go To")
            .keyboard([Keyboard(KeyCode::KeyG)])
            .gamepad([Gamepad(GamepadButton::North)]),
        // South: the scenario-advance confirm was moved off South to DPadDown
        // so one pad press cannot both skip the scenario and park the ship.
        ActionBinding::new("autopilot_orbit", "FLIGHT", "Autopilot: Orbit")
            .keyboard([Keyboard(KeyCode::KeyO)])
            .gamepad([Gamepad(GamepadButton::South)]),
        ActionBinding::new("autopilot_off", "FLIGHT", "Autopilot: Off")
            .keyboard([Keyboard(KeyCode::KeyZ)])
            .gamepad([Gamepad(GamepadButton::West)]),
        // Hold and tap share the key and the threshold constant so the
        // boundary frame cannot fall between them.
        ActionBinding::new("radar_hold", "TARGETING", "Radar (hold search)")
            .keyboard([
                Keyboard(KeyCode::ControlLeft),
                Keyboard(KeyCode::ControlRight),
            ])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("radar_clear", "TARGETING", "Radar (tap clear)")
            .keyboard([
                Keyboard(KeyCode::ControlLeft),
                Keyboard(KeyCode::ControlRight),
            ])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("component_next", "TARGETING", "Lock / Component Next")
            .keyboard([Keyboard(KeyCode::BracketRight)])
            .gamepad([Gamepad(GamepadButton::DPadRight)]),
        ActionBinding::new("component_prev", "TARGETING", "Lock / Component Prev")
            .keyboard([Keyboard(KeyCode::BracketLeft)])
            .gamepad([Gamepad(GamepadButton::DPadLeft)]),
        ActionBinding::new("rcs_modifier", "FLIGHT", "RCS Fine Adjust")
            .keyboard([Keyboard(KeyCode::ShiftLeft), Keyboard(KeyCode::ShiftRight)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger2)]),
        // Raw mouse motion, accumulated into the ship-local RCS plane while
        // the modifier is held. No discrete source: nothing collides on an
        // axis and no rebind row can capture one.
        ActionBinding::new("rcs_aim", "FLIGHT", "RCS Aim"),
    ]
}

/// The chase-camera controller's actions.
pub fn camera_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard, Mouse};
    vec![
        // Mouse motion plus the right stick: both axes, neither reservable.
        ActionBinding::new("camera_rotate", "CAMERA", "Aim"),
        ActionBinding::new("free_look", "CAMERA", "Free Look")
            .keyboard([Keyboard(KeyCode::AltLeft)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger)]),
        ActionBinding::new("combat_stance", "TARGETING", "Raise Weapons")
            .keyboard([Mouse(MouseButton::Right)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger2)]),
    ]
}

/// The discrete input sources the always-on flight rig reserves, each paired
/// with the action label that holds it.
///
/// Derived from [`flight_bindings`], so a remap moves the reservation with it
/// and the content lint's input-overlap check cannot go stale. Wheel and
/// motion sources are absent because they are axes, not buttons a section
/// binding collides on.
pub fn flight_rig_reserved_sources() -> Vec<(InputSource, &'static str)> {
    flight_bindings()
        .into_iter()
        .flat_map(|action| {
            action
                .sources()
                .map(|source| (source, action.label))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registry's whole point is that one name means one thing. A
    /// duplicate would make the settings row and the dispatcher disagree.
    #[test]
    fn every_ship_action_name_is_unique() {
        let mut names: Vec<&str> = flight_bindings()
            .iter()
            .chain(camera_bindings().iter())
            .map(|action| action.name)
            .collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate action name in {names:?}");
    }

    /// The fixed rigs this task names: 11 flight and targeting, 3 camera.
    #[test]
    fn the_fixed_rigs_name_fourteen_actions() {
        assert_eq!(flight_bindings().len(), 11);
        assert_eq!(camera_bindings().len(), 3);
    }

    /// Names are runtime strings; nothing type-checks them. Pin the list so a
    /// rename has to be deliberate in two places.
    #[test]
    fn the_action_names_are_pinned() {
        let names: Vec<&str> = flight_bindings()
            .iter()
            .chain(camera_bindings().iter())
            .map(|action| action.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "main_drive",
                "autopilot_stop",
                "autopilot_goto",
                "autopilot_orbit",
                "autopilot_off",
                "radar_hold",
                "radar_clear",
                "component_next",
                "component_prev",
                "rcs_modifier",
                "rcs_aim",
                "camera_rotate",
                "free_look",
                "combat_stance",
            ]
        );
    }
}
