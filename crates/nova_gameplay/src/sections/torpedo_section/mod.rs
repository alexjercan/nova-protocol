use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_hanabi::prelude::*;

use crate::prelude::*;

/// In-flight torpedo behavior: target tracking, arming, detonation, and PN
/// guidance (steer / thrust). The bay/launcher stays in this module; the systems
/// here act on the spawned projectiles.
mod projectile;
/// Render/particle systems for the bay and the projectile (gated by the plugin's
/// `render` flag).
mod render;

use projectile::*;
use render::*;

pub mod prelude {
    pub use super::{
        torpedo_section, TorpedoArming, TorpedoBlast, TorpedoControllerMarker, TorpedoGuidance,
        TorpedoProjectileMarker, TorpedoSectionConfig, TorpedoSectionConfigHelper,
        TorpedoSectionInput, TorpedoSectionMarker, TorpedoSectionPartOf, TorpedoSectionPlugin,
        TorpedoSectionSpawnerFireState, TorpedoSectionSpawnerMarker, TorpedoSteering,
        TorpedoTargetChosen, TorpedoTargetEntity, TorpedoTargetPosition,
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
    /// Blast radius on detonation, in units. The proximity fuze fires when the
    /// torpedo is within half this radius of the target, and blast damage falls off
    /// linearly to zero at this radius.
    pub blast_radius: f32,
    /// Peak blast damage at the detonation centre, falling off to zero at
    /// `blast_radius`.
    pub blast_damage: f32,
    /// The explosion effect to play when the torpedo detonates.
    pub blast_effect: Option<Handle<EffectAsset>>,
    /// The launch particle burst played at the bay spawner each time a torpedo is
    /// fired. Mirrors the turret's `muzzle_effect`; when `None`, a default
    /// spawn-on-command burst is built in `insert_torpedo_spawner_effect`. A
    /// custom effect must be spawn-on-command and declare the `normal` and
    /// `base_velocity` `Vec3` properties, which `on_torpedo_launch_effect` sets
    /// per shot (unknown properties are ignored by hanabi).
    pub launch_effect: Option<Handle<EffectAsset>>,
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
            blast_radius: 30.0,
            blast_damage: 100.0,
            blast_effect: None,
            launch_effect: None,
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

/// The bay's full config, kept on the section entity. Pub (read-only via
/// `Deref`) so the AI input side can derive its launch envelope from the
/// same numbers the bay actually fires with (blast radius), like it reads
/// the turret's config helper for the fire-range gate.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct TorpedoSectionConfigHelper(TorpedoSectionConfig);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TorpedoSectionSpawnerFireState(pub Timer);

/// Back-pointer from a bay's spawned entities (spawner, body, projectile,
/// blast) to the torpedo SECTION they belong to. Pub so the AI input side
/// can attribute a fresh projectile to the bay that launched it and reset
/// that bay's launch cooldown.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct TorpedoSectionPartOf(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionSpawnerEntity(Entity);

/// Holds the configured launch-effect handle on the spawner entity so
/// `insert_torpedo_spawner_effect` can read it when the spawner is added. `None`
/// means "build the default burst".
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TorpedoSectionSpawnerEffect(Option<Handle<EffectAsset>>);

/// Marks the child `ParticleEffect` entity of the spawner, so the launch trigger
/// (`on_torpedo_launch_effect`) can find its `EffectSpawner` and `reset()` it.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TorpedoSectionSpawnerEffectMarker;

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetEntity(pub Entity);

/// The torpedo's launch-time targeting decision has been made. Inserted by the
/// input targeting system (player crosshair today, spaceship AI later) the first
/// time it processes a torpedo - together with a [`TorpedoTargetEntity`] when a
/// lock exists, or alone for a dumb-fire shot. Once present, no targeting system
/// assigns this torpedo a (new) target: a torpedo keeps its first target for
/// life (freezing on the last known position if it dies), and a dumb-fired one
/// never acquires anything mid-flight - e.g. bullets fired past it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetChosen;

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

/// Blast parameters carried by a torpedo projectile (copied from its
/// `TorpedoSectionConfig` at spawn): the proximity-fuze / damage `radius` and the
/// peak `damage` at the detonation centre.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoBlast {
    pub radius: f32,
    pub damage: f32,
}

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

            // Expanding-sphere blast-radius visual: a plain mesh + material, so unlike
            // the hanabi particle burst below it also renders on wasm.
            app.add_observer(insert_blast_radius_visual);
            app.add_systems(Update, animate_blast_radius_visual);

            // FIXME(20260706-162908): For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(insert_particle_effect);

