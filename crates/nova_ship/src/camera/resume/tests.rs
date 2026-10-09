use std::time::Duration;

use avian3d::prelude::Rotation;
use bevy::time::TimeUpdateStrategy;
use nova_gameplay::{
    shake::{CameraShake, CameraShakeInput, CameraShakeOutput, CameraShakePlugin},
    transform::prelude::PointRotationPlugin,
};

use super::*;
use crate::{
    camera::{
        authority::{CameraAuthorityPlugin, CameraAuthoritySystems},
        chase::{ChaseCamera, ChaseCameraInput, ChaseCameraPlugin, ChaseCameraState},
        framing::{update_camera_rig, update_chase_camera_input, BurnPush},
        handback::HANDBACK_BLEND_SECONDS,
        mode::SpaceshipCameraControlMode,
        rig::{
            destroy_camera_controller, insert_camera_controller, insert_camera_freelook,
            insert_camera_turret, SpaceshipCameraController, SpaceshipCameraFreeLookInputMarker,
            SpaceshipCameraNormalInputMarker, SpaceshipCameraTurretInputMarker,
            SpaceshipRotationInputActiveMarker,
        },
        zoom::{ChaseZoom, CHASE_ZOOM_MAX},
    },
    prelude::{AutopilotAction, HullEnvelopeRadius},
};

fn camera_app(with_shake: bool) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        ChaseCameraPlugin,
        PointRotationPlugin,
    ));
    if with_shake {
        app.add_plugins(CameraShakePlugin);
    }
    app.add_plugins(CameraAuthorityPlugin);
    app.init_resource::<SpaceshipCameraControlMode>();
    app.init_resource::<ChaseZoom>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_observer(insert_camera_controller);
    app.add_observer(insert_camera_freelook);
    app.add_observer(insert_camera_turret);
    app.add_observer(destroy_camera_controller);
    app.add_systems(
        Update,
        (update_chase_camera_input, update_camera_rig).chain(),
    );
    app
}

fn only_entity_with<M: Component>(world: &mut World) -> Entity {
    let mut query = world.query_filtered::<Entity, With<M>>();
    let entities: Vec<Entity> = query.iter(world).collect();
    assert_eq!(
        entities.len(),
        1,
        "expected one entity with {}",
        std::any::type_name::<M>()
    );
    entities[0]
}

fn relative_pose(camera: &GlobalTransform, ship: &GlobalTransform) -> (Vec3, Quat) {
    let from_ship = ship.rotation().inverse();
    (
        from_ship * (camera.translation() - ship.translation()),
        from_ship * camera.rotation(),
    )
}

fn quaternion_distance(left: Quat, right: Quat) -> f32 {
    (left - right).length().min((left + right).length())
}

fn assert_pose_close(actual_position: Vec3, actual_rotation: Quat, expected: &CameraView) {
    assert!(
        (actual_position - expected.position_from_ship).length() < 1e-4,
        "position {:?} differs from saved {:?}",
        actual_position,
        expected.position_from_ship
    );
    assert!(
        quaternion_distance(actual_rotation, expected.rotation_from_ship) < 1e-4,
        "rotation {:?} differs from saved {:?}",
        actual_rotation,
        expected.rotation_from_ship
    );
}

fn assert_transform_close(actual: &Transform, expected: &Transform) {
    assert!(
        (actual.translation - expected.translation).length() < 1e-4,
        "translation {:?} differs from chase solve {:?}",
        actual.translation,
        expected.translation
    );
    assert!(
        quaternion_distance(actual.rotation, expected.rotation) < 1e-4,
        "rotation {:?} differs from chase solve {:?}",
        actual.rotation,
        expected.rotation
    );
}

