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

    /// The keycap-table key for this source - what a surface with room for
    /// pictures looks a glyph up by (`nova_hud`'s `key_glyph_stem`).
    ///
    /// The same as [`Self::label`] for the desk, and DEVICE-QUALIFIED for the
    /// pad: a controller's face buttons read `A`/`B`/`X`/`Y`, which are also
    /// four keyboard keys, and one flat table cannot answer both with the
    /// right picture.
    pub fn glyph_label(&self) -> String {
        match self {
            InputSource::Gamepad(button) => format!("Pad {}", gamepad_label(*button)),
            desk => desk.label(),
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

/// The `Bindings` bundle for a list of sources a rig owns DIRECTLY, with no
/// registry name behind it: a section's content `input_mapping`, which is
/// authored per ship rather than declared as a named action.
///
/// A named action goes through [`InputBindings::bundle`] instead. Both end at
/// the same place, and this is the only other one - upstream's `Binding` is
/// built nowhere outside this crate except where a rig spawns an AXIS, which
/// has no source to name.
///
/// [`InputBindings::bundle`]: crate::registry::InputBindings::bundle
pub fn source_bindings(sources: impl IntoIterator<Item = InputSource>) -> impl Bundle {
    let bindings: Vec<Binding> = sources.into_iter().map(Binding::from).collect();
    Bindings::spawn(SpawnIter(bindings.into_iter()))
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

/// Every keyboard key a binding may name, in the order a `bindings` listing
/// prints them.
///
/// A written-out table rather than a derived one because `KeyCode` has no
/// iterator and most of its variants - media keys, IME keys, `Fn` - are not
/// things a player can rebind onto even though the type can hold them.
const BINDABLE_KEYS: &[KeyCode] = &[
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
    KeyCode::Numpad0,
    KeyCode::Numpad1,
    KeyCode::Numpad2,
    KeyCode::Numpad3,
    KeyCode::Numpad4,
    KeyCode::Numpad5,
    KeyCode::Numpad6,
    KeyCode::Numpad7,
    KeyCode::Numpad8,
    KeyCode::Numpad9,
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::Tab,
    KeyCode::Escape,
    KeyCode::Backspace,
    KeyCode::Delete,
    KeyCode::Insert,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::PageUp,
    KeyCode::PageDown,
    KeyCode::CapsLock,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::SuperLeft,
    KeyCode::SuperRight,
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::Backquote,
    KeyCode::Comma,
    KeyCode::Period,
    KeyCode::Slash,
    KeyCode::Semicolon,
    KeyCode::Quote,
    KeyCode::Minus,
    KeyCode::Equal,
    KeyCode::Backslash,
    KeyCode::NumpadAdd,
    KeyCode::NumpadSubtract,
    KeyCode::NumpadMultiply,
    KeyCode::NumpadDivide,
    KeyCode::NumpadDecimal,
    KeyCode::NumpadEnter,
];

/// Every mouse button a binding may name.
const BINDABLE_MOUSE: &[MouseButton] =
    &[MouseButton::Left, MouseButton::Right, MouseButton::Middle];

/// Every gamepad button a binding may name.
const BINDABLE_PAD: &[GamepadButton] = &[
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::North,
    GamepadButton::West,
    GamepadButton::LeftTrigger,
    GamepadButton::LeftTrigger2,
    GamepadButton::RightTrigger,
    GamepadButton::RightTrigger2,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::Mode,
    GamepadButton::LeftThumb,
    GamepadButton::RightThumb,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];

impl InputSource {
    /// Every source a player may bind an action to, keyboard then mouse then
    /// pad. The completion and the `bindings` listing read this, so the set a
    /// command offers is the set a command accepts.
    pub fn bindable() -> impl Iterator<Item = InputSource> {
        BINDABLE_KEYS
            .iter()
            .copied()
            .map(InputSource::Keyboard)
            .chain(BINDABLE_MOUSE.iter().copied().map(InputSource::Mouse))
            .chain(BINDABLE_PAD.iter().copied().map(InputSource::Gamepad))
    }

    /// Read a source back from a written name.
    ///
    /// Accepts any spelling the game PRINTS for the source - the keycap label
    /// (`W`, `LMB`), the readout label (`Left Ctrl`, `;`) and the variant name
    /// (`KeyW`, `ControlLeft`) - case- and space-insensitively, so a player can
    /// type back whatever the `bindings` listing showed them. A pad button may
    /// be qualified with `pad:` to say which device is meant, since four face
    /// buttons and four keyboard keys share the names A/B/X/Y.
    pub fn parse(text: &str) -> Option<InputSource> {
        let wanted = normalize_source_name(text);
        if wanted.is_empty() {
            return None;
        }
        if let Some(pad) = wanted.strip_prefix("pad:") {
            return BINDABLE_PAD
                .iter()
                .copied()
                .find(|button| pad_names(*button).iter().any(|name| name == pad))
                .map(InputSource::Gamepad);
        }
        InputSource::bindable().find(|source| {
            let names = match source {
                InputSource::Keyboard(key) => vec![
                    normalize_source_name(&keyboard_label(*key)),
                    normalize_source_name(&key_symbol(*key)),
                    normalize_source_name(&format!("{key:?}")),
                ],
                InputSource::Mouse(button) => vec![
                    normalize_source_name(&InputSource::Mouse(*button).label()),
                    normalize_source_name(&InputSource::Mouse(*button).readout_label()),
                    normalize_source_name(&format!("{button:?}")),
                ],
                InputSource::Gamepad(button) => pad_names(*button),
            };
            names.iter().any(|name| *name == wanted)
        })
    }
}

/// The names one pad button answers to, already normalized.
fn pad_names(button: GamepadButton) -> Vec<String> {
    vec![
        normalize_source_name(&gamepad_label(button)),
        normalize_source_name(&format!("{button:?}")),
    ]
}

/// Lower-case and strip the spaces, dashes and underscores a printed label
/// carries, so `Left Ctrl`, `left-ctrl` and `ControlLeft` all compare equal
/// against the same table.
///
/// A single character is left alone: `-` is the whole name of the minus key,
/// and stripping it would leave nothing to compare.
fn normalize_source_name(text: &str) -> String {
    let text = text.trim().to_ascii_lowercase();
    if text.chars().count() <= 1 {
        return text;
    }
    text.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .collect()
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

    /// Four face buttons and four keyboard keys read the same. Only the glyph
    /// key tells them apart, so it is the one a picture table is keyed on.
    #[test]
    fn a_pad_glyph_key_does_not_collide_with_a_keyboard_one() {
        assert_eq!(InputSource::Gamepad(GamepadButton::South).label(), "A");
        assert_eq!(InputSource::Keyboard(KeyCode::KeyA).label(), "A");
        assert_eq!(
            InputSource::Gamepad(GamepadButton::South).glyph_label(),
            "Pad A"
        );
        assert_eq!(InputSource::Keyboard(KeyCode::KeyA).glyph_label(), "A");
        assert_eq!(
            InputSource::Mouse(MouseButton::Left).glyph_label(),
            "LMB",
            "the desk keeps one vocabulary"
        );
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
    /// Whatever the game PRINTS for a source has to read back as that source:
    /// a player types back what the bindings listing showed them. A pad button
    /// is qualified, because four of them share a name with a keyboard key.
    #[test]
    fn every_bindable_source_parses_back_from_what_it_prints() {
        for source in InputSource::bindable() {
            let qualify = |printed: String| match source {
                InputSource::Gamepad(_) => format!("pad:{printed}"),
                _ => printed,
            };
            for printed in [source.label(), source.readout_label()] {
                let printed = qualify(printed);
                assert_eq!(
                    InputSource::parse(&printed),
                    Some(source),
                    "{printed:?} must read back as {source:?}",
                );
            }
        }
    }

    /// A/B/X/Y name four face buttons AND four keyboard keys. The bare word is
    /// the keyboard one; the pad needs its device.
    #[test]
    fn a_pad_face_button_needs_its_device_to_beat_the_keyboard_key() {
        assert_eq!(
            InputSource::parse("A"),
            Some(InputSource::Keyboard(KeyCode::KeyA)),
        );
        assert_eq!(
            InputSource::parse("pad:A"),
            Some(InputSource::Gamepad(GamepadButton::South)),
        );
    }

    /// Spelling is not the point: the spacing and case a label happens to use
    /// must not decide whether a rebind works.
    #[test]
    fn a_source_name_ignores_case_spacing_and_the_variant_spelling() {
        let ctrl = Some(InputSource::Keyboard(KeyCode::ControlLeft));
        for spelling in [
            "Left Ctrl",
            "left ctrl",
            "left-ctrl",
            "ControlLeft",
            "controlleft",
        ] {
            assert_eq!(InputSource::parse(spelling), ctrl, "{spelling}");
        }
        assert_eq!(InputSource::parse("  "), None);
        assert_eq!(InputSource::parse("no such key"), None);
    }
}
