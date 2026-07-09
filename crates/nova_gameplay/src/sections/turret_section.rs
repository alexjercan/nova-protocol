//! A turret section is a component that can be added to an entity to give it a turret-like
//! behavior.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_hanabi::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        turret_section, TurretBulletProjectileMarker, TurretSectionAimPoint,
        TurretSectionBarrelMuzzleMarker, TurretSectionConfig, TurretSectionConfigHelper,
        TurretSectionInput, TurretSectionMarker, TurretSectionMuzzleEntity, TurretSectionPlugin,
        TurretSectionTargetInput, TurretSectionTargetVelocity,
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
    pub render_mesh_base: Option<Handle<WorldAsset>>,
    /// The offset of the base from the section origin
    pub base_offset: Vec3,
    /// The render mesh of the yaw rotator, defaults to a cylinder with ridges
    pub render_mesh_yaw: Option<Handle<WorldAsset>>,
    /// The offset of the yaw rotator from the base
    pub yaw_offset: Vec3,
    /// The render mesh of the pitch rotator, defaults to a cylinder with ridges
    pub render_mesh_pitch: Option<Handle<WorldAsset>>,
    /// The offset of the pitch rotator from the yaw rotator
    pub pitch_offset: Vec3,
    /// The render mesh of the barrel, defaults to a simple barrel shape
    pub render_mesh_barrel: Option<Handle<WorldAsset>>,
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
    pub projectile_render_mesh: Option<Handle<WorldAsset>>,
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
        TurretSectionTargetVelocity::default(),
        TurretSectionAimPoint::default(),
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

/// The world-space velocity of the turret's target, used to lead a moving target
/// (aim where it will be when a bullet arrives). Defaults to zero - a stationary
/// aim point (e.g. the player crosshair) needs no lead. Whoever aims the turret at
/// a moving object (auto-targeting, AI) sets this to the object's velocity.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct TurretSectionTargetVelocity(pub Vec3);

/// The world-space point the turret is actually aiming its barrel at: the lead
/// intercept of `TurretSectionTargetInput` given `TurretSectionTargetVelocity`,
/// the bullet `muzzle_speed`, and the shooter's own muzzle velocity that the
/// bullet inherits on launch (see `update_turret_aim_point` - the solve runs in
/// the shooter's frame). `None` when there is no target. Read by the yaw/pitch
/// systems to steer, and exposed so tooling (aim gizmos, the HUD lead pip) can
/// show where the turret leads.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct TurretSectionAimPoint(pub Option<Vec3>);

/// The Turret "parent" entity of the turret component.
///
/// `pub(crate)` so the audio module can key each gun's fire SFX by its turret
/// entity (multiple guns each sound). Not re-exported from the public prelude.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionPartOf(pub Entity);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct BulletProjectileRenderMesh(Option<Handle<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBaseRenderMesh(Option<Handle<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionYawRenderMesh(Option<Handle<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionPitchRenderMesh(Option<Handle<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBarrelRenderMesh(Option<Handle<WorldAsset>>);

/// The live tuning config carried by a turret section entity. The aim/shoot systems read
/// `muzzle_speed` from it directly every frame; the rotator speeds, pitch limits and fire rate
/// are snapshotted onto child entities when the turret is built, so edits to those are pushed to
/// the children by `apply_turret_config_to_children`. Editing this component (it derefs to
/// [`TurretSectionConfig`]) is the supported way to retune a turret live - see the turret range
/// example's sliders.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionConfigHelper(pub TurretSectionConfig);

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

            // FIXME(20260706-162908): For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(insert_turret_barrel_muzzle_effect);

            // FIXME(20260706-162908): For now we disable particle effects on wasm because it's not working
            #[cfg(not(target_family = "wasm"))]
            app.add_observer(on_projectile_marker_effect);
        }

        app.add_systems(
            Update,
            (
                apply_turret_config_to_children,
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
                update_turret_aim_point,
                update_turret_target_yaw_system,
                update_turret_target_pitch_system,
            )
                .chain()
                .after(TransformSystems::Propagate)
                .in_set(super::SpaceshipSectionSystems),
        );
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

