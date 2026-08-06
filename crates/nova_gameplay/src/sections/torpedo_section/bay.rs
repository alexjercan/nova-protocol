//! The bay's launch path: building the bay section, ticking its fire timer,
//! spawning a torpedo, and killing a torpedo whose body is shot down.

use super::*;

/// Mark the whole torpedo as killed when any of its body sections dies.
///
/// The torpedo root is collider-less: bullets kill its CHILD sections
/// (controller/thruster, 1 HP each) through the normal health pipeline, but
/// nothing told the root - the husk kept flying and its proximity fuze
/// still fired. On ordnance every section is vital, so one dead section
/// kills the torpedo. Deliberately NO `blast_damage` on this path:
/// defeating the warhead is the point of shooting a torpedo down - a
/// shot-down torpedo dies quietly, only a detonation
/// (torpedo_detonate_system) explodes.
pub(super) fn on_torpedo_body_destroyed(
    add: On<Add, HealthZeroMarker>,
    q_section: Query<&ChildOf>,
    q_torpedo: Query<Entity, With<TorpedoProjectileMarker>>,
    mut commands: Commands,
) {
    let entity = add.entity;
    let Ok(ChildOf(parent)) = q_section.get(entity) else {
        return;
    };
    let Ok(root) = q_torpedo.get(*parent) else {
        return;
    };
    // try_insert: both body sections can die in the same burst, and the
    // root itself may already be despawning for another reason.
    commands.entity(root).try_insert(TorpedoShotDownMarker);
}

/// Remove shot-down torpedoes, one schedule pass after the marker landed -
/// by then every command the integrity pipeline queued for the dying
/// section has been applied to a still-live entity (see
/// [`TorpedoShotDownMarker`] for the crash this ordering prevents).
pub(super) fn despawn_shot_down_torpedoes(
    q_torpedo: Query<Entity, (With<TorpedoProjectileMarker>, With<TorpedoShotDownMarker>)>,
    mut commands: Commands,
) {
    for torpedo in &q_torpedo {
        commands.entity(torpedo).try_despawn();
    }
}

