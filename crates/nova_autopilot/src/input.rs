//! Synthesized player input: the actions a scripted step performs.
//!
//! Every constructor here returns a plain `Fn(&mut World)`, which is exactly
//! the shape [`on_enter`](crate::autopilot::StepBuilder::on_enter) takes, so a
//! beat reads as the gesture it performs:
//!
//! ```rust,no_run
//! # use bevy::prelude::*;
//! # use nova_autopilot::prelude::*;
//! # #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)] enum S { #[default] Boot }
//! # fn build(app: &mut App) {
//! app.add_plugins(
//!     AutopilotPlugin::<S>::new()
//!         .step("thrust").on_enter(press_key(KeyCode::Space)).until(elapsed(2.0)).add()
//!         .step("coast").on_enter(release_key(KeyCode::Space)).until(elapsed(1.0)).add(),
//! );
//! # }
//! ```
//!
//! The driver runs these AFTER `InputSystems` has collected the frame's real
//! input, so a synthesized press is still `just_pressed` when the game's
//! `Update` systems read it. Bevy's own input collection only clears the
//! `just_*` edges, so a press persists across frames until a matching release -
//! a held key is one `press_key` beat and one `release_key` beat, not a press
//! repeated every frame.
//!
//! Gamepad, touch and drag synthesis are deliberately absent: nothing in the
//! example fleet uses them, and [`move_cursor`] plus [`press_mouse`] /
//! [`release_mouse`] compose a drag when something does.

use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    window::{CursorMoved, PrimaryWindow, WindowEvent},
};

/// Press `key`, and keep it pressed until [`release_key`].
pub fn press_key(key: KeyCode) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        world.resource_mut::<ButtonInput<KeyCode>>().press(key);
    }
}

/// Release `key`.
pub fn release_key(key: KeyCode) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        world.resource_mut::<ButtonInput<KeyCode>>().release(key);
    }
}

/// Press mouse `button`, and keep it pressed until [`release_mouse`].
pub fn press_mouse(button: MouseButton) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| set_mouse_button(world, button, ButtonState::Pressed)
}

/// Release mouse `button`.
pub fn release_mouse(button: MouseButton) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| set_mouse_button(world, button, ButtonState::Released)
}

/// Move the pointer to `position` (logical pixels in the primary window).
///
/// Writes BOTH halves of what a real pointer produces: the window's own
/// [`Window::cursor_position`], which UI code polls, and a [`CursorMoved`]
/// message, which the picking backend and any message reader consume. Writing
/// only one leaves half the app believing the pointer never moved.
///
/// A warn-and-continue no-op when the app has no primary window (a headless
/// run without `WindowPlugin`).
pub fn move_cursor(position: Vec2) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| set_cursor(world, position)
}

/// Move the pointer to `position` and press `button` there.
///
/// The press is NOT released: a real click's release lands on a later frame, so
/// it is its own beat ([`release_mouse`]) - which is also what lets a script
/// hold a click open across a drag or a charge.
pub fn click_at(
    position: Vec2,
    button: MouseButton,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        set_cursor(world, position);
        set_mouse_button(world, button, ButtonState::Pressed);
    }
}

/// The shared body of [`move_cursor`] and [`click_at`]: place the pointer in
/// the primary window and announce the move.
fn set_cursor(world: &mut World, position: Vec2) {
    let window = {
        let mut query = world.query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>();
        match query.single_mut(world) {
            Ok((entity, mut window)) => {
                window.set_cursor_position(Some(position));
                entity
            }
            Err(error) => {
                warn!("autopilot: cursor move to {position:?} has no primary window ({error})");
                return;
            }
        }
    };
    let moved = CursorMoved {
        window,
        position,
        // A synthesized pointer teleports; a real one reports the step it took.
        // `None` is the honest answer and the one Bevy itself writes for a
        // cursor that was outside the window last frame.
        delta: None,
    };
    world.write_message(moved.clone());
    // The picking backend reads `WindowEvent`, NOT the concrete message, and it
    // tracks the cursor from those events alone - so a click without this
    // wrapper lands at whatever position picking last saw. `bevy_winit` writes
    // both for every real pointer move; so does this.
    world.write_message(WindowEvent::CursorMoved(moved));
}

