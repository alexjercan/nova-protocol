use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_hanabi::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        torpedo_section, TorpedoArming, TorpedoControllerMarker, TorpedoGuidance,
        TorpedoProjectileMarker, TorpedoProjectileOwner, TorpedoSectionConfig, TorpedoSectionInput,
        TorpedoSectionMarker, TorpedoSectionPlugin, TorpedoSectionSpawnerFireState,
        TorpedoSectionSpawnerMarker, TorpedoSteering, TorpedoTargetEntity, TorpedoTargetPosition,
    };
}

#[derive(Clone, Debug, Reflect)]
pub struct TorpedoSectionConfig {
    pub render_mesh: Option<Handle<WorldAsset>>,
    pub projectile_render_mesh: Option<Handle<WorldAsset>>,
    /// The offset of the spawn point of the projectile relative to the torpedo section.
    pub spawn_offset: Vec3,
    /// The rotation of the spawn point of the projectile relative to the torpedo section.
    pub spawn_rotation: Quat,
    /// The fire rate of the turret in rounds per second.
    pub fire_rate: f32,
    /// The muzzle speed of the turret in units per second.
    pub spawner_speed: f32,
    /// The lifetime of the projectile in seconds.
    pub projectile_lifetime: f32,
    /// Arming delay: minimum seconds after firing before the torpedo may
    /// detonate. Prevents a torpedo fired at a nearby target from blowing up on
    /// (or right after) spawn. Armed when this OR `arm_distance` is reached.
    pub arm_time: f32,
    /// Arming distance: minimum distance from the muzzle the torpedo must travel
    /// before it may detonate, so it clears the firing ship first. Armed when
    /// this OR `arm_time` is reached.
    pub arm_distance: f32,
    /// Proportional-navigation constant (`N`). Higher values turn harder to null
    /// the line-of-sight rate, so the torpedo leads a moving target more
    /// aggressively. Typical PN values are 3-5.
    pub nav_constant: f32,
    /// Cruise speed cap in units per second. The thruster tapers off as the
    /// torpedo approaches this speed. Without a cap the torpedo accelerates the
    /// whole flight and arrives so fast that its minimum turning circle
    /// (speed / turn rate) is larger than the proximity fuze - it then orbits the
    /// target instead of hitting it. Keep `max_speed / turn rate` comfortably
    /// under the blast trigger radius.
    pub max_speed: f32,
    /// Linear damping (drag) on the torpedo body. The thrust cap alone gates only
    /// the along-nose speed, so repeated turns against a moving target "pump"
    /// total speed up sideways; drag gives a real terminal velocity regardless of
    /// thrust direction and relaxes the velocity toward wherever the nose points,
    /// so the flight path follows the guidance command.
    pub linear_damping: f32,
    /// The explosion effect to play when the torpedo detonates.
    pub blast_effect: Option<Handle<EffectAsset>>,
}

impl Default for TorpedoSectionConfig {
    fn default() -> Self {
        Self {
            render_mesh: None,
            projectile_render_mesh: None,
            spawn_offset: Vec3::Y * 2.0,
            spawn_rotation: Quat::IDENTITY,
            fire_rate: 1.0,
            spawner_speed: 1.0,
            projectile_lifetime: 100.0,
            arm_time: 0.5,
            arm_distance: 5.0,
            nav_constant: 3.0,
            max_speed: 35.0,
            linear_damping: 0.8,
            blast_effect: None,
        }
    }
}

pub fn torpedo_section(config: TorpedoSectionConfig) -> impl Bundle {
    debug!("torpedo_section: config {:?}", config);

    (
        TorpedoSectionMarker,
        TorpedoSectionConfigHelper(config),
        TorpedoSectionInput(false),
    )
}

