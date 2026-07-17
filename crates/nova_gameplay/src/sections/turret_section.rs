//! A turret section is a component that can be added to an entity to give it a turret-like
//! behavior.

use std::time::Duration;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_transform_interpolation::{RotationEasingState, TranslationEasingState};

use super::local_pose_in_root;
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        turret_section, LoadedBullet, TurretBulletProjectileMarker, TurretSectionAimPoint,
        TurretSectionAimSystems, TurretSectionBarrelMuzzleMarker, TurretSectionConfig,
        TurretSectionConfigHelper, TurretSectionInput, TurretSectionMarker,
        TurretSectionMuzzleEntity, TurretSectionPlugin, TurretSectionTargetInput,
        TurretSectionTargetVelocity,
    };
}

/// System set for the PostUpdate aim chain (intercept solve + rotator
/// targets), so HUD consumers can order same-frame readers after it (the
/// turret lead pips do - task 20260710-231929).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TurretSectionAimSystems;

/// Configuration for a turret section of a spaceship.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurretSectionConfig {
    /// The yaw speed of the turret section in radians per second.
    pub yaw_speed: f32,
    /// The pitch speed of the turret section in radians per second.
    pub pitch_speed: f32,
    /// The minimum pitch angle of the turret section in radians. If None, there is no limit.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub min_pitch: Option<f32>,
    /// The maximum pitch angle of the turret section in radians. If None, there is no limit.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max_pitch: Option<f32>,
    /// The render mesh of the base, defaults to a cylinder base
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_base: Option<AssetRef<WorldAsset>>,
    /// The offset of the base from the section origin
    pub base_offset: Vec3,
    /// The render mesh of the yaw rotator, defaults to a cylinder with ridges
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_yaw: Option<AssetRef<WorldAsset>>,
    /// The offset of the yaw rotator from the base
    pub yaw_offset: Vec3,
    /// The render mesh of the pitch rotator, defaults to a cylinder with ridges
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_pitch: Option<AssetRef<WorldAsset>>,
    /// The offset of the pitch rotator from the yaw rotator
    pub pitch_offset: Vec3,
    /// The render mesh of the barrel, defaults to a simple barrel shape
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_barrel: Option<AssetRef<WorldAsset>>,
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
    /// Authored Kinetic damage per hit (pre-resistance). Since the typed-damage
    /// pass (task 20260712-133343) weapon damage is AUTHORED here, not emergent
    /// from bullet mass x velocity: the bullet is spawned at a near-zero physical
    /// mass ([`NEUTRALIZED_BULLET_MASS`]) so bcs's kinetic term vanishes, and
    /// this fixed amount (scaled by the section resistance table) is the only
    /// weapon damage. Kinetic resistance is 1.0 everywhere, so authoring this to
    /// the old emergent per-hit (via [`representative_kinetic_damage`]) keeps the
    /// turret's feel unchanged.
    pub bullet_damage: f32,
    /// Damage TYPE of the round this turret is loaded with (task 20260712-133349).
    /// The authoring default for the turret's [`LoadedBullet`] slot; the fired
    /// projectile's `ProjectileDamage.kind` comes from that slot, and the ammo
    /// readout is colored by it. Catalog turrets are `Kinetic`, so the feel is
    /// unchanged; a future ship-management/station/scenario action swaps the
    /// loaded type by mutating `LoadedBullet`, not this authored default.
    pub bullet_kind: DamageType,
    /// The projectile mesh,
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub projectile_render_mesh: Option<AssetRef<WorldAsset>>,
    /// The muzzle particle effect when shooting.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub muzzle_effect: Option<AssetRef<EffectAsset>>,
    /// Magazine size in rounds. `None` fires without limit (the pre-ammo
    /// behavior); `Some(n)` gives the turret a [`SectionAmmo`] of `n` rounds
    /// that depletes one per bullet and blocks firing once empty. Reloading it
    /// is task 20260708-162005.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_capacity: Option<u32>,
    /// Auto-reload for the magazine. `None` = no reload (a spent magazine stays
    /// empty - the pre-reload behavior). `Some` attaches a [`SectionReload`]
    /// alongside the `SectionAmmo`, so it only applies when `ammo_capacity` is
    /// also `Some`; an unlimited turret never reloads. Task 20260717-085640.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub reload: Option<SectionReloadConfig>,
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
            // Matches the old emergent kinetic (mass 0.1 @ muzzle 100 u/s).
            bullet_damage: representative_kinetic_damage(0.1, 100.0),
            bullet_kind: DamageType::Kinetic,
            projectile_render_mesh: None,
            muzzle_effect: None,
            ammo_capacity: None,
            reload: None,
        }
    }
}

/// Helper function to create a turret section entity bundle.
pub fn turret_section(config: TurretSectionConfig) -> impl Bundle {
    debug!("turret_section: config {:?}", config);

    // The loaded-ammo slot, seeded from the authored config. Read before `config`
    // moves into the helper (both fields are `Copy`).
    let loaded = LoadedBullet {
        kind: config.bullet_kind,
        damage: config.bullet_damage,
    };

    (
        TurretSectionMarker,
        SectionDamageClass::Turret,
        loaded,
        TurretSectionTargetInput(None),
        TurretSectionTargetVelocity::default(),
        TurretSectionAimPoint::default(),
        TurretSectionConfigHelper(config),
        TurretSectionInput(false),
    )
}

