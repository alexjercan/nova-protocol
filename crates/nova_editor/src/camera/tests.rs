use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::RunSystemOnce,
    input::{mouse::MouseScrollUnit, touch::TouchPhase},
    picking::pointer::Location,
    time::TimeUpdateStrategy,
    window::WindowRef,
};

use super::*;

const FRAME: f32 = 0.1;

fn rig_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(FRAME),
    ));
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<AccumulatedMouseMotion>()
        .init_resource::<ViewportPointer>()
        .init_resource::<GalleryState>()
        .init_resource::<SelectedNode>()
        .init_resource::<HeldPart>()
        .init_resource::<CameraFraming>()
        .init_resource::<EditorStatus>()
        .add_message::<MouseWheel>()
        .add_systems(Update, drive_editor_camera);
    let eye = app
        .world_mut()
        .spawn((EditorCamera, EditorRig::default(), Transform::default()))
        .id();
    // ManualDuration's first update is a delta of zero.
    app.update();
    (app, eye)
}

fn pose(app: &App, eye: Entity) -> Transform {
    *app.world().get::<Transform>(eye).expect("a pose")
}

fn rig(app: &App, eye: Entity) -> EditorRig {
    *app.world().get::<EditorRig>(eye).expect("a rig")
}

fn press_mouse(app: &mut App, button: MouseButton) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(button);
}

fn release_mouse(app: &mut App, button: MouseButton) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(button);
}

fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

/// One frame, with `just_pressed` cleared after it the way the input plugin
/// clears it, so a press starts one gesture.
fn step(app: &mut App) {
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn wheel(app: &mut App, y: f32) {
    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y,
        window: Entity::PLACEHOLDER,
        phase: TouchPhase::Moved,
    });
}

fn mouse_travel(app: &mut App, delta: Vec2) {
    app.world_mut()
        .resource_mut::<AccumulatedMouseMotion>()
        .delta = delta;
}

#[test]
fn a_held_key_moves_at_constant_speed_across_frames() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Right);
    press_key(&mut app, KeyCode::KeyW);

    let mut last = pose(&app, eye).translation;
    let expected = FLY_SPEED.to_engine() * FRAME;
    for frame in 0..30 {
        step(&mut app);
        let now = pose(&app, eye).translation;
        let moved = now.distance(last);
        assert!(
            (moved - expected).abs() < 1e-3,
            "frame {frame} moved {moved}, not {expected}: no ramp"
        );
        last = now;
    }
}

#[test]
fn a_diagonal_is_no_faster_than_one_key() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Right);
    press_key(&mut app, KeyCode::KeyW);
    press_key(&mut app, KeyCode::KeyD);
    let before = pose(&app, eye).translation;

    step(&mut app);

    let moved = pose(&app, eye).translation.distance(before);
    assert!((moved - FLY_SPEED.to_engine() * FRAME).abs() < 1e-3);
}

#[test]
fn the_fly_keys_do_nothing_without_rmb() {
    let (mut app, eye) = rig_app();
    press_key(&mut app, KeyCode::KeyW);
    let before = pose(&app, eye);

    step(&mut app);

    assert_eq!(pose(&app, eye), before);
}

#[test]
fn shift_is_a_fixed_multiple_of_the_dial() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Right);
    press_key(&mut app, KeyCode::KeyW);
    press_key(&mut app, KeyCode::ShiftLeft);
    let before = pose(&app, eye).translation;

    step(&mut app);

    let moved = pose(&app, eye).translation.distance(before);
    assert!((moved - FLY_SPEED.to_engine() * SPRINT * FRAME).abs() < 1e-3);
}

#[test]
fn the_wheel_steps_the_dial_only_while_rmb_is_held() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Right);
    step(&mut app);

    wheel(&mut app, 1.0);
    step(&mut app);
    let one = rig(&app, eye).fly_speed().get();
    assert!((one - FLY_SPEED.get() * DIAL_STEP).abs() < 1e-3);

    wheel(&mut app, 1.0);
    step(&mut app);
    let two = rig(&app, eye).fly_speed().get();
    assert!((two - FLY_SPEED.get() * DIAL_STEP * DIAL_STEP).abs() < 1e-3);

    wheel(&mut app, -1.0);
    wheel(&mut app, -1.0);
    step(&mut app);
    assert!((rig(&app, eye).fly_speed().get() - FLY_SPEED.get()).abs() < 1e-3);

    release_mouse(&mut app, MouseButton::Right);
    step(&mut app);
    wheel(&mut app, 1.0);
    step(&mut app);
    assert!(
        (rig(&app, eye).fly_speed().get() - FLY_SPEED.get()).abs() < 1e-3,
        "with RMB up the wheel dollies, it does not touch the dial"
    );
}