/// Push live edits of a turret's [`TurretSectionConfigHelper`] onto the child entities that
/// snapshot it when the turret is built, so retuning takes effect immediately (the turret range
/// example's sliders, or the editor). `muzzle_speed` is read live by the aim/shoot systems and
/// needs no propagation; only the snapshotted knobs (rotator speeds, pitch limits, fire rate) are
/// pushed here. Gated on `Changed` so it costs nothing when nothing is being tuned.
fn apply_turret_config_to_children(
    q_turret: Query<
        (Entity, &TurretSectionConfigHelper),
        (
            With<TurretSectionMarker>,
            Changed<TurretSectionConfigHelper>,
        ),
    >,
    mut q_yaw: Query<
        (&TurretSectionPartOf, &mut SmoothLookRotation),
        (
            With<TurretSectionRotatorYawBaseMarker>,
            Without<TurretSectionRotatorPitchBaseMarker>,
        ),
    >,
    mut q_pitch: Query<
        (&TurretSectionPartOf, &mut SmoothLookRotation),
        (
            With<TurretSectionRotatorPitchBaseMarker>,
            Without<TurretSectionRotatorYawBaseMarker>,
        ),
    >,
    mut q_fire: Query<
        (&TurretSectionPartOf, &mut TurretSectionBarrelFireState),
        With<TurretSectionBarrelMuzzleMarker>,
    >,
) {
    for (turret, config) in &q_turret {
        for (part_of, mut yaw) in &mut q_yaw {
            if **part_of == turret {
                yaw.speed = config.yaw_speed;
            }
        }
        for (part_of, mut pitch) in &mut q_pitch {
            if **part_of == turret {
                pitch.speed = config.pitch_speed;
                pitch.min = config.min_pitch;
                pitch.max = config.max_pitch;
            }
        }
        for (part_of, mut fire_state) in &mut q_fire {
            if **part_of == turret {
                let interval = 1.0 / config.fire_rate.max(f32::EPSILON);
                fire_state.0.set_duration(Duration::from_secs_f32(interval));
            }
        }
    }
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

/// The point a turret should aim at to hit a moving target: the intercept point,
/// where a bullet leaving the muzzle at `projectile_speed` meets the target moving
/// at `target_vel`. Solves `|(target - shooter) + target_vel*t| = projectile_speed*t`
/// for the smallest positive time-to-intercept `t` and returns `target + target_vel*t`.
///
/// The solve assumes the bullet travels at `projectile_speed` from the shooter in
/// the frame `target_vel` is expressed in. Because bullets inherit the shooter's
/// muzzle velocity on launch, the caller passes the target velocity RELATIVE to
/// the shooter (see `update_turret_aim_point`); pass a world velocity only for a
/// shooter that is truly at rest.
///
/// Falls back to the target's current position when there is no valid intercept -
/// a target with no relative motion resolves to the target itself, and a target
/// too fast to catch (or receding faster than the bullet) has no positive solution,
/// so the turret simply aims where the target is now.
fn lead_intercept_point(
    shooter: Vec3,
    target: Vec3,
    target_vel: Vec3,
    projectile_speed: f32,
) -> Vec3 {
    let to_target = target - shooter;
    // a*t^2 + b*t + c = 0
    let a = target_vel.length_squared() - projectile_speed * projectile_speed;
    let b = 2.0 * to_target.dot(target_vel);
    let c = to_target.length_squared();

    let time_to_intercept = if a.abs() < 1e-4 {
        // Target speed ~ bullet speed: the equation is linear in t.
        (b.abs() > 1e-6).then(|| -c / b)
    } else {
        let discriminant = b * b - 4.0 * a * c;
        (discriminant >= 0.0)
            .then(|| {
                let sqrt_d = discriminant.sqrt();
                [(-b + sqrt_d) / (2.0 * a), (-b - sqrt_d) / (2.0 * a)]
                    .into_iter()
                    .filter(|&t| t > 0.0)
                    .reduce(f32::min)
            })
            .flatten()
    };

    match time_to_intercept {
        Some(t) if t > 0.0 && t.is_finite() => target + target_vel * t,
        _ => target,
    }
}

/// Resolve each turret's lead intercept point into [`TurretSectionAimPoint`] from
/// its target position, target velocity, and the bullet `muzzle_speed`. Runs
/// before the yaw/pitch systems, which steer toward this point.
///
/// The intercept is solved in the SHOOTER's frame: bullets inherit the full
/// muzzle point velocity at fire time (`shoot_spawn_projectile` adds ship
/// linear velocity plus the angular swing at the muzzle), so the solve uses
/// the target velocity RELATIVE to that same muzzle point velocity. Aiming
/// the barrel at the shooter-frame intercept and adding the inherited
/// velocity on launch lands the bullet on the true world-frame intercept:
/// dir*s*t = (target - muzzle) + (v_target - v_muzzle)*t. Solving in the
/// world frame instead makes every shot drift off by the shooter's own
/// motion (task 20260709-211701).
fn update_turret_aim_point(
    mut q_turret: Query<
        (
            &TurretSectionTargetInput,
            &TurretSectionTargetVelocity,
            &TurretSectionConfigHelper,
            &TurretSectionMuzzleEntity,
            &ChildOf,
            &mut TurretSectionAimPoint,
        ),
        With<TurretSectionMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_spaceship: Query<
        (
            &GlobalTransform,
            &LinearVelocity,
            &AngularVelocity,
            &ComputedCenterOfMass,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    for (
        target_input,
        target_velocity,
        config,
        TurretSectionMuzzleEntity(muzzle),
        ChildOf(spaceship),
        mut aim_point,
    ) in &mut q_turret
    {
        let Some(target_pos) = **target_input else {
            **aim_point = None;
            continue;
        };
        let Ok(muzzle_transform) = q_muzzle.get(*muzzle) else {
            continue;
        };
        let muzzle_pos = muzzle_transform.translation();

        // The same muzzle point velocity the bullet will inherit on launch
        // (same COM lift as shoot_spawn_projectile). A shooter without
        // physics components (test rigs) inherits nothing.
        let shooter_velocity = q_spaceship
            .get(*spaceship)
            .map(|(ship_transform, lin_vel, ang_vel, center)| {
                rigid_body_point_velocity(
                    **lin_vel,
                    **ang_vel,
                    ship_transform.transform_point(**center),
                    muzzle_pos,
                )
            })
            .unwrap_or(Vec3::ZERO);

        **aim_point = Some(lead_intercept_point(
            muzzle_pos,
            target_pos,
            **target_velocity - shooter_velocity,
            config.muzzle_speed,
        ));
    }
}

fn update_turret_target_yaw_system(
    q_turret: Query<
        (&TurretSectionAimPoint, Has<SectionInactiveMarker>),
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

        let Ok((aim_point, inactive)) = q_turret.get(*turret) else {
            error!(
                "update_turret_target_yaw_system: entity {:?} not found in q_turret",
                *turret
            );
            continue;
        };

        if inactive {
            continue;
        }

        let Some(target_pos) = **aim_point else {
            continue;
        };

        let world_to_yaw_base = yaw_chain.to_matrix().inverse();

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
        (&TurretSectionAimPoint, Has<SectionInactiveMarker>),
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

        let Ok((aim_point, inactive)) = q_turret.get(*turret) else {
            error!(
                "update_turret_target_pitch_system: entity {:?} not found in q_turret",
                *turret
            );
            continue;
        };

        if inactive {
            continue;
        }

        let Some(target_pos) = **aim_point else {
            continue;
        };

        let world_to_pitch_base = pitch_chain.to_matrix().inverse();

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

        // Inherit the full motion of the muzzle, not just the ship's linear velocity: a muzzle
        // offset from the center of mass of a rotating ship also swings tangentially. avian's
        // `ComputedCenterOfMass` is body-local, so lift it to world space with the ship's
        // global transform before taking the point velocity.
        let Ok(ship_transform) = transform_helper.compute_global_transform(*spaceship) else {
            error!(
                "on_shoot_spawn_projectile: entity {:?} global transform not found",
                spaceship
            );
            continue;
        };
        let center_of_mass = ship_transform.transform_point(**center);
        let inertia_vel =
            rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, projectile_position);

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
            ProjectileOwner(*spaceship),
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
                WorldAssetRoot(scene_handle.clone()),
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
                WorldAssetRoot(scene.clone()),
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
                WorldAssetRoot(scene.clone()),
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
                WorldAssetRoot(scene.clone()),
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
                WorldAssetRoot(scene.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_of_a_stationary_target_is_the_target() {
        // No motion, no lead: aim straight at the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let aim = lead_intercept_point(Vec3::ZERO, target, Vec3::ZERO, 100.0);
        assert!(
            (aim - target).length() < 1e-3,
            "expected the target itself, got {aim:?}"
        );
    }

    #[test]
    fn lead_intercepts_a_crossing_target() {
        // Shooter at origin; target 100 ahead crossing at +X. The intercept point
        // must be one a bullet at muzzle_speed and the target reach at the SAME
        // time - that is what "leading" means. It must also sit ahead of the target
        // in its direction of travel (+X).
        let shooter = Vec3::ZERO;
        let target = Vec3::new(0.0, 0.0, -100.0);
        let target_vel = Vec3::new(30.0, 0.0, 0.0);
        let speed = 100.0;

        let aim = lead_intercept_point(shooter, target, target_vel, speed);

        assert!(
            aim.x > 0.1,
            "intercept should lead a +X crosser, got {aim:?}"
        );
        // Consistency: at the bullet's flight time, the target is exactly there.
        let flight_time = (aim - shooter).length() / speed;
        let target_future = target + target_vel * flight_time;
        assert!(
            (target_future - aim).length() < 1e-2,
            "bullet and target should meet: aim {aim:?}, target at t {target_future:?}"
        );
    }

    #[test]
    fn lead_falls_back_when_the_target_cannot_be_caught() {
        // Target receding faster than the bullet: no positive intercept, so aim at
        // where it is now rather than returning a garbage/NaN point.
        let target = Vec3::new(0.0, 0.0, -50.0);
        let target_vel = Vec3::new(0.0, 0.0, -200.0); // fleeing at 200, bullet only 100
        let aim = lead_intercept_point(Vec3::ZERO, target, target_vel, 100.0);
        assert!(aim.is_finite());
        assert!(
            (aim - target).length() < 1e-3,
            "expected fallback to the target, got {aim:?}"
        );
    }

    /// World for aim-point tests: a ship with physics state, one turret with
    /// its muzzle 100 m ahead of a target-bearing setup. Returns (turret,
    /// muzzle position).
    fn aim_point_world(ship_velocity: Vec3, muzzle_pos: Vec3) -> (bevy::ecs::world::World, Entity) {
        let mut world = World::new();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::IDENTITY,
                LinearVelocity(ship_velocity),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                GlobalTransform::from_translation(muzzle_pos),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();
        (world, turret)
    }

    fn aim_point(world: &mut World, turret: Entity) -> Option<Vec3> {
        use bevy::ecs::system::RunSystemOnce;
        world.run_system_once(update_turret_aim_point).unwrap();
        **world.entity(turret).get::<TurretSectionAimPoint>().unwrap()
    }

    #[test]
    fn formation_flight_needs_no_lead() {
        // Shooter and target flying in formation (equal velocities): the
        // bullet inherits the shared motion on launch, so the correct aim is
        // the target itself. Solving in the world frame instead would lead a
        // target that, relatively, is not moving - the shots that never hit
        // from a moving ship (task 20260709-211701).
        let velocity = Vec3::new(40.0, 0.0, -10.0);
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(velocity, Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
            **entity.get_mut::<TurretSectionTargetVelocity>().unwrap() = velocity;
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            (aim - target).length() < 1e-2,
            "formation flight must aim at the target itself, got {aim:?}"
        );
    }

    #[test]
    fn moving_shooter_aims_retrograde_of_its_own_motion() {
        // Static target, shooter strafing +X: the bullet inherits +X on
        // launch, so the barrel must point BEHIND the shooter's motion for
        // the drift to carry the bullet onto the target.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let (mut world, turret) = aim_point_world(Vec3::new(30.0, 0.0, 0.0), Vec3::ZERO);
        {
            let mut entity = world.entity_mut(turret);
            **entity.get_mut::<TurretSectionTargetInput>().unwrap() = Some(target);
        }

        let aim = aim_point(&mut world, turret).expect("aim point computed");

        assert!(
            aim.x < -0.1,
            "a +X shooter must aim -X of the target, got {aim:?}"
        );
        // Consistency: bullet (barrel direction * speed + inherited velocity)
        // and target meet at the flight time the solve implies.
        let speed = TurretSectionConfig::default().muzzle_speed;
        let flight_time = aim.length() / speed;
        let barrel_dir = aim.normalize();
        let bullet_at_t = (barrel_dir * speed + Vec3::new(30.0, 0.0, 0.0)) * flight_time;
        assert!(
            (bullet_at_t - target).length() < 0.5,
            "inherited drift must carry the bullet onto the target: bullet \
             {bullet_at_t:?}, target {target:?}"
        );
    }

    #[test]
    fn shooter_without_physics_inherits_nothing() {
        // Test rigs and previews have marker-only ships: the aim point must
        // fall back to the plain world-frame solve instead of skipping.
        let target = Vec3::new(0.0, 0.0, -100.0);
        let mut world = World::new();
        let ship = world.spawn(SpaceshipRootMarker).id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(Some(target)),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionMuzzleEntity(muzzle),
                TurretSectionAimPoint(None),
                ChildOf(ship),
            ))
            .id();

        let aim = aim_point(&mut world, turret).expect("aim point computed");
        assert!((aim - target).length() < 1e-3);
    }

    /// Spawn a bare turret whose base rotators and fire timer are seeded from `config`,
    /// mimicking what `insert_turret_section` builds, without needing the render/physics
    /// plugins. Returns `(turret, yaw_base, pitch_base, muzzle)`.
    fn spawn_turret_rig(
        app: &mut App,
        config: &TurretSectionConfig,
    ) -> (Entity, Entity, Entity, Entity) {
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                TurretSectionConfigHelper(config.clone()),
            ))
            .id();
        let yaw = app
            .world_mut()
            .spawn((
                TurretSectionRotatorYawBaseMarker,
                TurretSectionPartOf(turret),
                SmoothLookRotation {
                    axis: Vec3::Y,
                    initial: 0.0,
                    speed: config.yaw_speed,
                    ..default()
                },
            ))
            .id();
        let pitch = app
            .world_mut()
            .spawn((
                TurretSectionRotatorPitchBaseMarker,
                TurretSectionPartOf(turret),
                SmoothLookRotation {
                    axis: Vec3::X,
                    initial: 0.0,
                    speed: config.pitch_speed,
                    min: config.min_pitch,
                    max: config.max_pitch,
                },
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                TurretSectionPartOf(turret),
                TurretSectionBarrelFireState(Timer::from_seconds(
                    1.0 / config.fire_rate,
                    TimerMode::Once,
                )),
            ))
            .id();
        (turret, yaw, pitch, muzzle)
    }

    #[test]
    fn editing_the_config_retunes_the_live_turret() {
        // The tuning sliders write `TurretSectionConfigHelper`; the snapshotted knobs on the
        // child rotators and the fire timer must follow.
        let mut app = App::new();
        app.add_systems(Update, apply_turret_config_to_children);

        let (turret, yaw, pitch, muzzle) =
            spawn_turret_rig(&mut app, &TurretSectionConfig::default());

        {
            let mut helper = app
                .world_mut()
                .get_mut::<TurretSectionConfigHelper>(turret)
                .unwrap();
            helper.yaw_speed = 5.0;
            helper.pitch_speed = 6.0;
            helper.min_pitch = Some(-0.25);
            helper.max_pitch = Some(0.5);
            helper.fire_rate = 25.0;
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
        // The `TurretSectionPartOf` guard must scope edits to the edited turret's own children.
        let mut app = App::new();
        app.add_systems(Update, apply_turret_config_to_children);

        let (edited, edited_yaw, _, _) =
            spawn_turret_rig(&mut app, &TurretSectionConfig::default());
        let (_other, other_yaw, _, _) = spawn_turret_rig(&mut app, &TurretSectionConfig::default());

        app.world_mut()
            .get_mut::<TurretSectionConfigHelper>(edited)
            .unwrap()
            .yaw_speed = 9.0;
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
            TurretSectionConfig::default().yaw_speed,
            "an untouched turret's rotators must not change"
        );
    }
}
