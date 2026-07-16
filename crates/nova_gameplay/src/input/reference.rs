//! A read-only keybind reference for the settings menu (task 20260711-180511).
//!
//! The settings panel renders in the main menu, where no input rig is spawned
//! (the flight/camera rigs only exist during a live scenario), so the reference
//! cannot be read live off the `Bindings` the way the in-flight verb hints are
//! ([`super::player::binding_label`]). It is therefore authored as static data
//! here, next to the rigs it describes, and a parity test pins the flight rig's
//! keyboard bindings so a future remap of the rig cannot silently desync this
//! list. Full remapping + key icons stay backlog (task 20260710-231927); this
//! is the read-only surface only.
//!
//! The camera-rig rows (aim / free-look / raise) and the pause row are static
//! prose: they live in `camera_controller`/`nova_menu`, are far lower-churn,
//! and the parity test covers the flight rig where the actual key churn is.

/// One row of the keybind reference: what the control does and how it is bound
/// on keyboard/mouse and on a gamepad. All plain display strings - this is a
/// read-only readout, not a binding source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeybindEntry {
    /// Grouping header this row sits under (e.g. "FLIGHT").
    pub section: &'static str,
    /// What the control does (e.g. "Main Drive").
    pub action: &'static str,
    /// Keyboard/mouse binding, display form (e.g. "W / Space").
    pub keyboard: &'static str,
    /// Gamepad binding, display form (e.g. "Right Trigger").
    pub gamepad: &'static str,
}

/// The canonical player controls, in reading order and grouped by section.
/// Sourced from the flight rig ([`super::player::flight_input_rig`]), the
/// targeting actions, the camera controller rig, and the pause toggle.
pub const KEYBINDS: &[KeybindEntry] = &[
    // FLIGHT - the flight rig in input/player.rs.
    KeybindEntry {
        section: "FLIGHT",
        action: "Aim",
        keyboard: "Mouse",
        gamepad: "Right Stick",
    },
    KeybindEntry {
        section: "FLIGHT",
        action: "Main Drive",
        keyboard: "W / Space",
        gamepad: "Right Trigger",
    },
    KeybindEntry {
        section: "FLIGHT",
        action: "Autopilot: Stop",
        keyboard: "X",
        gamepad: "B",
    },
    KeybindEntry {
        section: "FLIGHT",
        action: "Autopilot: Go To",
        keyboard: "G",
        gamepad: "Y",
    },
    KeybindEntry {
        section: "FLIGHT",
        action: "Autopilot: Orbit",
        keyboard: "O",
        gamepad: "A",
    },
    KeybindEntry {
        section: "FLIGHT",
        action: "Autopilot: Off",
        keyboard: "Z",
        gamepad: "X",
    },
    // TARGETING - the radar gestures and the fine-lock cycle.
    KeybindEntry {
        section: "TARGETING",
        action: "Raise Weapons",
        keyboard: "Right Mouse",
        gamepad: "Left Trigger 2",
    },
    KeybindEntry {
        section: "TARGETING",
        action: "Radar (hold search / tap clear)",
        keyboard: "Ctrl",
        gamepad: "D-Pad Up",
    },
    KeybindEntry {
        section: "TARGETING",
        action: "Lock / Component Next",
        keyboard: "] / Scroll Up",
        gamepad: "D-Pad Right",
    },
    KeybindEntry {
        section: "TARGETING",
        action: "Lock / Component Prev",
        keyboard: "[ / Scroll Down",
        gamepad: "D-Pad Left",
    },
    // CAMERA - the chase-camera controller rig in camera_controller.rs.
    KeybindEntry {
        section: "CAMERA",
        action: "Free Look",
        keyboard: "Left Alt",
        gamepad: "Left Trigger",
    },
    // SYSTEM - the pause toggle in nova_menu.
    KeybindEntry {
        section: "SYSTEM",
        action: "Pause / Menu",
        keyboard: "Esc",
        gamepad: "Start",
    },
];

/// The read-only keybind reference for the settings menu.
pub fn keybind_reference() -> &'static [KeybindEntry] {
    KEYBINDS
}

#[cfg(test)]
mod tests {
    use super::*;

    // The rig-parity test (the reference's keyboard labels must match the live
    // flight rig) lives in `input::player`'s test module, where the private
    // flight action types are visible; it asserts against these public
    // `KEYBINDS`. See `reference_rows_track_the_flight_rig` there.

    #[test]
    fn every_entry_is_fully_populated() {
        for entry in KEYBINDS {
            assert!(!entry.section.is_empty(), "empty section");
            assert!(
                !entry.action.is_empty(),
                "empty action for {}",
                entry.section
            );
            assert!(
                !entry.keyboard.is_empty() && !entry.gamepad.is_empty(),
                "a binding column is blank for {}",
                entry.action
            );
        }
    }
}
