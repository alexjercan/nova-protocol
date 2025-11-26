//! A turret section is a component that can be added to an entity to give it a turret-like
//! behavior.

use avian3d::prelude::*;
use bevy::{
    ecs::system::{lifetimeless::Read, SystemParam},
    prelude::*,
};
use bevy_common_systems::prelude::*;
use bevy_hanabi::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        turret_section, TurretBulletProjectileMarker, TurretProjectileHooks,
        TurretSectionBarrelMuzzleMarker, TurretSectionConfig, TurretSectionInput,
        TurretSectionMarker, TurretSectionMuzzleEntity, TurretSectionPlugin,
        TurretSectionTargetInput,
    };
}

/// Configuration for a turret section of a spaceship.
#[derive(Clone, Debug, Reflect)]
pub struct TurretSectionConfig {
    /// The yaw speed of the turret section in radians per second.
    pub yaw_speed: f32,
    /// The pitch speed of the turret section in radians per second.
    pub pitch_speed: f32,
    /// The minimum pitch angle of the turret section in radians. If None, there is no limit.
    pub min_pitch: Option<f32>,
    /// The maximum pitch angle of the turret section in radians. If None, there is no limit.
    pub max_pitch: Option<f32>,
    /// The render mesh of the base, defaults to a cylinder base
    pub render_mesh_base: Option<Handle<Scene>>,
    /// The offset of the base from the section origin
    pub base_offset: Vec3,
    /// The render mesh of the yaw rotator, defaults to a cylinder with ridges
    pub render_mesh_yaw: Option<Handle<Scene>>,
    /// The offset of the yaw rotator from the base
    pub yaw_offset: Vec3,
    /// The render mesh of the pitch rotator, defaults to a cylinder with ridges
    pub render_mesh_pitch: Option<Handle<Scene>>,
    /// The offset of the pitch rotator from the yaw rotator
    pub pitch_offset: Vec3,
    /// The render mesh of the barrel, defaults to a simple barrel shape
    pub render_mesh_barrel: Option<Handle<Scene>>,
    /// The offset of the barrel from the pitch rotator
    pub barrel_offset: Vec3,
    /// The offset of the muzzle from the barrel
    pub muzzle_offset: Vec3,
    /// The fire rate of the turret in rounds per second.
    pub fire_rate: f32,
    /// The muzzle speed of the turret in units per second.
    pub muzzle_speed: f32,
    /// The projectile lifetime
    pub projectile_lifetime: f32,
    /// The projectile mass
    pub projectile_mass: f32,
    /// The projectile mesh,
    pub projectile_render_mesh: Option<Handle<Scene>>,
    /// The muzzle particle effect when shooting.
    pub muzzle_effect: Option<Handle<EffectAsset>>,
}

impl Default for TurretSectionConfig {
    fn default() -> Self {
        Self {
            yaw_speed: std::f32::consts::PI,   // 180 degrees per second
            pitch_speed: std::f32::consts::PI, // 180 degrees per second
            min_pitch: Some(-std::f32::consts::FRAC_PI_6),
            max_pitch: Some(std::f32::consts::FRAC_PI_2),
            render_mesh_base: None,
            base_offset: Vec3::new(0.0, -0.5, 0.0),
            render_mesh_yaw: None,
            yaw_offset: Vec3::new(0.0, 0.1, 0.0),
            render_mesh_pitch: None,
            pitch_offset: Vec3::new(0.0, 0.2, 0.0),
            render_mesh_barrel: None,
            barrel_offset: Vec3::new(0.1, 0.2, 0.0),
            muzzle_offset: Vec3::new(0.0, 0.0, -0.5),
            fire_rate: 100.0,
            muzzle_speed: 100.0,
            projectile_lifetime: 5.0,
            projectile_mass: 0.1,
            projectile_render_mesh: None,
            muzzle_effect: None,
        }
    }
}

/// Helper function to create a turret section entity bundle.
pub fn turret_section(config: TurretSectionConfig) -> impl Bundle {
    debug!("turret_section: config {:?}", config);

    (
        TurretSectionMarker,
        TurretSectionTargetInput(None),
        TurretSectionConfigHelper(config),
        TurretSectionInput(false),
    )
}

/// Input to request the turret to shoot a projectile.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionInput(pub bool);

/// Marker component for turret sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TurretSectionMarker;

/// The muzzle marker of the turret section.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct TurretSectionBarrelMuzzleMarker;

