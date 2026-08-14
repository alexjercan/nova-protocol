//! Building a turret: spawning the joint tree from its config and pushing
//! live config edits back onto the spawned children.

use std::time::Duration;

use bevy::prelude::*;

use super::*;

/// Recursively spawn one entity per [`TurretJoint`] node, returning the spawned
/// entity. Articulated joints (axis Some) carry a [`SmoothLookRotation`]; a
/// joint with a `muzzle` also gets the muzzle bundle and its entity is recorded
/// in `muzzles`. Children are added as child entities in DFS order.
///
/// Collapsing the old base+rotator PAIR into ONE entity is safe because
/// [`SmoothLookRotation`] does not read `Transform` (verified): a single entity
/// carries both the offset `Transform` (with the axis's zero-angle rotation) and
/// the controller, and the sync system writes the hinge rotation back onto the
/// SAME transform. The composed math `parent * T(offset) * R(theta) * ...` is
/// identical to the old two-entity chain.
fn spawn_turret_joint(
    commands: &mut Commands,
    turret: Entity,
    joint: &TurretJoint,
    muzzles: &mut Vec<Entity>,
) -> Entity {
    // A hinge needs a non-zero, finite axis: `sync_turret_joint_rotation` and the
    // aim solver normalize it, so a zero/NaN axis would spread NaN through the
    // transform. The content lint (`lint_section_config`) rejects this at author
    // time; here is the runtime backstop for a turret built in code or a mod that
    // bypasses the lint - a degenerate axis degrades to a FIXED joint (marker
    // axis `None`, no controller) instead of NaN.
    let hinge_axis = joint
        .axis
        .filter(|a| a.is_finite() && a.length_squared() > 1e-12);
    if joint.axis.is_some() && hinge_axis.is_none() {
        warn!(
            "spawn_turret_joint: turret {:?} has a degenerate hinge axis {:?}; \
             treating the joint as fixed",
            turret, joint.axis
        );
    }

    let mut entity = commands.spawn((
        Name::new("Turret Joint"),
        TurretSectionPartOf(turret),
        TurretJointRenderMesh(joint.render_mesh.clone()),
        TurretJointRenderMeshTransform(joint.render_mesh_transform),
        Transform::from_translation(joint.offset),
        Visibility::Inherited,
    ));

    if let Some(axis) = hinge_axis {
        entity.insert((
            SmoothLookRotation {
                axis,
                initial: 0.0,
                speed: joint.speed,
                min: joint.min,
                max: joint.max,
            },
            TurretJointAimHeading::default(),
        ));
    }

    if let Some(muzzle) = &joint.muzzle {
        // `fire_rate` is an unvalidated authored f32: 0.0 gives +inf, and
        // `Duration::from_secs_f32(inf)` PANICS the moment the ship spawns.
        // Same clamp the retune path already carried.
        let interval = 1.0 / muzzle.fire_rate.max(f32::EPSILON);
        let mut timer = Timer::from_seconds(interval, TimerMode::Once);
        timer.finish(); // Ready to fire immediately
        entity.insert((
            TurretSectionBarrelMuzzleMarker,
            TurretSectionBarrelFireState(timer),
            TurretSectionBarrelMuzzleEffect(muzzle.muzzle_effect.clone()),
        ));
    }

    // Add the marker LAST: the render observer keys on `Add, TurretJointMarker`
    // and reads the muzzle marker to skip fire points, so the muzzle bundle must
    // already be on the entity when the marker lands.
    entity.insert(TurretJointMarker { axis: hinge_axis });

    let id = entity.id();

    if joint.muzzle.is_some() {
        muzzles.push(id);
    }

    for child in &joint.children {
        let child_id = spawn_turret_joint(commands, turret, child, muzzles);
        commands.entity(id).add_child(child_id);
    }

    id
}

