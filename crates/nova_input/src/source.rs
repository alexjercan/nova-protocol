//! Physical input sources: the discrete button a binding occupies, and the
//! short labels the settings and editor readouts print for it.
//!
//! A source is the collision key AND the persisted form of a binding. Two
//! bindings naming the same source drive the same physical button, which is
//! what a content `input_mapping` must not do against the always-on flight
//! rig. Axes - motion, wheel, sticks - have no single collision key and no
//! rebind row, so they are deliberately not representable here.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

/// A physical input source, stripped of modifiers and gesture conditions.
///
/// `From<KeyCode>`, `From<MouseButton>` and `From<GamepadButton>` mirror
/// upstream's conversions into `Binding`, so a caller writes
/// `KeyCode::KeyF.into()` here exactly as it did there.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InputSource {
    /// A keyboard key.
    Keyboard(KeyCode),
    /// A mouse button.
    Mouse(MouseButton),
    /// A gamepad button.
    Gamepad(GamepadButton),
}

impl InputSource {
    /// A short human label for the source (`W`, `Space`, `LMB`, `RightTrigger`).
    pub fn label(&self) -> String {
        match self {
            InputSource::Keyboard(key) => keyboard_label(*key),
            InputSource::Mouse(MouseButton::Left) => "LMB".to_string(),
            InputSource::Mouse(MouseButton::Right) => "RMB".to_string(),
            InputSource::Mouse(MouseButton::Middle) => "MMB".to_string(),
            InputSource::Mouse(button) => format!("{button:?}"),
            InputSource::Gamepad(button) => gamepad_label(*button),
        }
    }
}

impl From<KeyCode> for InputSource {
    fn from(key: KeyCode) -> Self {
        InputSource::Keyboard(key)
    }
}

impl From<MouseButton> for InputSource {
    fn from(button: MouseButton) -> Self {
        InputSource::Mouse(button)
    }
}

impl From<GamepadButton> for InputSource {
    fn from(button: GamepadButton) -> Self {
        InputSource::Gamepad(button)
    }
}

impl From<InputSource> for Binding {
    fn from(source: InputSource) -> Self {
        match source {
            InputSource::Keyboard(key) => Binding::from(key),
            InputSource::Mouse(button) => Binding::from(button),
            InputSource::Gamepad(button) => Binding::from(button),
        }
    }
}

/// The physical source a binding occupies, if it names a discrete button.
/// Motion, wheel, stick-axis, `AnyKey`, custom and empty bindings return
/// `None`: they are axes, not buttons something else can collide on.
pub fn binding_source(binding: &Binding) -> Option<InputSource> {
    match binding {
        Binding::Keyboard { key, .. } => Some(InputSource::Keyboard(*key)),
        Binding::MouseButton { button, .. } => Some(InputSource::Mouse(*button)),
        Binding::GamepadButton(button) => Some(InputSource::Gamepad(*button)),
        _ => None,
    }
}

/// A short display label for a keyboard key: the debug name with the `Key`
/// and `Digit` prefixes stripped, so `KeyCode::KeyW` reads `W`.
pub fn keyboard_label(key: KeyCode) -> String {
    let name = format!("{key:?}");
    name.strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(&name)
        .to_string()
}

/// The readout label for one source: what the settings screen prints in a
/// binding column. Longer and friendlier than [`InputSource::label`], which is
/// the keycap-chip key and must keep matching the glyph table.
impl InputSource {
    /// `Left Ctrl`, `]`, `Right Mouse`, `Right Trigger`.
    pub fn readout_label(&self) -> String {
        match self {
            InputSource::Keyboard(key) => key_symbol(*key),
            InputSource::Mouse(MouseButton::Left) => "Left Mouse".to_string(),
            InputSource::Mouse(MouseButton::Right) => "Right Mouse".to_string(),
            InputSource::Mouse(MouseButton::Middle) => "Middle Mouse".to_string(),
            InputSource::Mouse(button) => format!("{button:?}"),
            InputSource::Gamepad(button) => gamepad_label(*button),
        }
    }
}

/// The readout label for a key: the printed symbol for the punctuation keys,
/// the short name for the modifiers, the debug name for everything else.
///
/// Separate from [`keyboard_label`] on purpose. That one keys the keycap
/// glyphs (`nova_hud`'s `key_glyph_stem`) and a friendlier spelling there
/// silently loses a keycap image; this one is prose in a controls list.
pub fn key_symbol(key: KeyCode) -> String {
    let symbol = match key {
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Backquote => "`",
        KeyCode::Escape => "Esc",
        KeyCode::ControlLeft => "Left Ctrl",
        KeyCode::ControlRight => "Right Ctrl",
        KeyCode::ShiftLeft => "Left Shift",
        KeyCode::ShiftRight => "Right Shift",
        KeyCode::AltLeft => "Left Alt",
        KeyCode::AltRight => "Right Alt",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        KeyCode::Semicolon => ";",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::Backslash => "\\",
        _ => return keyboard_label(key),
    };
    symbol.to_string()
}

