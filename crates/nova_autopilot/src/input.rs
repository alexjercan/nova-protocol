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
//!
//! ## Reaching a row past the fold
//!
//! [`scroll_lines`] / [`scroll_pixels`] turn the wheel, so a driven run can move
//! a scrolling pane instead of measuring only what the first layout happened to
//! put on screen. Without them a list taller than its box costs the run every
//! row past the fold, and the gap reads as a property of the UI rather than of
//! the harness (task 20260804-190142).
//!
//! ## The driven pointer is authoritative
//!
//! A driven app runs on a real display, and a real pointer event - the window
//! manager's enter/motion pair, a developer nudging the mouse, the echo of the
//! OS-level warp [`Window::set_cursor_position`] itself performs - lands in the
//! same stream the synthesized one does. One of those between a press beat and
//! its release beat CANCELS the click silently: `bevy_picking` dispatches
//! `Pointer<Click>` from the PREVIOUS frame's hover map, so a pointer that
//! moved off the widget in between produces a release and no click, and a
//! button that acts on `Activate` never fires.
//!
//! So a driven run owns the pointer: the autopilot pins it, putting the last
//! synthesized position back before the picking backend reads the frame's
//! events, and a foreign move is overridden instead of obeyed.
//!
//! ## Naming a target instead of a coordinate
//!
//! [`click_at`] takes literal pixels, which couples a run to the layout it was
//! written against. [`click_named`] / [`hover_named`] resolve a [`Name`] to the
//! node's centre first ([`ui_node_centre`]), so moving a panel does not break
//! the beat - only renaming the widget does.

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput, NativeKeyCode},
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
        touch::TouchPhase,
        ButtonState,
    },
    picking::PickingSystems,
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

/// Type `text`: one press [`KeyboardInput`] per character, carrying that
/// character as the text the keypress produced.
///
/// [`press_key`] writes only `ButtonInput<KeyCode>`, which is where a game
/// polls for a HELD key. A text field reads the produced TEXT off the keyboard
/// message instead - what the key means under the player's layout - so typing
/// is its own gesture. The physical key code is reported unidentified: a driven
/// run knows the characters it means to type, not the keyboard that would have
/// produced them.
///
/// A warn-and-continue no-op without a primary window, like the pointer
/// gestures.
pub fn type_text(text: impl Into<String>) -> impl Fn(&mut World) + Send + Sync + 'static {
    let text = text.into();
    move |world: &mut World| {
        let Ok(window) = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
        else {
            warn!("autopilot: typing `{text}` has no primary window");
            return;
        };
        for character in text.chars() {
            let character = character.to_string();
            world.write_message(KeyboardInput {
                key_code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
                logical_key: Key::Character(character.as_str().into()),
                state: ButtonState::Pressed,
                text: Some(character.as_str().into()),
                repeat: false,
                window,
            });
        }
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

/// Turn the wheel by `lines`, the way a notched mouse reports it.
///
/// Positive scrolls the view UP, towards the top of the content - winit's own
/// sign, and the one a reader like `nova_ui`'s `scroll_viewports` already
/// subtracts. What a line is WORTH is the reader's business, not the driver's
/// (`nova_ui` spends 20 logical pixels on one), so a beat that has measured how
/// far it must travel wants [`scroll_pixels`] instead.
///
/// The wheel goes to whatever the pointer is over, so a script with two
/// scrollable panes on screen aims first: [`hover_named`] then this.
pub fn scroll_lines(lines: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| turn_wheel(world, MouseScrollUnit::Line, lines)
}

/// Scroll by `pixels`, the way a trackpad reports it - [`scroll_lines`] in the
/// unit a caller can COMPUTE.
///
/// A beat that scrolls a named row into view knows the gap in logical pixels
/// and must not have to guess the reader's line height to spend it.
pub fn scroll_pixels(pixels: f32) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| turn_wheel(world, MouseScrollUnit::Pixel, pixels)
}