fn chase_sync_pose(world: &World, camera: Entity) -> Transform {
    let chase = world.get::<ChaseCamera>(camera).expect("chase camera");
    let input = world
        .get::<ChaseCameraInput>(camera)
        .expect("chase camera input");
    let state = world
        .get::<ChaseCameraState>(camera)
        .expect("chase camera state");
    let mut pose = *world.get::<Transform>(camera).expect("camera transform");
    pose.translation = state.anchor_pos;
    let focus = input.anchor_pos
        + input.anchor_rot * Vec3::NEG_Z * chase.focus_offset.z
        + input.anchor_rot * Vec3::Y * chase.focus_offset.y
        + input.anchor_rot * Vec3::X * chase.focus_offset.x;
    pose.look_at(focus, input.anchor_rot * Vec3::Y);
    pose
}

/// A resumed camera shows its saved ship-relative pose first, then settles on
/// the live chase solve; a later respawn opens the Normal rig at identity.
#[test]
fn a_resumed_camera_renders_its_saved_pose_first_then_eases_onto_the_chase_pose() {
    fn move_and_turn_ship(
        time: Res<Time>,
        mut ship: Single<&mut Transform, With<PlayerSpaceshipMarker>>,
    ) {
        let dt = time.delta_secs();
        ship.translation += Vec3::new(2.0, -0.5, 1.0) * dt;
        ship.rotation = Quat::from_rotation_y(0.7 * dt) * ship.rotation;
    }

    let mut app = camera_app(false);
    app.add_systems(Update, move_and_turn_ship.before(update_chase_camera_input));
    let ship_rotation = Quat::from_euler(EulerRot::YXZ, 0.35, -0.2, 0.1);
    let ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            HullEnvelopeRadius(6.0),
            BurnPush::default(),
            Transform::from_translation(Vec3::new(42.0, -7.0, 19.0)).with_rotation(ship_rotation),
        ))
        .id();
    let view = CameraView {
        position_from_ship: Vec3::new(0.0, 5.0, 10.0),
        rotation_from_ship: Quat::from_euler(EulerRot::YXZ, -0.85, 0.3, -0.15),
        steer_from_ship: Quat::from_rotation_y(0.65),
        zoom: 3.0,
    };
    let steer = ship_rotation * view.steer_from_ship;
    app.insert_resource(ResumedCameraView { view, steer });
    let camera = app.world_mut().spawn(SpaceshipCameraController).id();

    // The first update has zero delta. The saved pose is still composed from
    // the ship's propagated transform and must be the first rendered frame.
    app.update();
    let camera_global = app
        .world()
        .get::<GlobalTransform>(camera)
        .expect("camera global");
    let ship_global = app
        .world()
        .get::<GlobalTransform>(ship)
        .expect("ship global");
    let (position_from_ship, rotation_from_ship) = relative_pose(camera_global, ship_global);
    assert_pose_close(position_from_ship, rotation_from_ship, &view);
    let normal = only_entity_with::<SpaceshipCameraNormalInputMarker>(app.world_mut());
    assert_eq!(
        app.world()
            .get::<PointRotation>(normal)
            .unwrap()
            .initial_rotation,
        steer,
        "the Normal rig opens at the saved steer"
    );
    assert_eq!(app.world().resource::<ChaseZoom>().manual(), 3.0);
    assert!(!app.world().contains_resource::<ResumedCameraView>());

    let initial_ship = *app.world().get::<Transform>(ship).unwrap();
    for _ in 0..40 {
        app.update();
    }
    assert!(
        app.world().resource::<Time>().elapsed_secs() >= HANDBACK_BLEND_SECONDS,
        "the fixed-step run covers the full resume blend"
    );
    assert!(
        app.world().get::<CameraResumeBlend>(camera).is_none(),
        "the finished resume blend removes itself"
    );
    let moved_ship = app.world().get::<Transform>(ship).unwrap();
    assert!((moved_ship.translation - initial_ship.translation).length() > 0.5);
    assert!(moved_ship.rotation.angle_between(initial_ship.rotation) > 0.1);
    let expected_chase = chase_sync_pose(app.world(), camera);
    let actual = app.world().get::<Transform>(camera).unwrap();
    assert_transform_close(actual, &expected_chase);

    // A death respawn consumes no second saved view and opens the default rig.
    app.world_mut()
        .entity_mut(camera)
        .remove::<SpaceshipCameraController>();
    app.update();
    app.world_mut()
        .entity_mut(camera)
        .insert(SpaceshipCameraController);
    app.update();
    assert!(app.world().get::<CameraResumeBlend>(camera).is_none());
    let normal = only_entity_with::<SpaceshipCameraNormalInputMarker>(app.world_mut());
    assert_eq!(
        app.world()
            .get::<PointRotation>(normal)
            .unwrap()
            .initial_rotation,
        Quat::IDENTITY,
        "a respawn opens the Normal rig at identity"
    );
}

