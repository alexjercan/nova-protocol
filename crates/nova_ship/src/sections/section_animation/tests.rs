use std::time::Duration;

use super::*;

/// A minimal app with the animation plugin, one animated section, and a
/// hand-built "scene": named child nodes standing in for spawned glTF
/// nodes, with the rig marked dirty the way the ready observer would.
fn door_app(track: SectionAnimation) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn((
            SectionAnimations::new(vec![track]),
            SectionAnimationRigDirty,
        ))
        .id();
    // The petal's authored placement: a rest rotation the motion must
    // compose with, not overwrite.
    let petal = app
        .world_mut()
        .spawn((
            Name::new("door_petal_0"),
            Transform::from_translation(Vec3::new(0.0, 0.3, -0.99))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_3)),
            ChildOf(section),
        ))
        .id();
    (app, section, petal)
}

fn muzzle_door() -> SectionAnimation {
    SectionAnimation {
        cue: SectionAnimationCue::MuzzleDoor,
        node_prefix: "door_petal_".to_string(),
        motion: SectionAnimationMotion::RotateX { degrees: 100.0 },
        open_seconds: 0.5,
        close_seconds: 1.0,
    }
}

fn step(app: &mut App, dt_ms: u64) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(dt_ms));
    app.update();
}

fn progress(app: &mut App, section: Entity) -> f32 {
    app.world_mut()
        .get::<SectionAnimations>(section)
        .unwrap()
        .cue_progress(SectionAnimationCue::MuzzleDoor)
        .unwrap()
}

#[test]
fn a_track_rests_closed_until_its_cue_is_steered() {
    let (mut app, section, petal) = door_app(muzzle_door());
    let rest = *app.world_mut().get::<Transform>(petal).unwrap();
    step(&mut app, 250);
    step(&mut app, 250);
    assert_eq!(progress(&mut app, section), 0.0);
    assert_eq!(*app.world_mut().get::<Transform>(petal).unwrap(), rest);
}

#[test]
fn progress_travels_at_the_authored_open_and_close_speeds() {
    let (mut app, section, _) = door_app(muzzle_door());
    // Warm-up: resolve the rig on the first (dt 0) update.
    step(&mut app, 0);

    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    step(&mut app, 250);
    assert!((progress(&mut app, section) - 0.5).abs() < 1e-4);
    step(&mut app, 500);
    assert_eq!(progress(&mut app, section), 1.0, "clamped at the target");

    // Closing runs the slower authored speed: 1.0 s for the full travel.
    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 0.0);
    step(&mut app, 500);
    assert!((progress(&mut app, section) - 0.5).abs() < 1e-4);
    step(&mut app, 600);
    assert_eq!(progress(&mut app, section), 0.0);
}

#[test]
fn the_motion_composes_the_hinge_swing_onto_the_authored_rest_pose() {
    let (mut app, section, petal) = door_app(muzzle_door());
    let rest = *app.world_mut().get::<Transform>(petal).unwrap();
    step(&mut app, 0);

    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    step(&mut app, 250);

    let moved = *app.world_mut().get::<Transform>(petal).unwrap();
    let expected = rest.rotation * Quat::from_rotation_x(100_f32.to_radians() * 0.5);
    // abs_diff_eq, not angle_between: identical quats can dot to just
    // above 1.0 in f32, and acos of that is NaN.
    assert!(moved.rotation.abs_diff_eq(expected, 1e-5));
    assert_eq!(
        moved.translation, rest.translation,
        "a hinge swings; it does not slide"
    );
}

#[test]
fn rig_resolution_matches_nodes_by_prefix_only() {
    let (mut app, section, _) = door_app(muzzle_door());
    // A named node the prefix must NOT catch: the tube's static collar.
    let collar = app
        .world_mut()
        .spawn((
            Name::new("muzzle_collar"),
            Transform::default(),
            ChildOf(section),
        ))
        .id();
    step(&mut app, 0);

    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    step(&mut app, 500);

    assert_eq!(
        *app.world_mut().get::<Transform>(collar).unwrap(),
        Transform::default(),
        "only prefix-matched nodes move"
    );
}

#[test]
fn a_late_scene_lands_on_the_current_pose_when_its_rig_resolves() {
    // The scene readies AFTER the cue opened: the resolve must not leave
    // the new nodes at rest while the track says open.
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn(SectionAnimations::new(vec![muzzle_door()]))
        .id();
    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    step(&mut app, 0);
    step(&mut app, 500);
    assert_eq!(progress(&mut app, section), 1.0);

    let petal = app
        .world_mut()
        .spawn((
            Name::new("door_petal_0"),
            Transform::default(),
            ChildOf(section),
        ))
        .id();
    app.world_mut()
        .entity_mut(section)
        .insert(SectionAnimationRigDirty);
    step(&mut app, 0);

    let expected = Quat::from_rotation_x(100_f32.to_radians());
    let moved = *app.world_mut().get::<Transform>(petal).unwrap();
    assert!(moved.rotation.abs_diff_eq(expected, 1e-5));
}