/// The shared body of [`scroll_lines`] and [`scroll_pixels`].
///
/// Writes the concrete [`MouseWheel`] message AND the [`WindowEvent`] wrapper,
/// because `bevy_winit` writes both for every real notch and they have
/// different readers: a game system reads the message, and `bevy_picking`
/// builds its `PointerAction::Scroll` from the wrapper alone. Unlike a button
/// press there is no `ButtonInput`-style accumulator in between, so neither
/// lands a frame late.
///
/// Horizontal scroll is left out with gamepad and touch: nothing in the fleet
/// scrolls sideways.
///
/// A warn-and-continue no-op without a primary window, like the other pointer
/// gestures.
fn turn_wheel(world: &mut World, unit: MouseScrollUnit, y: f32) {
    let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let window = match query.single(world) {
        Ok(entity) => entity,
        Err(error) => {
            warn!("autopilot: a wheel scroll of {y} {unit:?} has no primary window ({error})");
            return;
        }
    };
    // What a mouse always reports. A trackpad's Started/Ended bracket belongs to
    // the gesture synthesis this driver does not do.
    let wheel = MouseWheel {
        unit,
        x: 0.0,
        y,
        window,
        phase: TouchPhase::Moved,
    };
    world.write_message(wheel);
    world.write_message(WindowEvent::MouseWheel(wheel));
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

/// The logical-pixel RECT of the laid-out UI node called `name`, if there is
/// one.
///
/// The single home of the physical-to-logical conversion, and the resolve half
/// of [`click_named`] / [`hover_named`] via [`ui_node_centre`]. Public because a
/// drag needs the rect it is dragging along and a caller may want to ask whether
/// one node's centre lies inside another's box.
///
/// A UI node's [`UiGlobalTransform`] translation is its CENTRE and
/// [`ComputedNode::size`] its extent, both in PHYSICAL pixels - the space
/// `bevy_ui`'s picking backend compares against. [`move_cursor`] takes LOGICAL
/// pixels, so both are scaled back through
/// [`ComputedNode::inverse_scale_factor`]. Skipping that step reads right only
/// at scale factor 1, which is exactly why it is spelled out here instead of at
/// each call site.
///
/// `None` until the node has been through a layout pass - a UI entity spawned
/// this frame has no transform to point at yet, and likewise `None` for a node
/// its ancestry has HIDDEN. A hidden node keeps its layout, so resolving one
/// meant a script could click a panel nobody could see and an assertion on a
/// toggled-by-[`Visibility`] overlay could not fail (a settings-panel
/// screenshot shipped as the bare main menu at exit 0).
///
/// "Through a layout pass" is checked, not assumed: a node that has not been
/// laid out carries [`ComputedNode::default()`] - zero size at the origin - and
/// so does a `Display::None` one. Having the components is not having a BOX, and
/// a zero-size rect at the window corner is not somewhere a pointer can be put.
/// Both are rejected here, so a beat that waits on this waits for a target that
/// exists rather than for an entity that has been spawned (review a4a6 R2).
///
/// A name two laid-out nodes share resolves to whichever the query yields
/// first, which is arbitrary - so it WARNS. A driven run silently clicking a
/// ghost is the exact failure naming targets exists to make visible (review
/// R1.5).
///
/// `&World`, so the same resolve backs both the gestures (which hold `&mut
/// World`, and reborrow into this) and
/// [`ui_node_present`](crate::predicate::ui_node_present), the predicate a beat
/// waits on before pointing at a widget.
pub fn ui_node_rect(world: &World, name: &str) -> Option<Rect> {
    let Some(mut query) = world.try_query::<(
        &Name,
        &UiGlobalTransform,
        &ComputedNode,
        &InheritedVisibility,
    )>() else {
        // A component this reads is not registered, which is the same answer as
        // "no such node": nothing has spawned one yet.
        return None;
    };
    let rects: Vec<Rect> = query
        .iter(world)
        .filter(|(node_name, _, _, visibility)| node_name.as_str() == name && visibility.get())
        .map(|(_, transform, computed, _)| {
            let scale = computed.inverse_scale_factor();
            Rect::from_center_size(transform.translation * scale, computed.size() * scale)
        })
        .filter(is_a_target)
        .collect();
    if rects.len() > 1 {
        warn!(
            "autopilot: {} laid-out UI nodes are named `{name}`; pointing at the first",
            rects.len()
        );
    }
    rects.first().copied()
}

/// Whether a resolved rect is somewhere a pointer can actually be put: a finite
/// box with area.
///
/// The pre-layout and `Display::None` cases both arrive here as a zero-size rect
/// (`ComputedNode::default()` is zero size, and its inverse scale factor is zero
/// too, which collapses the centre as well). A degenerate box is not a target,
/// and pointing at one is how a gesture is silently lost.
fn is_a_target(rect: &Rect) -> bool {
    rect.min.is_finite() && rect.max.is_finite() && rect.width() > 0.0 && rect.height() > 0.0
}

/// The logical-pixel centre of the laid-out UI node called `name`, if there is
/// one - [`ui_node_rect`]'s centre, and what the pointer beats aim at.
pub fn ui_node_centre(world: &World, name: &str) -> Option<Vec2> {
    ui_node_rect(world, name).map(|rect| rect.center())
}

/// A beat that PANICS unless the UI node called `name` is laid out AND
/// visible.
///
/// The deliberate counterpart to [`click_named`]'s warn-and-continue: a
/// gesture that misses its target is one lost beat, but a script asserting the
/// screen it has just navigated to must take the run down rather than let the
/// next beat capture the wrong screen.
pub fn assert_named_visible(
    name: impl Into<String>,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    let name = name.into();
    move |world: &mut World| {
        assert!(
            ui_node_rect(world, &name).is_some(),
            "no laid-out, visible UI node named `{name}`"
        );
    }
}

/// Move the pointer to the centre of the UI node called `name` and press the
/// left button there - [`click_at`] without the literal coordinates.
///
/// Resolving by [`Name`] is what makes a run survive a layout move: only a
/// RENAME breaks it. Like [`move_cursor`] without a window, an unresolved name
/// warns and continues - a beat that cannot find its target must not take the
/// whole run down.
pub fn click_named(name: impl Into<String>) -> impl Fn(&mut World) + Send + Sync + 'static {
    let name = name.into();
    move |world: &mut World| {
        let Some(centre) = resolve(world, &name, "click") else {
            return;
        };
        set_cursor(world, centre);
        set_mouse_button(world, MouseButton::Left, ButtonState::Pressed);
    }
}