/// A captured view follows the base chase solve, not shake or a scripted
/// override, and keeps the ship-relative pose and steering contract.
#[test]
fn a_captured_view_is_the_rendered_pose_without_shake_relative_to_a_rotated_ship() {
    fn pin_scripted_pose(mut camera: Single<&mut Transform, With<SpaceshipCameraController>>) {
        camera.translation = Vec3::new(130.0, -48.0, 76.0);
        camera.rotation = Quat::from_euler(EulerRot::YXZ, 1.3, -0.7, 0.4);
    }

    let mut app = camera_app(true);
    let ship_rotation = Quat::from_euler(EulerRot::YXZ, -0.55, 0.25, 0.18);
    let ship = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            BurnPush::default(),
            Transform::from_translation(Vec3::new(-24.0, 13.0, 37.0)).with_rotation(ship_rotation),
            Rotation(ship_rotation),
        ))
        .id();
    let camera = app.world_mut().spawn(SpaceshipCameraController).id();
    for _ in 0..80 {
        app.update();
    }
    let normal = only_entity_with::<SpaceshipCameraNormalInputMarker>(app.world_mut());
    let free = only_entity_with::<SpaceshipCameraFreeLookInputMarker>(app.world_mut());
    let turret = only_entity_with::<SpaceshipCameraTurretInputMarker>(app.world_mut());
    let normal_steer = Quat::from_euler(EulerRot::YXZ, 0.42, -0.17, 0.08);
    app.world_mut()
        .get_mut::<PointRotation>(normal)
        .unwrap()
        .initial_rotation = normal_steer;
    for _ in 0..3 {
        app.update();
    }

    // Normal play: the base solve and the propagated rendered frame agree.
    let normal_view = CameraView::capture(app.world_mut()).expect("normal camera is ready");
    let normal_global = app.world().get::<GlobalTransform>(camera).unwrap();
    let ship_global = app.world().get::<GlobalTransform>(ship).unwrap();
    let (normal_position, normal_rotation) = relative_pose(normal_global, ship_global);
    assert_pose_close(normal_position, normal_rotation, &normal_view);
    let normal_output = **app.world().get::<PointRotationOutput>(normal).unwrap();
    assert!(
        quaternion_distance(
            normal_view.steer_from_ship,
            ship_rotation.inverse() * normal_output,
        ) < 1e-4,
        "steer is the Normal rig output relative to the physics rotation"
    );

    // FreeLook changes the solved pose but leaves the saved steer on Normal.
    let free_look = Quat::from_euler(EulerRot::YXZ, 1.15, -0.4, 0.2);
    app.world_mut()
        .resource_mut::<SpaceshipCameraControlMode>()
        .clone_from(&SpaceshipCameraControlMode::FreeLook);
    app.world_mut()
        .entity_mut(normal)
        .remove::<SpaceshipRotationInputActiveMarker>();
    app.world_mut()
        .entity_mut(free)
        .insert(SpaceshipRotationInputActiveMarker);
    app.world_mut()
        .get_mut::<PointRotation>(free)
        .unwrap()
        .initial_rotation = free_look;
    for _ in 0..40 {
        app.update();
    }
    let free_view = CameraView::capture(app.world_mut()).expect("free-look camera is ready");
    let free_global = app.world().get::<GlobalTransform>(camera).unwrap();
    let ship_global = app.world().get::<GlobalTransform>(ship).unwrap();
    let (free_position, free_rotation) = relative_pose(free_global, ship_global);
    assert_pose_close(free_position, free_rotation, &free_view);
    assert!(
        (free_view.position_from_ship - normal_view.position_from_ship).length() > 1.0
            || free_view
                .rotation_from_ship
                .angle_between(normal_view.rotation_from_ship)
                > 0.1,
        "FreeLook must produce a pose distinct from Normal"
    );

    // Turret uses its distinct combat distance; capture follows that solved pose.
    app.world_mut()
        .resource_mut::<SpaceshipCameraControlMode>()
        .clone_from(&SpaceshipCameraControlMode::Turret);
    app.world_mut()
        .entity_mut(free)
        .remove::<SpaceshipRotationInputActiveMarker>();
    app.world_mut()
        .entity_mut(turret)
        .insert(SpaceshipRotationInputActiveMarker);
    app.world_mut()
        .get_mut::<PointRotation>(turret)
        .unwrap()
        .initial_rotation = normal_steer;
    for _ in 0..40 {
        app.update();
    }
    let turret_view = CameraView::capture(app.world_mut()).expect("turret camera is ready");
    let turret_global = app.world().get::<GlobalTransform>(camera).unwrap();
    let ship_global = app.world().get::<GlobalTransform>(ship).unwrap();
    let (turret_position, turret_rotation) = relative_pose(turret_global, ship_global);
    assert_pose_close(turret_position, turret_rotation, &turret_view);
    assert!(
        (turret_view.position_from_ship - normal_view.position_from_ship).length() > 1.0,
        "Turret must use its distinct combat distance"
    );

    // Return to Normal and shake the actual rendered Transform. The private
    // solved pose remains the unshaken capture source.
    app.world_mut()
        .resource_mut::<SpaceshipCameraControlMode>()
        .clone_from(&SpaceshipCameraControlMode::Normal);
    app.world_mut()
        .entity_mut(turret)
        .remove::<SpaceshipRotationInputActiveMarker>();
    app.world_mut()
        .entity_mut(normal)
        .insert(SpaceshipRotationInputActiveMarker);
    app.world_mut().entity_mut(camera).insert(CameraShake {
        decay: 0.0,
        max_offset: Vec3::splat(3.0),
        max_kick: Vec3::splat(0.5),
        ..default()
    });
    app.world_mut()
        .get_mut::<CameraShakeInput>(camera)
        .unwrap()
        .add_trauma = 1.0;
    let mut shaken = false;
    for _ in 0..12 {
        app.update();
        let output = app.world().get::<CameraShakeOutput>(camera).unwrap();
        let live =
            output.offset.length() > 1e-4 || output.kick.angle_between(Quat::IDENTITY) > 1e-4;
        let solved = app
            .world()
            .get::<ChaseCameraState>(camera)
            .unwrap()
            .solved
            .unwrap();
        let rendered = app.world().get::<GlobalTransform>(camera).unwrap();
        if live
            && ((rendered.translation() - solved.translation).length() > 1e-4
                || quaternion_distance(rendered.rotation(), solved.rotation) > 1e-4)
        {
            shaken = true;
            break;
        }
    }
    assert!(
        shaken,
        "live trauma must move the rendered camera off the solve"
    );
    let solved = app
        .world()
        .get::<ChaseCameraState>(camera)
        .unwrap()
        .solved
        .unwrap();
    let shake_view = CameraView::capture(app.world_mut()).expect("shaken camera is ready");
    let ship_global = app.world().get::<GlobalTransform>(ship).unwrap();
    let solved_global = GlobalTransform::from(solved);
    let (solved_position, solved_rotation) = relative_pose(&solved_global, ship_global);
    assert_pose_close(solved_position, solved_rotation, &shake_view);

    // A later scripted writer changes what renders, not the base capture.
    app.add_systems(
        PostUpdate,
        pin_scripted_pose.in_set(CameraAuthoritySystems::Override),
    );
    app.update();
    let scripted_solved = app
        .world()
        .get::<ChaseCameraState>(camera)
        .unwrap()
        .solved
        .unwrap();
    let scripted_global = app.world().get::<GlobalTransform>(camera).unwrap();
    assert!(
        (scripted_global.translation() - scripted_solved.translation).length() > 1.0,
        "the scripted Override must replace the rendered pose"
    );
    let scripted_view = CameraView::capture(app.world_mut()).expect("overridden camera is ready");
    let ship_global = app.world().get::<GlobalTransform>(ship).unwrap();
    let solved_global = GlobalTransform::from(scripted_solved);
    let (solved_position, solved_rotation) = relative_pose(&solved_global, ship_global);
    assert_pose_close(solved_position, solved_rotation, &scripted_view);

    // Autopilot is a Load disengage: STEER is identity while it owns the hull.
    app.world_mut()
        .entity_mut(ship)
        .insert(Autopilot::engage(AutopilotAction::Stop));
    app.update();
    let autopilot_view = CameraView::capture(app.world_mut()).expect("autopilot camera is ready");
    assert_eq!(autopilot_view.steer_from_ship, Quat::IDENTITY);

    // Each missing prerequisite is explicitly not ready.
    app.world_mut()
        .get_mut::<ChaseCameraState>(camera)
        .unwrap()
        .solved = None;
    assert!(
        CameraView::capture(app.world_mut()).is_none(),
        "no solve means no capture"
    );
    app.update();
    app.world_mut()
        .entity_mut(normal)
        .remove::<SpaceshipCameraNormalInputMarker>();
    app.update();
    assert!(
        CameraView::capture(app.world_mut()).is_none(),
        "no Normal rig means no capture"
    );
    app.world_mut()
        .entity_mut(camera)
        .remove::<SpaceshipCameraController>();
    app.update();
    assert!(
        CameraView::capture(app.world_mut()).is_none(),
        "no controller means no capture"
    );
}