/// The shared body of [`press_mouse`], [`release_mouse`] and [`click_at`].
///
/// Writes the button state DIRECTLY, so it is `just_pressed` on this frame the
/// way [`press_key`] is, and announces the same transition as a `WindowEvent`
/// for the picking backend. The concrete `MouseButtonInput` message is
/// deliberately not written: `bevy_input`'s own collector reads it and would
/// re-apply the transition a frame late.
fn set_mouse_button(world: &mut World, button: MouseButton, state: ButtonState) {
    {
        let mut input = world.resource_mut::<ButtonInput<MouseButton>>();
        match state {
            ButtonState::Pressed => input.press(button),
            ButtonState::Released => input.release(button),
        }
    }

    let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let window = match query.single(world) {
        Ok(entity) => entity,
        Err(error) => {
            warn!("autopilot: mouse {button:?} {state:?} has no primary window ({error})");
            return;
        }
    };
    world.write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
        button,
        state,
        window,
    }));
}

#[cfg(test)]
mod tests {
    use bevy::{input::InputPlugin, window::WindowResolution};

    use super::*;

    /// A headless app with real input collection and a primary window, so the
    /// actions write the resources and messages the game reads.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));
        app.add_message::<CursorMoved>();
        app.world_mut().spawn((
            Window {
                resolution: WindowResolution::new(800, 600),
                ..default()
            },
            PrimaryWindow,
        ));
        app
    }

    fn cursor_moves(app: &mut App) -> Vec<CursorMoved> {
        app.world_mut()
            .resource_mut::<Messages<CursorMoved>>()
            .drain()
            .collect()
    }

    #[test]
    fn keys_stay_pressed_until_released() {
        let mut app = app();
        press_key(KeyCode::Space)(app.world_mut());
        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::Space));

        release_key(KeyCode::Space)(app.world_mut());
        assert!(!app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::Space));
    }

    #[test]
    fn mouse_buttons_stay_pressed_until_released() {
        let mut app = app();
        press_mouse(MouseButton::Left)(app.world_mut());
        assert!(app
            .world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left));

        release_mouse(MouseButton::Left)(app.world_mut());
        assert!(!app
            .world()
            .resource::<ButtonInput<MouseButton>>()
            .pressed(MouseButton::Left));
    }

    /// A synthesized click leaves the world in the state a real pointer would:
    /// the window's own cursor position, a `CursorMoved` message at that same
    /// position, and a `just_pressed` button. Anything reading only one of the
    /// three must still see the click.
    #[test]
    fn click_at_leaves_the_pointer_state_a_real_click_leaves() {
        let mut app = app();
        let at = Vec2::new(320.0, 240.0);

        click_at(at, MouseButton::Left)(app.world_mut());

        let window = app
            .world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .single(app.world())
            .expect("the rig has a primary window")
            .clone();
        assert_eq!(
            window.cursor_position(),
            Some(at),
            "the window itself must report the pointer, for UI that polls it"
        );
        let moves = cursor_moves(&mut app);
        assert_eq!(
            moves.iter().map(|moved| moved.position).collect::<Vec<_>>(),
            vec![at],
            "exactly one CursorMoved, at the click position, for readers that \
             follow the message instead"
        );
        assert!(
            app.world()
                .resource::<ButtonInput<MouseButton>>()
                .just_pressed(MouseButton::Left),
            "the button edge is fresh, not merely held"
        );
    }

    /// `move_cursor` positions without clicking - the half of the pair a hover
    /// or a drag leg needs.
    #[test]
    fn move_cursor_positions_without_pressing() {
        let mut app = app();
        move_cursor(Vec2::new(10.0, 20.0))(app.world_mut());

        assert_eq!(cursor_moves(&mut app).len(), 1);
        assert!(
            !app.world()
                .resource::<ButtonInput<MouseButton>>()
                .just_pressed(MouseButton::Left),
            "moving the pointer must not synthesize a click"
        );
    }

    /// Without a window the actions warn and continue: a headless smoke run
    /// that pokes the cursor must not die on it.
    #[test]
    fn a_cursor_move_without_a_window_is_harmless() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));
        app.add_message::<CursorMoved>();

        move_cursor(Vec2::ZERO)(app.world_mut());

        assert!(cursor_moves(&mut app).is_empty());
    }
}