/// Marker for turret bullet projectiles.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TurretBulletProjectileMarker;

/// The target input for the turret section. This is a world-space position that the turret will
/// aim at. If None, the turret will not rotate.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionTargetInput(pub Option<Vec3>);

/// The Turret "parent" entity of the turret component.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionPartOf(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct BulletProjectileRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBaseRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionYawRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionPitchRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBarrelRenderMesh(Option<Handle<Scene>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionConfigHelper(TurretSectionConfig);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionBarrelFireState(pub Timer);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBarrelMuzzleEffect(Option<Handle<EffectAsset>>);

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretRotatorBaseMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionRotatorYawBaseMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionRotatorYawMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionRotatorPitchBaseMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionRotatorPitchMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionRotatorBarrelMarker;

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionBarrelMuzzleEffectMarker;

/// The entity that represents the muzzle of the turret.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzleEntity(pub Entity);

/// The spaceship entity that owns the projectile.
#[derive(Component, Clone, Debug, Reflect)]
struct TurretBulletProjectileOwner(pub Entity);

/// A plugin that enables the TurretSection component and its related systems.
#[derive(Default)]
pub struct TurretSectionPlugin {
    pub render: bool,
}

impl Plugin for TurretSectionPlugin {
    fn build(&self, app: &mut App) {
        debug!("TurretSectionPlugin: build");

        app.add_observer(insert_turret_section);

        if self.render {
            app.add_observer(insert_turret_section_render);
            app.add_observer(insert_turret_yaw_rotator_render);
            app.add_observer(insert_turret_pitch_rotator_render);
            app.add_observer(insert_turret_barrel_render);
            app.add_observer(insert_projectile_render);

            // FIXME: For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(insert_turret_barrel_muzzle_effect);

            // FIXME: For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(on_projectile_marker_effect);
        }

        app.add_systems(
            Update,
            (
                update_barrel_fire_state,
                sync_turret_rotator_yaw_system,
                sync_turret_rotator_pitch_system,
                shoot_spawn_projectile,
            )
                .in_set(super::SpaceshipSectionSystems),
        );

        app.add_systems(
            PostUpdate,
            (
                update_turret_target_yaw_system,
                update_turret_target_pitch_system,
            )
                .after(TransformSystems::Propagate)
                .in_set(super::SpaceshipSectionSystems),
        );
    }
}

// Define a custom `SystemParam` for our collision hooks.
// It can have read-only access to queries, resources, and other system parameters.
#[derive(SystemParam)]
pub struct TurretProjectileHooks<'w, 's> {
    projectile_query: Query<'w, 's, (Read<TurretBulletProjectileOwner>,)>,
    colider_of_query: Query<'w, 's, (Read<ColliderOf>,)>,
}

impl CollisionHooks for TurretProjectileHooks<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _commands: &mut Commands) -> bool {
        // Don't allow collision between a projectile and its owner

        if let Ok((&TurretBulletProjectileOwner(owner),)) = self.projectile_query.get(collider1) {
            if let Ok((&ColliderOf { body },)) = self.colider_of_query.get(collider2) {
                if owner == body {
                    return false;
                }
            }
        }

        if let Ok((&TurretBulletProjectileOwner(owner),)) = self.projectile_query.get(collider2) {
            if let Ok((&ColliderOf { body },)) = self.colider_of_query.get(collider1) {
                if owner == body {
                    return false;
                }
            }
        }

        true
    }
}

