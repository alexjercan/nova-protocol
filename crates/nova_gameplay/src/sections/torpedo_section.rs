use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        torpedo_section, TorpedoControllerMarker, TorpedoProjectileMarker, TorpedoProjectileOwner,
        TorpedoSectionConfig, TorpedoSectionInput, TorpedoSectionMarker, TorpedoSectionPlugin,
        TorpedoSectionSpawnerFireState, TorpedoSectionSpawnerMarker, TorpedoTargetEntity,
        TorpedoTargetPosition,
    };
}

#[derive(Clone, Debug, Reflect)]
pub struct TorpedoSectionConfig {
    pub render_mesh: Option<Handle<Scene>>,
    pub projectile_render_mesh: Option<Handle<Scene>>,
    /// The offset of the spawn point of the projectile relative to the torpedo section.
    pub spawn_offset: Vec3,
    /// The fire rate of the turret in rounds per second.
    pub fire_rate: f32,
    /// The muzzle speed of the turret in units per second.
    pub spawner_speed: f32,
    /// The lifetime of the projectile in seconds.
    pub projectile_lifetime: f32,
}

impl Default for TorpedoSectionConfig {
    fn default() -> Self {
        Self {
            render_mesh: None,
            projectile_render_mesh: None,
            spawn_offset: Vec3::Y * 2.0,
            fire_rate: 1.0,
            spawner_speed: 1.0,
            projectile_lifetime: 100.0,
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
pub struct TorpedoProjectileMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoControllerMarker;

#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoThrusterMarker;

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
struct TorpedoProjectileRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TorpedoTargetPosition(pub Vec3);

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
        }

        app.add_systems(
            Update,
            (
                update_spawner_fire_state,
                shoot_spawn_projectile,
                (
                    update_target_position,
                    torpedo_detonate_system,
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
            Transform::from_translation(config.spawn_offset),
            Visibility::Inherited,
        ))
        .id();

    commands
        .entity(entity)
        .insert(TorpedoSectionSpawnerEntity(spawner))
        .add_child(spawner);
}

fn update_spawner_fire_state(
    mut q_spawner: Query<&mut TorpedoSectionSpawnerFireState, With<TorpedoSectionSpawnerMarker>>,
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
        With<TorpedoSectionMarker>,
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
        // FIXME: Currently we are only using the linear velocity as inertia
        let inertia_vel = **lin_vel;

        let spawner_exit_velocity = spawner_direction * config.spawner_speed;
        let linear_velocity = spawner_exit_velocity + inertia_vel;

        let projectile_transform = Transform {
            translation: projectile_position + spawner_exit_velocity * 0.01,
            rotation: projectile_rotation,
            ..default()
        };

        commands
            .spawn((
                Name::new("Torpedo Projectile"),
                TorpedoProjectileMarker,
                TorpedoProjectileOwner(*spaceship),
                projectile_transform,
                RigidBody::Dynamic,
                LinearVelocity(linear_velocity),
                TorpedoSectionPartOf(section),
                TorpedoSectionSpawnerEntity(**spawner),
                TorpedoProjectileRenderMesh(config.projectile_render_mesh.clone()),
                TorpedoTargetPosition(Vec3::new(0.0, 0.0, 0.0)),
                // TODO: Adjust health as needed
                Health::new(1.0),
                CollisionImpactMarker,
                ExplodableEntity,
                Visibility::Visible,
                children![
                    (
                        TorpedoControllerMarker,
                        base_section(BaseSectionConfig {
                            id: "torpedo_controller".to_string(),
                            name: "Torpedo Controller".to_string(),
                            description: "The controller for the torpedo warhead".to_string(),
                            mass: 1.0,
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
                                exhaust_max: 1.0,
                                exhaust_radius: 0.15,
                                emissive_color: LinearRgba::new(10.0, 5.0, 0.0, 1.0),
                            },
                        )],
                    )
                ],
            ))
            .insert((
                TempEntity(config.projectile_lifetime),
            ));

        // Reset the fire state timer
        fire_state.reset();
    }
}