/// Move the pointer to the centre of the UI node called `name` without
/// pressing - the hover half of [`click_named`], and the first leg of a drag.
pub fn hover_named(name: impl Into<String>) -> impl Fn(&mut World) + Send + Sync + 'static {
    let name = name.into();
    move |world: &mut World| {
        let Some(centre) = resolve(world, &name, "hover") else {
            return;
        };
        set_cursor(world, centre);
    }
}

/// The shared warn-and-continue resolve behind [`click_named`] and
/// [`hover_named`].
fn resolve(world: &World, name: &str, gesture: &str) -> Option<Vec2> {
    let centre = ui_node_centre(world, name);
    if centre.is_none() {
        warn!("autopilot: {gesture} on `{name}` found no laid-out UI node with that Name");
    }
    centre
}

/// The shared body of [`move_cursor`] and [`click_at`]: place the pointer in
/// the primary window, announce the move, and PIN the position so a foreign
/// event cannot take the pointer somewhere else mid-gesture.
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
    world.insert_resource(PinnedCursor(position));
    let moved = cursor_moved(window, position);
    world.write_message(moved.clone());
    // The picking backend reads `WindowEvent`, NOT the concrete message, and it
    // tracks the cursor from those events alone - so a click without this
    // wrapper lands at whatever position picking last saw. `bevy_winit` writes
    // both for every real pointer move; so does this.
    world.write_message(WindowEvent::CursorMoved(moved));
}

/// The move a synthesized pointer reports.
///
/// A synthesized pointer teleports; a real one reports the step it took. `None`
/// is the honest answer and the one Bevy itself writes for a cursor that was
/// outside the window last frame.
fn cursor_moved(window: Entity, position: Vec2) -> CursorMoved {
    CursorMoved {
        window,
        position,
        delta: None,
    }
}