pub(super) fn insert_turret_section(
    add: On<Add, TurretSectionMarker>,
    mut commands: Commands,
    q_config: Query<&TurretSectionConfigHelper, With<TurretSectionMarker>>,
) {
    let turret = add.entity;
    trace!("insert_turret_section: entity {:?}", turret);

    let Ok(config) = q_config.get(turret) else {
        error!(
            "insert_turret_section: entity {:?} not found in q_config",
            turret
        );
        return;
    };
    let config = (**config).clone();

    let mut muzzles = Vec::new();
    let root = spawn_turret_joint(&mut commands, turret, &config.root, &mut muzzles);

    // The section-wide fire/aim path drives ALL muzzles; the FIRST is the
    // PRIMARY, kept in `TurretSectionMuzzleEntity` for the
    // single-point consumers (lead HUD pip, the aim-point lead solve, AI gate).
    // A well-formed tree has at least one muzzle joint.
    let Some((&muzzle, _)) = muzzles.split_first() else {
        error!(
            "insert_turret_section: turret {:?} tree has no muzzle joint",
            turret
        );
        return;
    };

    // Snapshot the authorable fire sound (unresolved) onto the turret, like the
    // render-mesh refs; the audio module resolves + plays it (see
    // [`TurretSectionFireSound`]). `None` leaves the observer on the bank cue.
    commands
        .entity(turret)
        .insert((
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(muzzles.clone()),
            TurretSectionFireSound(config.fire_sound.clone()),
            TurretSectionDryFireSound(config.dry_fire_sound.clone()),
        ))
        .add_child(root);

    // Opt-in finite ammo: a magazine on the turret SECTION entity (the one
    // `shoot_spawn_projectile` queries), so the fire loop spends and gates on it
    // with the query it already runs. `None` leaves the turret unlimited.
    if let Some(capacity) = config.ammo_capacity {
        commands.entity(turret).insert(SectionAmmo::new(capacity));
        // Auto-reload rides on the magazine: only a finite turret can reload, so
        // an unlimited one (config.reload set but ammo_capacity None) gets none.
        if let Some(reload) = config.reload {
            commands
                .entity(turret)
                .insert(SectionReload::from_config(reload));
        }
    }
}