fn insert_turret_section(
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

    let interval = 1.0 / config.fire_rate;
    let mut timer = Timer::from_seconds(interval, TimerMode::Once);
    timer.finish(); // Ready to fire immediately

    let muzzle = commands
        .spawn((
            Name::new("Turret Barrel Muzzle"),
            TurretSectionBarrelMuzzleMarker,
            TurretSectionPartOf(turret),
            TurretSectionBarrelFireState(timer),
            TurretSectionBarrelMuzzleEffect(config.muzzle_effect.clone()),
            Transform::from_translation(config.muzzle_offset),
            Visibility::Inherited,
        ))
        .id();

    let rotator_barrel = commands
        .spawn((
            Name::new("Turret Rotator Barrel"),
            TurretSectionRotatorBarrelMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionBarrelRenderMesh(config.render_mesh_barrel),
            Transform::from_translation(config.barrel_offset),
            Visibility::Inherited,
        ))
        .add_child(muzzle)
        .id();

    let rotator_pitch = commands
        .spawn((
            Name::new("Turret Rotator Pitch"),
            TurretSectionRotatorPitchMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            Transform::default(),
            TurretSectionPitchRenderMesh(config.render_mesh_pitch),
            Visibility::Inherited,
        ))
        .add_child(rotator_barrel)
        .id();

    let rotator_pitch_base = commands
        .spawn((
            Name::new("Turret Rotator Pitch Base"),
            TurretSectionRotatorPitchBaseMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            SmoothLookRotation {
                axis: Vec3::X,
                initial: 0.0,
                speed: config.pitch_speed,
                min: config.min_pitch,
                max: config.max_pitch,
            },
            Transform::from_translation(config.pitch_offset),
            Visibility::Inherited,
        ))
        .add_child(rotator_pitch)
        .id();

    let rotator_yaw = commands
        .spawn((
            Name::new("Turret Rotator Yaw"),
            TurretSectionRotatorYawMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            Transform::default(),
            TurretSectionYawRenderMesh(config.render_mesh_yaw),
            Visibility::Inherited,
        ))
        .add_child(rotator_pitch_base)
        .id();

    let rotator_yaw_base = commands
        .spawn((
            Name::new("Turret Rotator Yaw Base"),
            TurretSectionRotatorYawBaseMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            SmoothLookRotation {
                axis: Vec3::Y,
                initial: 0.0,
                speed: config.yaw_speed,
                ..default()
            },
            Transform::from_translation(config.yaw_offset),
            Visibility::Inherited,
        ))
        .add_child(rotator_yaw)
        .id();

    let rotator_base = commands
        .spawn((
            Name::new("Turret Rotator Base"),
            TurretRotatorBaseMarker,
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(muzzle),
            Transform::from_translation(config.base_offset),
            TurretSectionBaseRenderMesh(config.render_mesh_base),
            Visibility::Inherited,
        ))
        .add_child(rotator_yaw_base)
        .id();

    commands
        .entity(turret)
        .insert((TurretSectionMuzzleEntity(muzzle),))
        .add_child(rotator_base);
}

fn update_barrel_fire_state(
    q_turret: Query<Entity, (With<TurretSectionMarker>, Without<SectionInactiveMarker>)>,
    mut q_barrel: Query<
        (&mut TurretSectionBarrelFireState, &TurretSectionPartOf),
        With<TurretSectionBarrelMuzzleMarker>,
    >,
    time: Res<Time>,
) {
    let dt = time.delta();
    for (mut fire_state, part_of) in &mut q_barrel {
        if !q_turret.contains(**part_of) {
            continue;
        }

        fire_state.tick(dt);
    }
}

fn update_turret_target_yaw_system(
    q_turret: Query<
        (&TurretSectionTargetInput, Has<SectionInactiveMarker>),
        With<TurretSectionMarker>,
    >,
    mut q_rotator_yaw_base: Query<
        (
            &mut SmoothLookRotationTarget,
            &GlobalTransform,
            &TurretSectionPartOf,
            &TurretSectionMuzzleEntity,
        ),
        With<TurretSectionRotatorYawBaseMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
) {
    for (mut target, yaw_chain, TurretSectionPartOf(turret), TurretSectionMuzzleEntity(muzzle)) in
        &mut q_rotator_yaw_base
    {
        let Ok(muzzle_transform) = q_muzzle.get(*muzzle) else {
            error!(
                "update_turret_target_yaw_system: entity {:?} not found in q_muzzle",
                *muzzle
            );
            continue;
        };

        let Ok((target_input, inactive)) = q_turret.get(*turret) else {
            error!(
                "update_turret_target_yaw_system: entity {:?} not found in q_turret",
                *turret
            );
            continue;
        };

        if inactive {
            continue;
        }

        let Some(target_input) = **target_input else {
            continue;
        };

        let world_to_yaw_base = yaw_chain.to_matrix().inverse();

        let target_pos = target_input;
        let barrel_pos = muzzle_transform.translation();
        let barrel_dir = muzzle_transform.forward().into();
        if target_pos == barrel_pos {
            continue;
        }

        let barrel_yaw_local_pos = world_to_yaw_base.transform_point3(barrel_pos);
        let target_yaw_local_pos = world_to_yaw_base.transform_point3(target_pos);
        let barrel_yaw_local_dir = world_to_yaw_base.transform_vector3(barrel_dir);

        // phi is the angle from the x axis to the (x,-z) position
        let phi = (-target_yaw_local_pos.z).atan2(target_yaw_local_pos.x);
        // r is the distance from the origin to the barrel direction projected onto the xz plane
        let r = barrel_yaw_local_pos.cross(barrel_yaw_local_dir).y;
        let target_r = target_yaw_local_pos.xz().length();
        if target_r > r.abs() {
            let theta = (phi - (r / target_r).acos()) % (std::f32::consts::TAU);
            **target = theta;
        }
    }
}

