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