/// Push live edits of a turret's [`TurretSectionConfigHelper`] onto the child entities that
/// snapshot it when the turret is built, so retuning takes effect immediately (the turret range
/// example's sliders, or the editor). `muzzle_speed` is read live by the aim/shoot systems and
/// needs no propagation; only the snapshotted knobs (rotator speeds, pitch limits, fire rate) are
/// pushed here. Gated on `Changed` so it costs nothing when nothing is being tuned.
pub(super) fn apply_turret_config_to_children(
    q_turret: Query<
        (&TurretSectionConfigHelper, &Children),
        (
            With<TurretSectionMarker>,
            Changed<TurretSectionConfigHelper>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_joint: Query<(
        Option<&mut SmoothLookRotation>,
        Option<&mut TurretSectionBarrelFireState>,
    )>,
) {
    // Match each config node to its joint entity by tree POSITION (DFS order):
    // `insert_turret_section` spawns children in `joint.children` order, so the
    // config tree and the entity tree walk in lockstep.
    fn apply_node(
        joint: &TurretJoint,
        entity: Entity,
        q_children: &Query<&Children>,
        q_joint: &mut Query<(
            Option<&mut SmoothLookRotation>,
            Option<&mut TurretSectionBarrelFireState>,
        )>,
    ) {
        if let Ok((rotation, fire_state)) = q_joint.get_mut(entity) {
            if let (Some(axis), Some(mut rotation)) = (joint.axis, rotation) {
                rotation.axis = axis;
                rotation.speed = joint.speed;
                rotation.min = joint.min;
                rotation.max = joint.max;
            }
            if let (Some(muzzle), Some(mut fire_state)) = (&joint.muzzle, fire_state) {
                let interval = 1.0 / muzzle.fire_rate.max(f32::EPSILON);
                fire_state.0.set_duration(Duration::from_secs_f32(interval));
            }
        }

        // Recurse over the matching child entities (skipping non-joint children
        // like render meshes/effects, which are appended AFTER the joint kids).
        let joint_children: Vec<Entity> = q_children
            .get(entity)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        for (child_joint, &child_entity) in joint.children.iter().zip(joint_children.iter()) {
            apply_node(child_joint, child_entity, q_children, q_joint);
        }
    }

    for (config, children) in &q_turret {
        // The turret section entity has exactly one joint child: the tree root
        // (render children are added to the joint entities, not the section).
        if let Some(&root) = children.iter().collect::<Vec<_>>().first() {
            apply_node(&config.root, root, &q_children, &mut q_joint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::test_support::*, *};

    /// The mutable config joint whose hinge axis is roughly `axis` (DFS order).
    fn config_joint_mut(root: &mut TurretJoint, axis: Vec3) -> Option<&mut TurretJoint> {
        if root
            .axis
            .is_some_and(|a| a.normalize().dot(axis.normalize()) > 0.99)
        {
            return Some(root);
        }
        root.children
            .iter_mut()
            .find_map(|c| config_joint_mut(c, axis))
    }

    /// The articulated joint entity of `turret` whose axis is roughly `axis`.
    fn joint_entity_with_axis(app: &App, turret: Entity, axis: Vec3) -> Entity {
        let world = app.world();
        let mut best = None;
        // Every joint entity carries TurretSectionPartOf(turret) + a marker.
        for entity in world.iter_entities() {
            let id = entity.id();
            let Some(part_of) = world.get::<TurretSectionPartOf>(id) else {
                continue;
            };
            if **part_of != turret {
                continue;
            }
            let Some(marker) = world.get::<TurretJointMarker>(id) else {
                continue;
            };
            if marker
                .axis
                .is_some_and(|a| a.normalize().dot(axis.normalize()) > 0.99)
            {
                best = Some(id);
            }
        }
        best.expect("a joint with the requested axis exists")
    }

    /// Build a real turret via the spawn observer, so the joint entities and
    /// their `Children`/`ChildOf` links exist for the config-sync systems.
    fn spawn_real_turret(app: &mut App, config: TurretSectionConfig) -> Entity {
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut().flush();
        turret
    }

    #[test]
    fn editing_the_config_retunes_the_live_turret() {
        // The tuning sliders write `TurretSectionConfigHelper`; the snapshotted knobs on the
        // joint rotators and the fire timer must follow.
        let mut app = App::new();
        app.add_observer(insert_turret_section);
        app.add_systems(Update, apply_turret_config_to_children);

        let turret = spawn_real_turret(&mut app, TurretSectionConfig::default());
        let yaw = joint_entity_with_axis(&app, turret, Vec3::Y);
        let pitch = joint_entity_with_axis(&app, turret, Vec3::X);
        let muzzle = muzzle_entity(&app, turret);

        {
            let mut helper = app
                .world_mut()
                .get_mut::<TurretSectionConfigHelper>(turret)
                .unwrap();
            config_joint_mut(&mut helper.root, Vec3::Y).unwrap().speed = 5.0;
            let pitch_cfg = config_joint_mut(&mut helper.root, Vec3::X).unwrap();
            pitch_cfg.speed = 6.0;
            pitch_cfg.min = Some(-0.25);
            pitch_cfg.max = Some(0.5);
            // The muzzle is the pitch joint's fixed descendant; retune its rate.
            fn set_fire_rate(joint: &mut TurretJoint, rate: f32) {
                if let Some(m) = &mut joint.muzzle {
                    m.fire_rate = rate;
                }
                for c in &mut joint.children {
                    set_fire_rate(c, rate);
                }
            }
            set_fire_rate(&mut helper.root, 25.0);
        }
        app.update();

        assert_eq!(
            app.world().get::<SmoothLookRotation>(yaw).unwrap().speed,
            5.0
        );
        let pitch_rot = app.world().get::<SmoothLookRotation>(pitch).unwrap();
        assert_eq!(pitch_rot.speed, 6.0);
        assert_eq!(pitch_rot.min, Some(-0.25));
        assert_eq!(pitch_rot.max, Some(0.5));
        let duration = app
            .world()
            .get::<TurretSectionBarrelFireState>(muzzle)
            .unwrap()
            .0
            .duration();
        assert!((duration.as_secs_f32() - 1.0 / 25.0).abs() < 1e-6);
    }

    #[test]
    fn retuning_one_turret_leaves_another_alone() {
        // The tree-position match must scope edits to the edited turret's own joints.
        let mut app = App::new();
        app.add_observer(insert_turret_section);
        app.add_systems(Update, apply_turret_config_to_children);

        let edited = spawn_real_turret(&mut app, TurretSectionConfig::default());
        let other = spawn_real_turret(&mut app, TurretSectionConfig::default());
        let edited_yaw = joint_entity_with_axis(&app, edited, Vec3::Y);
        let other_yaw = joint_entity_with_axis(&app, other, Vec3::Y);

        {
            let mut helper = app
                .world_mut()
                .get_mut::<TurretSectionConfigHelper>(edited)
                .unwrap();
            config_joint_mut(&mut helper.root, Vec3::Y).unwrap().speed = 9.0;
        }
        app.update();

        assert_eq!(
            app.world()
                .get::<SmoothLookRotation>(edited_yaw)
                .unwrap()
                .speed,
            9.0
        );
        assert_eq!(
            app.world()
                .get::<SmoothLookRotation>(other_yaw)
                .unwrap()
                .speed,
            std::f32::consts::PI,
            "an untouched turret's rotators must not change"
        );
    }

    #[test]
    fn default_turret_builds_the_base_yaw_pitch_barrel_muzzle_chain() {
        // GOLDEN CHAIN: the migrated default turret must produce the SAME
        // kinematic chain the flat config used to: 5 joint entities
        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) -> muzzle(fixed),
        // with SmoothLookRotation only on yaw+pitch (right axes/speeds/limits)
        // and the muzzle marker + fire timer on the leaf. Offsets preserved.
        let mut app = App::new();
        app.add_observer(insert_turret_section);
        let turret = spawn_real_turret(&mut app, TurretSectionConfig::default());

        let joints = joint_entities(&app);
        assert_eq!(joints.len(), 5, "the default tree has five joints");

        let axes: Vec<Option<Vec3>> = joints
            .iter()
            .map(|&e| app.world().get::<TurretJointMarker>(e).unwrap().axis)
            .collect();
        // Exactly two articulated joints, on Y then X.
        let articulated: Vec<Vec3> = axes.iter().filter_map(|a| *a).collect();
        assert_eq!(articulated.len(), 2, "yaw + pitch are the only hinges");

        let yaw = joint_entity_with_axis(&app, turret, Vec3::Y);
        let pitch = joint_entity_with_axis(&app, turret, Vec3::X);

        let yaw_rot = app.world().get::<SmoothLookRotation>(yaw).unwrap();
        assert_eq!(yaw_rot.axis, Vec3::Y);
        assert_eq!(yaw_rot.speed, std::f32::consts::PI);
        assert_eq!(yaw_rot.min, None);
        assert_eq!(yaw_rot.max, None);

        let pitch_rot = app.world().get::<SmoothLookRotation>(pitch).unwrap();
        assert_eq!(pitch_rot.axis, Vec3::X);
        assert_eq!(pitch_rot.speed, std::f32::consts::PI);
        assert_eq!(pitch_rot.min, Some(-std::f32::consts::PI / 18.0));
        assert_eq!(pitch_rot.max, Some(std::f32::consts::FRAC_PI_2));

        // The muzzle joint is a fixed leaf carrying the muzzle marker + timer.
        let muzzle = muzzle_entity(&app, turret);
        assert!(app
            .world()
            .get::<TurretSectionBarrelMuzzleMarker>(muzzle)
            .is_some());
        assert!(app
            .world()
            .get::<TurretSectionBarrelFireState>(muzzle)
            .is_some());
        assert_eq!(
            app.world().get::<TurretJointMarker>(muzzle).unwrap().axis,
            None,
            "the muzzle joint is fixed"
        );

        // Offsets are preserved on the joint transforms.
        assert_eq!(
            app.world().get::<Transform>(muzzle).unwrap().translation,
            Vec3::new(0.0, 0.0, -0.5)
        );
        assert_eq!(
            app.world().get::<Transform>(yaw).unwrap().translation,
            Vec3::new(0.0, 0.1, 0.0)
        );
    }

    /// F10: `fire_rate` is a plain required f32 on the serde-deserialized
    /// turret config. `0.0` gives `1.0 / 0.0 == +inf`, and
    /// `Duration::from_secs_f32(inf)` PANICS - so authored content took the
    /// game down the moment the ship spawned. The retune path one function
    /// away already carried the clamp; the spawn path did not.
    #[test]
    fn a_degenerate_fire_rate_does_not_panic_the_spawn() {
        fn set_fire_rate(joint: &mut TurretJoint, rate: f32) {
            if let Some(muzzle) = &mut joint.muzzle {
                muzzle.fire_rate = rate;
            }
            for child in &mut joint.children {
                set_fire_rate(child, rate);
            }
        }

        for rate in [0.0, -1.0] {
            let mut app = App::new();
            app.add_observer(insert_turret_section);

            let mut config = TurretSectionConfig::default();
            set_fire_rate(&mut config.root, rate);
            let turret = spawn_real_turret(&mut app, config);

            let muzzle = muzzle_entity(&app, turret);
            let state = app
                .world()
                .get::<TurretSectionBarrelFireState>(muzzle)
                .expect("the muzzle still spawns with a fire timer");
            assert!(
                state.0.duration().as_secs_f32().is_finite(),
                "fire_rate {rate} yields a finite interval"
            );
        }
    }
}
