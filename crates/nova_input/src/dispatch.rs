//! Drive a named action: look it up in [`InputBindings`] and press what it is
//! bound to.
//!
//! This is a lookup and an injector, not an interpreter. It presses the same
//! physical source a player presses, so every condition and modifier the rig
//! declares still runs - a driven `radar_hold` is a real [`Hold`], and a driven
//! `radar_clear` a real tap. That is what makes an input test truthful.
//!
//! [`Hold`]: bevy_enhanced_input::prelude::Hold
//!
//! # Why not the mock
//!
//! `bevy_enhanced_input` has an `ActionMock` component, and it does not compose
//! with the conditions - it REPLACES them. Its own module doc says input
//! reading, conditions and modifiers are all skipped. A mocked `radar_hold`
//! proves the state, never the gesture, so the mock is not the route here.
//!
//! # When to run it
//!
//! In `PreUpdate`, AFTER bevy's `InputSystems` has collected the frame's real
//! input, so a synthesized press is still `just_pressed` when `Update` reads
//! it. Bevy clears only the `just_*` edges, so a press holds across frames
//! until a matching release - a held action is one [`InputPhase::Press`] and
//! one [`InputPhase::Release`], not a press repeated every frame.
//!
//! The axis writers are the exception in kind but not in timing: bevy CLEARS
//! [`AccumulatedMouseMotion`] and [`AccumulatedMouseScroll`] each frame in the
//! same set, so [`apply_axis`] must land after it for the same reason.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
};

use crate::{
    registry::{InputBindings, WheelDirection},
    source::InputSource,
};

/// Which half of a press a call performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputPhase {
    /// Push the bound source down, and leave it down.
    Press,
    /// Let it back up.
    Release,
}