fn update_turret_target_pitch_system(
    q_turret: Query<
        (&TurretSectionTargetInput, Has<SectionInactiveMarker>),
        With<TurretSectionMarker>,
    >,
    mut q_rotator_pitch_base: Query<
        (
            &mut SmoothLookRotationTarget,
            &GlobalTransform,
            &TurretSectionPartOf,
            &TurretSectionMuzzleEntity,
        ),
        With<TurretSectionRotatorPitchBaseMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
) {
    for (mut target, pitch_chain, TurretSectionPartOf(turret), TurretSectionMuzzleEntity(muzzle)) in
        &mut q_rotator_pitch_base
    {
        let Ok(muzzle_transform) = q_muzzle.get(*muzzle) else {
            error!(
                "update_turret_target_pitch_system: entity {:?} not found in q_muzzle",
                *muzzle
            );
            continue;
        };

        let Ok((target_input, inactive)) = q_turret.get(*turret) else {
            error!(
                "update_turret_target_pitch_system: entity {:?} not found in q_turret",
                *turret
            );
            continue;
        };

        if inactive {
            continue;
        }

        let Some(target_input) = **target_input else {
            continue;
        };

        let world_to_pitch_base = pitch_chain.to_matrix().inverse();

        let target_pos = target_input;
        let barrel_pos = muzzle_transform.translation();
        let barrel_dir = muzzle_transform.forward().into();
        if target_pos == barrel_pos {
            continue;
        }

        let barrel_pitch_local_pos = world_to_pitch_base.transform_point3(barrel_pos);
        let target_pitch_local_pos = world_to_pitch_base.transform_point3(target_pos);
        let barrel_pitch_local_dir = world_to_pitch_base.transform_vector3(barrel_dir);

        let phi = (-target_pitch_local_pos.z).atan2(target_pitch_local_pos.y);
        let r = -barrel_pitch_local_pos.cross(barrel_pitch_local_dir).x;
        let target_r = target_pitch_local_pos.yz().length();
        if target_r > r.abs() {
            let theta = phi - (r / target_r).acos();
            **target = -theta;
        }
    }
}

fn sync_turret_rotator_yaw_system(
    q_base: Query<&SmoothLookRotationOutput, With<TurretSectionRotatorYawBaseMarker>>,
    mut q_yaw_rotator: Query<(&mut Transform, &ChildOf), With<TurretSectionRotatorYawMarker>>,
) {
    for (mut yaw_transform, &ChildOf(entity)) in &mut q_yaw_rotator {
        let Ok(rotator_output) = q_base.get(entity) else {
            error!(
                "sync_turret_rotator_yaw_system: entity {:?} not found in q_base",
                entity
            );
            continue;
        };

        yaw_transform.rotation = Quat::from_euler(EulerRot::YXZ, **rotator_output, 0.0, 0.0);
    }
}

fn sync_turret_rotator_pitch_system(
    q_base: Query<&SmoothLookRotationOutput, With<TurretSectionRotatorPitchBaseMarker>>,
    mut q_pitch_rotator: Query<(&mut Transform, &ChildOf), With<TurretSectionRotatorPitchMarker>>,
) {
    for (mut pitch_transform, &ChildOf(entity)) in &mut q_pitch_rotator {
        let Ok(rotator_output) = q_base.get(entity) else {
            error!(
                "sync_turret_rotator_pitch_system: entity {:?} not found in q_base",
                entity
            );
            continue;
        };

        pitch_transform.rotation = Quat::from_euler(EulerRot::YXZ, 0.0, **rotator_output, 0.0);
    }
}