#[test]
fn the_dial_stops_at_each_end() {
    let mut rig = EditorRig::default();
    rig.turn_dial(1_000);
    let fastest = rig.fly_speed();
    rig.turn_dial(1);
    assert_eq!(rig.fly_speed(), fastest);

    rig.turn_dial(-10_000);
    let slowest = rig.fly_speed();
    rig.turn_dial(-1);
    assert_eq!(rig.fly_speed(), slowest);
    assert!(slowest.get() > 0.0);
}

#[test]
fn the_status_line_shows_the_fly_speed_while_flying() {
    let (mut app, _) = rig_app();
    press_mouse(&mut app, MouseButton::Right);
    step(&mut app);
    let line = app
        .world()
        .resource::<EditorStatus>()
        .line()
        .map(|(line, _)| line.to_string());
    assert_eq!(line.as_deref(), Some("FLY 60 m/s"));

    release_mouse(&mut app, MouseButton::Right);
    step(&mut app);
    assert!(app.world().resource::<EditorStatus>().line().is_none());
}

#[test]
fn orbit_keeps_the_distance_to_the_pivot() {
    let pivot = Vec3::new(3.0, -2.0, 7.0);
    let mut pose = Transform::from_xyz(20.0, 15.0, -4.0).looking_at(pivot, Vec3::Y);
    let reach = pose.translation.distance(pivot);

    for _ in 0..200 {
        pose = orbited(&pose, pivot, Vec2::new(0.07, 0.03));
        assert!((pose.translation.distance(pivot) - reach).abs() < 1e-2);
    }
}

#[test]
fn orbit_keeps_the_pivot_in_the_middle_of_the_view() {
    let pivot = Vec3::new(3.0, -2.0, 7.0);
    let start = Transform::from_xyz(20.0, 15.0, -4.0).looking_at(pivot, Vec3::Y);

    let swung = orbited(&start, pivot, Vec2::new(0.4, -0.2));

    let aim = (pivot - swung.translation).normalize();
    assert!(aim.dot(*swung.forward()) > 0.9999);
}

#[test]
fn looking_never_rolls_or_flips_the_camera() {
    let mut rotation = Quat::IDENTITY;
    for _ in 0..100 {
        rotation = turned(rotation, Vec2::new(0.1, -0.2));
    }
    let up = rotation * Vec3::Y;
    assert!(up.y > 0.0, "still the right way up");
    let right = rotation * Vec3::X;
    assert!(right.y.abs() < 1e-4, "and level");
}

#[test]
fn a_dolly_closes_on_the_pivot_and_never_passes_it() {
    let pivot = Vec3::new(10.0, 0.0, 0.0);
    let mut eye = Vec3::new(10.0, 0.0, 100.0);

    eye = dollied(eye, pivot, 1.0);
    assert!(eye.distance(pivot) < 100.0);
    assert!(eye.x == 10.0 && eye.z > 0.0);

    for _ in 0..500 {
        eye = dollied(eye, pivot, 1.0);
    }
    assert!(eye.distance(pivot) >= DOLLY_NEAREST.to_engine() - 1e-3);
    assert!(eye.z > 0.0, "the eye stays on its own side");
}

#[test]
fn the_wheel_with_no_gesture_dollies_and_a_held_part_keeps_it() {
    let (mut app, eye) = rig_app();
    let start = Transform::from_xyz(0.0, 0.0, 30.0);
    *app.world_mut().get_mut::<Transform>(eye).expect("a pose") = start;

    wheel(&mut app, 1.0);
    step(&mut app);
    assert_ne!(pose(&app, eye), start);

    let closer = pose(&app, eye);
    app.world_mut().resource_mut::<HeldPart>().0 = Some("hull".to_string().into());
    wheel(&mut app, 1.0);
    step(&mut app);
    assert_eq!(pose(&app, eye), closer, "the part in hand owns the wheel");
}

#[test]
fn no_gesture_starts_over_a_panel() {
    let (mut app, eye) = rig_app();
    app.world_mut().resource_mut::<ViewportPointer>().over_ui = true;
    let before = pose(&app, eye);

    press_mouse(&mut app, MouseButton::Right);
    press_key(&mut app, KeyCode::KeyW);
    mouse_travel(&mut app, Vec2::new(30.0, 0.0));
    wheel(&mut app, 1.0);
    step(&mut app);

    assert_eq!(rig(&app, eye).gesture, CameraGestureType::Idle);
    assert_eq!(pose(&app, eye), before);
}

#[test]
fn a_gesture_carries_on_when_the_pointer_crosses_a_panel() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Middle);
    step(&mut app);
    app.world_mut().resource_mut::<ViewportPointer>().over_ui = true;
    step(&mut app);

    assert_eq!(rig(&app, eye).gesture, CameraGestureType::Orbit);
}

