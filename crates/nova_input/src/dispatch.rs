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
//!
//! # The pad is two halves
//!
//! A pad-only action is driven on a pad this module CONNECTS
//! ([`SynthesizedGamepad`]), because bevy models a controller as a [`Gamepad`]
//! component on an entity and there is no controller plugged in on a test box.
//!
//! Writing that pad means writing both halves of it, the way bevy's own
//! `gamepad_event_processing_system` does. [`Gamepad::digital`] is the button
//! set a poller reads (this crate's `InputSources`); [`Gamepad::analog`] is
//! what `bevy_enhanced_input` reads, through [`Gamepad::get`]. Setting one
//! half drives half the game and looks like it works.
//!
//! Bevy clears the pad's digital EDGES in the same `PreUpdate` set it clears
//! the keyboard's, so the press/release rule above is the pad's rule too.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    registry::{GamepadStick, InputBindings, WheelDirection},
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
    /// The action exists but has no button on any device to press: it is an
    /// axis, like `rcs_aim`, which is mouse motion and nothing else.
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
                "`{name}` has no button on any device to press; it is an axis"
            ),
            DispatchError::NoAxis(name) => write!(f, "`{name}` is not driven by an axis"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Press or release whatever `name` is bound to, writing the button state a
/// game polls.
///
/// [`driven_source`] picks which source a PRESS goes to. A release lets up the
/// source that press actually pushed down ([`DrivenPresses`]), not whatever
/// the name resolves to now.
pub fn apply(world: &mut World, name: &str, phase: InputPhase) -> Result<(), DispatchError> {
    let source = match phase {
        InputPhase::Press => driven_source(world, name)?,
        InputPhase::Release => held_source(world, name)?,
    };
    match (source, phase) {
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
        (InputSource::Gamepad(button), phase) => press_pad(world, button, phase),
    }
    match phase {
        InputPhase::Press => {
            world
                .get_resource_or_init::<DrivenPresses>()
                .0
                .insert(name.to_string(), source);
        }
        InputPhase::Release => {
            if let Some(mut held) = world.get_resource_mut::<DrivenPresses>() {
                held.0.remove(name);
            }
        }
    }
    Ok(())
}

/// The source each named action is currently held down through.
///
/// A press and its release are two calls, and a rebind can land between them.
/// Resolving the name twice would then let up a source nothing is holding and
/// leave the pressed one down for the rest of the run - on the pad half with
/// its analog value still at 1.0, so a rig keeps firing with no key held. The
/// press records what it pushed, and the release lets up exactly that.
///
/// Only [`apply`] writes this. A caller pressing a source itself - the pointer
/// helpers, which need a window - owns its own release.
#[derive(Resource, Debug, Default)]
pub struct DrivenPresses(HashMap<String, InputSource>);

/// The source a release lets up: the one the press recorded, falling back to
/// [`driven_source`] for a release with no press before it.
fn held_source(world: &World, name: &str) -> Result<InputSource, DispatchError> {
    match world
        .get_resource::<DrivenPresses>()
        .and_then(|held| held.0.get(name))
    {
        Some(&source) => Ok(source),
        None => driven_source(world, name),
    }
}

/// The pad this module presses when an action reaches no desk button.
///
/// Marks the entity [`apply`] connects the first time it needs one, so the
/// next press lands on the SAME pad instead of connecting another, and so a
/// real controller plugged in beside it is never written to.
#[derive(Component, Debug, Default)]
pub struct SynthesizedGamepad;

/// Write both halves of one pad button. See the pad section of the module doc
/// for why one half is not enough.
fn press_pad(world: &mut World, button: GamepadButton, phase: InputPhase) {
    let pad = synthesized_pad(world);
    let mut pad = world.entity_mut(pad);
    let mut gamepad = pad
        .get_mut::<Gamepad>()
        .expect("the synthesized pad is spawned with one");
    match phase {
        InputPhase::Press => {
            gamepad.digital_mut().press(button);
            gamepad.analog_mut().set(button, 1.0);
        }
        InputPhase::Release => {
            gamepad.digital_mut().release(button);
            gamepad.analog_mut().set(button, 0.0);
        }
    }
}

/// The synthesized pad, connecting one if this is the first pad press.
fn synthesized_pad(world: &mut World) -> Entity {
    let mut pads = world.query_filtered::<Entity, With<SynthesizedGamepad>>();
    if let Some(pad) = pads.iter(world).next() {
        return pad;
    }
    world
        .spawn((
            Name::new("Input: Synthesized Gamepad"),
            SynthesizedGamepad,
            Gamepad::default(),
        ))
        .id()
}

/// The source [`apply`] drives `name` through: [`primary_source`] where the
/// action holds a desk button, its first PAD button where it does not.
///
/// The desk comes first because that is what a player on a keyboard presses
/// and what a driven UI run can also click; the pad is the fallback for the
/// actions that only ever had one, not a second surface to press in parallel.
pub fn driven_source(world: &World, name: &str) -> Result<InputSource, DispatchError> {
    match primary_source(world, name) {
        Err(DispatchError::NoButton(label)) => world
            .resource::<InputBindings>()
            .get(name)
            .and_then(|action| action.gamepad.first().copied())
            .ok_or(DispatchError::NoButton(label)),
        resolved => resolved,
    }
}

/// The one source `name` is driven through: the first keyboard or mouse button
/// it is bound to.
///
/// Public as the seam for a caller with a WINDOW, which wants more than
/// [`apply`] writes: `bevy_picking` builds a click from the `MouseButtonInput`
/// message and its `WindowEvent` wrapper, so a driven UI run would resolve the
/// source here and press it through `nova_autopilot`'s pointer helpers. No
/// such caller exists yet - every driven action today is read through
/// `bevy_enhanced_input`, which the button state alone satisfies.
///
/// The FIRST bound source wins: an action on several keys is driven by one of
/// them, because pressing all of them is a gesture no player performs. A
/// gamepad source is never returned here - a caller that resolves a source to
/// press it through a WINDOW has no window to press a pad button in - so a
/// pad-only action is [`DispatchError::NoButton`]. [`driven_source`] is the
/// one that falls back to the pad, and [`apply`] uses it.
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

/// Deflect the STICK `name` declares, by `delta` in the -1..=1 a pad reports.
///
/// Separate from [`apply_axis`] rather than another branch inside it, because
/// the two are different devices and an action can declare both: `rcs_aim` and
/// `camera_rotate` are each mouse motion AND a stick, and a caller driving one
/// is not driving the other.
///
/// Absolute, not additive: a stick reports where it IS, and it reports that
/// every frame. So a deflection HOLDS until it is written back to zero - the
/// press/release rule of this module, in analog.
pub fn apply_stick(world: &mut World, name: &str, delta: Vec2) -> Result<(), DispatchError> {
    let Some(action) = world.resource::<InputBindings>().get(name) else {
        return Err(DispatchError::Unknown(name.to_string()));
    };
    let (label, stick) = (action.name, action.axes.stick);
    let Some(stick) = stick else {
        return Err(DispatchError::NoAxis(label));
    };
    let (x, y) = match stick {
        GamepadStick::Left => (GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
        GamepadStick::Right => (GamepadAxis::RightStickX, GamepadAxis::RightStickY),
    };
    let pad = synthesized_pad(world);
    let mut pad = world.entity_mut(pad);
    let mut gamepad = pad
        .get_mut::<Gamepad>()
        .expect("the synthesized pad is spawned with one");
    gamepad.analog_mut().set(x, delta.x.clamp(-1.0, 1.0));
    gamepad.analog_mut().set(y, delta.y.clamp(-1.0, 1.0));
    Ok(())
}

#[cfg(test)]
mod tests {
    use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};

    use super::*;
    use crate::registry::{ActionBinding, BindingSpec, GamepadStick};

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<AccumulatedMouseMotion>();
        world.init_resource::<AccumulatedMouseScroll>();
        world.insert_resource(bindings());
        world
    }

    fn bindings() -> InputBindings {
        InputBindings::from_actions([
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
        ])
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

    /// The pad half. A pad-only action reaches a pad this module connects
    /// itself, and BOTH halves of it are written: the digital set a poller
    /// reads and the analog value `bevy_enhanced_input` reads.
    #[test]
    fn a_pad_only_action_presses_a_pad_that_did_not_exist() {
        let mut world = world();
        apply(&mut world, "pad_only", InputPhase::Press).unwrap();

        let pads: Vec<&Gamepad> = world
            .query_filtered::<&Gamepad, With<SynthesizedGamepad>>()
            .iter(&world)
            .collect();
        assert_eq!(pads.len(), 1, "one pad was connected for the press");
        let pad = pads[0];
        assert!(
            pad.digital().pressed(GamepadButton::South),
            "the poller's half is down"
        );
        assert_eq!(
            pad.get(GamepadButton::South),
            Some(1.0),
            "and so is the half bevy_enhanced_input reads"
        );

        apply(&mut world, "pad_only", InputPhase::Release).unwrap();
        let pads = world
            .query_filtered::<&Gamepad, With<SynthesizedGamepad>>()
            .iter(&world)
            .count();
        assert_eq!(pads, 1, "the release reuses the pad; it does not add one");
        let pad = world
            .query_filtered::<&Gamepad, With<SynthesizedGamepad>>()
            .single(&world)
            .unwrap();
        assert!(!pad.digital().pressed(GamepadButton::South));
        assert_eq!(pad.get(GamepadButton::South), Some(0.0));
    }

    /// The pad is the FALLBACK, not a second surface: an action bound on both
    /// is still driven by its key, so a keyboard-only game never grows a pad.
    #[test]
    fn an_action_bound_on_both_is_driven_by_the_desk() {
        let mut world = world();
        apply(&mut world, "main_drive", InputPhase::Press).unwrap();
        assert!(world
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::KeyW));
        assert_eq!(
            world
                .query_filtered::<Entity, With<SynthesizedGamepad>>()
                .iter(&world)
                .count(),
            0,
            "no pad was connected"
        );
    }

    /// The claim the module opens with, on the PAD: a driven pad press reaches
    /// a REAL `bevy_enhanced_input` rig, and the rig's `Hold` still runs on it.
    ///
    /// This is the half a keyboard test cannot cover, and the half a
    /// developer with no controller cannot press: upstream reads the pad's
    /// ANALOG value through `Gamepad::get`, not the digital set, so writing
    /// only the digital half would leave every rig-driven pad action dead
    /// while the polled ones (the NOVA OS toggle) looked fine.
    #[test]
    fn a_driven_pad_press_reaches_a_real_rig_and_its_hold() {
        use core::time::Duration;

        use bevy::{input::InputPlugin, time::TimeUpdateStrategy};
        use bevy_enhanced_input::prelude::*;

        use crate::source::source_bindings;

        const HOLD_SECS: f32 = 0.2;
        const TICK: Duration = Duration::from_millis(50);

        #[derive(Component)]
        struct PadContext;

        #[derive(InputAction)]
        #[action_output(bool)]
        struct PadFire;

        #[derive(Resource, Default)]
        struct Held(bool);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(TICK));
        app.insert_resource(bindings());
        app.init_resource::<Held>();
        app.add_input_context::<PadContext>();
        app.add_observer(|_: On<Fire<PadFire>>, mut held: ResMut<Held>| held.0 = true);
        app.finish();
        app.cleanup();
        // The first update advances the clock by nothing; the hold would
        // measure from a frame that never elapsed.
        app.update();

        app.world_mut().spawn((
            PadContext,
            actions!(
                PadContext[(
                    Action::<PadFire>::new(),
                    Hold::new(HOLD_SECS),
                    source_bindings([InputSource::Gamepad(GamepadButton::South)]),
                )]
            ),
        ));
        app.update();

        apply(app.world_mut(), "pad_only", InputPhase::Press).expect("the pad is a source now");
        app.update();
        assert!(
            !app.world().resource::<Held>().0,
            "one tick is short of the hold; the condition is running, not bypassed"
        );

        for _ in 0..4 {
            app.update();
        }
        assert!(
            app.world().resource::<Held>().0,
            "the driven pad button crosses the same threshold a real one does"
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

    #[test]
    fn a_release_lets_up_the_source_the_press_pushed_down() {
        let mut world = world();
        apply(&mut world, "main_drive", InputPhase::Press).unwrap();

        world.resource_mut::<InputBindings>().rebind(
            "main_drive",
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::KeyD)],
                gamepad: vec![InputSource::Gamepad(GamepadButton::RightTrigger)],
            },
        );
        apply(&mut world, "main_drive", InputPhase::Release).unwrap();

        let keys = world.resource::<ButtonInput<KeyCode>>();
        assert!(
            !keys.pressed(KeyCode::KeyW),
            "the key the press pushed comes back up, not the one the name means now"
        );
        assert!(
            !keys.pressed(KeyCode::KeyD),
            "and the new key was never pressed, so the release does not strand IT down"
        );
    }

    #[test]
    fn a_stick_deflection_writes_the_pad_axes_the_action_declares() {
        let mut world = world();
        apply_stick(&mut world, "camera_rotate", Vec2::new(0.5, -1.0)).unwrap();

        let mut pads = world.query_filtered::<&Gamepad, With<SynthesizedGamepad>>();
        let pad = pads.iter(&world).next().expect("a pad was connected");
        assert_eq!(pad.get(GamepadAxis::RightStickX), Some(0.5));
        assert_eq!(pad.get(GamepadAxis::RightStickY), Some(-1.0));
        assert_eq!(
            pad.get(GamepadAxis::LeftStickX),
            Some(0.0),
            "the action declares the RIGHT stick; the other one stays at rest"
        );

        assert_eq!(
            apply_stick(&mut world, "main_drive", Vec2::X),
            Err(DispatchError::NoAxis("main_drive")),
            "an action with no stick is refused by name rather than doing nothing"
        );
    }
}
