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
/// `FLIGHT`, `TARGETING` and `CAMERA` are three headers a player reads apart
/// and ONE firing context: they go live together when a player ship is on the
/// field, and they go quiet together when the NOVA OS takes the screen. So the
/// context is applied to the whole list rather than typed on each row - a new
/// action cannot be added without one.
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
        // boundary frame cannot fall between them. `radar_clear` FOLLOWS the
        // hold: one settings row, one rebind, and the tap cannot be left
        // behind on the old key.
        ActionBinding::new("radar_hold", "TARGETING", "Radar (hold / tap)")
            .keyboard([
                Keyboard(KeyCode::ControlLeft),
                Keyboard(KeyCode::ControlRight),
            ])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("radar_clear", "TARGETING", "Radar (tap clear)")
            .follows("radar_hold")
            .keyboard([
                Keyboard(KeyCode::ControlLeft),
                Keyboard(KeyCode::ControlRight),
            ])
            .gamepad([Gamepad(GamepadButton::DPadUp)]),
        ActionBinding::new("component_next", "TARGETING", "Lock / Component Next")
            .keyboard([Keyboard(KeyCode::BracketRight)])
            .gamepad([Gamepad(GamepadButton::DPadRight)])
            .wheel(WheelDirection::Up),
        ActionBinding::new("component_prev", "TARGETING", "Lock / Component Prev")
            .keyboard([Keyboard(KeyCode::BracketLeft)])
            .gamepad([Gamepad(GamepadButton::DPadLeft)])
            .wheel(WheelDirection::Down),
        // Left Thumb, not Left Trigger 2: LT2 is the aim button, and
        // `combat_stance` holds it. Both rigs run with `consume_input: false`,
        // so while they shared it one trigger raised the weapons AND engaged
        // fine adjust. Combat keeps the trigger - it is the idiomatic pair with
        // fire on RT2 - and the modifier takes the one free button.
        ActionBinding::new("rcs_modifier", "FLIGHT", "RCS Fine Adjust")
            .keyboard([Keyboard(KeyCode::ShiftLeft), Keyboard(KeyCode::ShiftRight)])
            .gamepad([Gamepad(GamepadButton::LeftThumb)]),
        // Raw mouse motion, or the LEFT stick, read into the ship-local RCS
        // plane while the modifier is held. The left stick is the pad's half
        // of the same gesture: the modifier IS its click, so a thumb that
        // engages RCS is already on the stick that steers it. No discrete
        // source either way - nothing collides on an axis and no rebind row
        // can capture one.
        ActionBinding::new("rcs_aim", "FLIGHT", "RCS Aim")
            .mouse_motion()
            .stick(GamepadStick::Left),
    ]
    .into_iter()
    .map(|action| action.context(ActionContext::Flight))
    .collect()
}

/// The chase-camera controller's actions.
pub fn camera_bindings() -> Vec<ActionBinding> {
    use InputSource::{Gamepad, Keyboard, Mouse};
    vec![
        // Mouse motion plus the right stick: both axes, neither reservable.
        ActionBinding::new("camera_rotate", "CAMERA", "Aim")
            .mouse_motion()
            .stick(GamepadStick::Right),
        ActionBinding::new("free_look", "CAMERA", "Free Look")
            .keyboard([Keyboard(KeyCode::AltLeft)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger)]),
        ActionBinding::new("combat_stance", "TARGETING", "Raise Weapons")
            .keyboard([Mouse(MouseButton::Right)])
            .gamepad([Gamepad(GamepadButton::LeftTrigger2)]),
    ]
    .into_iter()
    .map(|action| action.context(ActionContext::Flight))
    .collect()
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

    /// No two fixed-rig actions may hold the same source, because both rigs
    /// run with `consume_input: false` - a shared key or button drives both at
    /// once. `radar_clear` FOLLOWS `radar_hold`, so the table knows that pair
    /// is one gesture and this test needs no exemption list.
    ///
    /// The reserved-sources check guards content `input_mapping` against the
    /// flight rig; it never guarded the fixed rigs against EACH OTHER, and the
    /// two lists are separate, so `rcs_modifier` and `combat_stance` sat on
    /// Left Trigger 2 together unnoticed. `conflicts` reads the whole table and
    /// only pairs actions that can be live at the same instant, so this covers
    /// the keyboard half too without flagging a key flight shares with a
    /// surface it can never be up beside.
    #[test]
    fn no_two_fixed_rig_actions_share_a_source() {
        let table =
            InputBindings::from_actions(flight_bindings().into_iter().chain(camera_bindings()));
        let found: Vec<String> = table
            .conflicts()
            .into_iter()
            .map(|(one, other, source)| {
                format!(
                    "`{}` and `{}` both hold {}",
                    one.name,
                    other.name,
                    source.label()
                )
            })
            .collect();
        assert!(found.is_empty(), "{}", found.join("; "));
    }

    /// What the settings screen actually prints for every shipped row. The
    /// readout is derived now, so this is where a label regression shows up -
    /// a key that reads `BracketRight` to a player, or a wheel alternate that
    /// quietly stopped being mentioned.
    #[test]
    fn the_shipped_readout_columns_are_pinned() {
        let rows: Vec<(&str, String, String)> = flight_bindings()
            .into_iter()
            .chain(camera_bindings())
            .map(|action| {
                (
                    action.label,
                    action.keyboard_display(),
                    action.gamepad_display(),
                )
            })
            .collect();
        let rows: Vec<(&str, &str, &str)> = rows
            .iter()
            .map(|(label, keyboard, gamepad)| (*label, keyboard.as_str(), gamepad.as_str()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("Main Drive", "W / Space", "Right Trigger"),
                ("Autopilot: Stop", "X", "B"),
                ("Autopilot: Go To", "G", "Y"),
                ("Autopilot: Orbit", "O", "A"),
                ("Autopilot: Off", "Z", "X"),
                ("Radar (hold / tap)", "Ctrl", "D-Pad Up"),
                ("Radar (tap clear)", "Ctrl", "D-Pad Up"),
                ("Lock / Component Next", "] / Scroll Up", "D-Pad Right"),
                ("Lock / Component Prev", "[ / Scroll Down", "D-Pad Left"),
                ("RCS Fine Adjust", "Shift", "Left Thumb"),
                ("RCS Aim", "Mouse", "Unbound"),
                ("Aim", "Mouse", "Right Stick"),
                ("Free Look", "Left Alt", "Left Trigger"),
                ("Raise Weapons", "Right Mouse", "Left Trigger 2"),
            ]
        );
    }
}