#[test]
fn a_mode_above_normal_ends_the_gesture() {
    for mode in [InputMode::Browse, InputMode::Insert, InputMode::Bind] {
        let (mut app, eye) = rig_app();
        app.insert_resource(InputMode::Normal);
        press_mouse(&mut app, MouseButton::Right);
        step(&mut app);
        assert_eq!(rig(&app, eye).gesture, CameraGestureType::Fly);

        *app.world_mut().resource_mut::<InputMode>() = mode;
        press_key(&mut app, KeyCode::KeyW);
        let before = pose(&app, eye);
        step(&mut app);

        assert_eq!(rig(&app, eye).gesture, CameraGestureType::Idle, "{mode:?}");
        assert_eq!(pose(&app, eye), before, "{mode:?} has the keys");
    }
}

#[test]
fn each_button_starts_its_own_gesture() {
    let cases: [(&[KeyCode], MouseButton, CameraGestureType); 5] = [
        (&[], MouseButton::Right, CameraGestureType::Fly),
        (
            &[KeyCode::AltLeft],
            MouseButton::Right,
            CameraGestureType::Dolly,
        ),
        (&[], MouseButton::Middle, CameraGestureType::Orbit),
        (
            &[KeyCode::ShiftLeft],
            MouseButton::Middle,
            CameraGestureType::Pan,
        ),
        (
            &[KeyCode::AltLeft],
            MouseButton::Left,
            CameraGestureType::Orbit,
        ),
    ];
    for (keys, button, wanted) in cases {
        let (mut app, eye) = rig_app();
        for key in keys {
            press_key(&mut app, *key);
        }
        press_mouse(&mut app, button);
        step(&mut app);
        assert_eq!(rig(&app, eye).gesture, wanted, "{keys:?} {button:?}");
    }
}

#[test]
fn a_plain_left_click_moves_no_camera() {
    let (mut app, eye) = rig_app();
    press_mouse(&mut app, MouseButton::Left);
    step(&mut app);
    assert_eq!(rig(&app, eye).gesture, CameraGestureType::Idle);
}

#[test]
fn the_pivot_is_the_selection_then_a_point_ahead() {
    let (mut app, eye) = rig_app();
    let node = app
        .world_mut()
        .spawn(GlobalTransform::from_translation(Vec3::new(50.0, 0.0, 0.0)))
        .id();
    app.world_mut()
        .resource_mut::<SelectedNode>()
        .set(Some(node));
    let before = pose(&app, eye)
        .translation
        .distance(Vec3::new(50.0, 0.0, 0.0));

    press_mouse(&mut app, MouseButton::Middle);
    mouse_travel(&mut app, Vec2::new(40.0, 0.0));
    step(&mut app);
    let after = pose(&app, eye)
        .translation
        .distance(Vec3::new(50.0, 0.0, 0.0));
    assert!(
        (before - after).abs() < 1e-2,
        "orbit went round the selection"
    );

    release_mouse(&mut app, MouseButton::Middle);
    mouse_travel(&mut app, Vec2::ZERO);
    step(&mut app);
    app.world_mut().resource_mut::<SelectedNode>().set(None);
    let framing = app.world().resource::<CameraFraming>().distance;
    assert!(framing > Meters(0.0));

    let start = pose(&app, eye);
    let ahead = start.translation + start.forward() * framing.to_engine();
    press_mouse(&mut app, MouseButton::Middle);
    mouse_travel(&mut app, Vec2::new(40.0, 0.0));
    step(&mut app);
    let swung = pose(&app, eye).translation.distance(ahead);
    assert!((swung - framing.to_engine()).abs() < 1e-2);
}

#[test]
fn a_frame_request_pose_survives_the_next_frame() {
    let (mut app, eye) = rig_app();
    let framed = Transform::from_xyz(4.0, 5.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y);
    *app.world_mut().get_mut::<Transform>(eye).expect("a pose") = framed;

    step(&mut app);

    assert_eq!(pose(&app, eye), framed, "the pose is the Transform");
}

/// The CRT's pointer stands off screen for the whole session. Read as the mouse,
/// it put every box-select press at the same spot, so no drag ever wandered.
#[test]
fn only_the_mouse_says_where_the_pointer_is() {
    let mut world = World::new();
    world.init_resource::<ViewportPointer>();
    let screen = world.spawn(Window::default()).id();
    let target = NormalizedRenderTarget::Window(
        WindowRef::Entity(screen)
            .normalize(None)
            .expect("a named window normalizes"),
    );
    let at = |position| {
        PointerLocation::new(Location {
            target: target.clone(),
            position,
        })
    };
    let mouse = Vec2::new(300.0, 200.0);
    world.spawn((PointerId::Mouse, PointerInteraction::default(), at(mouse)));
    world.spawn((
        PointerId::Custom(bevy::asset::uuid::Uuid::nil()),
        PointerInteraction::default(),
        at(Vec2::splat(-1000.0)),
    ));

    world
        .run_system_once(sync_viewport_pointer)
        .expect("the pointer read runs");

    assert_eq!(world.resource::<ViewportPointer>().at, Some(mouse));
}