/// Saved views refuse invalid positions, rotations, steering and zoom levels,
/// while accepting both zoom endpoints.
#[test]
fn a_saved_view_refuses_a_non_finite_or_non_unit_view_or_an_out_of_range_zoom() {
    let valid = CameraView {
        position_from_ship: Vec3::new(1.0, 2.0, 3.0),
        rotation_from_ship: Quat::IDENTITY,
        steer_from_ship: Quat::IDENTITY,
        zoom: 1.0,
    };

    for position in [Vec3::splat(f32::NAN), Vec3::new(f32::INFINITY, 0.0, 0.0)] {
        let invalid = CameraView {
            position_from_ship: position,
            ..valid
        };
        assert!(matches!(
            invalid.validate(),
            Err(CameraViewFault::NonFinitePosition(_))
        ));
    }

    for rotation in [
        Quat::from_xyzw(0.0, 0.0, 0.0, 2.0),
        Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
    ] {
        let invalid_view = CameraView {
            rotation_from_ship: rotation,
            ..valid
        };
        assert!(matches!(
            invalid_view.validate(),
            Err(CameraViewFault::NonUnitRotation(_))
        ));
        let invalid_steer = CameraView {
            steer_from_ship: rotation,
            ..valid
        };
        assert!(matches!(
            invalid_steer.validate(),
            Err(CameraViewFault::NonUnitRotation(_))
        ));
    }

    for zoom in [0.5, f32::NAN, CHASE_ZOOM_MAX + 0.1] {
        let invalid = CameraView { zoom, ..valid };
        assert!(matches!(
            invalid.validate(),
            Err(CameraViewFault::ZoomOutOfRange(_))
        ));
    }

    assert!(CameraView { zoom: 1.0, ..valid }.validate().is_ok());
    assert!(CameraView {
        zoom: CHASE_ZOOM_MAX,
        ..valid
    }
    .validate()
    .is_ok());
}
