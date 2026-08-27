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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
            InputSource::Gamepad(button) => format!("{button:?}"),
        }
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

/// A short display chip for a list of live bindings: the first keyboard or
/// mouse binding in the list. Empty when the list is gamepad-only or bound to
/// an axis.
pub fn binding_label(bindings: &[Binding]) -> String {
    bindings
        .iter()
        .filter_map(binding_source)
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
    fn a_gamepad_only_list_has_no_display_chip() {
        let gamepad = [Binding::from(GamepadButton::South)];
        assert_eq!(binding_label(&gamepad), "");

        let mixed = [
            Binding::from(GamepadButton::South),
            Binding::from(KeyCode::KeyO),
        ];
        assert_eq!(binding_label(&mixed), "O");

        assert_eq!(binding_label(&[]), "");
        assert_eq!(binding_label(&[Binding::from(MouseButton::Left)]), "LMB");
        assert_eq!(
            binding_label(&[Binding::mouse_motion(), Binding::from(KeyCode::Space)]),
            "Space",
            "an axis binding is not a chip; the first real button wins"
        );
    }
}