/// The turret's loaded-ammo "slot": the round it currently fires. Runtime state
/// (seeded from [`TurretSectionConfig::bullet_kind`]/`bullet_damage`), NOT the
/// authored config - a future ship-management / station / scenario action swaps
/// the loaded round by mutating this one small component, and the fire path and
/// ammo readout both read it. Task 20260712-133349; the growth seam toward
/// per-type magazines + reload (spike 20260712-133135 phase 2).
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct LoadedBullet {
    /// The damage type of the loaded round (stamps `ProjectileDamage.kind`).
    pub kind: DamageType,
    /// The pre-resistance per-hit damage of the loaded round.
    pub damage: f32,
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
struct BulletProjectileRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBaseRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionYawRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionPitchRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretSectionBarrelRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

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
struct TurretSectionBarrelMuzzleEffect(#[reflect(ignore)] Option<AssetRef<EffectAsset>>);

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
        app.add_observer(despawn_bullet_on_hit);

        if self.render {
            app.add_observer(insert_turret_section_render);
            app.add_observer(insert_turret_yaw_rotator_render);
            app.add_observer(insert_turret_pitch_rotator_render);
            app.add_observer(insert_turret_barrel_render);
            app.add_observer(insert_projectile_render);

            // Hanabi muzzle-flash and projectile-trail effects: run on wasm too
            // now that the web build uses the WebGPU backend.
            app.add_observer(insert_turret_barrel_muzzle_effect);
            app.add_observer(on_projectile_marker_effect);
        }

        app.add_systems(
            Update,
            (
                apply_turret_config_to_children,
                sync_turret_rotator_yaw_system,
                sync_turret_rotator_pitch_system,
            )
                .in_set(super::SpaceshipSectionSystems),
        );

        // Firing lives on the physics clock (task 20260710-231930): the fire
        // timer accumulates fixed ticks and bullets spawn from the RAW root
        // pose, so shot spacing is exact at any ship velocity. In Update the
        // timer quantized shots to render frames and the muzzle pose was the
        // eased render pose - both errors scale with velocity and made
        // streams "spew" at speed.
        app.add_systems(
            FixedUpdate,
            shoot_spawn_projectile.in_set(super::SpaceshipSectionSystems),
        );

        // The aim chain runs EARLY in PostUpdate (before the HUD pips and
        // the indicator projection consume it - task 20260710-231929) and
        // composes fresh poses via TransformHelper instead of waiting for
        // transform propagation: bevy_ui lays out before propagation, so a
        // post-propagation aim point can only reach the screen one frame
        // late.
        app.add_systems(
            PostUpdate,
            (
                update_turret_aim_point,
                update_turret_target_yaw_system,
                update_turret_target_pitch_system,
            )
                .chain()
                .in_set(TurretSectionAimSystems)
                .in_set(super::SpaceshipSectionSystems),
        );
    }
}

