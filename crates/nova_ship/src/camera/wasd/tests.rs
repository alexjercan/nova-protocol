//! The free-fly rig's travel, ramp and lens.

use core::time::Duration;

use bevy::{camera::Projection, time::TimeUpdateStrategy};

use super::*;

/// The longest frame these tests ask for. The virtual clock clamps a frame to
/// a quarter second by default, which would quietly cut every one-second frame
/// below to a quarter of the travel it asked for.
const TEST_MAX_DELTA: Duration = Duration::from_secs(10);

/// A camera with the rig on it and a manual clock, so a test says how long a
/// frame took rather than hoping.
fn rig_app(camera: WASDCamera) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(WASDCameraPlugin);
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(TEST_MAX_DELTA);
    let entity = app
        .world_mut()
        .spawn((Transform::default(), Projection::default(), camera))
        .id();
    // The warm-up frame: a manual strategy pays its first update a zero delta,
    // so a test that read the first one would measure nothing.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();
    (app, entity)
}

/// Advance one frame of `seconds` with `input` held.
fn fly(app: &mut App, entity: Entity, input: WASDCameraInput, seconds: f32) {
    *app.world_mut().get_mut::<WASDCameraInput>(entity).unwrap() = input;
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        seconds,
    )));
    app.update();
}

fn position(app: &App, entity: Entity) -> Vec3 {
    app.world().get::<Transform>(entity).unwrap().translation
}

fn fov(app: &App, entity: Entity) -> f32 {
    match app.world().get::<Projection>(entity).unwrap() {
        Projection::Perspective(perspective) => perspective.fov,
        other => panic!("the rig is built on a perspective lens, not {other:?}"),
    }
}

/// Forward, held.
fn forward() -> WASDCameraInput {
    WASDCameraInput {
        wasd: Vec2::new(0.0, 1.0),
        ..default()
    }
}

/// The defect this rig was frame-stepped into: the same held key crossed twice
/// the distance on a machine drawing twice the frames. One second of travel has
/// to be one second of travel however it is cut up.
#[test]
fn a_held_key_crosses_the_same_distance_at_any_frame_rate() {
    let constant = WASDCamera {
        accelerate: false,
        fov_feedback: false,
        ..default()
    };

    let (mut slow, slow_camera) = rig_app(constant);
    for _ in 0..6 {
        fly(&mut slow, slow_camera, forward(), 1.0 / 6.0);
    }

    let (mut fast, fast_camera) = rig_app(constant);
    for _ in 0..120 {
        fly(&mut fast, fast_camera, forward(), 1.0 / 120.0);
    }

    let travelled = position(&slow, slow_camera).length();
    assert!(
        (travelled - position(&fast, fast_camera).length()).abs() < 1e-3,
        "six frames flew {travelled} u and 120 flew {}",
        position(&fast, fast_camera).length()
    );
    assert!(
        (travelled - WASD_BASE_SPEED.to_engine()).abs() < 1e-3,
        "a second at the base speed is {} u, flew {travelled}",
        WASD_BASE_SPEED.to_engine()
    );
}

/// Two keys held is one press in a diagonal direction, not two presses'
/// worth of speed: without the clamp a corner ran 41% faster than an axis.
#[test]
fn a_diagonal_is_no_faster_than_an_axis() {
    let constant = WASDCamera {
        accelerate: false,
        fov_feedback: false,
        ..default()
    };
    let (mut app, camera) = rig_app(constant);
    fly(
        &mut app,
        camera,
        WASDCameraInput {
            wasd: Vec2::new(1.0, 1.0),
            vertical: 1.0,
            ..default()
        },
        1.0,
    );

    let travelled = position(&app, camera).length();
    assert!(
        (travelled - WASD_BASE_SPEED.to_engine()).abs() < 1e-3,
        "three axes at once flew {travelled} u, not the base speed's {}",
        WASD_BASE_SPEED.to_engine()
    );
}

/// A stick half over is half speed: the clamp must not normalize a partial
/// press up to a full one.
#[test]
fn a_partial_press_keeps_its_share_of_the_speed() {
    let (mut app, camera) = rig_app(WASDCamera {
        accelerate: false,
        fov_feedback: false,
        ..default()
    });
    fly(
        &mut app,
        camera,
        WASDCameraInput {
            wasd: Vec2::new(0.0, 0.5),
            ..default()
        },
        1.0,
    );

    let travelled = position(&app, camera).length();
    assert!(
        (travelled - WASD_BASE_SPEED.to_engine() * 0.5).abs() < 1e-3,
        "half a stick flew {travelled} u"
    );
}

