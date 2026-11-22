//! The pilot's RCS through the real input rig: SHIFT, the mouse and the
//! MATCH chord on the hand-wired flight harness.

use avian3d::prelude::*;
use bevy::{
    input::{mouse::MouseMotion, InputPlugin},
    prelude::*,
};
use bevy_enhanced_input::prelude::EnhancedInputPlugin;
use nova_gameplay::{prelude::*, test_support::settle};

use super::support::*;
use crate::prelude::*;

/// The flight harness with the player's real input plugin on it.
fn piloted_app() -> App {
    let mut app = unfinished_flight_app();
    app.add_plugins((
        bevy::state::app::StatesPlugin,
        InputPlugin,
        EnhancedInputPlugin,
        crate::input::player::SpaceshipPlayerInputPlugin,
    ));
    app.init_state::<PauseStates>();
    app.finish();
    app
}

/// A player RCS hull with its flight rig spawned and settled.
fn spawn_pilot(app: &mut App) -> Entity {
    let (ship, _controller) = spawn_rcs_ship(app);
    app.world_mut()
        .entity_mut(ship)
        .insert((PlayerSpaceshipMarker, TravelLock(None)));
    settle(app);
    ship
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

fn mouse(app: &mut App, delta: Vec2) {
    app.world_mut().write_message(MouseMotion { delta });
}

fn budget_of(app: &App, ship: Entity) -> RcsBudget {
    *app.world().get::<RcsBudget>(ship).unwrap()
}

/// Hold SHIFT and sweep the mouse for `frames` frames: a real manual burn.
fn burn_with_the_mouse(app: &mut App, frames: usize) {
    for _ in 0..frames {
        mouse(app, Vec2::new(40.0, 0.0));
        app.update();
    }
}

/// SHIFT held with a hand resting on the mouse refills the magazine: holding
/// the mode is not RCS use, and neither is the one-count tick a resting
/// sensor reports now and then.
#[test]
fn the_rcs_magazine_refills_while_shift_is_held_with_no_input() {
    let mut app = piloted_app();
    let ship = spawn_pilot(&mut app);
    press(&mut app, KeyCode::ShiftLeft);
    app.update();
    assert!(app.world().get::<RcsActive>(ship).is_some(), "SHIFT is RCS");
    burn_with_the_mouse(&mut app, 60);
    let spent = budget_of(&app, ship).spent;
    assert!(spent > 0.0, "the sweep burned RCS");

    // Three seconds with SHIFT still held and the hand at rest: a one-count
    // tick, on either axis, every third of a second.
    for frame in 0..180 {
        match frame % 40 {
            0 => mouse(&mut app, Vec2::new(1.0, 0.0)),
            20 => mouse(&mut app, Vec2::new(0.0, -1.0)),
            _ => {}
        }
        app.update();
    }
    assert!(app.world().get::<RcsActive>(ship).is_some(), "still in RCS");
    let after = budget_of(&app, ship).spent;
    assert!(
        after < spent,
        "past the delay the magazine refills with SHIFT held ({after} of {spent} still spent)"
    );
}

/// A pilot still moving the mouse under SHIFT is burning, however slowly:
/// two counts a frame push the hull and keep the magazine off recovery.
#[test]
fn a_held_manual_burn_still_holds_the_refill_off() {
    let mut app = piloted_app();
    let ship = spawn_pilot(&mut app);
    press(&mut app, KeyCode::ShiftLeft);
    app.update();

    let mut last = budget_of(&app, ship).spent;
    // Three seconds, past the recovery delay.
    for _ in 0..180 {
        mouse(&mut app, Vec2::new(2.0, 0.0));
        app.update();
        let spent = budget_of(&app, ship).spent;
        assert!(
            spent > last,
            "every frame of a slow sweep spends ({spent} after {last})"
        );
        last = spent;
    }
    assert!(velocity_of(&app, ship).x > 0.0, "the sweep strafed +X");
}

/// Lock a body moving -Z and engage MATCH on it with SHIFT + X, SHIFT left
/// held, then let MATCH fly for a few frames.
fn engage_match(app: &mut App, ship: Entity) {
    let target = app
        .world_mut()
        .spawn((
            RigidBody::Dynamic,
            Transform::from_xyz(0.0, 0.0, -400.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
            LinearVelocity(Vec3::new(0.0, 0.0, -20.0)),
        ))
        .id();
    app.world_mut()
        .entity_mut(ship)
        .insert(TravelLock(Some(target)));
    settle(app);

    press(app, KeyCode::ShiftLeft);
    app.update();
    press(app, KeyCode::KeyX);
    app.update();
    app.update();
    assert!(matching(app, ship), "SHIFT + X engages MATCH");
    run(app, 10);
}

fn matching(app: &App, ship: Entity) -> bool {
    matches!(
        app.world().get::<Autopilot>(ship).map(|ap| ap.action),
        Some(AutopilotAction::MatchTarget { .. })
    )
}

/// Mouse RCS input during the SHIFT + X MATCH takes the hull back: MATCH
/// ends and the manual command flies on the same frame.
#[test]
fn manual_rcs_input_ends_the_shift_x_match_and_flies_the_manual_command() {
    let mut app = piloted_app();
    let ship = spawn_pilot(&mut app);
    engage_match(&mut app, ship);
    let before = velocity_of(&app, ship).x;

    mouse(&mut app, Vec2::new(40.0, 0.0));
    app.update();
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "the mouse under SHIFT ends MATCH"
    );
    let budget = budget_of(&app, ship);
    let intent = app.world().get::<RcsIntent>(ship).unwrap().0;
    assert!(
        budget.applied > 0.0,
        "the manual command burned on the same frame (intent {intent:?})"
    );
    let velocity = velocity_of(&app, ship);
    assert!(
        velocity.x > before + 1e-3,
        "the manual strafe pushed +X on that frame ({velocity:?})"
    );
}

/// A hand settling on the mouse under SHIFT moves a few counts a frame for a
/// few frames. MATCH holds the hull through it; a push that keeps going ends
/// MATCH once it adds up, and that frame flies.
#[test]
fn a_hand_settling_on_the_mouse_does_not_end_the_shift_x_match() {
    let mut app = piloted_app();
    let ship = spawn_pilot(&mut app);
    engage_match(&mut app, ship);

    for (x, y) in [(2.0, 0.0), (3.0, -1.0), (-2.0, 2.0), (0.0, 3.0), (2.0, 1.0)] {
        mouse(&mut app, Vec2::new(x, y));
        app.update();
        assert!(
            matching(&app, ship),
            "a settling hand ({x}, {y}) keeps MATCH"
        );
    }

    let mut frames = 0;
    while matching(&app, ship) {
        assert!(frames < 10, "a steady push ends MATCH once it adds up");
        let before = velocity_of(&app, ship).x;
        mouse(&mut app, Vec2::new(4.0, 0.0));
        app.update();
        frames += 1;
        if !matching(&app, ship) {
            assert!(
                velocity_of(&app, ship).x > before + 1e-4,
                "the push that ends MATCH strafes +X on that frame"
            );
        }
    }
}