/// A bullet deals its typed damage and dies on its first contact with something
/// TANGIBLE.
///
/// Nova OWNS the damage here: the bullet is a near-zero-mass Sensor (see the
/// spawn bundle), so bcs's emergent kinetic term is negligible; instead this
/// scales the bullet's authored [`ProjectileDamage`] by the hit section's
/// resistance and triggers `HealthApplyDamage` itself, which sidesteps Bevy
/// 0.19's arbitrary observer order - bcs's subtractor just subtracts what nova
/// decided (task 20260712-133343). The despawn keeps a sensor round from
/// crossing the target and dealing damage again against every event-enabled
/// collider along its line.
///
/// The OTHER side must not itself be a pure volume: scenario trigger
/// areas, beacon spheres and blast shells are Sensor colliders with
/// collision events enabled, and expending rounds at a beacon's 70u
/// trigger boundary made the pirate un-hittable while it patrolled near
/// one (review R1.1 BLOCKER of task 20260712-121101). A sensor-vs-sensor
/// pair is two intangibles crossing - nothing to expend on.
fn despawn_bullet_on_hit(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_bullets: Query<Option<&ProjectileDamage>, With<TurretBulletProjectileMarker>>,
    q_sensors: Query<(), With<Sensor>>,
    q_class: Query<&SectionDamageClass>,
) {
    let pairs = [
        (collision.body1, collision.collider2),
        (collision.body2, collision.collider1),
    ];
    for (body, other_collider) in pairs {
        let Some(body) = body else {
            continue;
        };
        // Membership gate: is this body a turret bullet? (`damage` is None only
        // for bare test rigs; production bullets always carry it.)
        let Ok(damage) = q_bullets.get(body) else {
            continue;
        };
        if q_sensors.contains(other_collider) {
            // A trigger/blast volume: the round flies on through.
            continue;
        }
        if let Some(&damage) = damage {
            // Own the trigger: scale by the hit section's resistance (unknown
            // targets - asteroids - take the raw amount). The bullet is the
            // source, carrying ProjectileOwner for threat attribution.
            let class = q_class.get(other_collider).ok().copied();
            apply_typed_damage(&mut commands, other_collider, Some(body), class, damage);
        }
        trace!("despawn_bullet_on_hit: bullet {:?} expended", body);
        commands.entity(body).try_despawn();
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
// `pub(crate)` so the turret-lead pip regression can register the real aim
// system with its production set constraints (the full TurretSectionPlugin
// drags render-material plugins into headless tests).
pub(crate) fn update_turret_aim_point(
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
    // Fresh render-clock poses: this runs BEFORE this frame's transform
    // propagation (see the registration comment), so GlobalTransform still
    // holds last frame - TransformHelper composes the eased poses the frame
    // will render with. The pip therefore marks the intercept as the player
    // SEES it; the physical bullet spawns from the raw pose (sub-tick
    // apart, see shoot_spawn_projectile).
    transform_helper: TransformHelper,
    q_spaceship: Query<
        (&LinearVelocity, &AngularVelocity, &ComputedCenterOfMass),
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
        let Ok(muzzle_transform) = transform_helper.compute_global_transform(*muzzle) else {
            continue;
        };
        let muzzle_pos = muzzle_transform.translation();

        // The same muzzle point velocity the bullet will inherit on launch
        // (same COM lift frame as the muzzle pose above). A shooter without
        // physics components (test rigs) inherits nothing.
        let shooter_velocity = q_spaceship
            .get(*spaceship)
            .ok()
            .zip(transform_helper.compute_global_transform(*spaceship).ok())
            .map(|((lin_vel, ang_vel, center), ship_transform)| {
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
            Entity,
            &mut SmoothLookRotationTarget,
            &TurretSectionPartOf,
            &TurretSectionMuzzleEntity,
        ),
        With<TurretSectionRotatorYawBaseMarker>,
    >,
    // Same fresh-pose composition as update_turret_aim_point: this chain
    // runs before this frame's transform propagation.
    transform_helper: TransformHelper,
) {
    for (yaw_base, mut target, TurretSectionPartOf(turret), TurretSectionMuzzleEntity(muzzle)) in
        &mut q_rotator_yaw_base
    {
        let Ok(yaw_chain) = transform_helper.compute_global_transform(yaw_base) else {
            error!(
                "update_turret_target_yaw_system: entity {:?} has no computable pose",
                yaw_base
            );
            continue;
        };
        let Ok(muzzle_transform) = transform_helper.compute_global_transform(*muzzle) else {
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
            Entity,
            &mut SmoothLookRotationTarget,
            &TurretSectionPartOf,
            &TurretSectionMuzzleEntity,
        ),
        With<TurretSectionRotatorPitchBaseMarker>,
    >,
    // Same fresh-pose composition as update_turret_aim_point.
    transform_helper: TransformHelper,
) {
    for (pitch_base, mut target, TurretSectionPartOf(turret), TurretSectionMuzzleEntity(muzzle)) in
        &mut q_rotator_pitch_base
    {
        let Ok(pitch_chain) = transform_helper.compute_global_transform(pitch_base) else {
            error!(
                "update_turret_target_pitch_system: entity {:?} has no computable pose",
                pitch_base
            );
            continue;
        };
        let Ok(muzzle_transform) = transform_helper.compute_global_transform(*muzzle) else {
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

/// A runaway-config backstop for the multi-shot loop: at 64 Hz ticks this
/// caps the effective fire rate at 512 rounds/s per barrel, far above any
/// authored turret; without it a zero-ish fire interval would spawn
/// unboundedly inside one tick.
const MAX_SHOTS_PER_TICK: u32 = 8;

fn shoot_spawn_projectile(
    mut commands: Commands,
    time: Res<Time>,
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
    mut q_turret: Query<
        (
            Entity,
            &TurretSectionMuzzleEntity,
            &ChildOf,
            &TurretSectionConfigHelper,
            Option<&LoadedBullet>,
            &TurretSectionInput,
            Option<&mut SectionAmmo>,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    mut q_muzzle: Query<&mut TurretSectionBarrelFireState, With<TurretSectionBarrelMuzzleMarker>>,
    q_chain: Query<(&Transform, &ChildOf)>,
    q_hot: Query<&WeaponsHot>,
) {
    let dt = time.delta_secs();
    for (turret, muzzle, ChildOf(spaceship), config, loaded, input, mut ammo) in &mut q_turret {
        // The weapons safety is a LIVE predicate (deliberate-radar task
        // 20260713-082337): a managed ship (player, mirrored AI) cannot fire
        // while SAFE even mid-held-trigger - the input bool is latched, so a
        // press-time gate alone would leak. Unmanaged ships (no WeaponsHot -
        // bare example turrets) fire freely.
        if q_hot.get(*spaceship).is_ok_and(|hot| !hot.0) {
            continue;
        }
        // The fired round: the runtime LoadedBullet slot if present (production
        // turrets carry one), else the authored config default (bare test rigs
        // and any turret not built via `turret_section`).
        let (bullet_kind, bullet_damage) = loaded
            .map(|loaded| (loaded.kind, loaded.damage))
            .unwrap_or((config.bullet_kind, config.bullet_damage));
        let Ok(mut fire_state) = q_muzzle.get_mut(**muzzle) else {
            error!(
                "shoot_spawn_projectile: entity {:?} not found in q_muzzle",
                **muzzle
            );
            continue;
        };

        // The cooldown elapses on the fixed clock whether or not the trigger
        // is held (absorbed from the old update_barrel_fire_state, which also
        // removed an unordered-tick-vs-shoot ambiguity in the Update set).
        // `elapsed` is sampled BEFORE the tick because a Once timer clamps at
        // its duration: `before + dt - interval` is the only way to recover
        // how far past due the shot came within this tick window.
        let before = fire_state.elapsed_secs();
        fire_state.tick(Duration::from_secs_f32(dt));

        if !**input || !fire_state.is_finished() {
            continue;
        }

        // Out of ammo: an empty magazine suppresses the whole turret this tick.
        // A turret with no `SectionAmmo` (unlimited) is never gated here, so the
        // pre-ammo behavior is untouched. The per-shot decrement below stops a
        // magazine that empties partway through this tick's burst.
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

        // Muzzle pose on the RAW physics clock: the root's avian pose
        // composed with the local mount chain (turret -> rotators ->
        // muzzle). This system runs in FixedUpdate, where `GlobalTransform`
        // still holds the previous frame's EASED render pose - sampling it
        // scattered spawn points by up to a tick of ship motion per shot
        // (task 20260710-231930). The rotator locals are written by the
        // Update-schedule aim systems; reading them here means the aim is at
        // most one frame old, which is control input staleness, not a
        // velocity-proportional error.
        let Some((muzzle_local_pos, muzzle_local_rot)) =
            local_pose_in_root(**muzzle, *spaceship, &q_chain)
        else {
            error!(
                "shoot_spawn_projectile: muzzle {:?} is not a descendant of ship {:?}",
                **muzzle, spaceship
            );
            continue;
        };
        let projectile_rotation = rotation.0 * muzzle_local_rot;
        let muzzle_position = position.0 + rotation.mul_vec3(muzzle_local_pos);
        let muzzle_direction = projectile_rotation * Vec3::NEG_Z;

        // Inherit the full motion of the muzzle, not just the ship's linear
        // velocity: a muzzle offset from the center of mass of a rotating
        // ship also swings tangentially. avian's `ComputedCenterOfMass` is
        // body-local; lift it with the same raw pose as everything else.
        let center_of_mass = position.0 + rotation.mul_vec3(**center);
        let inertia_vel =
            rigid_body_point_velocity(**lin_vel, **ang_vel, center_of_mass, muzzle_position);
        let muzzle_exit_velocity = muzzle_direction * config.muzzle_speed;
        let linear_velocity = muzzle_exit_velocity + inertia_vel;

        let interval = fire_state.duration().as_secs_f32();
        // How far past due the shot came within this tick window. A timer
        // that finished in an earlier tick (idle barrel, trigger just
        // pulled) reads `before == interval`, so the clamp lands the first
        // shot on this tick's start - fire NOW, exactly the old semantics.
        let mut excess = (before + dt - interval).clamp(0.0, dt);

        for _ in 0..MAX_SHOTS_PER_TICK {
            // Spend one round per bullet. A magazine that runs dry mid-burst
            // stops the stream exactly at zero (a high fire rate can queue
            // several shots per tick, so the gate above is not enough on its
            // own). Unlimited turrets carry no `SectionAmmo` and never break.
            if let Some(ammo) = ammo.as_deref_mut() {
                if !ammo.try_consume() {
                    break;
                }
            }

            // Sub-tick exactness: a shot due `lead` seconds into this tick
            // starts one lead-time of muzzle-exit travel BEHIND the muzzle,
            // so after this tick's integration it sits exactly where a
            // bullet fired at the due moment would - the stream stays
            // uniformly spaced at any ship velocity. (The ship-motion terms
            // cancel: spawn = muzzle + (v_muzzle - v_bullet) * lead, and
            // v_bullet - v_muzzle is the muzzle exit velocity.)
            let lead = dt - excess;
            let projectile_transform = Transform {
                translation: muzzle_position - muzzle_exit_velocity * lead,
                rotation: projectile_rotation,
                ..default()
            };

            let mut projectile = commands.spawn((
                Name::new("Turret Projectile"),
                TurretBulletProjectileMarker,
                ProjectileOwner(*spaceship),
                projectile_transform,
                RigidBody::Dynamic,
                LinearVelocity(linear_velocity),
                // Sensor: the impact-damage observer computes damage from
                // masses and velocities, never from the solver contact -
                // so a bullet needs NO physical contact response, and a
                // solid one was the knockback bug (mass 0.1 at 100 u/s
                // plus restitution shoved a ~4-mass ship ~3 u/s per hit;
                // playtest round 2 finding 2). despawn_bullet_on_hit
                // keeps a sensor round from crossing on through every
                // collider behind the first. CollisionEventsEnabled is
                // carried by the BULLET because the other side may not
                // have it: an invulnerable planetoid's collider has no
                // Health, so bcs never enables events on it, and an
                // event-less sensor pair raises nothing - rounds tunneled
                // straight through solid cover (review R1.2 MAJOR).
                // Nested tuple: bundle arity.
                (Collider::sphere(0.05), Sensor, CollisionEventsEnabled),
                ActiveCollisionHooks::FILTER_PAIRS,
                // Near-zero mass so bcs's emergent kinetic term (mass x velocity)
                // vanishes; nova's authored ProjectileDamage is the only weapon
                // damage. Gravity is mass-independent, so flight is unaffected
                // (task 20260712-133343). Nested tuple: bundle arity.
                (
                    Mass(NEUTRALIZED_BULLET_MASS),
                    // A Dynamic body needs finite, non-zero ANGULAR INERTIA too, or
                    // avian warns "no mass or inertia" once per fired round and
                    // risks NaN (task 20260716-205025). The Sensor collider above
                    // contributes no mass properties, and the neutralized `Mass`
                    // carries no inertia of its own, so derive a matching sphere
                    // inertia from the same shape + mass. The bullet never takes
                    // torque (sensor, authored damage, no angular velocity), so the
                    // value only has to be VALID, not tuned - flight is unaffected.
                    AngularInertia::from_shape(&Collider::sphere(0.05), NEUTRALIZED_BULLET_MASS),
                    // The fired round comes from the turret's loaded-ammo slot,
                    // not a hardcoded type (task 20260712-133349), so a future
                    // ammo switch changes what this stamps.
                    ProjectileDamage {
                        amount: bullet_damage,
                        kind: bullet_kind,
                    },
                ),
                TurretSectionPartOf(turret),
                TurretSectionMuzzleEntity(**muzzle),
                BulletProjectileRenderMesh(config.projectile_render_mesh.clone()),
                TempEntity(config.projectile_lifetime),
                Visibility::Visible,
                // Interpolation plus its render-clock seed (task
                // 20260711-121839): a body spawned mid-tick misses
                // FixedFirst, so its easing `start` stays None and the first
                // rendered frame would show the RAW spawn pose (sub-tick
                // lead offset and all) while the rest of the world renders
                // EASED - one visible frame of muzzle pop, cross-stream
                // error up to a tick of ship motion. Seeding `start` with
                // the tick-start muzzle pose (no lead offset) puts the first
                // frame at lerp(muzzle, raw_end, alpha): attached to the
                // rendered barrel, and only ever ahead of it along the
                // stream. FixedLast fills `end` with this tick's integrated
                // raw pose as usual, and the teleport-reset guard keeps the
                // seed because the written Transform equals `end` exactly.
                (
                    TransformInterpolation,
                    TranslationEasingState {
                        start: Some(muzzle_position),
                        end: None,
                    },
                    RotationEasingState {
                        start: Some(projectile_rotation),
                        end: None,
                    },
                ),
            ));
            // The projectile COPIES the shooter's allegiance instead of
            // resolving through ProjectileOwner at read time: it stays
            // attributable even if the owner dies mid-flight, and consumers
            // stay single-query.
            if let Some(&allegiance) = allegiance {
                projectile.insert(allegiance);
            }

            // Re-arm and immediately advance by the leftover: if the excess
            // spans another full interval the barrel fires again this tick
            // (fire rates above the tick rate keep their true cadence).
            fire_state.reset();
            fire_state.tick(Duration::from_secs_f32(excess));
            if !fire_state.is_finished() {
                break;
            }
            excess -= interval;
        }
    }
}

fn on_projectile_marker_effect(
    add: On<Add, TurretBulletProjectileMarker>,
    budget: Option<Res<GraphicsBudget>>,
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

    // On the Low tier `insert_turret_barrel_muzzle_effect` never spawned the muzzle
    // effect, so there is nothing to reset - skip before the lookup, otherwise the
    // missing-effect branch below would `error!` on every shot (task 20260525-133013).
    if !budget.as_deref().map_or(true, |b| b.particles) {
        return;
    }

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
    asset_server: Res<AssetServer>,
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
        Some(asset_ref) => {
            let scene_handle = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Bullet Projectile Render"),
                WorldAssetRoot(scene_handle),
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
    asset_server: Res<AssetServer>,
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
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Base"),
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
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
    asset_server: Res<AssetServer>,
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
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Yaw"),
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
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
    asset_server: Res<AssetServer>,
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
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Pitch"),
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
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
    asset_server: Res<AssetServer>,
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
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Barrel"),
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
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
    asset_server: Res<AssetServer>,
    budget: Option<Res<GraphicsBudget>>,
    q_effect: Query<&TurretSectionBarrelMuzzleEffect, With<TurretSectionBarrelMuzzleMarker>>,
) {
    let entity = add.entity;
    trace!("insert_turret_barrel_muzzle_effect: entity {:?}", entity);

    // Low graphics tier is spawn-less: skip the muzzle-flash hanabi (task
    // 20260525-133013). Absent budget (settings-less app) means full quality.
    if !budget.as_deref().map_or(true, |b| b.particles) {
        return;
    }

    let Ok(effect_handle) = q_effect.get(entity) else {
        error!(
            "insert_turret_barrel_muzzle_effect: entity {:?} not found in q_effect",
            entity
        );
        return;
    };

    match &**effect_handle {
        Some(asset_ref) => {
            let effect = asset_ref.resolve(&asset_server);
            commands.entity(entity).insert((children![(
                Name::new("Muzzle Effect"),
                TurretSectionBarrelMuzzleEffectMarker,
                ParticleEffect::new(effect),
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
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// A minimal app that runs ONLY `shoot_spawn_projectile` on a manual clock,
    /// so ammo behavior is observed by counting spawned bullets without the full
    /// physics/render stack. `dt` far larger than the fire interval keeps the
    /// barrel timer finished every tick, so firing is gated by ammo alone.
    fn firing_app(dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        app.add_systems(Update, shoot_spawn_projectile);
        app
    }

    /// Spawn a ship + one turret holding its trigger, optionally with a finite
    /// magazine. The muzzle is parented directly under the ship so
    /// `local_pose_in_root` resolves in one hop; the fire timer starts finished
    /// so the first tick can fire. `q_spaceship` reads avian `Position`/
    /// `Rotation`, so those are inserted directly (no physics stepping).
    fn spawn_firing_turret(app: &mut App, ammo: Option<u32>) -> Entity {
        let interval = 1.0 / TurretSectionConfig::default().fire_rate;
        let mut timer = Timer::from_seconds(interval, TimerMode::Once);
        timer.finish();

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
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(true),
                Transform::default(),
                ChildOf(ship),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                TurretSectionBarrelFireState(timer),
                Transform::default(),
                ChildOf(turret),
            ))
            .id();
        world
            .entity_mut(turret)
            .insert(TurretSectionMuzzleEntity(muzzle));
        if let Some(capacity) = ammo {
            world.entity_mut(turret).insert(SectionAmmo::new(capacity));
        }
        turret
    }

    fn bullet_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn a_turret_with_ammo_fires_exactly_its_magazine_then_stops() {
        // The core ammo claim: `try_consume` hard-caps total bullets at the
        // magazine size regardless of sub-tick fire timing, so an exact count is
        // a robust assertion. Ten wide ticks would fire far more than three
        // bullets unlimited (see the A/B below).
        let mut app = firing_app(1.0);
        let turret = spawn_firing_turret(&mut app, Some(3));

        for _ in 0..10 {
            app.update();
        }

        assert_eq!(
            bullet_count(&mut app),
            3,
            "a 3-round magazine must fire exactly three bullets, ever"
        );
        let ammo = app
            .world()
            .entity(turret)
            .get::<SectionAmmo>()
            .expect("the turret keeps its magazine");
        assert_eq!(
            ammo.rounds, 0,
            "the magazine must read empty after firing out"
        );
    }

    /// Every fired round is a Dynamic body, so avian needs it to have finite,
    /// non-zero mass AND angular inertia - otherwise it logs "no mass or inertia"
    /// once per shot and warns of NaN (task 20260716-205025). The Sensor collider
    /// contributes no mass properties and the neutralized `Mass` carries no
    /// inertia of its own, so the spawn adds an explicit sphere `AngularInertia`.
    /// Fire a real round through the production path under physics and read what
    /// avian actually COMPUTED (not just that a component is present).
    #[test]
    fn a_fired_bullet_has_finite_nonzero_mass_and_inertia() {
        use crate::integrity::test_support::{settle, unfinished_integrity_physics_app_with};

        // A physics app so avian's mass-property systems actually run; the helper
        // sets a 1/60 s manual step, and `settle` steps a few times (the first
        // fires the round; the rest let avian finalize the new body's masses).
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_systems(Update, shoot_spawn_projectile);
        app.finish();

        spawn_firing_turret(&mut app, Some(1));
        settle(&mut app);

        let world = app.world_mut();
        let (mass, inertia) = world
            .query_filtered::<(&ComputedMass, &ComputedAngularInertia), With<TurretBulletProjectileMarker>>()
            .single(world)
            .expect("exactly one fired bullet exists");

        let m = mass.value();
        assert!(
            m.is_finite() && m > 0.0,
            "a fired bullet needs finite non-zero mass, got {m}"
        );
        let (principal, _frame) = inertia.principal_angular_inertia_with_local_frame();
        assert!(
            principal.is_finite() && principal.min_element() > 0.0,
            "a fired bullet needs finite non-zero angular inertia on every axis \
             (else avian logs 'no mass or inertia' per shot and risks NaN), got {principal:?}"
        );
    }

    #[test]
    fn a_turret_without_ammo_keeps_firing_past_a_magazine() {
        // A/B control for the gate: the identical rig with no `SectionAmmo`
        // fires every tick, well past three bullets - proof that ammo, not some
        // other limit, stopped the stream above and that unlimited is the opt-in
        // default.
        let mut app = firing_app(1.0);
        spawn_firing_turret(&mut app, None);

        for _ in 0..10 {
            app.update();
        }

        assert!(
            bullet_count(&mut app) > 3,
            "an unlimited turret must not be capped at a magazine size, got {}",
            bullet_count(&mut app)
        );
    }

    #[test]
    fn an_auto_reloading_turret_fires_again_after_running_dry() {
        // End-to-end recovery: a finite turret fires out its 3-round magazine,
        // then the reload cycle refills it and it fires MORE than one magazine
        // over time - the whole point of auto-reload (task 20260717-085640).
        // Contrast with `a_turret_with_ammo_fires_exactly_its_magazine_then_stops`,
        // the same rig with no reload, which caps at 3 forever.
        let mut app = firing_app(1.0);
        app.add_systems(Update, crate::sections::ammo::tick_section_reload);
        let turret = spawn_firing_turret(&mut app, Some(3));
        // Discrete reload; ~0.2s is under the clock's 0.25s per-tick clamp so a
        // spent magazine refills within a couple of updates.
        app.world_mut()
            .entity_mut(turret)
            .insert(SectionReload::from_config(SectionReloadConfig {
                reload_time: 0.2,
                rounds_per_cycle: 3,
                only_when_empty: true,
            }));

        for _ in 0..20 {
            app.update();
        }

        assert!(
            bullet_count(&mut app) > 3,
            "an auto-reloading turret must fire past a single magazine, got {}",
            bullet_count(&mut app)
        );
    }

    #[test]
    fn turret_section_seeds_the_loaded_bullet_slot_from_config() {
        // The ammo slot is authored from config: bullet_kind/bullet_damage seed
        // LoadedBullet, and a default turret loads Kinetic.
        let mut world = World::new();
        let emp = world
            .spawn(turret_section(TurretSectionConfig {
                bullet_kind: DamageType::Emp,
                bullet_damage: 7.0,
                ..default()
            }))
            .id();
        let loaded = world
            .entity(emp)
            .get::<LoadedBullet>()
            .expect("turret_section inserts a LoadedBullet slot");
        assert_eq!(loaded.kind, DamageType::Emp);
        assert_eq!(loaded.damage, 7.0);

        let default_turret = world
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        assert_eq!(
            world
                .entity(default_turret)
                .get::<LoadedBullet>()
                .unwrap()
                .kind,
            DamageType::Kinetic,
            "catalog default loadout is Kinetic (feel-preserving)"
        );
    }

    #[test]
    fn fired_bullet_takes_the_loaded_slots_type_not_a_hardcoded_kind() {
        // Load a non-Kinetic round into the slot and confirm the fired bullet
        // carries it. Would fail if the fire path still stamped a hardcoded
        // Kinetic (the pre-slot behavior).
        let mut app = firing_app(1.0);
        let turret = spawn_firing_turret(&mut app, None);
        app.world_mut().entity_mut(turret).insert(LoadedBullet {
            kind: DamageType::Emp,
            damage: 5.0,
        });

        for _ in 0..3 {
            app.update();
        }

        let dmg = *app
            .world_mut()
            .query_filtered::<&ProjectileDamage, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .next()
            .expect("the turret fired at least one bullet");
        assert_eq!(
            dmg.kind,
            DamageType::Emp,
            "the fired round must take the loaded slot's type, not a hardcoded Kinetic"
        );
        assert_eq!(dmg.amount, 5.0, "and the slot's authored damage");
    }

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
                Transform::IDENTITY,
                LinearVelocity(ship_velocity),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                Transform::from_translation(muzzle_pos),
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
            .spawn((TurretSectionBarrelMuzzleMarker, Transform::IDENTITY))
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

    // -- projectile allegiance (task 20260708-203708) --

    /// A ready-to-fire ship + turret + muzzle rig for `shoot_spawn_projectile`,
    /// with the shooter's allegiance as given (`None` = unaligned shooter).
    /// The ship carries the raw avian pose and the muzzle hangs in its
    /// `ChildOf` tree, matching what the raw-clock spawn path reads.
    fn spawn_firing_rig(world: &mut World, allegiance: Option<Allegiance>) {
        let mut ship = world.spawn((
            SpaceshipRootMarker,
            Transform::default(),
            Position(Vec3::ZERO),
            Rotation::default(),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
            ComputedCenterOfMass(Vec3::ZERO),
        ));
        if let Some(allegiance) = allegiance {
            ship.insert(allegiance);
        }
        let ship = ship.id();
        let muzzle = world
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                Transform::default(),
                ChildOf(ship),
                {
                    // Pre-expired so the very first run fires.
                    let mut timer = Timer::from_seconds(0.1, TimerMode::Once);
                    timer.tick(std::time::Duration::from_secs(1));
                    TurretSectionBarrelFireState(timer)
                },
            ))
            .id();
        world.spawn((
            TurretSectionMarker,
            ChildOf(ship),
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionConfigHelper(TurretSectionConfig::default()),
            TurretSectionInput(true),
        ));
    }

    fn spawned_projectile_allegiance(world: &mut World) -> Option<Allegiance> {
        use bevy::ecs::system::RunSystemOnce;
        world.init_resource::<Time>();
        world.run_system_once(shoot_spawn_projectile).unwrap();
        world
            .query_filtered::<Option<&Allegiance>, With<TurretBulletProjectileMarker>>()
            .iter(world)
            .next()
            .expect("a projectile spawned")
            .copied()
    }

    #[test]
    fn spawned_projectile_copies_the_shooter_allegiance() {
        // The bullet must read as the shooter's side (relation model): copied
        // at spawn so it stays attributable even if the shooter dies.
        let mut world = World::new();
        spawn_firing_rig(&mut world, Some(Allegiance::Enemy));
        assert_eq!(
            spawned_projectile_allegiance(&mut world),
            Some(Allegiance::Enemy)
        );
    }

    #[test]
    fn spawned_projectile_of_an_unaligned_shooter_carries_no_allegiance() {
        let mut world = World::new();
        spawn_firing_rig(&mut world, None);
        assert_eq!(spawned_projectile_allegiance(&mut world), None);
    }

    // -- raw-clock spawn (task 20260710-231930) --

    /// A live-physics rig for the raw-clock spawn tests: a fast-capable ship
    /// root with a turret child and muzzle grandchild (non-identity local
    /// offsets AND a slewed rotator angle, so the local-chain composition is
    /// exercised, not just translations). Uses the projectile collision
    /// hooks so bullets ignore their own ship like production.
    fn spawn_stream_rig(app: &mut App, fire_rate: f32) -> (Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                RigidBody::Dynamic,
                Transform::default(),
                // Production ships interpolate; a raw-clock regression on a
                // non-faithful rig would understate the old bug (see the
                // 20260711-103527 retro).
                TransformInterpolation,
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                ChildOf(ship),
                Transform::from_xyz(0.0, 1.0, 0.0),
                // Trigger stays cold through settle(); tests arm it once the
                // rig's velocity is in place, so every bullet belongs to the
                // same stream.
                TurretSectionInput(false),
                TurretSectionConfigHelper(TurretSectionConfig {
                    fire_rate,
                    muzzle_speed: 200.0,
                    ..default()
                }),
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                ChildOf(turret),
                Transform::from_xyz(0.0, 0.0, -0.5).with_rotation(Quat::from_rotation_y(0.3)),
                TurretSectionBarrelFireState({
                    // Pre-expired: the first shot leaves on the first tick.
                    let mut timer = Timer::from_seconds(1.0 / fire_rate, TimerMode::Once);
                    timer.finish();
                    timer
                }),
            ))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionMuzzleEntity(muzzle));
        (ship, turret)
    }

    fn arm_turret(app: &mut App, turret: Entity) {
        app.world_mut()
            .get_mut::<TurretSectionInput>(turret)
            .unwrap()
            .0 = true;
    }

    /// Bullets from a fast ship must form a uniformly spaced, collinear
    /// stream (task 20260710-231930). The old Update-schedule spawn sampled
    /// the EASED muzzle pose at render-frame shot times with a static 0.01 s
    /// nudge, so each shot picked up a different fraction of a tick of ship
    /// motion - at 150 u/s the stream scattered by whole units ("bullets
    /// spew out"). On the raw clock with sub-tick lead compensation the
    /// inter-bullet spacing is exact: every consecutive delta equals
    /// Sensor bullets deal damage without knockback and die on the first
    /// hit (playtest round 2 finding 2). Before the Sensor change, a
    /// solid 0.1-mass round at 100 u/s shoved a unit-cube target ~2.5+
    /// u/s per hit (momentum 10 into the target mass, amplified by
    /// restitution 0.5) - "1 bullet sends you off like crazy". The bcs
    /// damage observer computes from masses and velocities, not the
    /// solver contact, so removing the contact response leaves damage
    /// intact. Delivery guards: the health drop proves the hit landed
    /// (a missed bullet would also read zero knockback), and the despawn
    /// proves a sensor round cannot sail on through everything behind
    /// the target.
    #[test]
    fn sensor_bullets_damage_without_knockback() {
        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_observer(despawn_bullet_on_hit);
        app.finish();

        // A free-floating target with health: one body, one collider.
        let target = app
            .world_mut()
            .spawn((
                Name::new("target"),
                RigidBody::Dynamic,
                Transform::default(),
                Collider::cuboid(2.0, 2.0, 2.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();
        settle(&mut app);

        // A bullet with the OLD emergent-kinetic shape (Mass 0.1, no
        // ProjectileDamage) on purpose: this test isolates the physics-contact
        // behavior - knockback and no-tunnel-through - so it drives bcs's
        // emergent damage rather than the typed path. The production bullet now
        // spawns near-zero mass + ProjectileDamage; its typed damage is covered
        // by `typed_bullet_applies_resistance_scaled_damage`.
        let bullet = app
            .world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 5.0),
                Sensor,
                Collider::sphere(0.05),
                Mass(0.1),
                LinearVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();

        // 5u at 100 u/s: contact within ~0.05s; run a quarter second.
        for _ in 0..15 {
            app.update();
        }

        let health = app
            .world()
            .get::<Health>(target)
            .expect("target still exists")
            .current;
        assert!(
            health < 100.0,
            "delivery guard: the bullet must actually hit and damage, health {health}"
        );
        let speed = app
            .world()
            .get::<LinearVelocity>(target)
            .expect("target body")
            .length();
        assert!(
            speed < 0.05,
            "a sensor bullet imparts no knockback (pre-fix: ~2.5+ u/s), got {speed}"
        );
        assert!(
            app.world().get_entity(bullet).is_err(),
            "the round is expended on its first hit"
        );
    }

    /// Production-faithful typed damage: a bullet as the turret now spawns it -
    /// near-zero mass (so bcs's emergent kinetic is negligible) plus an authored
    /// [`ProjectileDamage`] - hits a section and `despawn_bullet_on_hit` applies
    /// `amount x resistance(class, kind)` through the owned trigger. Proven
    /// across the table: Kinetic is unscaled everywhere (1.0), AP is amplified on
    /// the armored Turret (1.75) and penalised on the exposed Thruster (0.75).
    /// The drop is the nova-authored amount, NOT the old mass x velocity emergent
    /// (which the neutralized mass reduces to ~0), and lands exactly once.
    #[test]
    fn typed_bullet_applies_resistance_scaled_damage() {
        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        fn hit_drop(class: SectionDamageClass, damage: ProjectileDamage) -> f32 {
            let mut app = unfinished_integrity_physics_app_with(
                PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
            );
            app.add_observer(despawn_bullet_on_hit);
            app.finish();

            let start_hp = 1000.0;
            let target = app
                .world_mut()
                .spawn((
                    Name::new("target"),
                    RigidBody::Dynamic,
                    Transform::default(),
                    Collider::cuboid(2.0, 2.0, 2.0),
                    ColliderDensity(1.0),
                    Health::new(start_hp),
                    class,
                ))
                .id();
            settle(&mut app);

            app.world_mut().spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 5.0),
                Sensor,
                Collider::sphere(0.05),
                Mass(NEUTRALIZED_BULLET_MASS),
                damage,
                LinearVelocity(Vec3::NEG_Z * 100.0),
            ));
            for _ in 0..15 {
                app.update();
            }
            start_hp
                - app
                    .world()
                    .get::<Health>(target)
                    .expect("target still exists")
                    .current
        }

        let amount = 20.0;
        let kinetic = ProjectileDamage {
            amount,
            kind: DamageType::Kinetic,
        };
        let ap = ProjectileDamage {
            amount,
            kind: DamageType::ArmorPiercing,
        };

        // Kinetic: 1.0 on every section (feel-preserving). Tolerance covers the
        // ~2e-4 bcs residual from the neutralized mass.
        assert!(
            (hit_drop(SectionDamageClass::Turret, kinetic) - amount).abs() < 0.05,
            "Kinetic must be unscaled on the Turret"
        );
        // AP: 1.75 on the armored Turret, 0.75 on the exposed Thruster.
        assert!(
            (hit_drop(SectionDamageClass::Turret, ap) - amount * 1.75).abs() < 0.05,
            "AP must be amplified 1.75x on the Turret"
        );
        assert!(
            (hit_drop(SectionDamageClass::Thruster, ap) - amount * 0.75).abs() < 0.05,
            "AP must be penalised 0.75x on the Thruster"
        );
    }

    /// The two collision-event blind spots review R1.1/R1.2 caught in the
    /// sensor-bullet change: a round crossing a pure trigger volume (a
    /// beacon sphere - Sensor + events, no solidity) must SURVIVE, or the
    /// pirate goes un-hittable while patrolling near a beacon; and a round
    /// into an event-less solid (an invulnerable planetoid's collider has
    /// no Health, so bcs never enables events on it) must still expend
    /// instead of tunneling through cover - the bullet carries its own
    /// CollisionEventsEnabled for exactly that pair.
    #[test]
    fn bullets_ignore_trigger_volumes_and_stop_at_event_less_solids() {
        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_observer(despawn_bullet_on_hit);
        app.finish();

        // A beacon-style trigger volume in the flight path...
        app.world_mut().spawn((
            Name::new("trigger"),
            RigidBody::Static,
            Transform::from_translation(Vec3::Z * 6.0),
            Collider::sphere(2.0),
            Sensor,
            CollisionEventsEnabled,
        ));
        // ...and an invulnerable-planetoid stand-in behind it: solid,
        // no Health, no CollisionEventsEnabled of its own.
        app.world_mut().spawn((
            Name::new("event-less solid"),
            RigidBody::Static,
            Transform::default(),
            Collider::cuboid(3.0, 3.0, 1.0),
        ));
        settle(&mut app);

        let bullet = app
            .world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * 10.0),
                (Collider::sphere(0.05), Sensor, CollisionEventsEnabled),
                Mass(0.1),
                LinearVelocity(Vec3::NEG_Z * 100.0),
            ))
            .id();

        // Run to just past the trigger (4u of travel = 0.04s) but short of
        // the solid: the round must still be alive after crossing the
        // volume.
        for _ in 0..4 {
            app.update();
        }
        assert!(
            app.world().get_entity(bullet).is_ok(),
            "a round crossing a trigger volume must fly on (review R1.1)"
        );

        // Run into the solid: the round expends even though the solid has
        // no events of its own.
        for _ in 0..12 {
            app.update();
        }
        assert!(
            app.world().get_entity(bullet).is_err(),
            "a round must stop at an event-less solid instead of tunneling \
             (review R1.2)"
        );
    }

    /// muzzle_speed * fire_interval along the exit direction, regardless of
    /// ship velocity. The 24 rounds/s rate beats against the 64 Hz tick so
    /// shots sample every phase of the tick window.
    #[test]
    fn bullet_stream_stays_linear_at_high_ship_velocity() {
        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (ship, turret) = spawn_stream_rig(&mut app, 24.0);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        arm_turret(&mut app, turret);

        for _ in 0..40 {
            app.update();
        }

        let mut positions: Vec<Vec3> = app
            .world_mut()
            .query_filtered::<&Position, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .map(|p| p.0)
            .collect();
        assert!(
            positions.len() >= 10,
            "expected a stream, got {} bullets",
            positions.len()
        );

        // Sort along the exit direction (the muzzle's yaw-slewed -Z), then
        // every consecutive delta must be the SAME vector: equal spacing and
        // collinearity in one check.
        let exit_direction = Quat::from_rotation_y(0.3) * Vec3::NEG_Z;
        positions.sort_by(|a, b| a.dot(exit_direction).total_cmp(&b.dot(exit_direction)));
        let expected_spacing = 200.0 / 24.0;
        let first_delta = positions[1] - positions[0];
        // Delivery guard: uniform spacing alone is also satisfied by every
        // bullet sitting on one point; the spacing must be the real
        // muzzle_speed * interval stride.
        assert!(
            (first_delta.length() - expected_spacing).abs() < 0.1,
            "stream stride should be ~{expected_spacing}, got {}",
            first_delta.length()
        );
        for window in positions.windows(2) {
            let delta = window[1] - window[0];
            assert!(
                (delta - first_delta).length() < 0.05,
                "stream must stay uniform and collinear at speed: delta {delta} vs {first_delta}"
            );
        }
    }

    /// A bullet's FIRST rendered frame must sit on the world's render clock
    /// (task 20260711-121839). The spawn writes the RAW physics pose
    /// (tick-start muzzle minus the sub-tick lead), and a body spawned
    /// mid-tick misses FixedFirst, so its easing `start` is None and the
    /// first frame used to render that raw pose while the ship rendered
    /// EASED - one frame of muzzle pop, cross-stream error up to a full
    /// tick of ship motion (~2.3 u at 150 u/s). With the easing seed the
    /// first render is exactly `lerp(muzzle_tick_start, raw_end, alpha)`:
    /// zero cross-stream offset from the rendered barrel, and along-stream
    /// only ever FORWARD by at most one tick of muzzle-exit travel (a
    /// mid-tick shot has already flown; it must never render BEHIND the
    /// barrel, inside the turret). The raw physics stream is pinned
    /// separately by `bullet_stream_stays_linear_at_high_ship_velocity`;
    /// this test asserts the render clock, checking every bullet of a
    /// 24 rounds/s stream so the 64 Hz-vs-60 fps beat sweeps the easing
    /// alpha across its range.
    #[test]
    fn first_rendered_frame_attaches_the_bullet_to_the_eased_muzzle() {
        use std::collections::HashSet;

        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (ship, turret) = spawn_stream_rig(&mut app, 24.0);
        settle(&mut app);
        app.world_mut()
            .entity_mut(ship)
            .insert(LinearVelocity(Vec3::X * 150.0));
        arm_turret(&mut app, turret);

        // Rig locals (spawn_stream_rig): turret at (0, 1, 0), muzzle at
        // (0, 0, -0.5) yawed 0.3. The ship never spins here, so the exit
        // direction is constant in world space.
        let muzzle_local_rot = Quat::from_rotation_y(0.3);
        // Chain composition (local_pose_in_root): the muzzle's own rotation
        // aims its frame (the exit direction), it does not displace its
        // mount point.
        let muzzle_local_pos = Vec3::new(0.0, 1.0, 0.0) + Vec3::new(0.0, 0.0, -0.5);
        let exit_direction = muzzle_local_rot * Vec3::NEG_Z;
        // One tick of muzzle-exit travel: the most a mid-tick shot may lead
        // the barrel by on its first rendered frame.
        let max_lead = 200.0 * 1.0 / 64.0 + 0.05;

        let mut seen: HashSet<Entity> = HashSet::new();
        let mut sampled = 0usize;
        let mut max_cross = 0.0f32;
        let mut min_alpha = f32::MAX;
        for _ in 0..40 {
            app.update();
            // The ship's Transform is its EASED render pose this frame
            // (TransformInterpolation); compose the rendered muzzle from it.
            let ship_tf = *app.world().get::<Transform>(ship).unwrap();
            let rendered_muzzle = ship_tf.translation + ship_tf.rotation * muzzle_local_pos;
            let alpha = app.world().resource::<Time<Fixed>>().overstep_fraction();

            let bullets: Vec<(Entity, Vec3)> = app
                .world_mut()
                .query_filtered::<(Entity, &Transform), With<TurretBulletProjectileMarker>>()
                .iter(app.world())
                .map(|(e, t)| (e, t.translation))
                .collect();
            for (bullet, rendered) in bullets {
                if !seen.insert(bullet) {
                    continue;
                }
                // This bullet's FIRST rendered frame.
                sampled += 1;
                min_alpha = min_alpha.min(alpha);
                let offset = rendered - rendered_muzzle;
                let along = offset.dot(exit_direction);
                let cross = (offset - along * exit_direction).length();
                max_cross = max_cross.max(cross);
                assert!(
                    along > -0.05,
                    "a bullet must never first-render BEHIND the barrel: along {along}"
                );
                assert!(
                    along < max_lead,
                    "a bullet's first render may lead the barrel by at most one \
                     tick of muzzle travel: along {along} vs {max_lead}"
                );
            }
        }

        // Delivery guards: a real stream was sampled, and the beat actually
        // exercised frames where raw and eased poses diverge (small alpha is
        // where the pre-fix pop is largest).
        assert!(
            sampled >= 10,
            "expected a stream, sampled {sampled} first frames"
        );
        assert!(
            min_alpha < 0.5,
            "the beat must sample misaligned frames for the assertion to bite \
             (min alpha {min_alpha})"
        );
        assert!(
            max_cross < 0.02,
            "first rendered frame must sit ON the rendered stream line: \
             max cross-stream offset {max_cross}"
        );
    }

    /// The shipped default fire rate (100 rounds/s) is faster than the 64 Hz
    /// physics tick: the multi-shot loop must deliver the TRUE cadence via
    /// several spawns per tick. The old render-schedule path silently capped
    /// fire rates at one bullet per frame.
    #[test]
    fn fire_rate_above_the_tick_rate_keeps_its_true_cadence() {
        use crate::{
            integrity::test_support::{settle, unfinished_integrity_physics_app_with},
            sections::projectile_hooks::ProjectileHooks,
        };

        let mut app = unfinished_integrity_physics_app_with(
            PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>(),
        );
        app.add_systems(FixedUpdate, shoot_spawn_projectile);
        app.finish();

        let (_ship, turret) = spawn_stream_rig(&mut app, 100.0);
        settle(&mut app);
        arm_turret(&mut app, turret);

        // 60 render frames = 1.0 s of manual time.
        for _ in 0..60 {
            app.update();
        }

        let count = app
            .world_mut()
            .query_filtered::<(), With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .count();
        assert!(
            (95..=105).contains(&count),
            "one second at 100 rounds/s must yield ~100 bullets, got {count}"
        );
    }
}
