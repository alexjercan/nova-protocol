use std::time::Duration;

use bevy::world_serialization::WorldSerializationPlugin;

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
fn a_spawned_scene_node_first_renders_at_its_rig_pose() {
    // A stowed emitter's scene spawns in `SpawnScene`, after the `Update`
    // driver: the first frame that renders its lid must already show the
    // snapped stow pose, not the deployed rest the art authors.
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins((
        AssetPlugin::default(),
        WorldSerializationPlugin,
        SectionAnimationPlugin,
    ))
    .register_type::<Name>()
    .register_type::<Transform>()
    .register_type::<ChildOf>()
    .register_type::<Children>();
    let track = SectionAnimation {
        cue: SectionAnimationCue::StowDoors,
        node_prefix: "stow_lid_".to_string(),
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::new(-0.24, 0.0, 0.0),
        },
        open_seconds: 0.3,
        close_seconds: 0.3,
    };
    let mut scene = World::new();
    scene.spawn((
        Name::new("stow_lid_right"),
        Transform::from_translation(Vec3::new(0.37, 0.18, 0.0)),
    ));
    let scene = app
        .world_mut()
        .resource_mut::<Assets<WorldAsset>>()
        .add(WorldAsset::new(scene));
    let mut animations = SectionAnimations::new(vec![track]);
    animations.snap_cue(SectionAnimationCue::StowDoors, 1.0);
    let section = app.world_mut().spawn(animations).id();
    app.world_mut()
        .spawn((WorldAssetRoot(scene), ChildOf(section)));

    let mut q_lid = app.world_mut().query::<(&Name, &Transform)>();
    let first = (0..4)
        .find_map(|_| {
            app.update();
            q_lid
                .iter(app.world())
                .find(|(name, _)| name.as_str() == "stow_lid_right")
                .map(|(_, transform)| *transform)
        })
        .expect("the lid scene spawns");
    assert!(
        first
            .translation
            .abs_diff_eq(Vec3::new(0.13, 0.18, 0.0), 1e-5),
        "the lid first renders shut, not at its authored rest: {:?}",
        first.translation
    );
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

fn intake_door() -> SectionAnimation {
    SectionAnimation {
        cue: SectionAnimationCue::IntakeDoor,
        node_prefix: "intake_slat_".to_string(),
        motion: SectionAnimationMotion::Fold {
            degrees: 80.0,
            slat_width: 0.1458,
            slat_thickness: 0.02,
        },
        open_seconds: 1.0,
        close_seconds: 1.0,
    }
}

#[test]
fn the_fold_motion_turns_and_slides_each_slat_into_its_pocket() {
    let (width, thickness) = (0.1458_f32, 0.02_f32);
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn((
            SectionAnimations::new(vec![intake_door()]),
            SectionAnimationRigDirty,
        ))
        .id();
    // Two slats per side, at the recipe's rest placement: the slat turned
    // so its width runs along the parent's X, closing the aperture edge to
    // edge from each pocket wall at x = -+0.8.
    let rest_rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let slats: Vec<(Entity, f32, u32)> = [("l", 1.0_f32), ("r", -1.0)]
        .into_iter()
        .flat_map(|(side, toward)| (0..2).map(move |index| (side, toward, index)))
        .map(|(side, toward, index)| {
            let rest_x = -toward * 0.8 + toward * (index as f32 + 0.5) * width;
            let slat = app
                .world_mut()
                .spawn((
                    Name::new(format!("intake_slat_{side}{index}")),
                    Transform::from_translation(Vec3::new(rest_x, 0.1, -0.3221))
                        .with_rotation(rest_rotation),
                    ChildOf(section),
                ))
                .id();
            (slat, toward, index)
        })
        .collect();
    step(&mut app, 0);
    app.world_mut()
        .get_mut::<SectionAnimations>(section)
        .unwrap()
        .set_cue(SectionAnimationCue::IntakeDoor, 1.0);

    for (dt_ms, progress) in [(0, 0.0_f32), (500, 0.5), (500, 1.0)] {
        step(&mut app, dt_ms);
        let fold = 80.0_f32.to_radians() * progress;
        for &(slat, toward, index) in &slats {
            // The example's pose (loop_intake_compare `Slat::pose`): each slat
            // hangs off its pocket wall by its place in the accordion.
            let pocket = -toward * 0.8;
            let x = pocket
                + toward
                    * ((index as f32 + 0.5) * width * fold.cos() + thickness * 0.5 * fold.sin());
            let turn = if index % 2 == 0 { fold } else { -fold };
            let pose = *app.world_mut().get::<Transform>(slat).unwrap();
            assert!(
                pose.translation
                    .abs_diff_eq(Vec3::new(x, 0.1, -0.3221), 1e-5),
                "slat {toward} {index} at {progress}: {:?}",
                pose.translation
            );
            assert!(
                pose.rotation
                    .abs_diff_eq(rest_rotation * Quat::from_rotation_x(turn), 1e-5),
                "slat {toward} {index} at {progress}: {:?}",
                pose.rotation
            );
        }
    }
}

#[test]
#[should_panic(expected = "Fold node \"intake_slat_x0\" is not a `intake_slat_<l|r><index>` slat")]
fn a_fold_node_name_without_side_and_index_panics_the_rig() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_plugins(SectionAnimationPlugin);
    let section = app
        .world_mut()
        .spawn((
            Name::new("cargo intake"),
            SectionAnimations::new(vec![intake_door()]),
            SectionAnimationRigDirty,
        ))
        .id();
    app.world_mut().spawn((
        Name::new("intake_slat_x0"),
        Transform::IDENTITY,
        ChildOf(section),
    ));
    step(&mut app, 0);
}