fn shoot_spawn_projectile(
    mut commands: Commands,
    q_spaceship: Query<
        (&LinearVelocity, &AngularVelocity, &ComputedCenterOfMass),
        With<SpaceshipRootMarker>,
    >,
    q_turret: Query<
        (
            Entity,
            &TurretSectionMuzzleEntity,
            &ChildOf,
            &TurretSectionConfigHelper,
            &TurretSectionInput,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_muzzle: Query<&mut TurretSectionBarrelFireState, With<TurretSectionBarrelMuzzleMarker>>,
    // We are using TransformHelper here because we need to compute the global transform; And it
    // should be fine, since it will not be called frequently.
    transform_helper: TransformHelper,
) {
    for (turret, muzzle, ChildOf(spaceship), config, input) in &q_turret {
        if !**input {
            continue;
        }

        let Ok((lin_vel, ang_vel, center)) = q_spaceship.get(*spaceship) else {
            error!(
                "on_shoot_spawn_projectile: entity {:?} not found in q_spaceship",
                spaceship
            );
            continue;
        };

        let Ok(mut fire_state) = q_muzzle.get_mut(**muzzle) else {
            error!(
                "on_shoot_spawn_projectile: entity {:?} not found in q_muzzle",
                **muzzle
            );
            continue;
        };

        if !fire_state.is_finished() {
            continue;
        }

        let Ok(muzzle_transform) = transform_helper.compute_global_transform(**muzzle) else {
            error!(
                "on_shoot_spawn_projectile: entity {:?} global transform not found",
                **muzzle
            );
            continue;
        };

        let muzzle_direction = muzzle_transform.forward();
        let projectile_position = muzzle_transform.translation();
        let projectile_rotation = muzzle_transform.rotation();
        let radius_vector = projectile_position - **center;
        let _inertia_vel = ang_vel.cross(radius_vector) + **lin_vel;
        // FIXME: Currently we are only using the linear velocity as inertia
        let inertia_vel = **lin_vel;

        let muzzle_exit_velocity = muzzle_direction * config.muzzle_speed;
        let linear_velocity = muzzle_exit_velocity + inertia_vel;

        let projectile_transform = Transform {
            translation: projectile_position + muzzle_exit_velocity * 0.01,
            rotation: projectile_rotation,
            ..default()
        };

        commands.spawn((
            Name::new("Turret Projectile"),
            TurretBulletProjectileMarker,
            TurretBulletProjectileOwner(*spaceship),
            projectile_transform,
            RigidBody::Dynamic,
            LinearVelocity(linear_velocity),
            Collider::sphere(0.05),
            ActiveCollisionHooks::FILTER_PAIRS,
            Mass(config.projectile_mass),
            TurretSectionPartOf(turret),
            TurretSectionMuzzleEntity(**muzzle),
            BulletProjectileRenderMesh(config.projectile_render_mesh.clone()),
            TempEntity(config.projectile_lifetime),
            Visibility::Visible,
            TransformInterpolation,
        ));

        // Reset the fire state timer
        fire_state.reset();
    }
}

fn on_projectile_marker_effect(
    add: On<Add, TurretBulletProjectileMarker>,
    q_projectile: Query<&TurretSectionMuzzleEntity, With<TurretBulletProjectileMarker>>,
    mut q_effect: Query<
        (&mut EffectProperties, &mut EffectSpawner, &ChildOf),
        (
            With<TurretSectionBarrelMuzzleEffectMarker>,
            Without<TurretSectionBarrelMuzzleMarker>,
        ),
    >,
    // We are using TransformHelper here because we need to compute the global transform; And it
    // should be fine, since it will not be called frequently.
    transform_helper: TransformHelper,
) {
    let projectile = add.entity;
    trace!("on_projectile_marker: entity {:?}", projectile);

    let Ok(muzzle) = q_projectile.get(projectile) else {
        error!(
            "on_projectile_marker: entity {:?} not found in q_projectile",
            projectile
        );
        return;
    };

    let Ok(muzzle_transform) = transform_helper.compute_global_transform(**muzzle) else {
        error!(
            "on_projectile_marker_effect: entity {:?} global transform not found",
            **muzzle
        );
        return;
    };

    // Spawn the effect muzzle
    let Some((mut properties, mut effect_spawner, _)) = q_effect
        .iter_mut()
        .find(|(_, _, &ChildOf(parent))| parent == **muzzle)
    else {
        error!(
            "on_shoot_spawn_projectile: effect for muzzle {:?} not found",
            **muzzle
        );
        return;
    };

    let normal = muzzle_transform.forward();

    let p: f32 = rand::random();

    let (r, g, b) = if p < 0.4 {
        let r = 255;
        let g = 240 + rand::random_range(0..16);
        let b = 200 + rand::random_range(0..56);
        (r, g, b)
    } else if p < 0.75 {
        let r = 255;
        let g = rand::random_range(100..180);
        let b = 0;
        (r, g, b)
    } else if p < 0.95 {
        let r = 255;
        let g = rand::random_range(50..120);
        let b = 0;
        (r, g, b)
    } else {
        let val = rand::random_range(30..80);
        (val, val, val)
    };
    let color = 0xFF000000u32 | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32);
    properties.set("spawn_color", color.into());

    // Set the collision normal
    let normal = normal.normalize();
    properties.set("normal", normal.into());

    let base_velocity = Vec3::ZERO;
    properties.set("base_velocity", base_velocity.into());

    // Spawn the particles
    effect_spawner.reset();
}

