//! Which device the player is using, so a hint draws the key or the button
//! that hand is on.
//!
//! Every hint surface reads the same answer: the flight dock, the anchored
//! cues, the pane footers and the menu key chips. The answer changes on a
//! press only: a key, a mouse button, a pad button, or a stick pushed past
//! [`STICK_USE_THRESHOLD`] from under it. Pointer motion, a held stick and
//! stick drift do not count, because a scripted run pins the cursor every
//! frame and a resting pad reports small axis noise.
//!
//! A press a script drives counts like a player's, a pad-only action
//! included, so a scripted run that presses one draws pad hints from then on.

use bevy::prelude::*;

use crate::{poll::InputSources, registry::ActionBinding, source::InputSource};

/// Stick deflection that counts as the player reaching for the pad.
const STICK_USE_THRESHOLD: f32 = 0.5;

/// The device the player pressed last: the keyboard and mouse, or a gamepad.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Resource)]
pub enum InputDeviceType {
    /// Keyboard and mouse. The state before any press.
    #[default]
    Desk,
    /// A gamepad.
    Pad,
}

impl ActionBinding {
    /// The button a one-key hint draws for `device`: the first source on that
    /// device, or the first on the other one when that column is empty.
    /// Buttons only; an action that is an axis and nothing else has none.
    pub fn hint_source(&self, device: InputDeviceType) -> Option<InputSource> {
        let (own, other) = match device {
            InputDeviceType::Desk => (&self.keyboard, &self.gamepad),
            InputDeviceType::Pad => (&self.gamepad, &self.keyboard),
        };
        own.first().or_else(|| other.first()).copied()
    }

    /// The column a text hint prints for `device`, falling back to the other
    /// column when that one is empty. `Unbound` when the action holds nothing
    /// at all. Unlike [`Self::hint_source`] it names axes too (`Mouse`,
    /// `Left Stick`), so use one of the two for a surface, not both.
    pub fn hint_display(&self, device: InputDeviceType) -> String {
        let desk = self.keyboard_chips();
        let pad = self.gamepad_chips();
        let use_pad = match device {
            InputDeviceType::Desk => desk.is_empty() && !pad.is_empty(),
            InputDeviceType::Pad => !pad.is_empty(),
        };
        if use_pad {
            self.gamepad_display()
        } else {
            self.keyboard_display()
        }
    }
}

/// Follow the player's hands: record the device of this frame's press. The
/// desk wins a frame that presses both.
///
/// `stick_past` holds whether a stick sat past the threshold last frame, so a
/// held stick counts once, when it crosses. Without it a held stick won every
/// frame, and a key press beside it flipped the hints for one frame.
pub(crate) fn track_input_device(
    sources: InputSources,
    mut device: ResMut<InputDeviceType>,
    mut stick_past: Local<bool>,
) {
    let stick = sources.any_stick_past(STICK_USE_THRESHOLD);
    let stick_crossed = stick && !*stick_past;
    *stick_past = stick;
    let pressed = if sources.any_desk_just_pressed() {
        InputDeviceType::Desk
    } else if sources.any_pad_just_pressed() || stick_crossed {
        InputDeviceType::Pad
    } else {
        return;
    };
    device.set_if_neq(pressed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<InputDeviceType>();
        app.add_systems(PostUpdate, track_input_device);
        app
    }

    fn pad_with(button: GamepadButton) -> Gamepad {
        let mut pad = Gamepad::default();
        pad.digital_mut().press(button);
        pad
    }

    #[test]
    fn a_pad_press_selects_the_pad_and_a_key_press_selects_the_desk() {
        let mut app = app();
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Desk
        );

        let pad = app.world_mut().spawn(pad_with(GamepadButton::South)).id();
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Pad
        );

        // A held pad button is not a new press: the device holds.
        app.world_mut()
            .entity_mut(pad)
            .get_mut::<Gamepad>()
            .expect("the pad was spawned above")
            .digital_mut()
            .clear_just_pressed(GamepadButton::South);
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Pad
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyX);
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Desk
        );
    }

    #[test]
    fn a_held_stick_selects_the_pad_once_and_a_later_key_press_keeps_the_desk() {
        let mut app = app();
        let mut pad = Gamepad::default();
        pad.analog_mut().set(GamepadAxis::LeftStickX, 0.8);
        app.world_mut().spawn(pad);
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Pad
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyX);
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Desk
        );

        // The stick is still held and the key is no longer a new press.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear_just_pressed(KeyCode::KeyX);
        app.update();
        assert_eq!(
            *app.world().resource::<InputDeviceType>(),
            InputDeviceType::Desk
        );
    }

    #[test]
    fn a_hint_names_the_active_device_and_falls_back_to_the_other_column() {
        let both = ActionBinding::new("autopilot_stop", "FLIGHT", "Stop")
            .keyboard([InputSource::Keyboard(KeyCode::KeyX)])
            .gamepad([InputSource::Gamepad(GamepadButton::East)]);
        assert_eq!(
            both.hint_source(InputDeviceType::Desk),
            Some(InputSource::Keyboard(KeyCode::KeyX))
        );
        assert_eq!(
            both.hint_source(InputDeviceType::Pad),
            Some(InputSource::Gamepad(GamepadButton::East))
        );
        assert_eq!(both.hint_display(InputDeviceType::Desk), "X");
        assert_eq!(both.hint_display(InputDeviceType::Pad), "B");

        let desk_only = ActionBinding::new("roll_left", "FLIGHT", "Roll")
            .keyboard([InputSource::Keyboard(KeyCode::KeyQ)]);
        assert_eq!(
            desk_only.hint_source(InputDeviceType::Pad),
            Some(InputSource::Keyboard(KeyCode::KeyQ))
        );
        assert_eq!(desk_only.hint_display(InputDeviceType::Pad), "Q");
    }
}