fn update_target_position(
    mut commands: Commands,
    mut q_torpedo: Query<
        (Entity, &mut TorpedoTargetPosition, &TorpedoTargetEntity),
        With<TorpedoProjectileMarker>,
    >,
    q_target: Query<&Transform>,
) {
    for (torpedo, mut torpedo_target_position, target_entity) in &mut q_torpedo {
        let Ok(target_transform) = q_target.get(**target_entity) else {
            debug!(
                "update_target_position: target entity {:?} not found in q_target",
                **target_entity
            );
            commands.entity(torpedo).despawn();
            continue;
        };

        **torpedo_target_position = target_transform.translation;
    }
}

// TODO: Unhardcode blast parameters
const BLAST_RADIUS: f32 = 30.0;
const BLAST_DAMAGE: f32 = 100.0;

// TODO: Add some nice visuals for the explosion itself
fn torpedo_detonate_system(
    mut commands: Commands,
    q_torpedo: Query<(Entity, &Transform, &TorpedoTargetPosition), With<TorpedoProjectileMarker>>,
) {
    for (torpedo, torpedo_transform, torpedo_target_position) in &q_torpedo {
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
                TempEntity(0.1),
            ));
        }
    }
}

fn torpedo_sync_system(
    q_torpedo: Query<
        (&Transform, &TorpedoTargetPosition, &LinearVelocity),
        With<TorpedoProjectileMarker>,
    >,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        (With<ControllerSectionMarker>, With<TorpedoControllerMarker>),
    >,
) {
    for (mut controller_input, ChildOf(torpedo)) in &mut q_controller {
        if let Ok((torpedo_transform, torpedo_target_position, linear_velocity)) =
            q_torpedo.get(*torpedo)
        {
            let to_target = (**torpedo_target_position - torpedo_transform.translation).normalize();
            let forward = torpedo_transform.forward();

            let velocity = **linear_velocity;
            let sideways = velocity - forward * velocity.dot(forward.into());
            let drift_correction = -sideways * 0.05;

            let desired_dir = (to_target + drift_correction).normalize();
            let new_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, desired_dir);

            **controller_input = new_rotation;
        }
    }
}

fn torpedo_thrust_system(
    q_torpedo: Query<
        (&Transform, &TorpedoTargetPosition, &LinearVelocity),
        With<TorpedoProjectileMarker>,
    >,
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &ChildOf),
        (With<ThrusterSectionMarker>, With<TorpedoThrusterMarker>),
    >,
) {
    for (mut thruster_input, ChildOf(torpedo)) in &mut q_thruster {
        if let Ok((torpedo_transform, torpedo_target_position, linear_velocity)) =
            q_torpedo.get(*torpedo)
        {
            let to_target = (**torpedo_target_position - torpedo_transform.translation).normalize();
            let forward = torpedo_transform.forward();

            let alignment = forward.dot(to_target).clamp(0.0, 1.0);

            let velocity = **linear_velocity;
            let sideways = velocity - forward * velocity.dot(forward.into());
            let drift_correction = -sideways.length() * 0.1;

            let steering = (alignment + drift_correction).clamp(0.0, 1.0);
            **thruster_input = steering;
        }
    }
}

fn insert_torpedo_section_render(
    add: On<Add, TorpedoSectionMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_section: Query<&TorpedoSectionConfigHelper, With<TorpedoSectionMarker>>,
) {
    let entity = add.entity;
    trace!("insert_torpedo_section_render: entity {:?}", entity);

    let Ok(config) = q_section.get(entity) else {
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
                SceneRoot(scene.clone()),
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
            SceneRoot(scene.clone()),
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

// TODO: Factor out the torpedo logic into a separate module.
// TODO: Implement a separate plugin for the targeting system.