fn insert_projectile_render(
    add: On<Add, TurretBulletProjectileMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_render_mesh: Query<&BulletProjectileRenderMesh>,
) {
    let entity = add.entity;
    trace!("insert_projectile_render: entity {:?}", entity);

    let Ok(render_mesh) = q_render_mesh.get(entity) else {
        error!(
            "insert_projectile_render: entity {:?} not found in q_render_mesh",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene_handle) => {
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                SceneRoot(scene_handle.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.3))),
                MeshMaterial3d(materials.add(Color::srgb(1.0, 0.9, 0.2))),
            ),],));
        }
    }
}

fn insert_turret_section_render(
    add: On<Add, TurretRotatorBaseMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_base: Query<
        (&TurretSectionPartOf, &TurretSectionBaseRenderMesh),
        With<TurretRotatorBaseMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_section_render: entity {:?}", entity);

    let Ok((turret, render_mesh)) = q_base.get(entity) else {
        error!(
            "insert_turret_section_render: entity {:?} not found in q_base",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Base"),
                SectionRenderOf(**turret),
                SceneRoot(scene.clone()),
            ),],));
        }
        None => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Base"),
                Transform::from_xyz(0.0, 0.05, 0.0),
                SectionRenderOf(**turret),
                Mesh3d(meshes.add(Cylinder::new(0.5, 0.1))),
                MeshMaterial3d(materials.add(Color::srgb(0.25, 0.25, 0.25))),
            ),],));
        }
    }
}

fn insert_turret_yaw_rotator_render(
    add: On<Add, TurretSectionRotatorYawMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_yaw: Query<
        (&TurretSectionPartOf, &TurretSectionYawRenderMesh),
        With<TurretSectionRotatorYawMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_yaw_rotator_render: entity {:?}", entity);

    let Ok((turret, render_mesh)) = q_yaw.get(entity) else {
        error!(
            "insert_turret_yaw_rotator_render: entity {:?} not found in q_yaw",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Yaw"),
                SectionRenderOf(**turret),
                SceneRoot(scene.clone()),
            ),],));
        }
        None => {
            let base_mat = materials.add(Color::srgb(0.4, 0.4, 0.4));
            let ridge_mat = materials.add(Color::srgb(0.3, 0.3, 0.3));

            let base_cylinder = meshes.add(Cylinder::new(0.2, 0.2));

            let ridge_count = 16;
            let ridge_radius = 0.22;
            let ridge_height = 0.2;
            let ridge_width = 0.04;
            let ridge_depth = 0.02;

            commands.entity(entity).with_children(|parent| {
                parent
                    .spawn((
                        Name::new("Render Turret Yaw"),
                        Transform::from_xyz(0.0, 0.1, 0.0),
                        Visibility::Inherited,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Name::new("Yaw Base"),
                            SectionRenderOf(**turret),
                            Mesh3d(base_cylinder.clone()),
                            MeshMaterial3d(base_mat.clone()),
                        ));

                        for i in 0..ridge_count {
                            let angle = i as f32 / ridge_count as f32 * std::f32::consts::TAU;
                            parent.spawn((
                                Name::new(format!("Ridge {i}")),
                                Transform::from_xyz(
                                    angle.cos() * ridge_radius,
                                    0.0,
                                    angle.sin() * ridge_radius,
                                )
                                .with_rotation(Quat::from_rotation_y(angle)),
                                Mesh3d(meshes.add(Cuboid::new(
                                    ridge_depth,
                                    ridge_height,
                                    ridge_width,
                                ))),
                                MeshMaterial3d(ridge_mat.clone()),
                            ));
                        }
                    });
            });
        }
    }
}

