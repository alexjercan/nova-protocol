//! Authoring surface for input bindings (task: scenario config serde).
//!
//! `bevy_enhanced_input::Binding` is the runtime binding type the spawn and
//! editor paths use, but it has no `serde` impls, so a
//! `HashMap<SectionId, Vec<Binding>>` cannot round-trip through a hand-authored
//! RON scenario file. [`BindingInput`] is the small serializable stand-in: it
//! covers the simple, no-modifier button forms a scenario actually authors
//! (a key, a mouse button, a gamepad button) and converts to/from `Binding`.
//! The runtime field keeps its `Binding` type; `binding_map_serde` bridges
//! the two on (de)serialize.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

/// Glob-import surface: `use crate::objects::binding_input::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::BindingInput;
}

/// A serializable input binding: the no-modifier button forms a scenario
/// authors. `KeyCode`/`MouseButton`/`GamepadButton` (de)serialize through
/// `bevy/serialize`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum BindingInput {
    /// A keyboard key binding.
    Keyboard(KeyCode),
    /// A mouse button binding.
    Mouse(MouseButton),
    /// A gamepad button binding.
    Gamepad(GamepadButton),
}

impl BindingInput {
    /// Build the runtime [`Binding`] (no modifier keys). Uses the same
    /// `From<..>` conversions the hand-written bindings use.
    pub fn to_binding(&self) -> Binding {
        match self {
            BindingInput::Keyboard(key) => Binding::from(*key),
            BindingInput::Mouse(button) => Binding::from(*button),
            BindingInput::Gamepad(button) => Binding::from(*button),
        }
    }
}

impl TryFrom<&Binding> for BindingInput {
    type Error = ();

    /// Only the simple, modifier-free button forms are authorable. Bindings
    /// with modifier keys, mouse motion/wheel, gamepad axes, `AnyKey`,
    /// `Custom` and `None` are rejected (`Err(())`).
    fn try_from(binding: &Binding) -> Result<Self, Self::Error> {
        match binding {
            Binding::Keyboard { key, mod_keys } if mod_keys.is_empty() => {
                Ok(BindingInput::Keyboard(*key))
            }
            Binding::MouseButton { button, mod_keys } if mod_keys.is_empty() => {
                Ok(BindingInput::Mouse(*button))
            }
            Binding::GamepadButton(button) => Ok(BindingInput::Gamepad(*button)),
            _ => Err(()),
        }
    }
}

/// serde bridge for a `HashMap<SectionId, Vec<Binding>>` field: (de)serializes
/// it as `HashMap<SectionId, Vec<BindingInput>>`. Serialization fails if any
/// binding is not authorable (see [`BindingInput::try_from`]).
#[cfg(feature = "serde")]
pub(crate) mod binding_map_serde {
    use bevy::platform::collections::HashMap;
    use bevy_enhanced_input::prelude::Binding;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::BindingInput;
    use crate::objects::spaceship::SectionId;

    pub(crate) fn serialize<S: Serializer>(
        map: &HashMap<SectionId, Vec<Binding>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut authored: HashMap<SectionId, Vec<BindingInput>> = HashMap::default();
        for (section, bindings) in map.iter() {
            let mut inputs = Vec::with_capacity(bindings.len());
            for binding in bindings {
                let input = BindingInput::try_from(binding).map_err(|()| {
                    serde::ser::Error::custom(format!(
                        "binding {binding:?} is not authorable (only modifier-free \
                         key/mouse/gamepad buttons serialize)"
                    ))
                })?;
                inputs.push(input);
            }
            authored.insert(section.clone(), inputs);
        }
        authored.serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<SectionId, Vec<Binding>>, D::Error> {
        let authored = HashMap::<SectionId, Vec<BindingInput>>::deserialize(deserializer)?;
        let mut map: HashMap<SectionId, Vec<Binding>> = HashMap::default();
        for (section, inputs) in authored {
            map.insert(
                section,
                inputs.iter().map(BindingInput::to_binding).collect(),
            );
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_forms_round_trip_through_binding() {
        for input in [
            BindingInput::Keyboard(KeyCode::KeyW),
            BindingInput::Mouse(MouseButton::Left),
            BindingInput::Gamepad(GamepadButton::South),
        ] {
            let binding = input.to_binding();
            let back = BindingInput::try_from(&binding).expect("simple form is authorable");
            assert_eq!(back, input);
        }
    }

    #[test]
    fn modified_and_axis_bindings_are_rejected() {
        // A key with a modifier is not authorable.
        let modified = Binding::Keyboard {
            key: KeyCode::KeyW,
            mod_keys: ModKeys::CONTROL,
        };
        assert!(BindingInput::try_from(&modified).is_err());

        // Mouse motion / wheel and gamepad axes are not authorable.
        assert!(BindingInput::try_from(&Binding::mouse_motion()).is_err());
        assert!(BindingInput::try_from(&Binding::mouse_wheel()).is_err());
        assert!(BindingInput::try_from(&Binding::GamepadAxis(GamepadAxis::LeftStickX)).is_err());
    }
}
