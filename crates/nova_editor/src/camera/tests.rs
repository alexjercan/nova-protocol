use bevy::{
    camera::NormalizedRenderTarget,
    ecs::system::RunSystemOnce,
    input::{mouse::MouseScrollUnit, touch::TouchPhase},
    picking::pointer::Location,
    window::WindowRef,
};

use super::*;

fn rig_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<AccumulatedMouseMotion>()
        .init_resource::<ViewportPointer>()
        .init_resource::<GalleryState>()
        .init_resource::<SelectedNode>()
        .init_resource::<HeldPart>()
        .init_resource::<CameraFraming>()
        .add_message::<MouseWheel>()
        .add_systems(Update, drive_editor_camera);
    let eye = app
        .world_mut()
        .spawn((EditorCamera, EditorRig::default(), Transform::default()))
        .id();
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
fn rmb_turns_the_view_and_the_letter_keys_never_move_the_eye() {
    let (mut app, eye) = rig_app();
    let start = Transform::from_xyz(4.0, 2.0, 30.0);
    *app.world_mut().get_mut::<Transform>(eye).expect("a pose") = start;

    press_mouse(&mut app, MouseButton::Right);
    for key in [
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyQ,
        KeyCode::KeyE,
        KeyCode::ShiftLeft,
    ] {
        press_key(&mut app, key);
    }
    mouse_travel(&mut app, Vec2::new(30.0, 10.0));
    wheel(&mut app, 1.0);
    for _ in 0..5 {
        app.update();
    }

    assert_eq!(rig(&app, eye).gesture, CameraGestureType::Look);
    let looked = pose(&app, eye);
    assert_eq!(
        looked.translation, start.translation,
        "nothing flies the eye"
    );
    assert_ne!(looked.rotation, start.rotation, "the view turned");
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
        assert_eq!(rig(&app, eye).gesture, CameraGestureType::Look);

        *app.world_mut().resource_mut::<InputMode>() = mode;
        mouse_travel(&mut app, Vec2::new(30.0, 0.0));
        let before = pose(&app, eye);
        step(&mut app);

        assert_eq!(rig(&app, eye).gesture, CameraGestureType::Idle, "{mode:?}");
        assert_eq!(pose(&app, eye), before, "{mode:?} has the mouse");
    }
}

#[test]
fn each_button_starts_its_own_gesture() {
    let cases: [(&[KeyCode], MouseButton, CameraGestureType); 5] = [
        (&[], MouseButton::Right, CameraGestureType::Look),
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