fn insert_turret_pitch_rotator_render(
    add: On<Add, TurretSectionRotatorPitchMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_pitch: Query<
        (&TurretSectionPartOf, &TurretSectionPitchRenderMesh),
        With<TurretSectionRotatorPitchMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_pitch_rotator_render: entity {:?}", entity);

    let Ok((turret, render_mesh)) = q_pitch.get(entity) else {
        error!(
            "insert_turret_pitch_rotator_render: entity {:?} not found in q_pitch",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Pitch"),
                SectionRenderOf(**turret),
                SceneRoot(scene.clone()),
            ),],));
        }
        None => {
            let base_mat = materials.add(Color::srgb(0.5, 0.5, 0.5));
            let ridge_mat = materials.add(Color::srgb(0.3, 0.3, 0.3));

            let base_cylinder = meshes.add(Cylinder::new(0.2, 0.2));

            let ridge_count = 16;
            let ridge_radius = 0.22;
            let ridge_height = 0.2;
            let ridge_width = 0.04;
            let ridge_depth = 0.02;

            commands.entity(entity).with_children(|parent| {
                parent
                    .spawn((
                        Name::new("Render Turret Pitch"),
                        Transform::from_xyz(0.3, 0.2, 0.0)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        Visibility::Inherited,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Name::new("Pitch Base"),
                            SectionRenderOf(**turret),
                            Mesh3d(base_cylinder.clone()),
                            MeshMaterial3d(base_mat.clone()),
                        ));

                        for i in 0..ridge_count {
                            let angle = i as f32 / ridge_count as f32 * std::f32::consts::TAU;
                            parent.spawn((
                                Name::new(format!("Ridge {i}")),
                                Transform::from_xyz(
                                    angle.cos() * ridge_radius,
                                    0.0,
                                    angle.sin() * ridge_radius,
                                )
                                .with_rotation(Quat::from_rotation_y(angle)),
                                Mesh3d(meshes.add(Cuboid::new(
                                    ridge_depth,
                                    ridge_height,
                                    ridge_width,
                                ))),
                                MeshMaterial3d(ridge_mat.clone()),
                            ));
                        }
                    });
            });
        }
    }
}

fn insert_turret_barrel_render(
    add: On<Add, TurretSectionRotatorBarrelMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_barrel: Query<
        (&TurretSectionPartOf, &TurretSectionBarrelRenderMesh),
        With<TurretSectionRotatorBarrelMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_barrel_render: entity {:?}", entity);

    let Ok((turret, render_mesh)) = q_barrel.get(entity) else {
        error!(
            "insert_turret_barrel_render: entity {:?} not found in q_barrel",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(scene) => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Barrel"),
                SectionRenderOf(**turret),
                SceneRoot(scene.clone()),
            ),],));
        }
        None => {
            let body_mat = materials.add(Color::srgb(0.2, 0.2, 0.5));
            let barrel_mat = materials.add(Color::srgb(0.2, 0.2, 0.7));
            let tip_mat = materials.add(Color::srgb(0.9, 0.2, 0.2));

            let body_mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.3));
            let barrel_mesh = meshes.add(Cuboid::new(0.12, 0.12, 0.2));
            let tip_mesh = meshes.add(Cone::new(0.08, 0.18));

            commands.entity(entity).with_children(|parent| {
                parent
                    .spawn((
                        Name::new("Render Turret Barrel"),
                        Transform::default(),
                        Visibility::Inherited,
                    ))
                    .with_children(|parent| {
                        parent
                            .spawn((
                                Name::new("Turret Body"),
                                Transform::from_xyz(0.0, 0.0, -0.05),
                                SectionRenderOf(**turret),
                                Mesh3d(body_mesh.clone()),
                                MeshMaterial3d(body_mat.clone()),
                            ))
                            .with_children(|parent| {
                                parent
                                    .spawn((
                                        Name::new("Turret Barrel"),
                                        Transform::from_xyz(0.0, 0.0, -0.25),
                                        Mesh3d(barrel_mesh.clone()),
                                        MeshMaterial3d(barrel_mat.clone()),
                                    ))
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Name::new("Barrel Tip"),
                                            Transform::from_xyz(0.0, 0.0, -0.05).with_rotation(
                                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                                            ),
                                            Mesh3d(tip_mesh.clone()),
                                            MeshMaterial3d(tip_mat.clone()),
                                        ));
                                    });
                            });
                    });
            });
        }
    }
}