/// Input to request the turret to shoot a projectile.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoSectionInput(pub bool);

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionSpawnerMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionBodyMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoProjectileMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoControllerMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoThrusterMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoBlastEffectMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionConfigHelper(TorpedoSectionConfig);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoSectionSpawnerFireState(pub Timer);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionPartOf(Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionSpawnerEntity(Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoProjectileOwner(pub Entity);

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetEntity(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoProjectileRenderMesh(Option<Handle<WorldAsset>>);

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetPosition(pub Vec3);

/// Guidance/propulsion tuning carried by a torpedo projectile (copied from its
/// `TorpedoSectionConfig` at spawn), so each bay's torpedoes can be tuned.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoGuidance {
    pub nav_constant: f32,
    pub max_speed: f32,
}

/// The unit direction the torpedo currently wants its nose pointed, produced by
/// `torpedo_pn_guidance` and consumed by the sync (orientation) and thrust
/// systems. Kept as one source of truth so both read the same command.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoSteering(pub Vec3);

/// Arming state of a torpedo projectile. A torpedo cannot detonate until it is
/// armed; it arms once it has either lived for `min_time` seconds or traveled
/// `min_distance` from its `origin` (the muzzle). This stops a torpedo fired at
/// a nearby target from self-detonating on spawn. Once armed it stays armed.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoArming {
    min_time: f32,
    min_distance: f32,
    origin: Vec3,
    elapsed: f32,
    armed: bool,
}

impl TorpedoArming {
    /// Create arming state for a torpedo spawned at `origin`.
    pub fn new(min_time: f32, min_distance: f32, origin: Vec3) -> Self {
        Self {
            min_time,
            min_distance,
            origin,
            elapsed: 0.0,
            armed: false,
        }
    }

    /// Whether the torpedo is armed and allowed to detonate.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Advance the arming state by `dt` seconds given the torpedo's current
    /// position, latching `armed` once the time or distance threshold is met.
    /// Returns the (possibly updated) armed state.
    fn tick(&mut self, dt: f32, position: Vec3) -> bool {
        if self.armed {
            return true;
        }
        self.elapsed += dt;
        let traveled = position.distance(self.origin);
        if self.elapsed >= self.min_time || traveled >= self.min_distance {
            self.armed = true;
        }
        self.armed
    }
}

#[derive(Default)]
pub struct TorpedoSectionPlugin {
    pub render: bool,
}

impl Plugin for TorpedoSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("TorpedoSectionPlugin: build");

        app.add_observer(insert_torpedo_section);

        if self.render {
            app.add_observer(insert_torpedo_section_render);

            app.add_observer(insert_torpedo_render);
            app.add_observer(insert_torpedo_controller_render);

            // FIXME(20260706-162908): For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(insert_particle_effect);
        }

        app.add_systems(
            Update,
            (
                update_spawner_fire_state,
                shoot_spawn_projectile,
                (
                    update_target_position,
                    update_torpedo_arming,
                    torpedo_detonate_system,
                    torpedo_pn_guidance,
                    torpedo_sync_system,
                    torpedo_thrust_system,
                )
                    .chain(),
            )
                .in_set(super::SpaceshipSectionSystems),
        );
    }
}