pub(super) fn insert_torpedo_section(
    add: On<Add, TorpedoSectionMarker>,
    mut commands: Commands,
    q_section: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_section: entity {:?}", entity);

    let Ok(config) = q_section.get(entity) else {
        error!(
            "insert_torpedo_section: entity {:?} not found in q_section",
            entity
        );
        return;
    };

    let interval = 1.0 / config.fire_rate;

    let spawner = commands
        .spawn((
            Name::new("Torpedo Section Spawner"),
            TorpedoSectionSpawnerMarker,
            TorpedoSectionPartOf(entity),
            TorpedoSectionSpawnerFireState(Cooldown::new(interval)),
            TorpedoSectionSpawnerEffect(config.launch_effect.clone()),
            TorpedoSectionLaunchSound(config.launch_sound.clone()),
            Transform::from_translation(config.spawn_offset).with_rotation(config.spawn_rotation),
            Visibility::Inherited,
        ))
        .id();

    let body = commands
        .spawn((
            Name::new("Torpedo Section Body"),
            TorpedoSectionBodyMarker,
            TorpedoSectionPartOf(entity),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    commands
        .entity(entity)
        .insert(TorpedoSectionSpawnerEntity(spawner))
        .add_children(&[body, spawner]);

    // Opt-in finite ammo: a magazine on the torpedo SECTION entity (the one
    // `shoot_spawn_projectile` queries), depleted one per launch. `None` leaves
    // the bay unlimited, matching the pre-ammo behavior.
    if let Some(capacity) = config.ammo_capacity {
        commands.entity(entity).insert(SectionAmmo::new(capacity));
        // Auto-reload rides on the magazine: only a finite bay can rearm.
        if let Some(reload) = config.reload {
            commands
                .entity(entity)
                .insert(SectionReload::from_config(reload));
        }
    }
}

pub(super) fn update_spawner_fire_state(
    mut q_spawner: Query<
        &mut TorpedoSectionSpawnerFireState,
        (
            With<TorpedoSectionSpawnerMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    time: Res<Time>,
) {
    for mut fire_state in &mut q_spawner {
        fire_state.tick(time.delta_secs());
    }
}

pub(super) fn shoot_spawn_projectile(
    mut commands: Commands,
    q_spaceship: Query<
        (
            &Position,
            &Rotation,
            &LinearVelocity,
            &AngularVelocity,
            &ComputedCenterOfMass,
            Option<&Allegiance>,
        ),
        With<SpaceshipRootMarker>,
    >,
    mut q_section: Query<
        (
            Entity,
            &TorpedoSectionSpawnerEntity,
            &ChildOf,
            &TorpedoSectionConfigHelper,
            &TorpedoSectionInput,
            Option<&mut SectionAmmo>,
        ),
        (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_spawner: Query<&mut TorpedoSectionSpawnerFireState, With<TorpedoSectionSpawnerMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
) {
    for (section, spawner, ChildOf(spaceship), config, input, mut ammo) in &mut q_section {
        if !**input {
            continue;
        }
        // Live weapons-safety gate, same rule as the turret; unmanaged ships
        // fire freely.
        if q_hot.get(*spaceship).is_ok_and(|hot| !hot.0) {
            continue;
        }

        // Out of torpedoes: an empty bay launches nothing. A bay with no
        // `SectionAmmo` (unlimited) is never gated here.
        if ammo.as_deref().is_some_and(SectionAmmo::is_empty) {
            continue;
        }

        let Ok((position, rotation, lin_vel, ang_vel, center, allegiance)) =
            q_spaceship.get(*spaceship)
        else {
            error!(
                "shoot_spawn_projectile: entity {:?} not found in q_spaceship",
                spaceship
            );
            continue;
        };

        let Ok(mut fire_state) = q_spawner.get_mut(**spawner) else {
            error!(
                "shoot_spawn_projectile: entity {:?} not found in q_spawner",
                **spawner
            );
            continue;
        };

        if !fire_state.ready() {
            continue;
        }

        // Bay pose on the RAW physics clock: the root's avian pose composed
        // with the local mount chain (section -> spawner). This system runs
        // in FixedUpdate, where `GlobalTransform` still holds the previous
        // frame's EASED render pose - the old Update-schedule spawn sampled
        // that eased pose with raw velocities, so at speed the launch point
        // trailed the raw bay by up to a tick of ship motion (the turret's
        // two-clocks mix at single-shot severity).
        let Some((bay_local_pos, bay_local_rot)) =
            local_pose_in_root(**spawner, *spaceship, &q_chain)
        else {
            error!(
                "shoot_spawn_projectile: spawner {:?} is not a descendant of ship {:?}",
                **spawner, spaceship
            );
            continue;
        };
        let projectile_rotation = rotation.0 * bay_local_rot;
        let projectile_position = position.0 + rotation.mul_vec3(bay_local_pos);
        // The spawner launches along its +Y (the bay's "up", as authored).
        let spawner_direction = projectile_rotation * Vec3::Y;

        // Inherit the full motion of the bay, not just the ship's linear
        // velocity: a bay offset from the center of mass of a rotating ship
        // also swings tangentially. avian's `ComputedCenterOfMass` is
        // body-local; lift it with the same raw pose as everything else.
        let center_of_mass = position.0 + rotation.mul_vec3(**center);
        let inertia_vel =
            rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, projectile_position);

        let spawner_exit_velocity = spawner_direction * config.spawner_speed;
        let linear_velocity = spawner_exit_velocity + inertia_vel;

        // Spawn AT the bay, no nudge: the old `+ exit * 0.01` was a static
        // stand-in for sub-tick lead. A torpedo is a single guided launch -
        // there is no stream whose spacing could expose tick quantization,
        // and PN guidance absorbs the sub-tick residual - so launch timing
        // stays tick-quantized on purpose (contrast the turret's exact
        // sub-tick lead, which a 100 rounds/s stream does need).
        let projectile_transform = Transform {
            translation: projectile_position,
            rotation: projectile_rotation,
            ..default()
        };

        let mut projectile = commands.spawn((
            Name::new("Torpedo Projectile"),
            TorpedoProjectileMarker,
            ProjectileOwner(*spaceship),
            projectile_transform,
            RigidBody::Dynamic,
            // Fast mover watched by the smoothed chase camera: interpolate
            // between fixed ticks like turret bullets do, or it stair-steps.
            // The easing seed makes the FIRST rendered frame sit at the
            // rendered bay too: a body spawned mid-tick misses FixedFirst,
            // so without a seeded `start` the spawn frame would show the
            // raw pose while the world renders eased - one frame of
            // launch pop (same mechanism and fix as turret bullets).
            (
                TransformInterpolation,
                TranslationEasingState {
                    start: Some(projectile_position),
                    end: None,
                },
                RotationEasingState {
                    start: Some(projectile_rotation),
                    end: None,
                },
            ),
            LinearVelocity(linear_velocity),
            TorpedoSectionPartOf(section),
            // Nested tuple: keeps the outer bundle under bevy's tuple size
            // limit. The detonation voice rides the projectile: its
            // destruction (the blast) fires the destroy observer, which reads
            // this snapshot.
            (
                TorpedoSectionSpawnerEntity(**spawner),
                ImpactDestroySounds {
                    impact: None,
                    destroy: config.detonation_sound.clone(),
                },
            ),
            TorpedoProjectileRenderMesh(config.projectile_render_mesh.clone()),
            // No `TorpedoTargetPosition` yet: it is inserted only once a target is
            // locked (see `update_target_position`). Until then the torpedo has no
            // target and flies straight ahead rather than steering at the origin.
            (
                TorpedoGuidance {
                    nav_constant: config.nav_constant,
                    max_speed: config.max_speed,
                },
                TorpedoSteering(projectile_transform.forward().into()),
                LinearDamping(config.linear_damping),
                TorpedoBlast {
                    radius: config.blast_radius,
                    damage: config.blast_damage,
                },
            ),
            TorpedoArming::new(
                config.arm_time,
                config.arm_distance,
                projectile_transform.translation,
            ),
            TempEntity(config.projectile_lifetime),
            Visibility::Visible,
            children![
                (
                    TorpedoControllerMarker,
                    // The torpedo's colliders live on these child sections, so the
                    // owner collision filter (ProjectileHooks) opts in here, not on
                    // the collider-less root.
                    ActiveCollisionHooks::FILTER_PAIRS,
                    base_section(BaseSectionConfig {
                        id: "torpedo_controller".to_string(),
                        name: "Torpedo Controller".to_string(),
                        description: "The controller for the torpedo warhead".to_string(),
                        mass: 1.0,
                        health: 1.0,
                        ..default()
                    }),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)).with_rotation(
                        Quat::from_euler(EulerRot::XYZ, std::f32::consts::FRAC_PI_2, 0.0, 0.0)
                    ),
                    ControllerSectionRenderMarker,
                    controller_section(ControllerSectionConfig {
                        frequency: 4.0,
                        damping_ratio: 4.0,
                        max_torque: 10.0,
                        render_mesh: None,
                        // A torpedo's guidance computer has no radar/safety
                        // voice; the player-controller cue lookup never matches
                        // it anyway (its parent is the projectile, not a ship).
                        ..default()
                    }),
                ),
                (
                    TorpedoThrusterMarker,
                    ActiveCollisionHooks::FILTER_PAIRS,
                    base_section(BaseSectionConfig {
                        id: "torpedo_thruster".to_string(),
                        name: "Torpedo Thruster".to_string(),
                        description: "The thruster for the torpedo".to_string(),
                        mass: 1.0,
                        health: 1.0,
                        ..default()
                    }),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                    ThrusterSectionRenderMarker,
                    thruster_section(ThrusterSectionConfig {
                        magnitude: 1.0,
                        render_mesh: None,
                        render_mesh_transform: None,
                        // The torpedo's engine keeps the base hum (DIRECT path:
                        // this bundle is built at runtime outside the merge).
                        // Lifting it to a TorpedoSectionConfig field is a
                        // future authoring step if a mod wants a custom whine.
                        loop_sound: Some(AssetRef::from("base/sounds/thruster_loop.wav")),
                        exhaust: None,
                    }),
                    children![(
                        Name::new("Thruster Exhaust"),
                        Transform::from_rotation(Quat::from_rotation_x(
                            std::f32::consts::FRAC_PI_2
                        ))
                        .with_translation(Vec3::new(0.0, 0.0, -0.45)),
                        ThrusterExhaustConfig {
                            geometry: ThrusterExhaustShape::Cone,
                            width: 0.8,
                            height: 0.8,
                            exhaust_height: 0.1,
                            exhaust_radius: 0.15,
                            exhaust_max: 1.0,
                            exhaust_inner_height: 0.05,
                            exhaust_inner_radius: 0.05,
                            exhaust_inner_max: 0.5,
                            emissive_color: LinearRgba::new(10.0, 5.0, 0.0, 1.0),
                            emissive_inner_color: LinearRgba::new(10.0, 0.0, 0.0, 1.0),
                        },
                    )],
                )
            ],
        ));
        // The torpedo COPIES the shooter's allegiance instead of resolving
        // through ProjectileOwner at read time: it stays attributable even if
        // the owner dies mid-flight, and consumers stay single-query.
        if let Some(&allegiance) = allegiance {
            projectile.insert(allegiance);
        }

        // A torpedo left the bay: spend one round. The empty-bay gate above
        // already refused to reach here on a spent magazine, so this only ever
        // fires on a launch that actually happened. Unlimited bays carry no
        // `SectionAmmo` and are unaffected.
        if let Some(ammo) = ammo.as_deref_mut() {
            ammo.try_consume();
        }

        // Start the next launch wait.
        fire_state.trigger();
    }
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// A minimal app running ONLY `shoot_spawn_projectile` on a manual clock, so
    /// bay ammo is observed by counting launched torpedoes without the physics /
    /// render stack. A wide `dt` keeps the bay's fire timer finished each tick.
    fn firing_app(dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        // The bay's fire timer is ticked by a separate system; without it the
        // timer never re-arms after the first launch (unlike the turret, which
        // ticks in its own fire system). Tick before firing so the timer is
        // finished when the shot is considered.
        app.add_systems(
            Update,
            (update_spawner_fire_state, shoot_spawn_projectile).chain(),
        );
        app
    }

    /// Spawn a ship + one torpedo bay holding its trigger, optionally with a
    /// finite magazine. Spawner is parented under the section under the ship so
    /// `local_pose_in_root` resolves; the fire timer starts finished.
    fn spawn_firing_bay(app: &mut App, ammo: Option<u32>) -> Entity {
        // A fast bay so the fire timer re-arms every tick (the default 1/s
        // interval outruns the virtual clock's 0.25s max-delta clamp, which
        // would starve the timer). The bay still launches at most one torpedo
        // per tick, so ammo is what caps the total.
        let config = TorpedoSectionConfig {
            fire_rate: 100.0,
            ..TorpedoSectionConfig::default()
        };
        let interval = 1.0 / config.fire_rate;

        let world = app.world_mut();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let section = world
            .spawn((
                TorpedoSectionMarker,
                TorpedoSectionConfigHelper(config),
                TorpedoSectionInput(true),
                Transform::default(),
                ChildOf(ship),
            ))
            .id();
        let spawner = world
            .spawn((
                TorpedoSectionSpawnerMarker,
                TorpedoSectionSpawnerFireState(Cooldown::new(interval)),
                Transform::default(),
                ChildOf(section),
            ))
            .id();
        world
            .entity_mut(section)
            .insert(TorpedoSectionSpawnerEntity(spawner));
        if let Some(capacity) = ammo {
            world.entity_mut(section).insert(SectionAmmo::new(capacity));
        }
        section
    }

    fn torpedo_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn a_bay_with_ammo_launches_exactly_its_magazine_then_stops() {
        // One launch per tick spends one round; a 2-torpedo bay held open for
        // many ticks launches exactly two, then the empty-bay gate suppresses
        // the rest.
        let mut app = firing_app(2.0);
        let section = spawn_firing_bay(&mut app, Some(2));

        for _ in 0..6 {
            app.update();
        }

        assert_eq!(
            torpedo_count(&mut app),
            2,
            "a 2-torpedo bay must launch exactly two torpedoes, ever"
        );
        let ammo = app
            .world()
            .entity(section)
            .get::<SectionAmmo>()
            .expect("the bay keeps its magazine");
        assert_eq!(ammo.rounds, 0, "the bay must read empty after firing out");
    }

    #[test]
    fn a_bay_without_ammo_keeps_launching_past_a_magazine() {
        // A/B control: the identical rig with no `SectionAmmo` launches every
        // tick, past two - proof the empty-bay gate, not another limit, stopped
        // the salvo above.
        let mut app = firing_app(2.0);
        spawn_firing_bay(&mut app, None);

        for _ in 0..6 {
            app.update();
        }

        assert!(
            torpedo_count(&mut app) > 2,
            "an unlimited bay must not be capped at a magazine size, got {}",
            torpedo_count(&mut app)
        );
    }

    #[test]
    fn a_regenerating_bay_rearms_and_launches_past_its_magazine() {
        // End-to-end recovery for continuous regen (the torpedo tuning): a spent
        // 2-torpedo bay regrows rounds over time and launches MORE than its
        // magazine, versus the no-reload rig above which caps at 2 forever.
        let mut app = firing_app(2.0);
        app.add_systems(Update, crate::sections::ammo::tick_section_reload);
        let section = spawn_firing_bay(&mut app, Some(2));
        // Regen one round per ~0.2s cycle (under the 0.25s per-tick clamp).
        app.world_mut()
            .entity_mut(section)
            .insert(SectionReload::from_config(SectionReloadConfig {
                reload_time: 0.2,
                rounds_per_cycle: 1,
                only_when_empty: false,
            }));

        for _ in 0..12 {
            app.update();
        }

        assert!(
            torpedo_count(&mut app) > 2,
            "a regenerating bay must rearm and launch past its magazine, got {}",
            torpedo_count(&mut app)
        );
    }

    #[test]
    fn a_dead_body_section_kills_the_whole_torpedo() {
        // The root is collider-less: bullets kill child sections, and one
        // dead section must take the whole torpedo down before its fuze
        // can fire again.
        let mut app = App::new();
        app.add_observer(on_torpedo_body_destroyed);
        let root = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, Transform::default()))
            .id();
        let body = app
            .world_mut()
            .spawn((SectionMarker, Health::new(1.0), ChildOf(root)))
            .id();

        app.add_systems(Update, despawn_shot_down_torpedoes);
        app.world_mut().entity_mut(body).insert(HealthZeroMarker);
        assert!(
            app.world().get::<TorpedoShotDownMarker>(root).is_some(),
            "the observer marks the root immediately (and safely)"
        );
        app.update();

        assert!(
            !app.world().entities().contains(root),
            "the torpedo root must despawn with its dead body section"
        );
        assert!(!app.world().entities().contains(body));
    }

    #[test]
    fn a_shot_down_torpedo_dies_without_its_blast() {
        // Through the real health pipeline: damage a body section to zero
        // and assert the torpedo dies QUIETLY - no blast_damage entity.
        // Defeating the warhead is the point of shooting it down.
        let mut app = App::new();
        app.add_plugins(HealthPlugin);
        app.add_observer(on_torpedo_body_destroyed);
        app.add_systems(Update, despawn_shot_down_torpedoes);
        let root = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, Transform::default()))
            .id();
        let body = app
            .world_mut()
            .spawn((SectionMarker, Health::new(1.0), ChildOf(root)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: body,
            source: None,
            amount: 2.0,
        });
        app.update();

        assert!(
            !app.world().entities().contains(root),
            "one killed section ends the threat"
        );
        let blasts = app
            .world_mut()
            .query_filtered::<Entity, With<NovaBlast>>()
            .iter(app.world())
            .count();
        assert_eq!(blasts, 0, "a shot-down torpedo must not detonate");
    }

    #[test]
    fn a_dead_section_of_a_non_torpedo_parent_is_left_to_integrity() {
        // A ship section dying must NOT despawn its ship: the observer only
        // acts on torpedo roots.
        let mut app = App::new();
        app.add_observer(on_torpedo_body_destroyed);
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::default()))
            .id();
        let section = app
            .world_mut()
            .spawn((SectionMarker, Health::new(1.0), ChildOf(ship)))
            .id();

        app.world_mut().entity_mut(section).insert(HealthZeroMarker);
        app.update();

        assert!(app.world().entities().contains(ship));
        assert!(app.world().entities().contains(section));
    }

    #[test]
    fn the_kill_does_not_race_commands_queued_for_the_dying_section() {
        // Live-game crash regression (20260710): commands queued for the
        // dying section in the same flush as the zero-health marker (the
        // integrity pipeline's IntegrityDisabledMarker insert) must land on
        // a still-live entity. A same-flush insert on the section after the
        // observer ran must NOT panic - the despawn happens a pass later.
        let mut app = App::new();
        app.add_observer(on_torpedo_body_destroyed);
        app.add_systems(Update, despawn_shot_down_torpedoes);
        let root = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, Transform::default()))
            .id();
        let body = app
            .world_mut()
            .spawn((SectionMarker, Health::new(1.0), ChildOf(root)))
            .id();

        // Same-flush sequence: zero-health marker (observer runs, marks the
        // root), then another insert on the dying section - the integrity
        // pipeline's pattern.
        app.world_mut()
            .entity_mut(body)
            .insert(HealthZeroMarker)
            .insert(SectionInactiveMarker);

        app.update();
        assert!(!app.world().entities().contains(root));
    }

    #[test]
    fn a_shot_down_torpedo_cannot_detonate_in_the_removal_gap() {
        // Armed, on its target, but marked shot down: the fuze must stay
        // quiet for the tick between the marker and the despawn.
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);
        let mut arming = TorpedoArming::new(0.0, 0.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO);
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                TorpedoShotDownMarker,
                Transform::default(),
                TorpedoTargetPosition(Vec3::ZERO),
                arming,
                TorpedoBlast {
                    radius: 10.0,
                    damage: 5.0,
                },
                TorpedoSectionPartOf(Entity::PLACEHOLDER),
            ))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "the fuze must not fire on a shot-down torpedo"
        );
        let blasts = app
            .world_mut()
            .query_filtered::<Entity, With<NovaBlast>>()
            .iter(app.world())
            .count();
        assert_eq!(blasts, 0);
    }

    #[test]
    fn launched_torpedo_copies_the_shooter_allegiance() {
        // Same rule as turret bullets: the torpedo reads as the shooter's
        // side (relation model), copied at spawn so "your own torpedo" stays
        // yours even if the shooter dies mid-flight.
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Player,
                Transform::default(),
                // The raw-clock spawn composes the bay from the ship's avian
                // pose, so the rig carries it.
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let section = world
            .spawn((
                TorpedoSectionMarker,
                ChildOf(ship),
                Transform::default(),
                TorpedoSectionConfigHelper(TorpedoSectionConfig::default()),
                TorpedoSectionInput(true),
            ))
            .id();
        // The spawner sits in the ship's mount chain (section -> spawner),
        // as insert_torpedo_section builds it.
        let spawner = world
            .spawn((
                TorpedoSectionSpawnerMarker,
                ChildOf(section),
                Transform::default(),
                // A fresh Cooldown is ready, so the very first run fires.
                TorpedoSectionSpawnerFireState(Cooldown::new(0.1)),
            ))
            .id();
        world
            .entity_mut(section)
            .insert(TorpedoSectionSpawnerEntity(spawner));

        world.run_system_once(shoot_spawn_projectile).unwrap();

        let allegiance = world
            .query_filtered::<Option<&Allegiance>, With<TorpedoProjectileMarker>>()
            .iter(&world)
            .next()
            .expect("a torpedo spawned");
        assert_eq!(allegiance, Some(&Allegiance::Player));
    }

    /// A torpedo must launch FROM its bay on both clocks, at any ship
    /// velocity. The old spawn ran in Update from TransformHelper's
    /// EASED pose with raw velocities: at speed the spawn point trailed
    /// the raw bay by up to a tick of ship motion, cross-contaminating the
    /// launch (the same two-clocks mix as turret bullets, single-shot
    /// severity). On the raw clock the spawn Position must sit ON the bay
    /// composed from the ship's raw Position/Rotation (plus at most two
    /// ticks of exit travel along the launch direction), and - via the
    /// interpolation easing seed - the first RENDERED Transform must sit
    /// on the bay composed from the ship's eased Transform. The rig runs
    /// the REAL plugin, ship velocity perpendicular to the launch
    /// direction, damping zeroed so the only relative motion is the exit
    /// velocity, and a tilted spawner rotation so the frame composition
    /// is non-trivial.
    #[test]
    fn torpedo_launches_from_the_bay_on_both_clocks_at_speed() {
        use std::collections::HashSet;

        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_plugins(TorpedoSectionPlugin { render: false });
        app.finish();

        let spawn_rotation = Quat::from_rotation_x(0.3);
        let section_local = Vec3::new(0.0, 1.0, 0.0);
        let spawn_offset = Vec3::new(0.0, 0.0, -1.0);
        let spawner_speed = 30.0;
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                Transform::default(),
                TransformInterpolation,
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                TorpedoSectionMarker,
                ChildOf(ship),
                Transform::from_translation(section_local),
                TorpedoSectionConfigHelper(TorpedoSectionConfig {
                    spawn_offset,
                    spawn_rotation,
                    spawner_speed,
                    // ~2 launches/second against 60 fps frames: several
                    // launches sample different easing phases.
                    fire_rate: 2.0,
                    // Zero damping: the only relative motion between bay
                    // and torpedo is the exit velocity, so the geometry
                    // bounds below are exact.
                    linear_damping: 0.0,
                    ..default()
                }),
                TorpedoSectionInput(false),
            ))
            .id();
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        app.world_mut()
            .get_mut::<TorpedoSectionInput>(section)
            .unwrap()
            .0 = true;

        // Bay locals composed once (the chain never changes): the spawner
        // sits at section + spawn_offset, its frame is spawn_rotation, and
        // its +Y is the exit direction.
        let bay_local_pos = section_local + spawn_offset;
        let dt = 1.0 / 64.0;

        let mut seen: HashSet<Entity> = HashSet::new();
        let mut launches = 0usize;
        let mut max_raw_cross = 0.0f32;
        let mut max_render_cross = 0.0f32;
        for _ in 0..180 {
            app.update();
            let ship_raw_pos = app.world().get::<Position>(ship).unwrap().0;
            let ship_raw_rot = app.world().get::<Rotation>(ship).unwrap().0;
            let ship_tf = *app.world().get::<Transform>(ship).unwrap();
            let raw_bay = ship_raw_pos + ship_raw_rot * bay_local_pos;
            let eased_bay = ship_tf.translation + ship_tf.rotation * bay_local_pos;
            let exit_dir = (ship_raw_rot * spawn_rotation) * Vec3::Y;

            let torpedoes: Vec<(Entity, Vec3, Vec3)> = app
                .world_mut()
                .query_filtered::<(Entity, &Position, &Transform), With<TorpedoProjectileMarker>>()
                .iter(app.world())
                .map(|(e, p, t)| (e, p.0, t.translation))
                .collect();
            for (torpedo, raw, rendered) in torpedoes {
                if !seen.insert(torpedo) {
                    continue;
                }
                launches += 1;

                // Raw clock: on the bay, plus at most two ticks of exit
                // travel strictly along the launch direction.
                let offset = raw - raw_bay;
                let along = offset.dot(exit_dir);
                let cross = (offset - along * exit_dir).length();
                max_raw_cross = max_raw_cross.max(cross);
                assert!(
                    along > -0.05 && along < 2.0 * spawner_speed * dt + 0.05,
                    "raw launch must sit within two ticks of exit travel \
                     ahead of the bay: along {along}"
                );

                // Render clock: the first rendered frame is attached to the
                // RENDERED bay (easing seed), again only ever ahead of it
                // along the launch direction.
                let render_offset = rendered - eased_bay;
                let render_along = render_offset.dot(exit_dir);
                let render_cross = (render_offset - render_along * exit_dir).length();
                max_render_cross = max_render_cross.max(render_cross);
                assert!(
                    render_along > -0.05 && render_along < 2.0 * spawner_speed * dt + 0.05,
                    "first rendered frame must sit within two ticks of exit \
                     travel ahead of the rendered bay: along {render_along}"
                );
            }
        }

        // Delivery guards: several real launches at real speed, or the
        // bounds above never bit.
        assert!(launches >= 2, "expected repeated launches, got {launches}");
        let speed = app.world().get::<LinearVelocity>(ship).unwrap().length();
        assert!(speed > 100.0, "the ship must still be at speed ({speed})");
        assert!(
            max_raw_cross < 0.05,
            "the raw spawn must sit ON the raw bay's launch line at any ship \
             velocity: max cross-offset {max_raw_cross}"
        );
        assert!(
            max_render_cross < 0.05,
            "the first rendered frame must sit ON the rendered bay's launch \
             line: max cross-offset {max_render_cross}"
        );
    }
}