fn insert_turret_barrel_muzzle_effect(
    add: On<Add, TurretSectionBarrelMuzzleMarker>,
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    q_effect: Query<&TurretSectionBarrelMuzzleEffect, With<TurretSectionBarrelMuzzleMarker>>,
) {
    let entity = add.entity;
    trace!("insert_turret_barrel_muzzle_effect: entity {:?}", entity);

    let Ok(effect_handle) = q_effect.get(entity) else {
        error!(
            "insert_turret_barrel_muzzle_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    match &**effect_handle {
        Some(effect) => {
            commands.entity(entity).insert((children![(
                Name::new("Muzzle Effect"),
                TurretSectionBarrelMuzzleEffectMarker,
                ParticleEffect::new(effect.clone()),
                EffectProperties::default(),
            ),],));
        }
        None => {
            let spawner = SpawnerSettings::once(100.0.into())
                // Disable starting emitting particles when the EffectSpawner is instantiated. We want
                // complete control, and only emit when reset() is called.
                .with_emit_on_start(false);

            let writer = ExprWriter::new();

            let age = writer.lit(0.).expr();
            let init_age = SetAttributeModifier::new(Attribute::AGE, age);

            // Give a bit of variation by randomizing the lifetime per particle
            let lifetime = writer.lit(0.01).uniform(writer.lit(0.1)).expr();
            let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

            // Bind the initial particle color to the value of the 'spawn_color' property
            // when the particle spawns. The particle will keep that color afterward,
            // even if the property changes, because the color will be saved
            // per-particle (due to the Attribute::COLOR).
            let spawn_color = writer.add_property("spawn_color", 0xFFFFFFFFu32.into());
            let color = writer.prop(spawn_color).expr();
            let init_color = SetAttributeModifier::new(Attribute::COLOR, color);

            let normal = writer.add_property("normal", Vec3::ZERO.into());
            let normal = writer.prop(normal);

            let base_velocity = writer.add_property("base_velocity", Vec3::ZERO.into());
            let base_velocity = writer.prop(base_velocity);

            // Set the position to be the collision point, which in this example is always
            // the emitter position (0,0,0) at the ball center, minus the ball radius
            // alongside the collision normal. Also raise particle to Z=0.2 so they appear
            // above the black background box.
            //   pos = -normal * BALL_RADIUS + Z * 0.2;
            // let pos = normal.clone() * writer.lit(-BALL_RADIUS) + writer.lit(Vec3::Z * 0.2);
            let pos = writer.lit(Vec3::ZERO);
            let init_pos = SetAttributeModifier::new(Attribute::POSITION, pos.expr());

            // Set the velocity to be a random direction mostly along the collision normal,
            // but with some spread. This cheaply ensures that we spawn only particles
            // inside the black background box (or almost; we ignore the edge case around
            // the corners). An alternative would be to use something
            // like a KillAabbModifier, but that would spawn particles and kill them
            // immediately, wasting compute resources and GPU memory.
            let spread_x = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread_y = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread_z = (writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(0.2);
            let spread = writer.lit(Vec3::X) * spread_x
                + writer.lit(Vec3::Y) * spread_y
                + writer.lit(Vec3::Z) * spread_z;
            let speed = writer.rand(ScalarType::Float) * writer.lit(5.0);
            let velocity = (normal + spread * writer.lit(2.5)).normalized() * speed;
            let velocity = velocity + base_velocity;
            let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

            let effect = effects.add(
                EffectAsset::new(32768, spawner, writer.finish())
                    .with_name("spawn_on_command")
                    .init(init_pos)
                    .init(init_vel)
                    .init(init_age)
                    .init(init_lifetime)
                    .init(init_color)
                    // Set a size of 3 (logical) pixels, constant in screen space, independent of projection
                    .render(SetSizeModifier {
                        size: Vec3::splat(3.).into(),
                    })
                    .render(ScreenSpaceSizeModifier),
            );

            commands.entity(entity).insert((children![(
                Name::new("Muzzle Effect"),
                TurretSectionBarrelMuzzleEffectMarker,
                ParticleEffect::new(effect),
                EffectProperties::default(),
            ),],));
        }
    }
}