/// The ramp: two seconds of held translation reach the 32x top, and a
/// constant-speed camera beside it never leaves the base speed.
#[test]
fn the_ramp_reaches_its_top_in_two_seconds_and_only_when_asked() {
    let (mut app, camera) = rig_app(WASDCamera {
        fov_feedback: false,
        ..default()
    });
    // Two seconds in steps, then one more frame to read the speed AT the top.
    for _ in 0..20 {
        fly(&mut app, camera, forward(), 0.1);
    }
    let before = position(&app, camera).length();
    fly(&mut app, camera, forward(), 0.1);
    let step = position(&app, camera).length() - before;
    let top = WASD_BASE_SPEED.to_engine() * WASD_ACCELERATION_MAX * 0.1;
    assert!(
        (step - top).abs() < 1e-2,
        "the top of the ramp stepped {step} u, not {top}"
    );

    let (mut constant, constant_camera) = rig_app(WASDCamera {
        accelerate: false,
        fov_feedback: false,
        ..default()
    });
    for _ in 0..21 {
        fly(&mut constant, constant_camera, forward(), 0.1);
    }
    let flown = position(&constant, constant_camera).length();
    let expected = WASD_BASE_SPEED.to_engine() * 2.1;
    assert!(
        (flown - expected).abs() < 1e-2,
        "a constant-speed camera flew {flown} u in 2.1 s, not {expected}"
    );
}

/// Releasing every translation key resets the ramp; turning the camera, or
/// swapping which key is held, does not - the journey is the same journey.
#[test]
fn releasing_translation_resets_the_ramp_and_a_direction_change_does_not() {
    let (mut app, camera) = rig_app(WASDCamera {
        fov_feedback: false,
        ..default()
    });
    for _ in 0..20 {
        fly(&mut app, camera, forward(), 0.1);
    }

    // Strafe instead of run, while panning: still translation, so the ramp
    // holds.
    let before = position(&app, camera).length();
    fly(
        &mut app,
        camera,
        WASDCameraInput {
            wasd: Vec2::new(1.0, 0.0),
            pan: Vec2::new(4.0, 0.0),
            ..default()
        },
        0.1,
    );
    let step = position(&app, camera).distance(Vec3::ZERO) - before;
    assert!(
        step.abs() > WASD_BASE_SPEED.to_engine() * 0.1,
        "a direction change must not drop the ramp (stepped {step} u)"
    );

    // Everything released, then pressed again: back in the base register. Not
    // exactly the base step - the press this frame is already 0.1 s of ramp -
    // but nowhere near the 32x the release threw away.
    fly(&mut app, camera, WASDCameraInput::default(), 0.1);
    let resting = *app.world().get::<Transform>(camera).unwrap();
    fly(&mut app, camera, forward(), 0.1);
    let after = app
        .world()
        .get::<Transform>(camera)
        .unwrap()
        .translation
        .distance(resting.translation);
    let base = WASD_BASE_SPEED.to_engine() * 0.1;
    assert!(
        after < base * 1.1,
        "the first frame after a release stepped {after} u, which is not the base {base}"
    );
}

/// The lens widens with the ramp, eases home in a quarter second once the keys
/// are let go, and is put back exactly as found when the rig comes off.
#[test]
fn the_lens_widens_with_the_ramp_and_is_given_back_on_removal() {
    let (mut app, camera) = rig_app(WASDCamera::default());
    let rest = fov(&app, camera);

    for _ in 0..20 {
        fly(&mut app, camera, forward(), 0.1);
    }
    let widened = fov(&app, camera);
    assert!(
        (widened - (rest + WASD_FOV_FEEDBACK.to_radians())).abs() < 1e-4,
        "the top of the ramp widened the lens to {widened} rad, not {} rad",
        rest + WASD_FOV_FEEDBACK.to_radians()
    );

    // A quarter of a second of nothing held brings it all the way home.
    fly(
        &mut app,
        camera,
        WASDCameraInput::default(),
        WASD_FOV_EASE_SECS,
    );
    assert!(
        (fov(&app, camera) - rest).abs() < 1e-4,
        "the lens must ease home in {WASD_FOV_EASE_SECS}s, sits at {}",
        fov(&app, camera)
    );

    // Widened again, then the rig taken away mid-ramp.
    for _ in 0..20 {
        fly(&mut app, camera, forward(), 0.1);
    }
    assert!(fov(&app, camera) > rest, "delivery guard: the lens is wide");
    app.world_mut().entity_mut(camera).remove::<WASDCamera>();
    app.update();
    assert!(
        (fov(&app, camera) - rest).abs() < 1e-4,
        "removing the rig must give the lens back, sits at {}",
        fov(&app, camera)
    );
}

/// The two feedbacks are independent: a camera may ramp with a still lens.
#[test]
fn a_camera_can_ramp_without_moving_its_lens() {
    let (mut app, camera) = rig_app(WASDCamera {
        fov_feedback: false,
        ..default()
    });
    let rest = fov(&app, camera);
    for _ in 0..20 {
        fly(&mut app, camera, forward(), 0.1);
    }

    assert!(
        position(&app, camera).length() > WASD_BASE_SPEED.to_engine() * 2.0,
        "delivery guard: the ramp ran"
    );
    assert_eq!(fov(&app, camera), rest, "the lens was told to stay put");
}
