//! The bay's launch path: building the bay section, ticking its fire timer,
//! spawning a torpedo, and killing a torpedo whose body is shot down.

use super::*;

/// Mark the whole torpedo as killed when any of its body sections dies.
///
/// The torpedo root is collider-less: bullets kill its CHILD sections
/// (controller/thruster, 1 HP each) through the normal health pipeline, and
/// nothing else would tell the root, so the husk flies on with a live
/// proximity fuze. On ordnance every section is vital, so one dead section
/// kills the torpedo. Deliberately NO `blast_damage` on this path:
/// defeating the warhead is the point of shooting a torpedo down - a
/// shot-down torpedo dies quietly, only a detonation
/// (torpedo_detonate_system) explodes.
///
/// The root goes on RAILS in the same insert. Its sections carried every
/// collider it had and they are despawning, so between the marker and the
/// reaper the root is a DYNAMIC body with no mass - which avian reports as
/// "has no mass or inertia. This can cause NaN values", once per shot-down
/// torpedo. Static states what is already true of a body awaiting removal: it
/// is not simulated. It also keeps the warning meaningful, because a dynamic
/// massless body is then always a defect rather than an expected frame.
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
    commands
        .entity(root)
        .try_insert((TorpedoShotDownMarker, RigidBody::Static));
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
        debug!("despawn_shot_down_torpedoes: torpedo {torpedo:?} shot down");
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
        .insert((
            TorpedoSectionSpawnerEntity(spawner),
            TorpedoSectionDoorSound(config.door_sound.clone()),
        ))
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
        (&mut TorpedoSectionSpawnerFireState, &TorpedoSectionPartOf),
        With<TorpedoSectionSpawnerMarker>,
    >,
    q_section: Query<(), (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>)>,
    time: Res<Time>,
) {
    // `SectionInactiveMarker` only ever lands on the SECTION, never on the
    // spawner, so the liveness has to be read through the back-reference. A bay
    // disabled in place must stop rearming, matching the turret, whose
    // per-muzzle cooldown ticks inside its section-gated loop.
    for (mut fire_state, section) in &mut q_spawner {
        if q_section.contains(**section) {
            fire_state.tick(time.delta_secs());
        }
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
            Option<&SectionAnimations>,
            Option<&mut SectionAmmo>,
            Option<&mut SectionReload>,
        ),
        (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_spawner: Query<&mut TorpedoSectionSpawnerFireState, With<TorpedoSectionSpawnerMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
) {
    for (section, spawner, ChildOf(spaceship), config, input, animations, mut ammo, mut reload) in
        &mut q_section
    {
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

        // The muzzle door gates the ejection: a bay with an authored
        // `MuzzleDoor` track launches only through a fully open iris. The held
        // trigger is what opens it (`drive_muzzle_doors`), so the first shot
        // of a salvo waits out the door travel and the rest leave on cadence
        // through the held-open door. A doorless bay launches immediately.
        if animations
            .and_then(|animations| animations.cue_progress(SectionAnimationCue::MuzzleDoor))
            .is_some_and(|progress| progress < 1.0)
        {
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
        let spawner_rotation = rotation.0 * bay_local_rot;
        // The spawner launches along its +Y (the bay's "up", as authored).
        let spawner_direction = spawner_rotation * Vec3::Y;
        // Born `spawn_recess` behind the muzzle: the spawner entity stays ON
        // the muzzle - the launch flash and the spatial sound play there -
        // and the torpedo starts this deep inside the tube, sliding its whole
        // travel out through the open iris.
        let projectile_position =
            position.0 + rotation.mul_vec3(bay_local_pos) - spawner_direction * config.spawn_recess;
        // Born NOSE ALONG TRAVEL. A torpedo's nose is its own -Z and the tube
        // ejects along the spawner's +Y, so spawning it in the spawner's raw
        // frame laid the warhead across its own velocity - by construction,
        // whatever `spawn_rotation` was authored as. Thrust from tick one hid
        // it, because the controller swung the nose round inside the frames
        // it took the drive to matter. The cold coast does not: the torpedo
        // now spends `ignition_delay` seconds visibly broadside to the way it
        // is travelling. This quarter turn puts its nose on the launch axis
        // and keeps the bay's roll about that axis.
        let projectile_rotation =
            spawner_rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);

        // Inherit the full motion of the bay, not just the ship's linear
        // velocity: a bay offset from the center of mass of a rotating ship
        // also swings tangentially. avian's `ComputedCenterOfMass` is
        // body-local; lift it with the same raw pose as everything else.
        let center_of_mass = position.0 + rotation.mul_vec3(**center);
        let inertia_vel =
            rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, projectile_position);

        // Engine boundary: the authored ejection speed is meters per second,
        // and avian's `LinearVelocity` counts world units.
        let spawner_exit_velocity = spawner_direction * config.spawner_speed.to_engine();
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

        let torpedo_type = &config.torpedo_type;
        let mut projectile = commands.spawn((
            // Named for the ORDNANCE, not for the mechanism: every reader that
            // resolves an entity to a label (a log line, the event timeline, a
            // probe snapshot's `owner` / `target`) then says WHICH torpedo it
            // is looking at, with no extra plumbing. Nested with the type so
            // the outer bundle stays inside bevy's tuple size limit.
            (
                Name::new(format!("{} Torpedo", torpedo_type.name)),
                TorpedoType {
                    name: torpedo_type.name.clone(),
                    tint: torpedo_type.tint,
                },
            ),
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
                DestroySound(config.detonation_sound.clone()),
            ),
            TorpedoProjectileRenderMesh(config.projectile_render_mesh.clone()),
            // No `TorpedoTargetPosition` yet: it is inserted only once a target is
            // locked (see `update_target_position`). Until then the torpedo has no
            // target and flies straight ahead rather than steering at the origin.
            (
                TorpedoGuidance {
                    nav_constant: config.nav_constant,
                    max_speed: config.torpedo_type.max_speed.to_engine(),
                },
                // The LAUNCH direction, which `projectile_rotation` above now
                // agrees with: the torpedo is born pointing this way and is
                // asked to hold it. Seeding this from the spawner's own frame
                // instead asked for a nose 90 degrees off the way the torpedo
                // was travelling, and the coast gave the controller the whole
                // window to act on it - the drive then lit across the run-in
                // and threw the flight 13 u off the line.
                TorpedoSteering(spawner_direction),
                LinearDamping(config.linear_damping),
                TorpedoBlast {
                    radius: config.blast_radius.to_engine(),
                    damage: config.blast_damage,
                },
            ),
            (
                TorpedoArming::new(
                    config.arm_time,
                    config.arm_distance.to_engine(),
                    projectile_transform.translation,
                ),
                // Every launch is a COLD launch. The torpedo leaves on the
                // ejection charge alone and `ignite_cold_torpedoes` lights it;
                // a bay authoring `ignition_delay` zero ignites on the next
                // tick rather than taking a different path out of the tube.
                TorpedoColdLaunch {
                    remaining: config.ignition_delay,
                },
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
                    // Inert until the drive lights - see `TorpedoColdLaunch`.
                    // `ColliderDisabled` applies only to the entity carrying
                    // it, so it goes HERE and not on the collider-less root.
                    ColliderDisabled,
                    base_section(BaseSectionConfig {
                        id: "torpedo_controller".to_string(),
                        name: "Torpedo Controller".to_string(),
                        description: "The controller for the torpedo warhead".to_string(),
                        health: config.projectile_health,
                        ..default()
                    }),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)).with_rotation(
                        Quat::from_euler(EulerRot::XYZ, std::f32::consts::FRAC_PI_2, 0.0, 0.0)
                    ),
                    ControllerSectionRenderMarker,
                    controller_section(ControllerSectionConfig {
                        steering_lag: 0.5,
                        // A torpedo is a hull like any other under the attitude
                        // model, and this leaves it structure-bound like every
                        // other: two 1 u sections put its arm at 1.0 u, whose
                        // structural ceiling needs about 6.5 of torque, so 50
                        // is comfortably past it and the warhead turns at what
                        // its airframe takes rather than at what its computer
                        // can push.
                        max_torque: 50.0,
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
                    ColliderDisabled,
                    base_section(BaseSectionConfig {
                        id: "torpedo_thruster".to_string(),
                        name: "Torpedo Thruster".to_string(),
                        description: "The thruster for the torpedo".to_string(),
                        health: config.projectile_health,
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

        // The weave rides on the projectile, phased off its own entity index so
        // the torpedoes of one salvo are each somewhere different on their
        // helix. Inserted after the spawn because the phase needs the id.
        let torpedo = projectile.id();
        projectile.insert(TorpedoWeave::new(
            config.torpedo_type.weave_angle,
            config.torpedo_type.weave_rate,
            projectile_transform.forward().into(),
            TorpedoWeave::phase_for(torpedo),
        ));

        // A torpedo left the bay: spend one round. The empty-bay gate above
        // already refused to reach here on a spent magazine, so this only ever
        // fires on a launch that actually happened. Unlimited bays carry no
        // `SectionAmmo` and are unaffected.
        if let Some(ammo) = ammo.as_deref_mut() {
            if ammo.try_consume() {
                if let Some(reload) = reload.as_deref_mut() {
                    reload.on_shot();
                }
            }
        }

        // Start the next launch wait.
        fire_state.trigger();

        // The door was already open to let this launch out; the hold keeps it
        // open until the torpedo has coasted clear. Insert-refresh, so a
        // follow-up launch extends the clearance window instead of cycling
        // the door.
        commands
            .entity(section)
            .insert(MuzzleDoorHold::for_launch(config.ignition_delay));
    }
}

/// Seconds the muzzle door stays open past ignition, so the closing petals
/// never overlap the drive lighting right outside them.
const MUZZLE_DOOR_LINGER: f32 = 0.3;

/// Keeps a bay's muzzle-door cue open while a launched torpedo clears the
/// tube. Inserted (or refreshed) by [`shoot_spawn_projectile`] on every
/// launch; [`drive_muzzle_doors`] ticks it down and closes the door when it
/// expires. The art itself is the section's authored `MuzzleDoor` animation
/// track - a bay authoring none carries this harmlessly.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub(super) struct MuzzleDoorHold {
    /// Seconds until the door may close.
    remaining: f32,
}

impl MuzzleDoorHold {
    /// The hold for one launch: the cold coast plus the closing margin.
    fn for_launch(ignition_delay: f32) -> Self {
        Self {
            remaining: ignition_delay + MUZZLE_DOOR_LINGER,
        }
    }
}

/// Steer the `MuzzleDoor` cue. Two things hold it open: the trigger and the
/// clearance hold. The HELD TRIGGER is the door intent - press fire and the
/// iris starts opening (the ejection itself waits on it, see
/// `shoot_spawn_projectile`), keep holding and it gapes ready across
/// cooldowns and reloads. The [`MuzzleDoorHold`] covers the launched torpedo:
/// it spans the cold coast plus a closing margin, so a tap-fired door still
/// closes behind the round, not on it. Everything else reads closed.
///
/// The trigger only counts when the bay could genuinely fire - weapons
/// safety and an empty magazine keep the doors shut - so an open iris always
/// telegraphs a live launch threat, for the player and for whoever is
/// reading the raider carrying it.
///
/// On the fixed clock with the launch chain, so the door starts opening on
/// the intent tick and its timing is deterministic.
pub(super) fn drive_muzzle_doors(
    mut commands: Commands,
    time: Res<Time>,
    mut q_section: Query<
        (
            Entity,
            &mut SectionAnimations,
            &TorpedoSectionInput,
            &ChildOf,
            Option<&mut MuzzleDoorHold>,
            Option<&SectionAmmo>,
        ),
        (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_hot: Query<&WeaponsHot>,
) {
    for (section, mut animations, input, ChildOf(spaceship), hold, ammo) in &mut q_section {
        let safety_on = q_hot.get(*spaceship).is_ok_and(|hot| !hot.0);
        let wants_fire = **input && !safety_on && !ammo.is_some_and(SectionAmmo::is_empty);

        let clearing = match hold {
            Some(mut hold) => {
                hold.remaining -= time.delta_secs();
                let live = hold.remaining > 0.0;
                if !live {
                    commands.entity(section).remove::<MuzzleDoorHold>();
                }
                live
            }
            None => false,
        };

        let target = if wants_fire || clearing { 1.0 } else { 0.0 };
        // The report is the change of TARGET, not of progress: a door 40% open
        // reads the same whether it is opening or closing, and only the target
        // says which. A bay with no authored `MuzzleDoor` track has no iris and
        // reports nothing - there is no door to hear.
        //
        // Once per SALVO rather than once per shot, and for free: the held
        // trigger keeps the target at 1 across the whole burst, so the edge is
        // the burst's beginning and the hold's expiry is its end.
        if let Some(was) = animations.cue_target(SectionAnimationCue::MuzzleDoor) {
            if was != target {
                commands.trigger(TorpedoBayDoorsMoved {
                    entity: section,
                    opening: target > was,
                });
            }
        }
        animations.set_cue(SectionAnimationCue::MuzzleDoor, target);
    }
}

/// A bay's muzzle iris started to move. The seam the audio half hangs the
/// servo on, so the launch path itself stays headless - the same shape the
/// turret housing's [`TurretStowDoorsMoved`] uses.
///
/// [`TurretStowDoorsMoved`]: crate::sections::turret_section::TurretStowDoorsMoved
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TorpedoBayDoorsMoved {
    /// The bay whose iris moved.
    pub entity: Entity,
    /// True when the petals are unseating, false when they are seating.
    pub opening: bool,
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// Advance the manual clock by `dt_ms` and run one frame - for the door
    /// tests, whose apps drive `Time` by hand.
    fn step(app: &mut App, dt_ms: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(dt_ms));
        app.update();
    }

    /// The section's `MuzzleDoor` progress: 0 closed, 1 fully open.
    fn door_progress(app: &mut App, section: Entity) -> f32 {
        app.world_mut()
            .get::<SectionAnimations>(section)
            .unwrap()
            .cue_progress(SectionAnimationCue::MuzzleDoor)
            .unwrap()
    }

    /// The bay's authored door track, fast, for the door tests.
    fn door_track(open_seconds: f32, close_seconds: f32) -> SectionAnimations {
        SectionAnimations::new(vec![SectionAnimation {
            cue: SectionAnimationCue::MuzzleDoor,
            node_prefix: "door_petal_".to_string(),
            motion: SectionAnimationMotion::RotateX { degrees: 105.0 },
            open_seconds,
            close_seconds,
        }])
    }

    /// The launch inserts a hold; the door opens across it, and once the
    /// hold expires the cue closes again and the hold component is gone.
    #[test]
    fn a_launch_hold_opens_the_muzzle_door_and_expiry_closes_it() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_plugins(SectionAnimationPlugin);
        app.add_systems(Update, drive_muzzle_doors);
        let ship = app.world_mut().spawn_empty().id();
        let section = app
            .world_mut()
            .spawn((
                TorpedoSectionMarker,
                // Trigger released: the door stays open on the hold alone.
                TorpedoSectionInput(false),
                ChildOf(ship),
                door_track(0.1, 0.1),
                // ignition_delay 0: the hold is exactly the closing linger.
                MuzzleDoorHold::for_launch(0.0),
            ))
            .id();

        // Warm-up tick (dt 0), then run inside the hold window.
        step(&mut app, 0);
        step(&mut app, 150);
        assert_eq!(
            door_progress(&mut app, section),
            1.0,
            "open during the hold"
        );

        // Run past the remaining hold, then let the close travel finish.
        step(&mut app, 200);
        step(&mut app, 200);
        assert_eq!(door_progress(&mut app, section), 0.0, "closed after expiry");
        assert!(
            app.world_mut().get::<MuzzleDoorHold>(section).is_none(),
            "the hold is removed once it expires"
        );
    }

    /// The held trigger is the door intent: pressing fire opens the iris and
    /// keeps it open with no launch and no hold, releasing closes it - and a
    /// bay whose magazine is empty keeps its doors shut however hard the
    /// trigger is held, so an open iris always telegraphs a live threat.
    #[test]
    fn the_held_trigger_opens_the_muzzle_door_and_an_empty_magazine_refuses() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_plugins(SectionAnimationPlugin);
        app.add_systems(Update, drive_muzzle_doors);
        let ship = app.world_mut().spawn_empty().id();
        let section = app
            .world_mut()
            .spawn((
                TorpedoSectionMarker,
                TorpedoSectionInput(true),
                ChildOf(ship),
                door_track(0.1, 0.1),
            ))
            .id();
        let empty = app
            .world_mut()
            .spawn((
                TorpedoSectionMarker,
                TorpedoSectionInput(true),
                ChildOf(ship),
                door_track(0.1, 0.1),
                SectionAmmo::new(0),
            ))
            .id();

        // Warm-up tick (dt 0), then let the open travel finish.
        step(&mut app, 0);
        step(&mut app, 150);
        assert_eq!(door_progress(&mut app, section), 1.0, "held trigger opens");
        assert_eq!(door_progress(&mut app, empty), 0.0, "empty bay stays shut");

        // Release: no launch ever happened, so nothing holds the door.
        app.world_mut()
            .get_mut::<TorpedoSectionInput>(section)
            .unwrap()
            .0 = false;
        step(&mut app, 150);
        assert_eq!(door_progress(&mut app, section), 0.0, "release closes");
    }

    /// The door gates the ejection: a bay with an authored `MuzzleDoor` track
    /// holds its launch until the iris is FULLY open, then fires - so the
    /// torpedo emerges through an open door instead of materializing on a
    /// closed one. The doorless bays in the other tests are the control: no
    /// track, and the same trigger launches on the first ready tick.
    #[test]
    fn the_muzzle_door_gates_the_launch_until_it_is_fully_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(SectionAnimationPlugin);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(0.05),
        ));
        app.add_systems(
            Update,
            (
                update_spawner_fire_state,
                shoot_spawn_projectile,
                drive_muzzle_doors,
            )
                .chain(),
        );
        let section = spawn_firing_bay(&mut app, None);
        app.world_mut()
            .entity_mut(section)
            .insert(door_track(0.2, 0.2));

        // Warm-up tick (dt 0), then two 0.05 s ticks: the door is mid-travel
        // (0.2 s open), so the ready and triggered bay must not have fired.
        app.update();
        app.update();
        app.update();
        let mid = door_progress(&mut app, section);
        assert!(0.0 < mid && mid < 1.0, "door mid-travel, at {mid}");
        assert_eq!(
            torpedo_count(&mut app),
            0,
            "no launch through a moving door"
        );

        // Let the door finish opening: the pending trigger fires through it.
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(door_progress(&mut app, section), 1.0, "door fully open");
        assert!(torpedo_count(&mut app) >= 1, "the launch went through");
    }

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
                TorpedoSectionPartOf(section),
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

    /// The authored `projectile_health` is what the launched ordnance's two
    /// collider sections actually carry - the armored-torpedo knob is real,
    /// not a config field the bay ignores (both children were hardcoded to
    /// 1.0 before the field existed).
    #[test]
    fn projectile_health_lands_on_the_launched_ordnance() {
        let mut app = firing_app(2.0);
        let section = spawn_firing_bay(&mut app, None);
        app.world_mut()
            .get_mut::<TorpedoSectionConfigHelper>(section)
            .unwrap()
            .0
            .projectile_health = 250.0;

        app.update();
        app.update();
        assert!(torpedo_count(&mut app) >= 1, "the bay launched");

        let healths: Vec<f32> = app
            .world_mut()
            .query_filtered::<&Health, Or<(With<TorpedoControllerMarker>, With<TorpedoThrusterMarker>)>>()
            .iter(app.world())
            .map(|health| health.max)
            .collect();
        assert_eq!(healths.len(), torpedo_count(&mut app) * 2);
        assert!(
            healths.iter().all(|health| *health == 250.0),
            "both collider sections carry the authored durability: {healths:?}"
        );
    }

    /// The disable marker lands on the section, never on the spawner, so the
    /// obvious `Without<SectionInactiveMarker>` on the spawner query excludes
    /// nothing and a disabled bay silently keeps rearming. Both halves are
    /// asserted: the control proves the rig can rearm at all, so the disabled
    /// case is measuring the gate and not a starved clock.
    #[test]
    fn a_disabled_bay_stops_rearming() {
        for disabled in [false, true] {
            let mut app = firing_app(2.0);
            let section = spawn_firing_bay(&mut app, None);
            let spawner = **app
                .world()
                .entity(section)
                .get::<TorpedoSectionSpawnerEntity>()
                .expect("the rig wires the spawner back to its section");

            // Release the trigger: a firing bay re-triggers its cooldown in the
            // same tick that re-arms it, which would read as "never rearmed".
            // With the input closed the tick system is the only writer.
            app.world_mut()
                .entity_mut(section)
                .insert(TorpedoSectionInput(false));
            // `ManualDuration` only takes effect from the SECOND update - the
            // first one establishes the clock's baseline at dt 0, which would
            // starve the tick under test.
            app.update();

            // Spend the cooldown, then disable before it is re-earned.
            app.world_mut()
                .get_mut::<TorpedoSectionSpawnerFireState>(spawner)
                .unwrap()
                .trigger();
            if disabled {
                app.world_mut()
                    .entity_mut(section)
                    .insert(SectionInactiveMarker);
            }

            app.update();

            let ready = app
                .world()
                .entity(spawner)
                .get::<TorpedoSectionSpawnerFireState>()
                .unwrap()
                .ready();
            assert_eq!(
                ready,
                !disabled,
                "a bay with disabled={disabled} must{} rearm",
                if disabled { " not" } else { "" }
            );
        }
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
    fn an_idle_reloading_bay_rearms_and_launches_past_its_magazine() {
        // End-to-end recovery for idle batch reload: a spent
        // 2-torpedo bay regrows rounds over time and launches MORE than its
        // magazine, versus the no-reload rig above which caps at 2 forever.
        let mut app = firing_app(2.0);
        app.add_systems(Update, crate::sections::ammo::tick_section_reload);
        let section = spawn_firing_bay(&mut app, Some(2));
        // Regen one round per ~0.2s cycle (under the 0.25s per-tick clamp).
        app.world_mut()
            .entity_mut(section)
            .insert(SectionReload::from_config(SectionReloadConfig {
                delay: 0.2,
                amount: 1,
            }));

        for _ in 0..12 {
            app.update();
        }

        assert!(
            torpedo_count(&mut app) > 2,
            "an idle-reloading bay must launch past its magazine, got {}",
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
        app.add_plugins(nova_gameplay::integrity::health::NovaHealthPlugin);
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

    /// A shot-down torpedo stops being simulated the moment it is marked. Its
    /// sections carried every collider it had, so a dynamic root in the
    /// removal gap is a massless body avian warns about, once per kill
    /// (task 20260817-091716).
    #[test]
    fn a_shot_down_torpedo_is_on_rails_before_the_reaper_runs() {
        let mut app = App::new();
        app.add_plugins(nova_gameplay::integrity::health::NovaHealthPlugin);
        // Observer only: the reaper is deliberately NOT registered, so this
        // reads the root exactly in the gap the warning was emitted from.
        app.add_observer(on_torpedo_body_destroyed);
        let root = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::default(),
                RigidBody::Dynamic,
            ))
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

        assert_eq!(
            app.world().get::<RigidBody>(root),
            Some(&RigidBody::Static),
            "a torpedo awaiting the reaper must not be a dynamic body"
        );
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
        // The contact fuze reads the clock for its swept window and the broad
        // phase for the target's skin.
        app.init_resource::<Time>();
        app.init_resource::<avian3d::collider_tree::ColliderTrees>();
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

    /// A torpedo has to be pointed the way it is thrown.
    ///
    /// It leaves along the bay's +Y while its own nose is its -Z, so the two
    /// agree only when a bay is authored to make them agree. Seeding the
    /// attitude command from the transform's `forward()` asked for a nose up
    /// to 90 degrees off the way the warhead was travelling, and nothing
    /// showed it: PN guidance overwrote the seed on the first powered tick.
    /// The cold launch has no powered tick to hide behind.
    #[test]
    fn a_launched_torpedo_is_steered_along_the_way_it_was_ejected() {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_plugins(TorpedoSectionPlugin { render: false });
        app.finish();

        // A bay pointed nowhere in particular, so `forward()` and the exit
        // axis cannot coincide by luck.
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Static,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        app.world_mut().spawn((
            TorpedoSectionMarker,
            ChildOf(ship),
            Transform::default(),
            TorpedoSectionConfigHelper(TorpedoSectionConfig {
                spawn_rotation: Quat::from_euler(EulerRot::XYZ, 0.4, -0.9, 0.2),
                spawner_speed: MetersPerSecond(120.0),
                ..default()
            }),
            TorpedoSectionInput(true),
        ));
        settle(&mut app);

        let mut torpedo = None;
        for _ in 0..120 {
            app.update();
            torpedo = app
                .world_mut()
                .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
                .iter(app.world())
                .next();
            if torpedo.is_some() {
                break;
            }
        }
        let torpedo = torpedo.expect("the bay launched");

        let steering = **app
            .world()
            .get::<TorpedoSteering>(torpedo)
            .expect("a launched torpedo is steered");
        // The ship is static, so the torpedo's whole velocity is the ejection
        // charge - the direction it is actually travelling.
        let travel = app
            .world()
            .get::<LinearVelocity>(torpedo)
            .expect("a launched torpedo is moving")
            .0;

        assert!(
            steering.normalize().dot(travel.normalize()) > 0.999,
            "the nose must be held along the exit axis: steering {steering:?}, travel {travel:?}"
        );

        // And it must already BE pointed there, not merely commanded to turn.
        // The torpedo used to be born in the spawner's own frame, which laid
        // its -Z nose across the +Y it was leaving on - a right angle by
        // construction, for every authored `spawn_rotation`. The coast has no
        // thrust to swing it round inside, so the warhead flew visibly
        // broadside for `ignition_delay` seconds.
        let nose = app
            .world()
            .get::<Transform>(torpedo)
            .expect("a launched torpedo has a transform")
            .forward()
            .as_vec3();
        assert!(
            nose.dot(travel.normalize()) > 0.999,
            "the nose must be BORN on the exit axis: nose {nose:?}, travel {travel:?}"
        );
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

        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };
        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_plugins(TorpedoSectionPlugin { render: false });
        app.finish();

        let spawn_rotation = Quat::from_rotation_x(0.3);
        let section_local = Vec3::new(0.0, 1.0, 0.0);
        let spawn_offset = Vec3::new(0.0, 0.0, -1.0);
        let spawner_speed = MetersPerSecond(300.0);
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
                    along > -0.05 && along < 2.0 * spawner_speed.to_engine() * dt + 0.05,
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
                    render_along > -0.05
                        && render_along < 2.0 * spawner_speed.to_engine() * dt + 0.05,
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

    /// How far downrange [`fly`] puts its target. Past the terminal fade band
    /// (three blast radii, 90 u) with room to spare, so most of the flight is
    /// flown at full weave - which is exactly the part of a real engagement
    /// the weave exists for.
    const TARGET_RANGE: f32 = 300.0;

    /// One midcourse run-in, as flown by the real body.
    #[derive(Debug)]
    struct Flight {
        /// Widest lateral offset off the launch-to-target line, over the
        /// midcourse. The VISIBLE amplitude - a weave the player cannot see is
        /// not evasion.
        swing: f32,
        /// Closest the torpedo came to the target.
        closest: f32,
        /// Seconds from the midcourse injection to that closest approach.
        seconds: f32,
        /// Path length flown over those seconds. Against the straight-line
        /// [`TARGET_RANGE`] this is the `1 / cos(weave_angle)` stretch the
        /// corkscrew costs, measured on the real body rather than assumed.
        path: f32,
    }

    impl Flight {
        /// Ground speed along the launch-to-target LINE, in u/s: how fast the
        /// gap to the target actually shuts. This, not cruise speed, is what
        /// decides when a torpedo arrives and how far it can reach.
        fn closing_speed(&self) -> f32 {
            (TARGET_RANGE - self.closest) / self.seconds
        }

        /// How far this type can reach in `lifetime` seconds, in units: the
        /// closing speed above carried out to the bay's authored
        /// `projectile_lifetime`.
        fn reach(&self, lifetime: f32) -> f32 {
            self.closing_speed() * lifetime
        }
    }

    /// One torpedo of `torpedo_type`, injected onto the line at cruise
    /// [`TARGET_RANGE`] from a stationary target, flown on the REAL stack
    /// (avian, the PD attitude controller, the thruster, the whole
    /// `TorpedoSectionPlugin` chain) until it fuzes.
    ///
    /// The pose and velocity are overwritten at pickup to put the torpedo ON
    /// the line at cruise. A bay fires along its +Y while the torpedo's nose is
    /// its -Z, so every real launch starts with the nose across its own
    /// velocity and costs ~10 u of lateral excursion turning onto course - on
    /// every type, monotone, and an order of magnitude wider than the weave
    /// under test. This measures the midcourse, so it starts at the midcourse.
    fn fly(torpedo_type: TorpedoTypeConfig) -> Flight {
        use nova_gameplay::{
            projectile_hooks::ProjectileHooks,
            test_support::{settle, unfinished_integrity_physics_app_with},
        };

        // `unfinished_integrity_physics_app_with` pins a manual clock, so a
        // frame is exactly this long and elapsed time is a frame count.
        const DT: f32 = 1.0 / 60.0;

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_plugins(crate::physics::prelude::PDControllerPlugin);
        app.add_plugins(crate::sections::SpaceshipSectionPlugin { render: false });
        app.finish();

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Static,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                TorpedoSectionMarker,
                ChildOf(ship),
                Transform::default(),
                TorpedoSectionConfigHelper(TorpedoSectionConfig {
                    // Launch straight downrange rather than out of the bay's
                    // side, so the run-in starts on the line the swing is
                    // measured against.
                    spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    spawner_speed: MetersPerSecond(100.0),
                    torpedo_type,
                    ..default()
                }),
                TorpedoSectionInput(true),
            ))
            .id();
        settle(&mut app);

        let target = Vec3::new(0.0, 0.0, -TARGET_RANGE);
        let mut flight = Flight {
            swing: 0.0,
            closest: f32::INFINITY,
            seconds: 0.0,
            path: 0.0,
        };
        // Seconds and path length are banked at the CLOSEST approach, not at
        // the end of the loop: a torpedo that overshoots and turns back would
        // otherwise be credited with the recovery.
        let mut elapsed = 0.0f32;
        let mut path = 0.0f32;
        let mut previous = None;
        let mut torpedo = None;
        for _ in 0..1400 {
            app.update();

            // Take the first torpedo out of the bay and hold the trigger
            // closed, so the measurement follows ONE flight.
            if torpedo.is_none() {
                torpedo = app
                    .world_mut()
                    .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
                    .iter(app.world())
                    .next();
                if let Some(torpedo) = torpedo {
                    // The guidance target: normally written by the player or
                    // AI targeting, supplied directly here so the rig needs no
                    // second ship.
                    app.world_mut().entity_mut(torpedo).insert((
                        TorpedoTargetPosition(target),
                        Position(Vec3::ZERO),
                        Rotation::default(),
                        LinearVelocity(Vec3::NEG_Z * 35.0),
                    ));
                    app.world_mut()
                        .get_mut::<TorpedoSectionInput>(section)
                        .unwrap()
                        .0 = false;
                    continue; // the injected pose is next frame's start
                }
            }
            let Some(id) = torpedo else { continue };
            let Some(position) = app.world().get::<Position>(id).map(|p| p.0) else {
                break; // fuzed or expired
            };
            elapsed += DT;
            if let Some(previous) = previous {
                path += position.distance(previous);
            }
            previous = Some(position);

            let range = position.distance(target);
            if range < flight.closest {
                flight.closest = range;
                flight.seconds = elapsed;
                flight.path = path;
            }
            // The direct line runs down -Z from the origin, so the lateral
            // offset off it is the distance from the Z axis.
            //
            // Measured over the MIDCOURSE only. The near end is the terminal
            // fade band, where the weave is deliberately winding down; the far
            // end cuts the ~50 u the launch spends settling onto the line.
            if (90.0..TARGET_RANGE - 50.0).contains(&range) {
                flight.swing = flight.swing.max(position.xy().length());
            }
        }
        flight
    }

    /// THE weave test that matters, and the one the pure-guidance sims cannot
    /// stand in for.
    ///
    /// The weave is a command; what a player sees is a FLIGHT PATH, and between
    /// the two sit the torpedo's PD attitude controller (its torque limit and
    /// damping), its thrust law and its linear drag. Each of those attenuates
    /// the commanded cone, and the pure-math rigs in `projectile` model none of
    /// them - they scored a weave that the real body flies at a fraction of the
    /// amplitude. This runs the real stack against a stationary target and
    /// measures the path itself.
    ///
    /// Two arms off one rig: the Lance (no weave at all) is the control, so the
    /// swing measured here is the weave and not the guidance settling.
    #[test]
    fn the_weave_puts_a_visible_bend_in_the_real_flight_path() {
        /// Lateral swing off the launch-to-target line that counts as visible,
        /// in units. The torpedo is ~2 u long, so anything under this is inside
        /// its own drive plume and reads as a straight run.
        const VISIBLE_SWING: f32 = 3.0;

        let straight = fly(straight_type());
        let weaving = fly(TorpedoTypeConfig::default());
        println!("real flight: straight {straight:?}, weaving {weaving:?}");

        // The control: with the weave off the torpedo runs straight down the
        // line, so the rig itself contributes no swing worth measuring.
        assert!(
            straight.swing < VISIBLE_SWING,
            "an unweaved torpedo must fly the direct line: swung {:.1} u",
            straight.swing
        );
        assert!(
            weaving.swing > VISIBLE_SWING,
            "the weave must put a VISIBLE bend in the real flight path, not just \
             in the steering command: swung {:.1} u (straight arm: {:.1} u)",
            weaving.swing,
            straight.swing
        );
        // And it still arrives: inside the proximity fuze of a target that
        // never moved, which is the trap the terminal fade exists to avoid.
        //
        // The rig's target carries no section colliders, so the fuze takes its
        // fallback arm - centre distance against half the blast radius, 15 u -
        // and `closest` is the last position SAMPLED before the fuze despawns
        // the torpedo. Landing just outside 15 is therefore the arrival, not a
        // miss: the sample either side of the threshold is one step apart. The
        // straight arm has always read 15.17 here for that reason, and only the
        // weaving arm was ever asserted. It read 14.92 while the fuze ran on the
        // render clock and fired a frame late, deeper in than it should have;
        // now that it is sampled on the clock that moves the torpedo, both arms
        // fuze at the threshold and read just over it.
        //
        // A torpedo that genuinely fails to arrive - the trap - orbits at its
        // turn radius and reads far wider than one step, so this still catches
        // it.
        assert!(
            weaving.closest < 16.0,
            "a weaving torpedo must still reach the fuze of a stationary target, \
             closest was {:.1} u",
            weaving.closest
        );
    }

    /// **Evasion costs time to target and effective reach - because the
    /// evasive type authors a lower cruise cap, NOT because the corkscrew is a
    /// longer path.** Both halves are measured, because the second one is the
    /// finding that made the first one necessary.
    ///
    /// The design originally assumed the weave priced itself: a corkscrew of
    /// half-angle `a` is a longer path by `1 / cos(a)`, so the evasive type
    /// should fly ~11% further, arrive proportionally later, and reach
    /// proportionally less far for the same `projectile_lifetime`. It does not
    /// work, for two reasons this test pins with a third arm:
    ///
    /// 1. The flown path is longer by only ~1.7%, not 11%. The body's drag is
    ///    a first-order lag on the velocity, so the flown helix is far
    ///    shallower than the commanded cone.
    /// 2. [`thrust_headroom`](super::thrust_headroom) gates thrust on the
    ///    ALONG-NOSE speed, not the total. A torpedo holding its nose off its
    ///    own velocity never reaches the taper band, keeps its engine lit, and
    ///    settles at a HIGHER terminal speed against the same drag. Evasion
    ///    runs a hotter engine, and 1.7% of extra path is not a fee it notices.
    ///
    /// Do not "fix" that by capping total speed instead: the doc on
    /// `thrust_headroom` rejects it, because a total-speed cap leaves the
    /// torpedo ballistic at cruise and unable to steer at all. The lever is the
    /// cap itself, lowered on the evasive type, which lowers the band it never
    /// quite reaches. What the real body flies over a 300 u run-in:
    ///
    /// | | straight (35.0) | evasive (32.0) | evasive AT 35.0 |
    /// |---|---|---|---|
    /// | path flown | 284.3 u | 289.2 u | 289.6 u |
    /// | time to fuze | 9.10 s | **9.78 s** | **8.97 s** |
    /// | speed along the line | 31.30 u/s | 29.14 u/s | 31.83 u/s |
    /// | reach at a 100 s lifetime | 3130 u | 2914 u | 3183 u |
    ///
    /// The third column is the control and the whole argument for the second:
    /// give the weave the straight type's cap and it arrives SOONER, longer
    /// path and all.
    #[test]
    fn evasion_costs_time_because_the_type_is_slower_not_because_the_path_is_longer() {
        /// The shipped bay's `projectile_lifetime`, the horizon a reach figure
        /// is quoted against.
        const LIFETIME: f32 = 100.0;

        let straight = fly(straight_type());
        let evasive = fly(TorpedoTypeConfig::default());
        // The same weave at the straight type's cruise cap: the control that
        // isolates the corkscrew from the cap.
        let evasive_uncapped = fly(TorpedoTypeConfig {
            max_speed: straight_type().max_speed,
            ..TorpedoTypeConfig::default()
        });
        println!(
            "the trade over {TARGET_RANGE:.0} u: straight {:.2} s, path {:.1} u, \
             closing {:.2} u/s, reach {:.0} u | evasive {:.2} s, path {:.1} u, \
             closing {:.2} u/s, reach {:.0} u | evasive at the straight cap \
             {:.2} s, path {:.1} u, closing {:.2} u/s",
            straight.seconds,
            straight.path,
            straight.closing_speed(),
            straight.reach(LIFETIME),
            evasive.seconds,
            evasive.path,
            evasive.closing_speed(),
            evasive.reach(LIFETIME),
            evasive_uncapped.seconds,
            evasive_uncapped.path,
            evasive_uncapped.closing_speed(),
        );

        // The control: the straight arm flies the direct line, so its path is
        // the ground it covered. Without this the comparison could be measuring
        // a rig that wanders on both arms.
        let straight_stretch = straight.path / (TARGET_RANGE - straight.closest);
        assert!(
            straight_stretch < 1.02,
            "the straight arm must fly the direct line: {straight_stretch:.3}x the range"
        );

        // THE TRADE. Both halves of it, at the shipped caps.
        assert!(
            evasive.seconds > straight.seconds * 1.03,
            "the evasive type must arrive meaningfully later: {:.2} s vs {:.2} s",
            evasive.seconds,
            straight.seconds
        );
        assert!(
            evasive.reach(LIFETIME) < straight.reach(LIFETIME) * 0.97,
            "and reach meaningfully less far for the same lifetime: {:.0} u vs {:.0} u",
            evasive.reach(LIFETIME),
            straight.reach(LIFETIME)
        );

        // AND WHY IT HAD TO BE AUTHORED. The corkscrew is a longer path, and
        // that costs the torpedo nothing: at the straight type's cap the
        // weaving arm still arrives FIRST. This is the arm to read when someone
        // proposes deleting the evasive type's speed penalty as redundant.
        assert!(
            evasive_uncapped.path > straight.path,
            "the corkscrew must be a LONGER path: {:.1} u weaving vs {:.1} u straight",
            evasive_uncapped.path,
            straight.path
        );
        assert!(
            evasive_uncapped.seconds < straight.seconds,
            "and the longer path must still not be a cost: at the straight type's \
             cap the weave arrives in {:.2} s against {:.2} s. If this ever flips, \
             the thrust law changed and the authored speed penalty can be \
             revisited",
            evasive_uncapped.seconds,
            straight.seconds
        );
    }

    /// The straight-running type, as `sections::ordnance::lance` authors it.
    /// Named here rather than imported: nova_ship owns the mechanism and the
    /// default, and the catalog that names the two shipped types is content.
    ///
    /// `max_speed` is spelled out rather than inherited from the default. The
    /// default IS the evasive type, which now authors a LOWER cap as the price
    /// of its weave, so `..default()` would quietly slow the control arm to
    /// match and hide the very difference these rigs measure.
    fn straight_type() -> TorpedoTypeConfig {
        TorpedoTypeConfig {
            name: "Straight".to_string(),
            max_speed: MetersPerSecond(350.0),
            weave_angle: 0.0,
            weave_rate: 0.0,
            ..default()
        }
    }

    /// Every `TorpedoBayDoorsMoved` a rig has seen, newest last.
    #[derive(Resource, Default)]
    struct Reported(Vec<bool>);

    /// A bay on a ship, its trigger under the test's control, watched for
    /// door reports. `track` is `None` for a bay with no iris at all.
    fn reporting_bay_app(track: Option<SectionAnimations>) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.init_resource::<Reported>();
        app.add_plugins(SectionAnimationPlugin);
        app.add_systems(Update, drive_muzzle_doors);
        app.add_observer(
            |moved: On<TorpedoBayDoorsMoved>, mut seen: ResMut<Reported>| {
                seen.0.push(moved.opening);
            },
        );
        let ship = app.world_mut().spawn_empty().id();
        let mut section = app.world_mut().spawn((
            TorpedoSectionMarker,
            TorpedoSectionInput(false),
            ChildOf(ship),
        ));
        if let Some(track) = track {
            section.insert(track);
        }
        let section = section.id();
        (app, section)
    }

    /// Hold or release the bay's trigger, then run one frame.
    fn trigger(app: &mut App, section: Entity, held: bool) {
        app.world_mut()
            .entity_mut(section)
            .insert(TorpedoSectionInput(held));
        step(app, 16);
    }

    /// The report is the EDGE of the door's target, which is what the audio
    /// half needs and what a progress read cannot give it: one cue per salvo,
    /// with the direction attached, and nothing at all while the petals are
    /// mid-travel.
    #[test]
    fn the_iris_reports_once_per_salvo_with_the_direction_it_is_travelling() {
        let (mut app, section) = reporting_bay_app(Some(door_track(0.1, 0.1)));
        step(&mut app, 0);
        assert!(
            app.world().resource::<Reported>().0.is_empty(),
            "a resting bay says nothing"
        );

        trigger(&mut app, section, true);
        assert_eq!(app.world().resource::<Reported>().0, vec![true]);

        // Still held, petals still travelling: no second cue.
        step(&mut app, 16);
        step(&mut app, 16);
        assert_eq!(
            app.world().resource::<Reported>().0,
            vec![true],
            "a held trigger is one salvo, not one cue per frame"
        );

        trigger(&mut app, section, false);
        assert_eq!(
            app.world().resource::<Reported>().0,
            vec![true, false],
            "the release is the salvo's end, reported closing"
        );
    }

    /// A bay with no authored `MuzzleDoor` track has no iris. It launches
    /// immediately and must report nothing, or the audio half would voice a
    /// door the cut-cube pods do not have.
    #[test]
    fn a_doorless_bay_never_reports() {
        let (mut app, section) = reporting_bay_app(None);
        step(&mut app, 0);
        trigger(&mut app, section, true);
        trigger(&mut app, section, false);
        assert!(app.world().resource::<Reported>().0.is_empty());
    }
}