/// The short name a left/right modifier pair reads as when BOTH halves are
/// bound, and the half that has to be bound with it. `Ctrl`, not `Left Ctrl /
/// Right Ctrl`.
pub fn modifier_pair(key: KeyCode) -> Option<(&'static str, KeyCode)> {
    match key {
        KeyCode::ControlLeft => Some(("Ctrl", KeyCode::ControlRight)),
        KeyCode::ControlRight => Some(("Ctrl", KeyCode::ControlLeft)),
        KeyCode::ShiftLeft => Some(("Shift", KeyCode::ShiftRight)),
        KeyCode::ShiftRight => Some(("Shift", KeyCode::ShiftLeft)),
        KeyCode::AltLeft => Some(("Alt", KeyCode::AltRight)),
        KeyCode::AltRight => Some(("Alt", KeyCode::AltLeft)),
        KeyCode::SuperLeft => Some(("Super", KeyCode::SuperRight)),
        KeyCode::SuperRight => Some(("Super", KeyCode::SuperLeft)),
        _ => None,
    }
}

/// A readable label for a gamepad button: the four face buttons under their
/// Xbox names, everything else de-camel-cased (`RightTrigger` -> `Right
/// Trigger`, `DPadUp` -> `D-Pad Up`).
///
/// The face buttons are the only ones that need a table: `South`/`East` say
/// where the button IS, and no pad prints that on its shell.
pub fn gamepad_label(button: GamepadButton) -> String {
    match button {
        GamepadButton::South => "A".to_string(),
        GamepadButton::East => "B".to_string(),
        GamepadButton::North => "Y".to_string(),
        GamepadButton::West => "X".to_string(),
        other => spaced_words(&format!("{other:?}")),
    }
}

/// `RightTrigger` -> `Right Trigger`, `LeftTrigger2` -> `Left Trigger 2`,
/// `DPadUp` -> `D-Pad Up`.
fn spaced_words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    let mut previous = '\0';
    for current in name.chars() {
        let boundary = (current.is_uppercase() && previous.is_lowercase())
            || (current.is_ascii_digit() && previous.is_alphabetic());
        if boundary {
            out.push(' ');
        }
        out.push(current);
        previous = current;
    }
    match out.strip_prefix("DPad") {
        Some(rest) => format!("D-Pad{rest}"),
        None => out,
    }
}

/// A short display chip for a list of sources: the first keyboard or mouse
/// source in the list. Empty when the list is gamepad-only, because a pad
/// button has no keycap to draw.
pub fn source_label(sources: &[InputSource]) -> String {
    sources
        .iter()
        .find(|source| !matches!(source, InputSource::Gamepad(_)))
        .map(|source| source.label())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_label_drops_the_key_and_digit_prefixes() {
        assert_eq!(keyboard_label(KeyCode::KeyW), "W");
        assert_eq!(keyboard_label(KeyCode::Digit1), "1");
        assert_eq!(keyboard_label(KeyCode::Space), "Space");
    }

    #[test]
    fn a_source_round_trips_through_a_binding() {
        for source in [
            InputSource::Keyboard(KeyCode::KeyW),
            InputSource::Mouse(MouseButton::Right),
            InputSource::Gamepad(GamepadButton::DPadUp),
        ] {
            let binding = Binding::from(source);
            assert_eq!(binding_source(&binding), Some(source));
        }
    }

    #[test]
    fn a_gamepad_button_reads_as_the_shell_prints_it() {
        assert_eq!(gamepad_label(GamepadButton::East), "B");
        assert_eq!(gamepad_label(GamepadButton::RightTrigger), "Right Trigger");
        assert_eq!(gamepad_label(GamepadButton::LeftTrigger2), "Left Trigger 2");
        assert_eq!(gamepad_label(GamepadButton::DPadUp), "D-Pad Up");
        assert_eq!(gamepad_label(GamepadButton::Start), "Start");
    }

    #[test]
    fn a_readout_label_prints_the_symbol_not_the_variant() {
        assert_eq!(
            InputSource::Keyboard(KeyCode::BracketRight).readout_label(),
            "]"
        );
        assert_eq!(
            InputSource::Keyboard(KeyCode::ControlLeft).readout_label(),
            "Left Ctrl"
        );
        assert_eq!(InputSource::Keyboard(KeyCode::KeyW).readout_label(), "W");
        assert_eq!(
            InputSource::Mouse(MouseButton::Right).readout_label(),
            "Right Mouse"
        );
        assert_eq!(
            keyboard_label(KeyCode::BracketRight),
            "BracketRight",
            "the keycap-chip label is untouched; the glyph table keys on it"
        );
    }

    #[test]
    fn a_gamepad_only_list_has_no_display_chip() {
        let gamepad = [InputSource::from(GamepadButton::South)];
        assert_eq!(source_label(&gamepad), "");

        let mixed = [
            InputSource::from(GamepadButton::South),
            InputSource::from(KeyCode::KeyO),
        ];
        assert_eq!(source_label(&mixed), "O");

        assert_eq!(source_label(&[]), "");
        assert_eq!(source_label(&[InputSource::from(MouseButton::Left)]), "LMB");
        assert_eq!(
            source_label(&[
                InputSource::from(GamepadButton::South),
                InputSource::from(KeyCode::Space)
            ]),
            "Space",
            "a pad button is not a chip; the first keycap wins"
        );
    }
}