#[test]
fn the_translate_motion_slides_along_the_rest_frame_without_turning() {
    // Two mirror-placed lids on ONE track: the rest rotation aims each
    // node's slide, so the same authored offset parts them in opposite
    // world directions - the slide sibling of the six-petal hinge trick.
    let track = SectionAnimation {
        cue: SectionAnimationCue::StowDoors,
        node_prefix: "stow_lid_".to_string(),
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::new(-0.24, 0.0, 0.0),
        },
        open_seconds: 0.5,
        close_seconds: 0.5,
    };
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn((
            SectionAnimations::new(vec![track]),
            SectionAnimationRigDirty,
        ))
        .id();
    let right = app
        .world_mut()
        .spawn((
            Name::new("stow_lid_right"),
            Transform::from_translation(Vec3::new(0.37, 0.18, 0.0)),
            ChildOf(section),
        ))
        .id();
    let left = app
        .world_mut()
        .spawn((
            Name::new("stow_lid_left"),
            Transform::from_translation(Vec3::new(-0.37, 0.18, 0.0))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            ChildOf(section),
        ))
        .id();
    step(&mut app, 0);

    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::StowDoors, 1.0);
    step(&mut app, 600);

    let right_moved = *app.world_mut().get::<Transform>(right).unwrap();
    let left_moved = *app.world_mut().get::<Transform>(left).unwrap();
    assert!(right_moved
        .translation
        .abs_diff_eq(Vec3::new(0.13, 0.18, 0.0), 1e-5));
    assert!(
        left_moved
            .translation
            .abs_diff_eq(Vec3::new(-0.13, 0.18, 0.0), 1e-5),
        "the mirrored lid slides the other way: {:?}",
        left_moved.translation
    );
    assert!(
        left_moved
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(std::f32::consts::PI), 1e-5),
        "a slide never turns its node"
    );
}

#[test]
fn snapping_a_cue_lands_the_pose_without_travel() {
    let (mut app, section, petal) = door_app(muzzle_door());
    step(&mut app, 0);

    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .snap_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    // One zero-dt frame: no travel time has passed, yet the pose lands.
    step(&mut app, 0);

    assert_eq!(progress(&mut app, section), 1.0);
    let moved = *app.world_mut().get::<Transform>(petal).unwrap();
    let expected = Quat::from_rotation_z(std::f32::consts::FRAC_PI_3)
        * Quat::from_rotation_x(100_f32.to_radians());
    assert!(moved.rotation.abs_diff_eq(expected, 1e-5));
}

#[test]
fn a_re_resolve_keeps_the_first_captured_rest_of_a_driven_node() {
    // A turret's scenes ready one by one: the second ready re-walks the
    // whole tree AFTER the driver has posed the nodes the first ready
    // captured. Re-capturing a driven pose as "rest" composes the motion
    // onto itself - a snapped-stowed lift would sink twice.
    let track = SectionAnimation {
        cue: SectionAnimationCue::StowLift,
        node_prefix: "stow_lift".to_string(),
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::new(0.0, -0.8, 0.0),
        },
        open_seconds: 0.0,
        close_seconds: 0.0,
    };
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn((
            SectionAnimations::new(vec![track]),
            SectionAnimationRigDirty,
        ))
        .id();
    let lift = app
        .world_mut()
        .spawn((
            Name::new("stow_lift"),
            Transform::from_translation(Vec3::ZERO),
            ChildOf(section),
        ))
        .id();
    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .snap_cue(SectionAnimationCue::StowLift, 1.0);
    step(&mut app, 0);
    let sunk = app.world_mut().get::<Transform>(lift).unwrap().translation;
    assert!(sunk.abs_diff_eq(Vec3::new(0.0, -0.8, 0.0), 1e-5));

    // A second scene readies: the rig re-resolves with the lift mid-drive.
    app.world_mut()
        .entity_mut(section)
        .insert(SectionAnimationRigDirty);
    step(&mut app, 0);
    step(&mut app, 100);

    let still = app.world_mut().get::<Transform>(lift).unwrap().translation;
    assert!(
        still.abs_diff_eq(sunk, 1e-5),
        "the re-resolve must keep the first rest, not double the sink: {still:?}"
    );
}

#[test]
fn a_zero_duration_track_snaps_between_poses() {
    let track = SectionAnimation {
        open_seconds: 0.0,
        close_seconds: 0.0,
        ..muzzle_door()
    };
    let (mut app, section, _) = door_app(track);
    step(&mut app, 0);
    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::MuzzleDoor, 1.0);
    step(&mut app, 1);
    assert_eq!(progress(&mut app, section), 1.0);
}