/// Where the driver last pointed. Absent until a run performs its first pointer
/// gesture, which is what keeps the pin inert in a keyboard-only script.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
struct PinnedCursor(Vec2);

/// How far the window may drift from the pin before it counts as a foreign
/// move, squared: half a logical pixel, which no real gesture lands inside and
/// no scale-factor round trip escapes.
const HELD_TOLERANCE_SQ: f32 = 0.25;

/// Give the driven run the pointer: re-assert the pinned position in `First`,
/// AFTER `bevy_winit` has written the frame's real events and BEFORE
/// `PickingSystems::Input` turns them into `PointerInput`.
///
/// Called by the autopilot's own `build`, so the pin is armed exactly when a
/// script is - see the module docs for what it defends against.
pub(crate) fn register_pointer_pin(app: &mut App) {
    app.add_systems(
        First,
        repin_cursor
            .after(bevy::ecs::message::MessageUpdateSystems)
            .before(PickingSystems::Input)
            // A windowless rig (`MinimalPlugins` without `WindowPlugin`) never
            // registers the pointer messages, and there is no pointer there to
            // hold anyway.
            .run_if(
                resource_exists::<Messages<CursorMoved>>
                    .and_then(resource_exists::<Messages<WindowEvent>>),
            ),
    );
}

/// Put the pointer back where the script left it.
///
/// A foreign move is detectable from the window alone: `bevy_winit` writes the
/// position into [`Window`] as well as into the messages
/// (`bevy_winit/src/state.rs:292`), and so does [`set_cursor`] - so a window
/// disagreeing with the pin means somebody else moved the pointer. `None`
/// (the cursor left the window) counts as a disagreement.
///
/// "Disagreeing" is half a logical pixel, not float equality: the driver's own
/// OS-level warp echoes back through winit as PHYSICAL pixels, and on a
/// fractional scale factor the trip back to logical need not land on the same
/// float. Exact comparison would read that rounding as a stray and re-warp on
/// every frame.
///
/// The correction is appended to the same batch the stray is in. Picking reads
/// the moves in order, so the driver's is the one that lands.
fn repin_cursor(
    pinned: Option<Res<PinnedCursor>>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut moves: MessageWriter<CursorMoved>,
    mut events: MessageWriter<WindowEvent>,
) {
    let Some(pinned) = pinned else {
        return;
    };
    let Ok((entity, mut window)) = windows.single_mut() else {
        return;
    };
    let held = window
        .cursor_position()
        .is_some_and(|position| position.distance_squared(pinned.0) <= HELD_TOLERANCE_SQ);
    if held {
        return;
    }
    debug!(
        "autopilot: a foreign pointer move to {:?} was overridden; the run holds {:?}",
        window.cursor_position(),
        pinned.0
    );
    window.set_cursor_position(Some(pinned.0));
    let moved = cursor_moved(entity, pinned.0);
    moves.write(moved.clone());
    events.write(WindowEvent::CursorMoved(moved));
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
    use crate::log_capture::capturing_logs;

    /// A headless app with real input collection and a primary window, so the
    /// actions write the resources and messages the game reads.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));
        app.add_message::<CursorMoved>();
        // `WindowPlugin` is what registers this in a real app. Without it the
        // wrapper half of every gesture goes nowhere, and a test asserting on
        // the wrapper alone would have nothing to read.
        app.add_message::<WindowEvent>();
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

    fn wheels(app: &mut App) -> Vec<MouseWheel> {
        app.world_mut()
            .resource_mut::<Messages<MouseWheel>>()
            .drain()
            .collect()
    }

    /// The wheel notches a `WindowEvent::MouseWheel` carries, in order.
    fn wrapped_wheels(app: &mut App) -> Vec<MouseWheel> {
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .drain()
            .filter_map(|event| match event {
                WindowEvent::MouseWheel(wheel) => Some(wheel),
                _ => None,
            })
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

    /// A synthesized notch leaves BOTH halves a real one leaves: the concrete
    /// message a game system reads, and the `WindowEvent` wrapper the picking
    /// backend turns into `PointerAction::Scroll`. Writing only the message
    /// scrolls the pane but never reaches picking; writing only the wrapper does
    /// the reverse.
    #[test]
    fn a_wheel_scroll_writes_both_halves_a_real_notch_writes() {
        let mut app = app();

        scroll_lines(-3.0)(app.world_mut());

        let wheels = wheels(&mut app);
        assert_eq!(wheels.len(), 1, "one gesture is one notch");
        assert_eq!(wheels[0].unit, MouseScrollUnit::Line);
        assert_eq!(
            wheels[0].y, -3.0,
            "the delta passes through with its sign, which is what says \
             which way the view moves"
        );
        assert_eq!(wheels[0].x, 0.0, "the driver does not scroll sideways");
        assert_eq!(
            wrapped_wheels(&mut app),
            wheels,
            "the wrapper carries the same notch, for the picking backend"
        );
    }

    /// The two constructors must stay apart: a reader multiplies a LINE by its
    /// own line height and takes a PIXEL as it stands, so a beat that measured
    /// its gap in pixels and got lines would travel 20x too far.
    #[test]
    fn scroll_pixels_reports_the_pixel_unit() {
        let mut app = app();

        scroll_pixels(-96.0)(app.world_mut());

        let wheels = wheels(&mut app);
        assert_eq!(wheels.len(), 1);
        assert_eq!(wheels[0].unit, MouseScrollUnit::Pixel);
        assert_eq!(wheels[0].y, -96.0);
    }

    /// Like the other pointer gestures: a headless smoke run that turns the
    /// wheel must not die on it.
    #[test]
    fn a_wheel_scroll_without_a_window_is_harmless() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));

        scroll_lines(1.0)(app.world_mut());

        assert!(wheels(&mut app).is_empty());
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

    /// A laid-out UI node named on a beat resolves to its own centre, in the
    /// LOGICAL pixels `move_cursor` takes.
    #[test]
    fn a_named_ui_node_resolves_to_its_centre() {
        let mut app = app();
        spawn_named_node(&mut app, "Play Button", Vec2::new(400.0, 120.0));

        assert_eq!(
            ui_node_centre(app.world_mut(), "Play Button"),
            Some(Vec2::new(400.0, 120.0))
        );
        assert_eq!(
            ui_node_centre(app.world_mut(), "No Such Button"),
            None,
            "an unknown name resolves to nothing rather than to some other node"
        );
    }

    /// Two laid-out nodes sharing a name resolve to ONE of them and SAY SO - the
    /// resolve must not pick silently, because a click on a ghost leaves the run
    /// green (review R1.5). Which one is arbitrary; that it is one of the two,
    /// that the beat survives, and that the ambiguity is logged once naming the
    /// count, is the contract.
    ///
    /// The log is captured rather than assumed: the warn has no return value, so
    /// without capture this test passes with the warn deleted and covers nothing
    /// the fix added (review R2.4).
    #[test]
    fn a_duplicated_name_warns_and_resolves_to_one_of_them() {
        let mut app = app();
        spawn_named_node(&mut app, "Play Button", Vec2::new(400.0, 120.0));
        spawn_named_node(&mut app, "Play Button", Vec2::new(10.0, 20.0));

        let (centre, logs) = capturing_logs(|| ui_node_centre(app.world_mut(), "Play Button"));

        let centre = centre.expect("a duplicated name still resolves rather than vanishing");
        assert!(
            centre == Vec2::new(400.0, 120.0) || centre == Vec2::new(10.0, 20.0),
            "the resolve lands on one of the two nodes, not somewhere else ({centre})"
        );
        assert_eq!(
            logs.matches("2 laid-out UI nodes are named `Play Button`")
                .count(),
            1,
            "the duplicate must be reported once, naming the count: {logs}"
        );
    }

    /// The complement: a name only ONE node carries resolves without any
    /// ambiguity warning, so the duplicate warn cannot be a blanket log that
    /// fires on every resolve.
    #[test]
    fn a_unique_name_resolves_without_warning() {
        let mut app = app();
        spawn_named_node(&mut app, "Play Button", Vec2::new(400.0, 120.0));

        let (centre, logs) = capturing_logs(|| ui_node_centre(app.world_mut(), "Play Button"));

        assert_eq!(centre, Some(Vec2::new(400.0, 120.0)));
        assert!(
            !logs.contains("laid-out UI nodes are named"),
            "a unique name must resolve silently: {logs}"
        );
    }

    /// [`ui_node_rect`] is where the physical-to-logical conversion lives, and
    /// it converts the node's SIZE as well as its centre - a caller asking
    /// whether one node's centre lies inside another's box needs both in the
    /// same space.
    ///
    /// Spawned at scale factor 2 ON PURPOSE: at scale 1 the size conversion is
    /// the identity, so the test passed with `* scale` deleted from the size
    /// term and covered nothing (review R3.1).
    #[test]
    fn a_named_node_resolves_to_its_logical_rect() {
        let mut app = app();
        spawn_named_node_at_scale(&mut app, "Scenarios List", Vec2::new(400.0, 120.0), 2.0);

        let rect = ui_node_rect(app.world_mut(), "Scenarios List")
            .expect("a laid-out node has a rect, not just a point");

        assert_eq!(rect.center(), Vec2::new(400.0, 120.0));
        assert_eq!(rect.size(), NODE_SIZE);
        assert_eq!(
            ui_node_rect(app.world_mut(), "No Such List"),
            None,
            "an unknown name resolves to nothing rather than to some other node"
        );
    }

    /// `click_named` is `click_at` with the coordinate looked up: the pointer
    /// lands on the node's centre and the button edge is fresh there.
    #[test]
    fn click_named_lands_on_the_named_node() {
        let mut app = app();
        spawn_named_node(&mut app, "Hardware", Vec2::new(220.0, 60.0));

        click_named("Hardware")(app.world_mut());

        assert_eq!(
            cursor_moves(&mut app)
                .iter()
                .map(|moved| moved.position)
                .collect::<Vec<_>>(),
            vec![Vec2::new(220.0, 60.0)]
        );
        assert!(app
            .world()
            .resource::<ButtonInput<MouseButton>>()
            .just_pressed(MouseButton::Left));
    }

    /// `hover_named` positions without pressing, the way `move_cursor` does.
    #[test]
    fn hover_named_positions_without_pressing() {
        let mut app = app();
        spawn_named_node(&mut app, "Idle", Vec2::new(30.0, 40.0));

        hover_named("Idle")(app.world_mut());

        assert_eq!(
            cursor_moves(&mut app)
                .iter()
                .map(|moved| moved.position)
                .collect::<Vec<_>>(),
            vec![Vec2::new(30.0, 40.0)]
        );
        assert!(!app
            .world()
            .resource::<ButtonInput<MouseButton>>()
            .just_pressed(MouseButton::Left));
    }

    /// A name nothing answers to warns and continues: a widget that has not
    /// spawned yet must not take the run down, and it must not click SOMEWHERE
    /// as a consolation either.
    #[test]
    fn a_click_on_an_absent_name_is_harmless() {
        let mut app = app();
        spawn_named_node(&mut app, "Idle", Vec2::new(30.0, 40.0));

        click_named("Not Spawned Yet")(app.world_mut());

        assert!(cursor_moves(&mut app).is_empty());
        assert!(!app
            .world()
            .resource::<ButtonInput<MouseButton>>()
            .just_pressed(MouseButton::Left));
    }

    /// The LOGICAL extent every spawned test node lays out at. Non-zero so a
    /// rect resolve has a size to convert, and asymmetric so a transposed
    /// width/height cannot pass.
    const NODE_SIZE: Vec2 = Vec2::new(120.0, 40.0);

    /// A UI node as the resolve sees it: a `Name`, a centre in PHYSICAL px, and
    /// the `ComputedNode` carrying the scale factor back to logical. Spawned by
    /// hand because `MinimalPlugins` runs no layout pass.
    fn spawn_named_node_at_scale(app: &mut App, name: &str, centre: Vec2, scale_factor: f32) {
        spawn_node(
            app,
            name,
            centre,
            scale_factor,
            InheritedVisibility::VISIBLE,
        );
    }

    /// The same at scale factor 1, where physical and logical coincide.
    fn spawn_named_node(app: &mut App, name: &str, centre: Vec2) {
        spawn_named_node_at_scale(app, name, centre, 1.0);
    }

    /// A node laid out exactly like the others but hidden by its ancestry -
    /// what a [`Visibility`]-toggled panel looks like while it is closed.
    fn spawn_hidden_node(app: &mut App, name: &str, centre: Vec2) {
        spawn_node(app, name, centre, 1.0, InheritedVisibility::HIDDEN);
    }

    fn spawn_node(
        app: &mut App,
        name: &str,
        centre: Vec2,
        scale_factor: f32,
        visibility: InheritedVisibility,
    ) {
        app.world_mut().spawn((
            Name::new(name.to_string()),
            UiGlobalTransform::from_translation(centre * scale_factor),
            ComputedNode {
                inverse_scale_factor: 1.0 / scale_factor,
                size: NODE_SIZE * scale_factor,
                ..default()
            },
            visibility,
        ));
    }

    /// A node its ancestry has hidden keeps its layout, so a resolve that only
    /// asked for one would hand back the rect of a panel nobody can see.
    #[test]
    fn a_hidden_node_does_not_resolve() {
        let mut app = app();
        spawn_hidden_node(&mut app, "Settings Panel", Vec2::new(400.0, 120.0));

        assert_eq!(ui_node_rect(app.world_mut(), "Settings Panel"), None);
    }

    /// The duplicate-name path and the hidden path must not cancel out: a
    /// closed panel and the open one share a name while the toggle animates.
    #[test]
    fn a_hidden_node_does_not_shadow_its_visible_namesake() {
        let mut app = app();
        spawn_hidden_node(&mut app, "Settings Panel", Vec2::new(10.0, 20.0));
        spawn_named_node(&mut app, "Settings Panel", Vec2::new(400.0, 120.0));

        let (centre, logs) = capturing_logs(|| ui_node_centre(app.world_mut(), "Settings Panel"));

        assert_eq!(centre, Some(Vec2::new(400.0, 120.0)));
        assert!(
            !logs.contains("laid-out UI nodes are named"),
            "the hidden node is not a candidate, so there is no ambiguity: {logs}"
        );
    }

    /// The assertion beat is the counterpart to `click_named`'s
    /// warn-and-continue: it must take the run DOWN on a hidden target, which
    /// is the screenshot bug it exists to catch.
    #[test]
    #[should_panic(expected = "no laid-out, visible UI node named `Settings Panel`")]
    fn asserting_a_hidden_node_takes_the_run_down() {
        let mut app = app();
        spawn_hidden_node(&mut app, "Settings Panel", Vec2::new(400.0, 120.0));

        assert_named_visible("Settings Panel")(app.world_mut());
    }

    /// The complement, so the assertion cannot be a blanket panic.
    #[test]
    fn asserting_a_visible_node_passes() {
        let mut app = app();
        spawn_named_node(&mut app, "Settings Panel", Vec2::new(400.0, 120.0));

        assert_named_visible("Settings Panel")(app.world_mut());
    }

    /// The physical-vs-logical trap, pinned: a hi-dpi window lays the node out
    /// in physical px, and the pointer actions take logical ones. A resolve
    /// that skipped the conversion would click at twice the coordinates here.
    #[test]
    fn a_named_node_resolves_to_logical_pixels_on_a_scaled_window() {
        let mut app = app();
        spawn_named_node_at_scale(&mut app, "Play Button", Vec2::new(400.0, 120.0), 2.0);

        assert_eq!(
            ui_node_centre(app.world_mut(), "Play Button"),
            Some(Vec2::new(400.0, 120.0))
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

/// The synthesized input steps: key and mouse press/release, cursor moves,
/// wheel scrolls, clicks by position or name, and the node-geometry helpers they
/// resolve through.
pub mod prelude {
    pub use super::{
        assert_named_visible, click_at, click_named, hover_named, move_cursor, press_key,
        press_mouse, release_key, release_mouse, scroll_lines, scroll_pixels, ui_node_centre,
        ui_node_rect,
    };
}