fn insert_torpedo_section(
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
    let mut timer = Timer::from_seconds(interval, TimerMode::Once);
    timer.finish();

    let spawner = commands
        .spawn((
            Name::new("Torpedo Section Spawner"),
            TorpedoSectionSpawnerMarker,
            TorpedoSectionPartOf(entity),
            TorpedoSectionSpawnerFireState(timer),
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
}

fn update_spawner_fire_state(
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
        fire_state.tick(time.delta());
    }
}

fn shoot_spawn_projectile(
    mut commands: Commands,
    q_spaceship: Query<
        (&LinearVelocity, &AngularVelocity, &ComputedCenterOfMass),
        With<SpaceshipRootMarker>,
    >,
    q_section: Query<
        (
            Entity,
            &TorpedoSectionSpawnerEntity,
            &ChildOf,
            &TorpedoSectionConfigHelper,
            &TorpedoSectionInput,
        ),
        (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_spawner: Query<&mut TorpedoSectionSpawnerFireState, With<TorpedoSectionSpawnerMarker>>,
    // We are using TransformHelper here because we need to compute the global transform; And it
    // should be fine, since it will not be called frequently.
    transform_helper: TransformHelper,
) {
    for (section, spawner, ChildOf(spaceship), config, input) in &q_section {
        if !**input {
            continue;
        }

        let Ok((lin_vel, ang_vel, center)) = q_spaceship.get(*spaceship) else {
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

        if !fire_state.is_finished() {
            continue;
        }

        let Ok(spawner_transform) = transform_helper.compute_global_transform(**spawner) else {
            error!(
                "shoot_spawn_projectile: entity {:?} global transform not found",
                **spawner
            );
            continue;
        };

        let spawner_direction = spawner_transform.up();
        let projectile_position = spawner_transform.translation();
        let projectile_rotation = spawner_transform.rotation();
        let radius_vector = projectile_position - **center;
        let _inertia_vel = ang_vel.cross(radius_vector) + **lin_vel;
        // FIXME(20260706-162909): Currently we are only using the linear velocity as inertia
        let inertia_vel = **lin_vel;

        let spawner_exit_velocity = spawner_direction * config.spawner_speed;
        let linear_velocity = spawner_exit_velocity + inertia_vel;

        let projectile_transform = Transform {
            translation: projectile_position + spawner_exit_velocity * 0.01,
            rotation: projectile_rotation,
            ..default()
        };

        commands.spawn((
            Name::new("Torpedo Projectile"),
            TorpedoProjectileMarker,
            TorpedoProjectileOwner(*spaceship),
            projectile_transform,
            RigidBody::Dynamic,
            LinearVelocity(linear_velocity),
            TorpedoSectionPartOf(section),
            TorpedoSectionSpawnerEntity(**spawner),
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
                    base_section(BaseSectionConfig {
                        id: "torpedo_controller".to_string(),
                        name: "Torpedo Controller".to_string(),
                        description: "The controller for the torpedo warhead".to_string(),
                        mass: 1.0,
                        health: 1.0,
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
                    }),
                ),
                (
                    TorpedoThrusterMarker,
                    base_section(BaseSectionConfig {
                        id: "torpedo_thruster".to_string(),
                        name: "Torpedo Thruster".to_string(),
                        description: "The thruster for the torpedo".to_string(),
                        mass: 1.0,
                        health: 1.0,
                    }),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                    ThrusterSectionRenderMarker,
                    thruster_section(ThrusterSectionConfig {
                        magnitude: 1.0,
                        render_mesh: None,
                    }),
                    children![(
                        Name::new("Thruster Exhaust"),
                        Transform::from_rotation(Quat::from_rotation_x(
                            std::f32::consts::FRAC_PI_2
                        ))
                        .with_translation(Vec3::new(0.0, 0.0, -0.45)),
                        ThrusterExhaustConfig {
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

        // Reset the fire state timer
        fire_state.reset();
    }
}

fn update_target_position(
    mut commands: Commands,
    mut q_torpedo: Query<
        (Entity, Option<&mut TorpedoTargetPosition>, &TorpedoTargetEntity),
        With<TorpedoProjectileMarker>,
    >,
    q_target: Query<&Transform>,
) {
    for (torpedo, torpedo_target_position, target_entity) in &mut q_torpedo {
        let Ok(target_transform) = q_target.get(**target_entity) else {
            // The target died mid-flight. Don't delete the torpedo - that reads as
            // it blinking out of existence. Instead drop the dead target link and
            // let it keep flying toward the last known position (frozen in
            // `TorpedoTargetPosition`) until it arrives and detonates or its
            // lifetime expires. Removing the link also stops this lookup - and its
            // warning - from repeating every frame.
            debug!(
                "update_target_position: target {:?} gone; freezing torpedo {:?} on last known position",
                **target_entity, torpedo
            );
            commands.entity(torpedo).remove::<TorpedoTargetEntity>();
            continue;
        };

        // The position component is added on first lock and updated in place after,
        // so a never-locked torpedo has no `TorpedoTargetPosition` and flies straight.
        match torpedo_target_position {
            Some(mut position) => **position = target_transform.translation,
            None => {
                commands
                    .entity(torpedo)
                    .insert(TorpedoTargetPosition(target_transform.translation));
            }
        }
    }
}

/// Tick each torpedo's arming state so it can detonate only after it has cleared
/// the muzzle (see [`TorpedoArming`]).
fn update_torpedo_arming(
    time: Res<Time>,
    mut q_torpedo: Query<(&Transform, &mut TorpedoArming), With<TorpedoProjectileMarker>>,
) {
    let dt = time.delta_secs();
    for (torpedo_transform, mut arming) in &mut q_torpedo {
        arming.tick(dt, torpedo_transform.translation);
    }
}

// TODO(20260706-162913): Unhardcode blast parameters
const BLAST_RADIUS: f32 = 30.0;
const BLAST_DAMAGE: f32 = 100.0;

// TODO(20260525-133023): Add some nice visuals for the explosion itself
fn torpedo_detonate_system(
    mut commands: Commands,
    q_torpedo: Query<
        (
            Entity,
            &Transform,
            &TorpedoTargetPosition,
            &TorpedoArming,
            &TorpedoSectionPartOf,
        ),
        With<TorpedoProjectileMarker>,
    >,
) {
    for (torpedo, torpedo_transform, torpedo_target_position, arming, part_of) in &q_torpedo {
        // Do not detonate until the torpedo has armed (cleared the muzzle), so a
        // shot at a nearby target does not blow up on spawn.
        if !arming.is_armed() {
            continue;
        }

        let distance = torpedo_transform
            .translation
            .distance(**torpedo_target_position);

        if distance < BLAST_RADIUS * 0.5 {
            commands.entity(torpedo).despawn();
            commands.spawn((
                blast_damage(BlastDamageConfig {
                    radius: BLAST_RADIUS,
                    max_damage: BLAST_DAMAGE,
                }),
                Transform::from_translation(torpedo_transform.translation),
                part_of.clone(),
                TempEntity(0.1),
            ));
        }
    }
}

/// Proportional-navigation steering direction.
///
/// Returns the unit direction the torpedo should point its nose (and thrust)
/// toward to intercept the target. `rel_pos` is the line-of-sight `target - torpedo`
/// and `target_vel` / `missile_vel` are world-space velocities.
///
/// The command is anchored on the line of sight, not on the torpedo's velocity:
///
/// - Base course: the constant-bearing intercept ("lead collision course"). Split
///   the target's velocity into the component across the line of sight and match
///   it, spending the rest of the torpedo's speed closing along the line of sight:
///   `lead = (target_perp + los * sqrt(speed^2 - |target_perp|^2)) / speed`. For a
///   stationary target this is exactly "point at the target"; for a crossing
///   target it is the exact intercept heading at the given speed.
/// - PN damping: the classic LOS-rate term. With `omega = cross(rel_pos, rel_vel)
///   / dot(rel_pos, rel_pos)`, add `nav_constant * cross(omega, heading)` (clamped)
///   to null residual line-of-sight rotation - drift, disturbances, target
///   maneuvers.
///
/// Anchoring on the LOS matters because the torpedo launches slowly *sideways* out
/// of the bay: a velocity-anchored command (`V + N * cross(omega, V)`) from that
/// state keeps re-commanding the current drift direction (omega is tiny while the
/// target is far), so the torpedo climbs away instead of turning onto the target.
/// The LOS-anchored form points at/ahead of the target from any initial velocity.
fn pn_steer_direction(
    rel_pos: Vec3,
    target_vel: Vec3,
    missile_vel: Vec3,
    nav_constant: f32,
) -> Vec3 {
    let heading = missile_vel.try_normalize();

    let Some(los) = rel_pos.try_normalize() else {
        // Target coincident with the torpedo: keep the current heading.
        return heading.unwrap_or(Vec3::NEG_Z);
    };

    // Constant-bearing lead. Plan with at least the target's speed so the lead
    // stays defined while the torpedo is still accelerating up to speed.
    let target_perp = target_vel - target_vel.dot(los) * los;
    let planning_speed = missile_vel
        .length()
        .max(target_vel.length())
        .max(1e-3);
    let closing = (planning_speed * planning_speed - target_perp.length_squared())
        .max(0.0)
        .sqrt();
    let lead = (target_perp + los * closing) / planning_speed;

    // PN damping: null the residual line-of-sight rotation.
    let pn_correction = match heading {
        Some(heading) => {
            let los_rate = rel_pos.cross(target_vel - missile_vel) / rel_pos.length_squared();
            (nav_constant * los_rate.cross(heading)).clamp_length_max(1.0)
        }
        None => Vec3::ZERO,
    };

    (lead + pn_correction).try_normalize().unwrap_or(los)
}

/// Compute each torpedo's PN steering direction into [`TorpedoSteering`], using the
/// target entity's velocity (zero once the target is lost, so PN degrades to
/// pursuit of the frozen target position).
fn torpedo_pn_guidance(
    mut q_torpedo: Query<
        (
            &Transform,
            Option<&TorpedoTargetPosition>,
            &LinearVelocity,
            Option<&TorpedoTargetEntity>,
            &TorpedoGuidance,
            &mut TorpedoSteering,
        ),
        With<TorpedoProjectileMarker>,
    >,
    q_target_velocity: Query<&LinearVelocity>,
) {
    for (transform, target_position, velocity, target_entity, guidance, mut steering) in
        &mut q_torpedo
    {
        // No target locked (or ever locked): fly straight ahead, holding heading,
        // rather than steering toward the world origin.
        let Some(target_position) = target_position else {
            **steering = transform.forward().into();
            continue;
        };

        let target_velocity = target_entity
            .and_then(|target| q_target_velocity.get(**target).ok())
            .map(|v| **v)
            .unwrap_or(Vec3::ZERO);

        let rel_pos = **target_position - transform.translation;

        **steering =
            pn_steer_direction(rel_pos, target_velocity, **velocity, guidance.nav_constant);
    }
}

/// Orient the torpedo's PD controller toward the PN steering direction.
fn torpedo_sync_system(
    q_torpedo: Query<&TorpedoSteering, With<TorpedoProjectileMarker>>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        (With<ControllerSectionMarker>, With<TorpedoControllerMarker>),
    >,
) {
    for (mut controller_input, ChildOf(torpedo)) in &mut q_controller {
        if let Ok(steering) = q_torpedo.get(*torpedo) {
            **controller_input = Quat::from_rotation_arc(Vec3::NEG_Z, **steering);
        }
    }
}

/// Width of the taper band below `max_speed` over which thrust fades to zero, in
/// units per second.
const THRUST_TAPER_BAND: f32 = 5.0;

/// Thrust remaining given the velocity component *along the nose*: 1.0 well
/// below `max_speed`, fading linearly to 0.0 over the last
/// [`THRUST_TAPER_BAND`] u/s. Gating on the along-nose speed (not total speed)
/// caps cruise speed without killing steering: at cruise, pointing straight
/// ahead means no thrust, but the moment guidance swings the nose to turn, the
/// along-nose component drops and thrust returns as lateral authority. A cap on
/// total speed instead leaves the torpedo ballistic at cruise - unable to steer
/// at all. Never negative: the cap cuts thrust, it does not brake.
fn thrust_headroom(speed_along_nose: f32, max_speed: f32) -> f32 {
    ((max_speed - speed_along_nose) / THRUST_TAPER_BAND).clamp(0.0, 1.0)
}

/// Thrust along the nose: full thrust when the nose is aligned with the steering
/// direction, easing off while the torpedo is still turning onto course, and
/// tapering to zero when already at cruise speed along the nose (see
/// [`thrust_headroom`] and [`TorpedoSectionConfig::max_speed`]).
fn torpedo_thrust_system(
    q_torpedo: Query<
        (&Transform, &TorpedoSteering, &LinearVelocity, &TorpedoGuidance),
        With<TorpedoProjectileMarker>,
    >,
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &ChildOf),
        (With<ThrusterSectionMarker>, With<TorpedoThrusterMarker>),
    >,
) {
    for (mut thruster_input, ChildOf(torpedo)) in &mut q_thruster {
        if let Ok((transform, steering, velocity, guidance)) = q_torpedo.get(*torpedo) {
            let nose = transform.forward();
            let alignment = nose.dot(**steering).clamp(0.0, 1.0);
            let headroom = thrust_headroom(velocity.dot(nose.into()), guidance.max_speed);
            **thruster_input = alignment * headroom;
        }
    }
}

fn insert_torpedo_section_render(
    add: On<Add, TorpedoSectionBodyMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_section: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
    q_body: Query<&TorpedoSectionPartOf, With<TorpedoSectionBodyMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_section_render: entity {:?}", entity);

    let Ok(part_of) = q_body.get(entity) else {
        error!(
            "insert_torpedo_section_render: entity {:?} not found in q_body",
            entity
        );
        return;
    };

    let Ok(config) = q_section.get(**part_of) else {
        error!(
            "insert_torpedo_section_render: entity {:?} not found in q_section",
            entity
        );
        return;
    };
    let render_mesh = &config.render_mesh;

    match render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Torpedo Section Body"),
                SectionRenderOf(entity),
                WorldAssetRoot(scene.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Torpedo Section Body"),
                SectionRenderOf(entity),
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
            ),],));
        }
    }
}

fn insert_torpedo_render(
    add: On<Add, TorpedoProjectileMarker>,
    mut commands: Commands,
    q_projectile: Query<&TorpedoProjectileRenderMesh, With<TorpedoProjectileMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_render: entity {:?}", entity);

    let Ok(render_mesh) = q_projectile.get(entity) else {
        error!(
            "insert_torpedo_render: entity {:?} not found in q_projectile",
            entity
        );
        return;
    };

    if let Some(scene) = &**render_mesh {
        commands.entity(entity).insert((children![(
            Name::new("Torpedo Projectile Body"),
            SectionRenderOf(entity),
            WorldAssetRoot(scene.clone()),
        ),],));
    }
}

fn insert_torpedo_controller_render(
    add: On<Add, TorpedoControllerMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_controller: Query<&ChildOf, With<TorpedoControllerMarker>>,
    q_torpedo: Query<&TorpedoProjectileRenderMesh, With<TorpedoProjectileMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_controller_render: entity {:?}", entity);

    let Ok(ChildOf(torpedo)) = q_controller.get(entity) else {
        error!(
            "insert_torpedo_controller_render: entity {:?} not found in q_controller",
            entity
        );
        return;
    };

    let Ok(render_mesh) = q_torpedo.get(*torpedo) else {
        error!(
            "insert_torpedo_controller_render: entity {:?} not found in q_torpedo",
            *torpedo
        );
        return;
    };

    if render_mesh.is_some() {
        // If the torpedo has a render mesh, we skip rendering the controller
        return;
    }

    commands.entity(entity).insert((
        Mesh3d(meshes.add(Cylinder::new(0.2, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
    ));
}

fn insert_particle_effect(
    add: On<Add, BlastDamageMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    q_blast: Query<(&Transform, &TorpedoSectionPartOf), With<BlastDamageMarker>>,
    q_config: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_particle_effect: entity {:?}", entity);

    let Ok((blast_transform, TorpedoSectionPartOf(torpedo_section))) = q_blast.get(entity) else {
        error!(
            "insert_particle_effect: entity {:?} not found in q_blast",
            entity
        );
        return;
    };

    let Ok(config) = q_config.get(*torpedo_section) else {
        error!(
            "insert_turret_barrel_muzzle_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    let effect = match &config.blast_effect {
        Some(effect) => effect.clone(),
        None => {
            let spawner = SpawnerSettings::once(400.0.into())
                // In this case we want to emit on start to create an instantaneous explosion
                .with_emit_on_start(true);

            let writer = ExprWriter::new();

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // Lifetime: explosion should be fast but noticeable
            let lifetime = writer.lit(0.25).uniform(writer.lit(1.5)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Color over lifetime
            let mut color_gradient = bevy_hanabi::Gradient::new();
            // t=0: bright yellow/white
            color_gradient.add_key(0.0, Vec4::new(1.0, 0.95, 0.7, 1.0));
            // mid: hot orange
            color_gradient.add_key(0.3, Vec4::new(1.0, 0.6, 0.1, 0.7));
            // end: dark, almost transparent smoke
            color_gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.1, 0.0));

            let color_over_lifetime = ColorOverLifetimeModifier {
                gradient: color_gradient,
                blend: ColorBlendMode::default(),
                mask: ColorBlendMask::default(),
            };

            let init_color =
                SetAttributeModifier::new(Attribute::COLOR, writer.lit(0xFFFFFFFFu32).expr());

            // Size over lifetime: fast expansion then shrink/fade
            let mut size_gradient = bevy_hanabi::Gradient::new();
            size_gradient.add_key(0.0, Vec3::splat(0.02)); // just spawned
            size_gradient.add_key(0.1, Vec3::splat(0.2)); // big boom
            size_gradient.add_key(0.5, Vec3::splat(0.25)); // lingering cloud
            size_gradient.add_key(1.0, Vec3::splat(0.0)); // disappear

            let size_over_lifetime = SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            };

            // Position: explosion center
            let init_pos =
                SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

            // Velocity: spherical random burst
            let rand_x = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
            let rand_y = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
            let rand_z = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);

            let dir = writer.lit(Vec3::X) * rand_x
                + writer.lit(Vec3::Y) * rand_y
                + writer.lit(Vec3::Z) * rand_z;

            let speed = writer.lit(20.0).uniform(writer.lit(30.0));
            let velocity = dir * speed;
            let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

            effects.add(
                EffectAsset::new(32768, spawner, writer.finish())
                    .with_name("spawn_on_blast_explosion")
                    .init(init_pos)
                    .init(init_vel)
                    .init(init_age)
                    .init(init_lifetime)
                    .init(init_color)
                    .render(size_over_lifetime)
                    .render(color_over_lifetime),
            )
        }
    };

    commands.spawn(((
        Name::new("Blast Effect"),
        TorpedoBlastEffectMarker,
        Transform::from_translation(blast_transform.translation),
        ParticleEffect::new(effect),
        EffectProperties::default(),
        TempEntity(2.0),
    ),));
}

// TODO(20260706-162913): Factor out the torpedo logic into a separate module.
// TODO(20260706-162913): Implement a separate plugin for the targeting system.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torpedo_is_unarmed_on_spawn() {
        // A freshly spawned torpedo (no time elapsed, no distance travelled) must
        // not be armed, so it cannot detonate on the muzzle.
        let arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        assert!(!arming.is_armed());
    }

    #[test]
    fn torpedo_arms_after_min_time_even_without_moving() {
        // Point-blank shot: the target sits on the muzzle so the torpedo never
        // travels far, but the time threshold must still arm it eventually.
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        assert!(!arming.tick(0.4, Vec3::ZERO)); // below min_time, still at origin
        assert!(arming.tick(0.2, Vec3::ZERO)); // 0.6s total >= min_time
        assert!(arming.is_armed());
    }

    #[test]
    fn torpedo_arms_after_min_distance_before_min_time() {
        // A fast torpedo clears the muzzle before the time threshold; distance
        // arms it first.
        let mut arming = TorpedoArming::new(10.0, 5.0, Vec3::ZERO);
        assert!(!arming.tick(0.1, Vec3::new(4.0, 0.0, 0.0))); // under both
        assert!(arming.tick(0.1, Vec3::new(6.0, 0.0, 0.0))); // travelled >= 5.0
        assert!(arming.is_armed());
    }

    #[test]
    fn torpedo_stays_armed_once_armed() {
        // Arming latches: coming back inside the arm distance does not disarm it.
        let mut arming = TorpedoArming::new(10.0, 5.0, Vec3::ZERO);
        assert!(arming.tick(0.0, Vec3::new(6.0, 0.0, 0.0))); // armed via distance
        assert!(arming.tick(0.0, Vec3::ZERO)); // back at origin, still armed
        assert!(arming.is_armed());
    }

    #[test]
    fn unarmed_torpedo_does_not_detonate_on_target() {
        // Regression: a torpedo sitting right on its target must not detonate
        // while unarmed - this is the "spawns too close and just dies" bug.
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoTargetPosition(Vec3::ZERO), // on target: distance 0 < BLAST_RADIUS * 0.5
                TorpedoArming::new(0.5, 5.0, Vec3::ZERO), // not armed
                TorpedoSectionPartOf(part_of),
            ))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "unarmed torpedo should survive even on top of its target"
        );
    }

    #[test]
    fn armed_torpedo_detonates_on_target() {
        // Once armed, the same on-target torpedo detonates (despawns).
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO); // arm via time

        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::ZERO),
                TorpedoTargetPosition(Vec3::ZERO),
                arming,
                TorpedoSectionPartOf(part_of),
            ))
            .id();

        app.update();

        assert!(
            !app.world().entities().contains(torpedo),
            "armed torpedo on its target should detonate and despawn"
        );
    }

    #[test]
    fn torpedo_survives_target_loss_and_freezes_position() {
        // Regression: when the target dies mid-flight the torpedo must not vanish.
        // It should keep its last known target position and drop the dead link.
        let mut app = App::new();
        app.add_systems(Update, update_target_position);

        let target = app
            .world_mut()
            .spawn(Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetPosition(Vec3::ZERO),
                TorpedoTargetEntity(target),
            ))
            .id();

        // Frame 1: target alive -> the torpedo tracks it.
        app.update();
        assert_eq!(
            **app.world().get::<TorpedoTargetPosition>(torpedo).unwrap(),
            Vec3::new(1.0, 2.0, 3.0)
        );

        // Target dies mid-flight.
        app.world_mut().entity_mut(target).despawn();

        // Frame 2: torpedo must survive, freeze on the last known position, and
        // drop the dead target link (so it stops looking it up every frame).
        app.update();
        assert!(
            app.world().entities().contains(torpedo),
            "torpedo must not vanish when its target dies"
        );
        assert_eq!(
            **app.world().get::<TorpedoTargetPosition>(torpedo).unwrap(),
            Vec3::new(1.0, 2.0, 3.0),
            "torpedo should freeze on the last known target position"
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "the dead target link should be removed"
        );
    }

    #[test]
    fn pn_leads_a_crossing_target() {
        // Torpedo at origin flying forward (-Z); target ahead, crossing to +X.
        // PN must steer the nose to lead the target (a +X component), not point
        // straight down -Z at where the target is now.
        let missile_vel = Vec3::new(0.0, 0.0, -50.0);
        let rel_pos = Vec3::new(0.0, 0.0, -100.0); // target 100 ahead
        let target_vel = Vec3::new(20.0, 0.0, 0.0); // crossing to +X

        let dir = pn_steer_direction(rel_pos, target_vel, missile_vel, 3.0);

        assert!(
            dir.x > 0.01,
            "PN should lead a +X-crossing target with a +X heading component, got {dir:?}"
        );
        assert!(dir.z < 0.0, "torpedo should still be heading generally forward");
        assert!(dir.is_normalized(), "steering direction must be a unit vector");
    }

    #[test]
    fn pn_pursues_a_stationary_target_straight() {
        // Target directly ahead, not moving, torpedo closing straight in: there is
        // no line-of-sight rotation, so PN adds no lead - it points at the target.
        let missile_vel = Vec3::new(0.0, 0.0, -50.0);
        let rel_pos = Vec3::new(0.0, 0.0, -100.0);

        let dir = pn_steer_direction(rel_pos, Vec3::ZERO, missile_vel, 3.0);

        assert!((dir - Vec3::NEG_Z).length() < 1e-3, "expected straight pursuit, got {dir:?}");
    }

    #[test]
    fn pn_points_at_a_stationary_target_from_a_sideways_launch() {
        // THE regression for "the torpedo flies off and never turns toward the
        // target": the torpedo leaves the bay slowly and sideways (spawner up,
        // ~1 u/s), i.e. velocity perpendicular to the line of sight. The command
        // must point (essentially) at the target, not along the current velocity.
        // The old velocity-anchored form returned ~(0, 1, 0) here.
        let missile_vel = Vec3::new(0.0, 1.0, 0.0); // slow, straight up
        let rel_pos = Vec3::new(0.0, 0.0, -100.0); // target ahead

        let dir = pn_steer_direction(rel_pos, Vec3::ZERO, missile_vel, 3.0);

        assert!(
            dir.dot(Vec3::NEG_Z) > 0.95,
            "command must point at the target regardless of launch velocity, got {dir:?}"
        );
    }

    #[test]
    fn pn_handles_degenerate_inputs() {
        // Target on top of the torpedo, and a stationary torpedo: both must return
        // a finite unit direction, never NaN.
        let coincident = pn_steer_direction(Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), 3.0);
        assert!(coincident.is_finite() && coincident.is_normalized());

        let stationary = pn_steer_direction(Vec3::new(0.0, 0.0, -50.0), Vec3::ZERO, Vec3::ZERO, 3.0);
        assert!(stationary.is_finite() && stationary.is_normalized());
        assert!(
            (stationary - Vec3::NEG_Z).length() < 1e-3,
            "a stationary torpedo should pursue the target directly"
        );
    }

    /// Closed-loop model of the torpedo the way it actually flies: the nose turns
    /// toward `steer(...)` at up to `max_turn_rate` rad/s, and thrust accelerates
    /// along the nose scaled by nose/command alignment and by the cruise-speed
    /// headroom (mirroring `torpedo_thrust_system`). Starting conditions mirror
    /// the real launch: slow, sideways. Returns the closest approach to the
    /// target over the run.
    #[allow(clippy::too_many_arguments)]
    fn simulate_thrust_intercept(
        mut pos: Vec3,
        mut vel: Vec3,
        mut nose: Vec3,
        mut target: Vec3,
        target_vel: Vec3,
        max_turn_rate: f32,
        accel: f32,
        max_speed: f32,
        damping: f32,
        dt: f32,
        steps: usize,
        steer: impl Fn(Vec3, Vec3, Vec3) -> Vec3,
    ) -> f32 {
        let mut closest = pos.distance(target);
        for _ in 0..steps {
            let desired = steer(target - pos, target_vel, vel);
            let angle = nose.angle_between(desired);
            let axis = nose.cross(desired);
            if axis.length() > 1e-6 && angle > 1e-6 {
                let step = (max_turn_rate * dt).min(angle);
                nose = (Quat::from_axis_angle(axis.normalize(), step) * nose).normalize();
            }
            let thrust =
                nose.dot(desired).clamp(0.0, 1.0) * thrust_headroom(vel.dot(nose), max_speed);
            vel += nose * accel * thrust * dt;
            vel -= vel * damping * dt; // linear drag, as on the real body
            pos += vel * dt;
            target += target_vel * dt;
            closest = closest.min(pos.distance(target));
        }
        closest
    }

    /// The real launch state in the examples: at rest but drifting up at ~1 u/s
    /// (spawner up), nose forward (-Z), then guided by the PN law with the
    /// torpedo's rough turn rate, thrust authority, and cruise-speed cap.
    fn launch_closest_approach(target: Vec3, target_vel: Vec3) -> f32 {
        simulate_thrust_intercept(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0), // launched sideways at 1 u/s
            Vec3::NEG_Z,              // nose forward
            target,
            target_vel,
            3.0,  // max turn rate rad/s
            25.0, // thrust acceleration
            35.0, // cruise speed cap
            0.8,  // linear damping, as configured on the projectile
            0.02, // dt
            500,  // 10 s
            |r, tv, v| pn_steer_direction(r, tv, v, 3.0),
        )
    }

    #[test]
    fn thrust_tapers_to_zero_at_cruise_speed() {
        // Below the taper band: full thrust. At/above cruise: none. The cap keeps
        // the turning circle (speed / turn rate) inside the proximity fuze so the
        // torpedo cannot end up orbiting its target at high speed.
        assert_eq!(thrust_headroom(0.0, 35.0), 1.0);
        assert_eq!(thrust_headroom(20.0, 35.0), 1.0);
        assert!((thrust_headroom(32.5, 35.0) - 0.5).abs() < 1e-6);
        assert_eq!(thrust_headroom(35.0, 35.0), 0.0);
        assert_eq!(thrust_headroom(50.0, 35.0), 0.0, "cap cuts thrust, never brakes");
    }

    #[test]
    fn pn_turns_a_sideways_launch_onto_a_stationary_target() {
        // Closed-loop version of the reported bug: from the real launch state the
        // torpedo must come around and hit a stationary target ahead, instead of
        // thrusting off along its launch drift.
        let miss = launch_closest_approach(Vec3::new(0.0, 0.0, -60.0), Vec3::ZERO);
        assert!(miss < 5.0, "torpedo should reach the stationary target, closest was {miss}");
    }

    /// A closest approach that counts as a kill: inside the proximity fuze
    /// (`BLAST_RADIUS * 0.5` = 15). Crossing intercepts from a sideways launch
    /// carry a few units of turn-rate lag at the endgame (measured ~8), which the
    /// fuze absorbs; a broken law misses by the full crossing distance instead.
    const HIT: f32 = 10.0;

    #[test]
    fn pn_intercepts_a_crossing_target() {
        // From the real launch state, intercept a target crossing the range.
        let miss = launch_closest_approach(
            Vec3::new(-30.0, 0.0, -80.0),
            Vec3::new(15.0, 0.0, 0.0),
        );
        assert!(miss < HIT, "PN should intercept the crossing target, closest approach was {miss}");
    }

    #[test]
    fn pn_intercepts_a_target_crossing_either_way() {
        // Guards against a sign bug that only works for one crossing direction.
        for cross in [15.0f32, -15.0] {
            let miss = launch_closest_approach(
                Vec3::new(-2.0 * cross, 0.0, -80.0),
                Vec3::new(cross, 0.0, 0.0),
            );
            assert!(miss < HIT, "PN should intercept a target crossing at {cross}, miss was {miss}");
        }
    }

    #[test]
    fn untargeted_torpedo_flies_straight_not_toward_origin() {
        // Regression: a torpedo fired with no lock (no TorpedoTargetPosition) must
        // hold its heading, not steer at the world origin. Place it off-origin so
        // "straight ahead" (-Z) is clearly distinct from "toward origin" (-X).
        let mut app = App::new();
        app.add_systems(Update, torpedo_pn_guidance);

        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)), // forward is -Z
                LinearVelocity(Vec3::new(0.0, 0.0, -40.0)),
                TorpedoGuidance {
                    nav_constant: 3.0,
                    max_speed: 35.0,
                },
                TorpedoSteering(Vec3::NEG_Z),
            ))
            .id();

        app.update();

        let steering = **app.world().get::<TorpedoSteering>(torpedo).unwrap();
        assert!(
            (steering - Vec3::NEG_Z).length() < 1e-3,
            "untargeted torpedo should fly straight ahead (-Z), got {steering:?}"
        );
    }
}
