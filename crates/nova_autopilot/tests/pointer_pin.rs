//! A driven click survives a foreign pointer event landing mid-gesture.
//!
//! The regression this pins (task 20260805-091151): `bevy_picking` dispatches
//! `Pointer<Click>` from the PREVIOUS frame's hover map, so a real cursor event
//! between a press beat and its release beat takes the hover off the widget and
//! the click is never emitted - a button acting on `Activate` simply never
//! fires, with no warning and no error. On a shared display that made the
//! windowed example suite fail roughly one run in three.
//!
//! Its own test BINARY because it arms `NOVA_AUTOPILOT` process-wide.
//!
//! The rig is the real chain, one link faked: real pointer synthesis, real
//! `bevy_picking` input/hover/event systems, a real `bevy_ui_widgets::Button`
//! reporting `Activate`. Only the picking BACKEND is stood in for, because the
//! `bevy_ui` one needs a camera, a render target and a layout pass that a
//! headless test has no business booting - [`report_hits`] answers the same
//! question ("is the pointer inside this rect?") the UI backend would.

use std::time::Duration;

use bevy::{
    input::InputPlugin,
    picking::{
        backend::{HitData, PointerHits},
        input::PointerInputPlugin,
        pointer::{PointerId, PointerLocation},
        InteractionPlugin, PickingPlugin, PickingSystems,
    },
    prelude::*,
    state::app::StatesPlugin,
    time::TimeUpdateStrategy,
    ui_widgets::{Activate, Button, ButtonPlugin},
    window::{PrimaryWindow, WindowEvent, WindowResolution},
};
use nova_autopilot::prelude::*;

/// The name the script clicks, and the rect it lays out at.
const TARGET: &str = "Target Button";
const CENTRE: Vec2 = Vec2::new(400.0, 300.0);
const SIZE: Vec2 = Vec2::new(120.0, 40.0);

/// Where the foreign event drags the pointer: outside the target, so a pointer
/// that obeys it stops hovering the button.
const STRAY: Vec2 = Vec2::new(10.0, 10.0);

/// Frames are manual, so the run does not depend on how fast the test host is.
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// Beats between the gestures: enough for a press to be seen and reported, and
/// enough for a stray to do its damage if it is going to.
const SETTLE: u32 = 3;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum TestState {
    #[default]
    Driving,
}

/// Set by the button's own `Activate` observer - the ONE thing the test reads,
/// because it is the thing the game reacts to.
#[derive(Resource, Default)]
struct Activated(bool);

#[test]
fn a_foreign_cursor_event_mid_click_does_not_cancel_the_click() {
    // SAFETY: set before any thread of this binary reads it, and never unset.
    unsafe { std::env::set_var(AUTOPILOT_ENV, "1") };

    let mut app = app(true);
    run(&mut app, 30);

    assert!(
        app.world().resource::<Activated>().0,
        "the click must reach the button: the run pointed at it, pressed it, \
         and released it there - a foreign cursor event in between is not the \
         player's input and must not cancel the gesture"
    );
}

/// The rig control: a pointer the RUN ITSELF moves off the button before the
/// release does lose the click. It says the assertion above is not vacuous -
/// this chain really does depend on where the pointer was the frame before the
/// release - and it pins the behaviour a script must not write by accident.
///
/// It does NOT isolate the pin; nothing in-process can unregister it. That
/// number is in `tasks/20260805-091151/TASK.md`, measured by deleting the
/// registration.
#[test]
fn a_pointer_the_run_moves_away_does_cancel_the_click() {
    // SAFETY: as above.
    unsafe { std::env::set_var(AUTOPILOT_ENV, "1") };

    let mut app = app(false);
    run(&mut app, 30);

    assert!(
        !app.world().resource::<Activated>().0,
        "the rig must be able to break the click, or the case above proves \
         nothing"
    );
}

/// The rig. `pinned` says whether the foreign move is a genuine stray (the
/// pinned run overrides it) or is allowed to move the pin with it (which is
/// what an unpinned driver amounts to).
fn app(pinned: bool) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        InputPlugin,
        StatesPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        },
        PickingPlugin,
        PointerInputPlugin,
        InteractionPlugin,
        ButtonPlugin,
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    app.init_state::<TestState>();
    app.init_resource::<Activated>();
    app.add_systems(PreUpdate, report_hits.in_set(PickingSystems::Backend));
    app.add_systems(Startup, spawn_target);
    app.add_plugins(script(pinned));
    app
}

/// The target as the resolve sees it: a `Name` to aim at, a laid-out rect to
/// aim INTO, and the button behaviour under test. Spawned by hand because this
/// app runs no layout pass.
fn spawn_target(mut commands: Commands) {
    commands
        .spawn((
            Name::new(TARGET),
            Button,
            UiGlobalTransform::from_translation(CENTRE),
            ComputedNode {
                inverse_scale_factor: 1.0,
                size: SIZE,
                ..default()
            },
        ))
        .observe(|_: On<Activate>, mut activated: ResMut<Activated>| {
            activated.0 = true;
        });
}

/// The picking backend, stood in for: hit the target whenever the pointer is
/// inside its rect. `bevy_ui`'s own backend answers the same question against
/// the same components.
fn report_hits(
    pointers: Query<(&PointerId, &PointerLocation)>,
    target: Query<Entity, With<Button>>,
    mut hits: MessageWriter<PointerHits>,
) {
    let rect = Rect::from_center_size(CENTRE, SIZE);
    // No camera is booted here, and no consumer in this chain reads the hit's
    // camera - the hover map orders by `order` and depth.
    let camera = Entity::PLACEHOLDER;
    for (id, location) in &pointers {
        let Some(location) = location.location.as_ref() else {
            continue;
        };
        if !rect.contains(location.position) {
            continue;
        }
        for entity in &target {
            hits.write(PointerHits {
                pointer: *id,
                picks: vec![(entity, HitData::new(camera, 0.0, None, None))],
                order: 0.0,
            });
        }
    }
}

/// Point at the button, press it, let a foreign cursor event land, release.
/// Exactly the shape every `ui/` example's click carries.
fn script(pinned: bool) -> AutopilotPlugin<TestState> {
    AutopilotPlugin::<TestState>::new()
        .step("let the target lay out")
        .until(frames(SETTLE))
        .add()
        .step("click the target")
        .on_enter(click_named(TARGET))
        .until(frames(SETTLE))
        .add()
        .step("a foreign pointer event lands")
        .on_enter(move |world: &mut World| foreign_move(world, pinned))
        .until(frames(SETTLE))
        .add()
        .step("release the target")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
}

/// A cursor move the run did not make, written the way `bevy_winit` writes a
/// real one: the position into the window AND into both message halves
/// (`bevy_winit/src/state.rs:292`).
///
/// With `pinned` false it goes through the driver's own `move_cursor` instead,
/// which moves the pin with it - the unpinned control.
fn foreign_move(world: &mut World, pinned: bool) {
    if !pinned {
        move_cursor(STRAY)(world);
        return;
    }
    let mut windows = world.query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>();
    let Ok((entity, mut window)) = windows.single_mut(world) else {
        return;
    };
    window.set_cursor_position(Some(STRAY));
    let moved = CursorMoved {
        window: entity,
        position: STRAY,
        delta: None,
    };
    world.write_message(moved.clone());
    world.write_message(WindowEvent::CursorMoved(moved));
}

fn run(app: &mut App, frames: u32) {
    for _ in 0..frames {
        app.update();
    }
}