            // Launch burst at the bay: build the effect on the spawner, fire it
            // whenever a torpedo projectile is spawned. Hanabi-only, wasm-gated
            // like the blast burst above.
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(insert_torpedo_spawner_effect);
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(on_torpedo_launch_effect);
        }

        // A torpedo whose body is shot dead must die as a whole: without
        // this the collider-less root keeps flying, armed, and still
        // detonates (user report 20260710, task 20260710-003734).
        app.register_type::<TorpedoShotDownMarker>();
        app.add_observer(on_torpedo_body_destroyed);

        app.add_systems(
            Update,
            (
                update_spawner_fire_state,
                shoot_spawn_projectile,
                (
                    despawn_shot_down_torpedoes,
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

/// A torpedo whose body was shot dead, awaiting removal.
///
/// The kill is deliberately TWO-STEP: the observer that detects the dead
/// body section only inserts this marker (inserting on a live entity is
/// always safe), and [`despawn_shot_down_torpedoes`] removes the torpedo on
/// the next schedule pass. Despawning directly inside the observer raced
/// the integrity pipeline: its already-queued commands for the dying
/// section (e.g. `IntegrityDisabledMarker`) then hit a despawned entity and
/// panicked mid collision-event flush (live-game crash, 20260710).
/// [`torpedo_detonate_system`] excludes marked roots, so the warhead cannot
/// fire in the gap.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoShotDownMarker;

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
fn on_torpedo_body_destroyed(
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
fn despawn_shot_down_torpedoes(
    q_torpedo: Query<Entity, (With<TorpedoProjectileMarker>, With<TorpedoShotDownMarker>)>,
    mut commands: Commands,
) {
    for torpedo in &q_torpedo {
        commands.entity(torpedo).try_despawn();
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
            TorpedoSectionSpawnerEffect(config.launch_effect.clone()),
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
        (
            &LinearVelocity,
            &AngularVelocity,
            &ComputedCenterOfMass,
            Option<&Allegiance>,
        ),
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

        let Ok((lin_vel, ang_vel, center, allegiance)) = q_spaceship.get(*spaceship) else {
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

        // Inherit the full motion of the muzzle, not just the ship's linear velocity: a muzzle
        // offset from the center of mass of a rotating ship also swings tangentially. avian's
        // `ComputedCenterOfMass` is body-local, so lift it to world space with the ship's
        // global transform before taking the point velocity.
        let Ok(ship_transform) = transform_helper.compute_global_transform(*spaceship) else {
            error!(
                "shoot_spawn_projectile: entity {:?} global transform not found",
                spaceship
            );
            continue;
        };
        let center_of_mass = ship_transform.transform_point(**center);
        let inertia_vel =
            rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, projectile_position);

        let spawner_exit_velocity = spawner_direction * config.spawner_speed;
        let linear_velocity = spawner_exit_velocity + inertia_vel;

        let projectile_transform = Transform {
            translation: projectile_position + spawner_exit_velocity * 0.01,
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
            TransformInterpolation,
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
                    ActiveCollisionHooks::FILTER_PAIRS,
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
        // The torpedo COPIES the shooter's allegiance instead of resolving
        // through ProjectileOwner at read time: it stays attributable even if
        // the owner dies mid-flight, and consumers stay single-query.
        if let Some(&allegiance) = allegiance {
            projectile.insert(allegiance);
        }

        // Reset the fire state timer
        fire_state.reset();
    }
}

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
                TorpedoTargetPosition(Vec3::ZERO), // on target: distance 0 < blast radius * 0.5
                TorpedoArming::new(0.5, 5.0, Vec3::ZERO), // not armed
                TorpedoBlast {
                    radius: 30.0,
                    damage: 100.0,
                },
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
                TorpedoBlast {
                    radius: 30.0,
                    damage: 100.0,
                },
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
    fn the_detonation_blast_inherits_the_torpedo_owner() {
        // The blast entity must stay attributable to the ship that fired
        // the torpedo: bcs puts the blast collider into
        // HealthApplyDamage.source, and the AI threat model resolves it to
        // a shooter through ProjectileOwner (task 20260709-225731).
        let mut app = App::new();
        app.add_systems(Update, torpedo_detonate_system);

        let owner = app.world_mut().spawn_empty().id();
        let part_of = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO);
        arming.tick(1.0, Vec3::ZERO); // arm via time

        app.world_mut().spawn((
            TorpedoProjectileMarker,
            ProjectileOwner(owner),
            Transform::from_translation(Vec3::ZERO),
            TorpedoTargetPosition(Vec3::ZERO),
            arming,
            TorpedoBlast {
                radius: 30.0,
                damage: 100.0,
            },
            TorpedoSectionPartOf(part_of),
        ));

        app.update();

        let mut q_blast = app
            .world_mut()
            .query_filtered::<&ProjectileOwner, With<BlastDamageMarker>>();
        let blast_owner = q_blast
            .single(app.world())
            .expect("the detonation spawned exactly one owned blast");
        assert_eq!(**blast_owner, owner);
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
        assert!(
            dir.z < 0.0,
            "torpedo should still be heading generally forward"
        );
        assert!(
            dir.is_normalized(),
            "steering direction must be a unit vector"
        );
    }

    #[test]
    fn pn_pursues_a_stationary_target_straight() {
        // Target directly ahead, not moving, torpedo closing straight in: there is
        // no line-of-sight rotation, so PN adds no lead - it points at the target.
        let missile_vel = Vec3::new(0.0, 0.0, -50.0);
        let rel_pos = Vec3::new(0.0, 0.0, -100.0);

        let dir = pn_steer_direction(rel_pos, Vec3::ZERO, missile_vel, 3.0);

        assert!(
            (dir - Vec3::NEG_Z).length() < 1e-3,
            "expected straight pursuit, got {dir:?}"
        );
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
        let coincident =
            pn_steer_direction(Vec3::ZERO, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), 3.0);
        assert!(coincident.is_finite() && coincident.is_normalized());

        let stationary =
            pn_steer_direction(Vec3::new(0.0, 0.0, -50.0), Vec3::ZERO, Vec3::ZERO, 3.0);
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
        assert_eq!(
            thrust_headroom(50.0, 35.0),
            0.0,
            "cap cuts thrust, never brakes"
        );
    }

    #[test]
    fn pn_turns_a_sideways_launch_onto_a_stationary_target() {
        // Closed-loop version of the reported bug: from the real launch state the
        // torpedo must come around and hit a stationary target ahead, instead of
        // thrusting off along its launch drift.
        let miss = launch_closest_approach(Vec3::new(0.0, 0.0, -60.0), Vec3::ZERO);
        assert!(
            miss < 5.0,
            "torpedo should reach the stationary target, closest was {miss}"
        );
    }

    /// A closest approach that counts as a kill: inside the proximity fuze
    /// (`BLAST_RADIUS * 0.5` = 15). Crossing intercepts from a sideways launch
    /// carry a few units of turn-rate lag at the endgame (measured ~8), which the
    /// fuze absorbs; a broken law misses by the full crossing distance instead.
    const HIT: f32 = 10.0;

    #[test]
    fn pn_intercepts_a_crossing_target() {
        // From the real launch state, intercept a target crossing the range.
        let miss = launch_closest_approach(Vec3::new(-30.0, 0.0, -80.0), Vec3::new(15.0, 0.0, 0.0));
        assert!(
            miss < HIT,
            "PN should intercept the crossing target, closest approach was {miss}"
        );
    }

    #[test]
    fn pn_intercepts_a_target_crossing_either_way() {
        // Guards against a sign bug that only works for one crossing direction.
        for cross in [15.0f32, -15.0] {
            let miss = launch_closest_approach(
                Vec3::new(-2.0 * cross, 0.0, -80.0),
                Vec3::new(cross, 0.0, 0.0),
            );
            assert!(
                miss < HIT,
                "PN should intercept a target crossing at {cross}, miss was {miss}"
            );
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

    // -- shoot-down kills (task 20260710-003734) --

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
            .query_filtered::<Entity, With<BlastDamageMarker>>()
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
            .query_filtered::<Entity, With<BlastDamageMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(blasts, 0);
    }

    // -- torpedo allegiance (task 20260708-203708) --

    #[test]
    fn launched_torpedo_copies_the_shooter_allegiance() {
        // Same rule as turret bullets: the torpedo reads as the shooter's
        // side (relation model), copied at spawn so "your own torpedo" stays
        // yours even if the shooter dies mid-flight.
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        let spawner = world
            .spawn((TorpedoSectionSpawnerMarker, Transform::default(), {
                // Pre-expired so the very first run fires.
                let mut timer = Timer::from_seconds(0.1, TimerMode::Once);
                timer.tick(std::time::Duration::from_secs(1));
                TorpedoSectionSpawnerFireState(timer)
            }))
            .id();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Player,
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        world.spawn((
            TorpedoSectionMarker,
            ChildOf(ship),
            TorpedoSectionSpawnerEntity(spawner),
            TorpedoSectionConfigHelper(TorpedoSectionConfig::default()),
            TorpedoSectionInput(true),
        ));

        world.run_system_once(shoot_spawn_projectile).unwrap();

        let allegiance = world
            .query_filtered::<Option<&Allegiance>, With<TorpedoProjectileMarker>>()
            .iter(&world)
            .next()
            .expect("a torpedo spawned");
        assert_eq!(allegiance, Some(&Allegiance::Player));
    }
}