/// Why a named action could not be driven.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DispatchError {
    /// No action of that name is registered. Carries the name, because the
    /// caller's whole input was a string.
    Unknown(String),
    /// The action exists but has no keyboard or mouse button to press. Either
    /// it is an axis (`rcs_aim` is mouse motion and nothing else), or it is
    /// bound to a gamepad, which is not synthesizable here.
    NoButton(&'static str),
    /// The action exists but carries no axis, so there is no value to write.
    NoAxis(&'static str),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::Unknown(name) => write!(f, "no action named `{name}`"),
            DispatchError::NoButton(name) => write!(
                f,
                "`{name}` has no keyboard or mouse button to press; it is an axis or gamepad-only"
            ),
            DispatchError::NoAxis(name) => write!(f, "`{name}` is not driven by an axis"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Press or release whatever `name` is bound to, writing the button state a
/// game polls.
///
/// [`primary_source`] picks which source that is.
pub fn apply(world: &mut World, name: &str, phase: InputPhase) -> Result<(), DispatchError> {
    match (primary_source(world, name)?, phase) {
        (InputSource::Keyboard(key), InputPhase::Press) => {
            world.resource_mut::<ButtonInput<KeyCode>>().press(key);
        }
        (InputSource::Keyboard(key), InputPhase::Release) => {
            world.resource_mut::<ButtonInput<KeyCode>>().release(key);
        }
        (InputSource::Mouse(button), InputPhase::Press) => {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(button);
        }
        (InputSource::Mouse(button), InputPhase::Release) => {
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(button);
        }
        // `primary_source` never returns one.
        (InputSource::Gamepad(_), _) => unreachable!(),
    }
    Ok(())
}

/// The one source `name` is driven through: the first keyboard or mouse button
/// it is bound to.
///
/// Split out because a caller with a WINDOW wants more than [`apply`] writes.
/// `nova_autopilot`'s pointer helpers also send the `MouseButtonInput` message
/// and its `WindowEvent` wrapper, which is what `bevy_picking` builds a click
/// from - so a driven UI run resolves the source here and presses it through
/// those, while a headless one calls [`apply`] and writes the button state
/// alone.
///
/// The FIRST bound source wins: an action on several keys is driven by one of
/// them, because pressing all of them is a gesture no player performs. A
/// gamepad source is never returned - bevy's gamepad input is connection-gated
/// and synthesizing it is unverified - so a pad-only action is
/// [`DispatchError::NoButton`] rather than a silent no-op.
pub fn primary_source(world: &World, name: &str) -> Result<InputSource, DispatchError> {
    let Some(action) = world.resource::<InputBindings>().get(name) else {
        return Err(DispatchError::Unknown(name.to_string()));
    };
    action
        .keyboard
        .iter()
        .chain(&action.gamepad)
        .find(|source| !matches!(source, InputSource::Gamepad(_)))
        .copied()
        .ok_or(DispatchError::NoButton(action.name))
}

/// Drive the axis half of `name` by `delta`.
///
/// Mouse motion takes the delta as it comes. The wheel takes its MAGNITUDE and
/// spends it in the direction the action declares, so a caller says how far to
/// cycle and the registry says which way that is - `component_next` and
/// `component_prev` share the wheel and differ only in sign.
///
/// Additive, because both resources are accumulators: two calls in one frame
/// are one longer sweep, which is what two real notches would be.
pub fn apply_axis(world: &mut World, name: &str, delta: Vec2) -> Result<(), DispatchError> {
    let Some(action) = world.resource::<InputBindings>().get(name) else {
        return Err(DispatchError::Unknown(name.to_string()));
    };
    let (label, axes) = (action.name, action.axes);

    if axes.mouse_motion {
        let mut motion = world.resource_mut::<AccumulatedMouseMotion>();
        motion.delta += delta;
        return Ok(());
    }
    if let Some(direction) = axes.wheel {
        let magnitude = delta.y.abs().max(delta.x.abs());
        let signed = match direction {
            WheelDirection::Up => magnitude,
            WheelDirection::Down => -magnitude,
        };
        let mut scroll = world.resource_mut::<AccumulatedMouseScroll>();
        scroll.unit = MouseScrollUnit::Line;
        scroll.delta.y += signed;
        return Ok(());
    }
    Err(DispatchError::NoAxis(label))
}

#[cfg(test)]
mod tests {
    use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};

    use super::*;
    use crate::registry::{ActionBinding, GamepadStick};

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<AccumulatedMouseMotion>();
        world.init_resource::<AccumulatedMouseScroll>();
        world.insert_resource(InputBindings::from_actions([
            ActionBinding::new("main_drive", "FLIGHT", "Main Drive")
                .keyboard([
                    InputSource::Keyboard(KeyCode::KeyW),
                    InputSource::Keyboard(KeyCode::Space),
                ])
                .gamepad([InputSource::Gamepad(GamepadButton::RightTrigger)]),
            ActionBinding::new("combat_stance", "TARGETING", "Raise Weapons")
                .keyboard([InputSource::Mouse(MouseButton::Right)]),
            ActionBinding::new("pad_only", "FLIGHT", "Pad Only")
                .gamepad([InputSource::Gamepad(GamepadButton::South)]),
            ActionBinding::new("rcs_aim", "FLIGHT", "RCS Aim").mouse_motion(),
            ActionBinding::new("component_prev", "TARGETING", "Prev")
                .keyboard([InputSource::Keyboard(KeyCode::BracketLeft)])
                .wheel(WheelDirection::Down),
            ActionBinding::new("camera_rotate", "CAMERA", "Aim")
                .mouse_motion()
                .stick(GamepadStick::Right),
        ]));
        world
    }

    #[test]
    fn a_named_action_presses_and_releases_its_primary_source() {
        let mut world = world();

        apply(&mut world, "main_drive", InputPhase::Press).unwrap();
        let keys = world.resource::<ButtonInput<KeyCode>>();
        assert!(keys.pressed(KeyCode::KeyW), "the primary key goes down");
        assert!(
            !keys.pressed(KeyCode::Space),
            "the alternate key is not also pressed; no player holds both"
        );

        apply(&mut world, "main_drive", InputPhase::Release).unwrap();
        assert!(!world
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::KeyW));

        apply(&mut world, "combat_stance", InputPhase::Press).unwrap();
        assert!(world
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Right));
    }

    #[test]
    fn an_action_with_nothing_to_press_says_so_by_name() {
        let mut world = world();
        assert_eq!(
            apply(&mut world, "warp_drive", InputPhase::Press),
            Err(DispatchError::Unknown("warp_drive".to_string()))
        );
        assert_eq!(
            apply(&mut world, "pad_only", InputPhase::Press),
            Err(DispatchError::NoButton("pad_only")),
            "a gamepad-only action fails loudly rather than doing nothing"
        );
        assert_eq!(
            apply(&mut world, "rcs_aim", InputPhase::Press),
            Err(DispatchError::NoButton("rcs_aim")),
            "an axis action has no press"
        );
        assert_eq!(
            apply_axis(&mut world, "main_drive", Vec2::Y),
            Err(DispatchError::NoAxis("main_drive")),
            "a button action has no axis"
        );
    }

    #[test]
    fn the_registry_decides_which_way_the_wheel_turns() {
        let mut world = world();

        apply_axis(&mut world, "component_prev", Vec2::new(0.0, 2.0)).unwrap();
        assert_eq!(
            world.resource::<AccumulatedMouseScroll>().delta.y,
            -2.0,
            "the caller says how far; the action says which way"
        );

        apply_axis(&mut world, "component_prev", Vec2::new(0.0, 1.0)).unwrap();
        assert_eq!(
            world.resource::<AccumulatedMouseScroll>().delta.y,
            -3.0,
            "two calls in a frame are one longer sweep, like two real notches"
        );
    }

    #[test]
    fn a_motion_action_accumulates_the_delta_it_is_given() {
        let mut world = world();
        apply_axis(&mut world, "camera_rotate", Vec2::new(3.0, -1.0)).unwrap();
        apply_axis(&mut world, "camera_rotate", Vec2::new(1.0, 1.0)).unwrap();
        assert_eq!(
            world.resource::<AccumulatedMouseMotion>().delta,
            Vec2::new(4.0, 0.0)
        );
    }
}
