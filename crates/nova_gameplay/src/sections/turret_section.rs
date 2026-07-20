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
        turret_section, LoadedBullet, MuzzleConfig, TurretBulletProjectileMarker, TurretJoint,
        TurretSectionAimPoint, TurretSectionAimSystems, TurretSectionBarrelMuzzleMarker,
        TurretSectionConfig, TurretSectionConfigHelper, TurretSectionInput, TurretSectionMarker,
        TurretSectionMuzzleEntity, TurretSectionPlugin, TurretSectionTargetInput,
        TurretSectionTargetVelocity,
    };
}

/// System set for the PostUpdate aim chain (intercept solve + rotator
/// targets), so HUD consumers can order same-frame readers after it (the
/// turret lead pips do - task 20260710-231929).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TurretSectionAimSystems;

/// A fire point on a turret: where bullets leave. A joint carries at most one.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MuzzleConfig {
    /// Rounds per second for THIS muzzle.
    pub fire_rate: f32,
    /// Muzzle effect (flash) asset; None = no flash.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub muzzle_effect: Option<AssetRef<EffectAsset>>,
}

/// Default hinge speed (rad/s) when a joint's `speed` is not authored: 180
/// deg/s, matching the old yaw/pitch defaults.
fn default_joint_speed() -> f32 {
    std::f32::consts::PI
}

/// Skip serializing a joint's `speed` when it is the default (the common case:
/// every shipped joint traverses at 180 deg/s), so authored trees are not
/// littered with `speed: 3.1415927` on every node - fixed nodes included, where
/// it is meaningless. A joint that wants a different traverse speed still writes
/// it. Round-trips through [`default_joint_speed`].
#[cfg(feature = "serde")]
fn is_default_joint_speed(speed: &f32) -> bool {
    *speed == default_joint_speed()
}

/// One node of a turret's kinematic joint tree. Recursive. Today's turret is
/// the tree base(fixed) -> yaw(axis Y) -> pitch(axis X) -> barrel(fixed) ->
/// muzzle(fixed, has `muzzle`). Arbitrary arm count / multi-hinge = wider/deeper
/// trees.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurretJoint {
    /// Local translation from the parent joint (section origin for the root).
    pub offset: Vec3,
    /// Local hinge axis. None = fixed node (offsets + may carry mesh/muzzle,
    /// never rotates). Some(axis) = articulated, driven by the aim solver.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub axis: Option<Vec3>,
    /// Rotation speed rad/s (only when `axis` is Some).
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_joint_speed",
            skip_serializing_if = "is_default_joint_speed"
        )
    )]
    pub speed: f32,
    /// Lower rotation limit in radians (only when `axis` is Some).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub min: Option<f32>,
    /// Upper rotation limit in radians (only when `axis` is Some).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max: Option<f32>,
    /// This joint's render mesh; None = a generic default primitive.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh: Option<AssetRef<WorldAsset>>,
    /// Optional transform applied to THIS joint's render mesh only (position +
    /// rotation), relative to the joint frame. None = the mesh sits at the joint
    /// origin (unchanged behavior). Does not affect the joint's kinematics.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub render_mesh_transform: Option<RenderMeshTransform>,
    /// Present iff this joint is a fire point.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub muzzle: Option<MuzzleConfig>,
    /// Child joints, composed in this joint's ROTATED frame.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub children: Vec<TurretJoint>,
}

/// Configuration for a turret section of a spaceship.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurretSectionConfig {
    /// The turret's kinematic joint tree (base -> ... -> muzzle). Replaces the
    /// old flat yaw/pitch/offset/render-mesh fields (spike 20260717-214834).
    pub root: TurretJoint,
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
    /// The sound played when this turret fires a round. An authorable
    /// [`AssetRef<AudioSource>`] like the render meshes and muzzle effect, so a
    /// section (base or mod) can ship and reference its own weapon sound through
    /// the same `self://`/`dep://` scheme pipeline (task 20260717-002228).
    /// AUTHORED-OR-SILENT (spike 20260717-101524): `None` means the turret fires
    /// silently - base turrets author `self://sounds/turret_fire.wav` via
    /// gen_content, so the stock game is unchanged. Snapshotted (unresolved) at
    /// spawn onto a `TurretSectionFireSound` on the turret entity; the audio
    /// observer resolves and plays it. All throttle/attenuation/positioning is
    /// unchanged.
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub fire_sound: Option<AssetRef<AudioSource>>,
    /// The dry-fire click when this turret pulls its trigger on an empty
    /// magazine. Authorable like [`Self::fire_sound`] (task 20260717-101624):
    /// snapshotted onto the turret as `TurretSectionDryFireSound`, resolved by
    /// the audio cue. `None` means no click (authored-or-silent).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dry_fire_sound: Option<AssetRef<AudioSource>>,
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
            // The same kinematic chain the flat config used to build:
            // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) -> muzzle.
            root: TurretJoint {
                offset: Vec3::new(0.0, -0.5, 0.0),
                axis: None,
                speed: default_joint_speed(),
                min: None,
                max: None,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: Vec3::new(0.0, 0.1, 0.0),
                    axis: Some(Vec3::Y),
                    speed: std::f32::consts::PI, // 180 degrees per second
                    min: None,
                    max: None,
                    render_mesh: None,
                    render_mesh_transform: None,
                    muzzle: None,
                    children: vec![TurretJoint {
                        offset: Vec3::new(0.0, 0.2, 0.0),
                        axis: Some(Vec3::X),
                        speed: std::f32::consts::PI, // 180 degrees per second
                        min: Some(-std::f32::consts::FRAC_PI_6),
                        max: Some(std::f32::consts::FRAC_PI_2),
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: None,
                        children: vec![TurretJoint {
                            offset: Vec3::new(0.1, 0.2, 0.0),
                            axis: None,
                            speed: default_joint_speed(),
                            min: None,
                            max: None,
                            render_mesh: None,
                            render_mesh_transform: None,
                            muzzle: None,
                            children: vec![TurretJoint {
                                offset: Vec3::new(0.0, 0.0, -0.5),
                                axis: None,
                                speed: default_joint_speed(),
                                min: None,
                                max: None,
                                render_mesh: None,
                                render_mesh_transform: None,
                                muzzle: Some(MuzzleConfig {
                                    fire_rate: 100.0,
                                    muzzle_effect: None,
                                }),
                                children: vec![],
                            }],
                        }],
                    }],
                }],
            },
            muzzle_speed: 100.0,
            projectile_lifetime: 5.0,
            // Matches the old emergent kinetic (mass 0.1 @ muzzle 100 u/s).
            bullet_damage: representative_kinetic_damage(0.1, 100.0),
            bullet_kind: DamageType::Kinetic,
            projectile_render_mesh: None,
            fire_sound: None,
            dry_fire_sound: None,
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

/// A turret joint entity: the runtime of one [`TurretJoint`] node. Articulated
/// joints (axis Some) also carry a [`SmoothLookRotation`]. Paired with a
/// [`TurretSectionPartOf`] pointing at the turret section root.
#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretJointMarker {
    axis: Option<Vec3>,
}

/// This joint's render mesh (generic; was the per-type `*RenderMesh` zoo).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TurretJointRenderMesh(#[reflect(ignore)] Option<AssetRef<WorldAsset>>);

/// The authored transform for this joint's render mesh, snapshotted from
/// [`TurretJoint::render_mesh_transform`] so the render observer can apply it to
/// the mesh child without re-reading the joint tree.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
struct TurretJointRenderMeshTransform(Option<RenderMeshTransform>);

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

/// A turret's authored fire sound, snapshotted from
/// [`TurretSectionConfig::fire_sound`] onto the turret section entity at spawn -
/// the UNRESOLVED [`AssetRef`], exactly like [`TurretSectionBarrelMuzzleEffect`]
/// carries the unresolved muzzle effect. The audio module resolves it (against
/// its own `AssetServer`, only when it actually plays the cue). Authored-or-
/// silent: `None` (the config left `fire_sound` unset) means no fire sound.
///
/// Carrying the `AssetRef` rather than a resolved `Handle` keeps
/// `insert_turret_section` free of an `AssetServer` dependency (it is registered
/// unconditionally, so many headless section rigs spawn turrets through it);
/// resolution lives with the one system that needs it.
///
/// `pub(crate)` so the audio module can read it, keyed by the firing turret via
/// [`TurretSectionPartOf`] - the same seam the fire cue already uses.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionFireSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

/// The turret's authored dry-fire click, snapshotted UNRESOLVED from
/// [`TurretSectionConfig::dry_fire_sound`] like [`TurretSectionFireSound`];
/// the audio cue resolves it. `pub(crate)` for the audio module.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub(crate) struct TurretSectionDryFireSound(#[reflect(ignore)] pub Option<AssetRef<AudioSource>>);

#[derive(Component, Clone, Copy, Debug, Reflect)]
struct TurretSectionBarrelMuzzleEffectMarker;

/// The entity that represents the muzzle of the turret.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzleEntity(pub Entity);

/// Every muzzle (fire point) of a turret, in tree DFS order. The section-wide
/// fire/aim path iterates these; [`TurretSectionMuzzleEntity`] stays as the
/// PRIMARY muzzle (the first) for the single-point consumers (lead HUD pip, the
/// aim-point lead solve, AI alignment gate).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzles(pub Vec<Entity>);

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
            app.add_observer(insert_turret_joint_render);
            app.add_observer(insert_projectile_render);

            // Hanabi muzzle-flash and projectile-trail effects: run on wasm too
            // now that the web build uses the WebGPU backend.
            app.add_observer(insert_turret_barrel_muzzle_effect);
            app.add_observer(on_projectile_marker_effect);
        }

        app.add_systems(
            Update,
            (apply_turret_config_to_children, sync_turret_joint_rotation)
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
            (update_turret_aim_point, update_turret_target_joints_system)
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
        entity.insert(SmoothLookRotation {
            axis,
            initial: 0.0,
            speed: joint.speed,
            min: joint.min,
            max: joint.max,
        });
    }

    if let Some(muzzle) = &joint.muzzle {
        let interval = 1.0 / muzzle.fire_rate;
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

    let mut muzzles = Vec::new();
    let root = spawn_turret_joint(&mut commands, turret, &config.root, &mut muzzles);

    // The section-wide fire/aim path drives ALL muzzles (task 20260717-215857);
    // the FIRST is the PRIMARY, kept in `TurretSectionMuzzleEntity` for the
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
fn apply_turret_config_to_children(
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

/// The signed angle (radians) that rotates `from` onto `to` about `axis`,
/// measured in the plane perpendicular to `axis`. `axis` is assumed normalized.
///
/// Near-antiparallel `from`/`to` (target dead behind the barrel) is a rotation
/// singularity: either half-turn is valid and the cross product carries no
/// reliable sign, so a naive `atan2` dithers frame to frame and the hinge freezes
/// in place ("stuck, won't turn around"). When the vectors are within ~1.6 deg of
/// opposite, commit to a deterministic +pi half-turn so the joint swings around;
/// once it moves off the singularity the normal solve resumes and finishes the
/// turn the short way.
fn signed_angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let from = from.normalize();
    let to = to.normalize();
    let c = from.cross(to).dot(axis);
    let d = from.dot(to);
    if d < -0.9996 {
        return std::f32::consts::PI;
    }
    c.atan2(d)
}

/// Damping gain on each hinge's per-frame CCD correction. The step target is
/// `output + AIM_CORRECTION_GAIN * delta`, NOT the full `output + delta`. A hinge
/// solves its `delta` assuming every OTHER joint holds still, but a turret's
/// joints are coupled (yaw and pitch both swing the offset muzzle), so applying
/// each full correction at once overshoots the joint solution and settles into a
/// visible limit cycle - the barrel shakes a few degrees around the aim. A gain
/// below 1 makes each joint under-correct, so the coupled system converges
/// monotonically instead of ringing. Large errors are still rate-limited by
/// `SmoothLookRotation` (not the gain), so a slew stays responsive; the gain
/// only shapes the settle. Lowered to 0.35 (from 0.5) after playtest - a
/// stronger damp for extra shake margin; lower further if a deep chain rings.
const AIM_CORRECTION_GAIN: f32 = 0.35;

/// Below this per-frame correction (radians, ~0.23 deg) a hinge is treated as
/// ON target and HOLDS (target = current output) instead of chasing sub-degree
/// residuals - stops the micro-jitter that the lead-point feedback (the aim
/// point moves with the muzzle it is aiming) would otherwise sustain forever.
const AIM_DEADBAND_RAD: f32 = 0.004;

/// Generic aim: a Jacobi per-frame hinge-CCD pass over each turret's muzzle
/// chain (one step per articulated joint, from the muzzle up to the root). Each
/// articulated joint nudges its [`SmoothLookRotationTarget`] toward the angle
/// that swings the muzzle forward (-Z) at the aim point, decomposed in that
/// joint's own hinge plane and DAMPED by [`AIM_CORRECTION_GAIN`] so coupled
/// joints do not overshoot into a shake; the [`SmoothLookRotation`] controller
/// rate-limits and clamps the visible motion. Basis-independent, so it reduces
/// to the old yaw/pitch behavior for the Y/X chain and solves arbitrary trees.
///
/// Same fresh-pose composition as [`update_turret_aim_point`]: runs BEFORE this
/// frame's transform propagation, so poses are composed via [`TransformHelper`].
fn update_turret_target_joints_system(
    q_turret: Query<
        (&TurretSectionAimPoint, &TurretSectionMuzzles),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_child_of: Query<&ChildOf>,
    q_joint: Query<(&TurretJointMarker, &Transform)>,
    mut q_target_mut: Query<&mut SmoothLookRotationTarget>,
    q_output: Query<&SmoothLookRotationOutput>,
    transform_helper: TransformHelper,
) {
    for (aim_point, muzzles) in &q_turret {
        let Some(target) = **aim_point else {
            continue;
        };

        // Steer EVERY muzzle of the turret at the shared aim point (task
        // 20260717-215857). Shared joints (a twin barrel's common yaw/pitch) are
        // written by each muzzle's pass and agree for symmetric barrels; a
        // muzzle's private joints refine only its own chain.
        for &muzzle in &muzzles.0 {
            let Ok(muzzle_transform) = transform_helper.compute_global_transform(muzzle) else {
                error!(
                    "update_turret_target_joints_system: muzzle {:?} has no computable pose",
                    muzzle
                );
                continue;
            };
            let m = muzzle_transform.translation();
            let d: Vec3 = muzzle_transform.forward().into();
            if target == m {
                continue;
            }

            // Walk from the muzzle up to the turret root via ChildOf, collecting
            // the articulated joints along the chain. A single Jacobi pass: every
            // joint steps from the CURRENT pose this frame.
            let mut chain = muzzle;
            while let Ok(&ChildOf(parent)) = q_child_of.get(chain) {
                // Only walk while the current node is a turret joint; the parent
                // of the root joint is the turret section entity (not a joint),
                // which stops the walk.
                let Ok((marker, joint_transform)) = q_joint.get(chain) else {
                    break;
                };

                if let Some(a_local) = marker.axis {
                    // Parent global pose, then the joint's pre-rotation frame F:
                    // parent orientation + this joint's origin (offset applied).
                    if let Ok(parent_global) = transform_helper.compute_global_transform(parent) {
                        let f = parent_global
                            * Transform::from_translation(joint_transform.translation);
                        let w2j = f.to_matrix().inverse();

                        let ml = w2j.transform_point3(m);
                        let dl = w2j.transform_vector3(d);
                        let tl = w2j.transform_point3(target);
                        let a = a_local.normalize();

                        let des = tl - ml;
                        let d_perp = dl - a * dl.dot(a);
                        let t_perp = des - a * des.dot(a);
                        if d_perp.length() > 1e-6 && t_perp.length() > 1e-6 {
                            let delta = signed_angle_about(d_perp, t_perp, a);
                            let out = q_output.get(chain).map(|o| **o).unwrap_or(0.0);
                            if let Ok(mut target_angle) = q_target_mut.get_mut(chain) {
                                // Damp the correction so coupled joints converge
                                // instead of ringing; hold once aimed (deadband).
                                **target_angle = if delta.abs() <= AIM_DEADBAND_RAD {
                                    out
                                } else {
                                    out + AIM_CORRECTION_GAIN * delta
                                };
                            }
                        }
                    }
                }

                chain = parent;
            }
        }
    }
}

/// Apply each articulated joint's controller output onto its own transform's
/// rotation (fixed joints keep identity). One generic sync, replacing the
/// per-yaw/per-pitch pair - the joint entity carries both the offset transform
/// and the controller, so the hinge rotation is written back onto the SAME
/// transform.
fn sync_turret_joint_rotation(
    mut q_joint: Query<(
        &TurretJointMarker,
        &SmoothLookRotationOutput,
        &mut Transform,
    )>,
) {
    for (marker, output, mut transform) in &mut q_joint {
        if let Some(axis) = marker.axis {
            transform.rotation = Quat::from_axis_angle(axis.normalize(), **output);
        }
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
            &TurretSectionMuzzles,
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
    for (turret, muzzles, ChildOf(spaceship), config, loaded, input, mut ammo) in &mut q_turret {
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

        // The spaceship pose is a per-TURRET quantity: every muzzle spawns
        // relative to the same root avian pose, so read it once before the
        // muzzle loop (task 20260717-215857).
        let Ok((position, rotation, lin_vel, ang_vel, center, allegiance)) =
            q_spaceship.get(*spaceship)
        else {
            error!(
                "shoot_spawn_projectile: entity {:?} not found in q_spaceship",
                spaceship
            );
            continue;
        };

        // Copy the muzzle Entity list out of the component BEFORE the inner
        // loop, so `q_muzzle` (fire timers) and `commands` are free to borrow
        // while we iterate. Every muzzle draws from the ONE `SectionAmmo` below:
        // a shared magazine, not a per-barrel one.
        let muzzle_entities: Vec<Entity> = muzzles.0.clone();
        for muzzle in muzzle_entities {
            let Ok(mut fire_state) = q_muzzle.get_mut(muzzle) else {
                error!(
                    "shoot_spawn_projectile: entity {:?} not found in q_muzzle",
                    muzzle
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

            // Out of ammo: the ONE shared magazine gates every muzzle. A mag that
            // empties on an earlier muzzle's burst this tick stops the later ones
            // too. A turret with no `SectionAmmo` (unlimited) is never gated here,
            // so the pre-ammo behavior is untouched.
            if ammo.as_deref().is_some_and(SectionAmmo::is_empty) {
                continue;
            }

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
                local_pose_in_root(muzzle, *spaceship, &q_chain)
            else {
                error!(
                    "shoot_spawn_projectile: muzzle {:?} is not a descendant of ship {:?}",
                    muzzle, spaceship
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
                        AngularInertia::from_shape(
                            &Collider::sphere(0.05),
                            NEUTRALIZED_BULLET_MASS,
                        ),
                        // The fired round comes from the turret's loaded-ammo slot,
                        // not a hardcoded type (task 20260712-133349), so a future
                        // ammo switch changes what this stamps.
                        ProjectileDamage {
                            amount: bullet_damage,
                            kind: bullet_kind,
                        },
                    ),
                    TurretSectionPartOf(turret),
                    TurretSectionMuzzleEntity(muzzle),
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
    if !budget.as_deref().is_none_or(|b| b.particles) {
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

/// Generic joint render (replaces the four per-type render observers). Fires on
/// `Add, TurretJointMarker` (gated by `self.render`). If the joint authored a
/// mesh, spawn a `WorldAssetRoot` child; otherwise spawn a small generic
/// default primitive so an unmeshed joint is still visible. The old bespoke
/// per-type placeholder art (ridged yaw/pitch cylinders, layered barrel shape)
/// is dropped in favor of one default; shipped turrets author GLB meshes so the
/// visible game is unchanged.
fn insert_turret_joint_render(
    add: On<Add, TurretJointMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_joint: Query<
        (
            &TurretSectionPartOf,
            &TurretJointRenderMesh,
            &TurretJointRenderMeshTransform,
            Has<TurretSectionBarrelMuzzleMarker>,
        ),
        With<TurretJointMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_turret_joint_render: entity {:?}", entity);

    let Ok((turret, render_mesh, render_mesh_transform, is_muzzle)) = q_joint.get(entity) else {
        error!(
            "insert_turret_joint_render: entity {:?} not found in q_joint",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            // Authored render-mesh transform, or identity (mesh at the joint
            // origin) when unset. It lives on the mesh CHILD, so it moves only
            // the art, never the joint's kinematic frame.
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Joint"),
                transform,
                SectionRenderOf(**turret),
                WorldAssetRoot(scene),
            ),],));
        }
        // A muzzle is an invisible fire point (the original never rendered it);
        // only a STRUCTURAL unmeshed joint (the base plate) gets a default
        // primitive so the mount is not floating meshes with a gap under it. The
        // shape matches the pre-refactor base plate (a wide flat disc slightly
        // above the joint origin) so an unmeshed base looks exactly as it did.
        None if !is_muzzle => {
            commands.entity(entity).insert((children![(
                Name::new("Render Turret Joint"),
                Transform::from_xyz(0.0, 0.05, 0.0),
                SectionRenderOf(**turret),
                Mesh3d(meshes.add(Cylinder::new(0.5, 0.1))),
                MeshMaterial3d(materials.add(Color::srgb(0.25, 0.25, 0.25))),
            ),],));
        }
        None => {}
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
    if !budget.as_deref().is_none_or(|b| b.particles) {
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
        // The default turret's single muzzle fires at 100 rounds/s.
        let interval = 1.0 / 100.0;
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
        world.entity_mut(turret).insert((
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(vec![muzzle]),
        ));
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

    /// The number of fired bullets stamped with each given muzzle entity.
    fn bullets_per_muzzle(app: &mut App, muzzles: &[Entity]) -> Vec<usize> {
        let stamped: Vec<Entity> = app
            .world_mut()
            .query_filtered::<&TurretSectionMuzzleEntity, With<TurretBulletProjectileMarker>>()
            .iter(app.world())
            .map(|m| **m)
            .collect();
        muzzles
            .iter()
            .map(|&muzzle| stamped.iter().filter(|&&s| s == muzzle).count())
            .collect()
    }

    #[test]
    fn a_twin_barrel_fires_both_muzzles_over_one_shared_magazine() {
        // MULTI-MUZZLE + SHARED MAG (task 20260717-215857): a turret whose barrel
        // joint carries TWO muzzles fires BOTH, and both draw from the ONE section
        // magazine. The key claim is the SHARED magazine: N muzzles do NOT each get
        // their own ammo pool - a 3-round mag yields 3 bullets TOTAL across both
        // barrels, not 3 per barrel (6). Built via the spawn observer so both
        // muzzle entities, their fire timers and `TurretSectionMuzzles` all exist.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0),
        ));
        app.add_observer(insert_turret_section);
        app.add_systems(Update, shoot_spawn_projectile);

        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) with TWO muzzle
        // children at symmetric lateral offsets. fire_rate 10, shared mag of 3.
        let muzzle = |x: f32| TurretJoint {
            offset: Vec3::new(x, 0.0, -0.5),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        let barrel = TurretJoint {
            offset: Vec3::new(0.0, 0.0, 0.0),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![muzzle(0.1), muzzle(-0.1)],
        };
        let config = TurretSectionConfig {
            root: barrel,
            ammo_capacity: Some(3),
            ..default()
        };

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            TurretSectionInput(true),
        ));
        app.world_mut().flush();

        // The two muzzle entities the observer recorded, in DFS order.
        let muzzles = app
            .world()
            .entity(turret)
            .get::<TurretSectionMuzzles>()
            .expect("the turret records its muzzles")
            .0
            .clone();
        assert_eq!(muzzles.len(), 2, "the twin barrel must record two muzzles");

        // Hold the trigger for far more ticks than the magazine can supply.
        for _ in 0..10 {
            app.update();
        }

        let per = bullets_per_muzzle(&mut app, &muzzles);
        assert!(
            per[0] > 0 && per[1] > 0,
            "both muzzles must produce bullets, got {per:?}"
        );
        assert_eq!(
            per[0] + per[1],
            3,
            "the magazine is SHARED: 3 rounds total across both barrels, not per \
             barrel, got {per:?}"
        );
        assert_eq!(
            bullet_count(&mut app),
            3,
            "exactly the shared magazine's worth of bullets ever spawn"
        );
        let ammo = app
            .world()
            .entity(turret)
            .get::<SectionAmmo>()
            .expect("the turret keeps its magazine");
        assert_eq!(
            ammo.rounds, 0,
            "the shared magazine reads empty after firing out"
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
    fn insert_turret_section_snapshots_the_configs_fire_sound_onto_the_turret() {
        // The declaration half of the section-authored audio seam (task
        // 20260717-002228): a turret whose CONFIG declares `fire_sound` must carry
        // that UNRESOLVED ref as a `TurretSectionFireSound` after the build
        // observer runs, so the audio module can resolve + play it. Pairs with the
        // audio-module test that resolves the component and plays its handle - the
        // two halves marry declaration -> component -> resolved playback. No
        // `AssetServer` needed here: the snapshot carries the ref, not a handle.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(insert_turret_section);

        let with_sound = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                fire_sound: Some(AssetRef::from("base/sounds/turret_fire.wav")),
                dry_fire_sound: Some(AssetRef::from("base/sounds/dry_fire.wav")),
                ..default()
            }))
            .id();
        let without_sound = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().flush();

        assert_eq!(
            app.world()
                .entity(with_sound)
                .get::<TurretSectionFireSound>()
                .and_then(|s| s.0.as_ref())
                .and_then(|r| r.path()),
            Some("base/sounds/turret_fire.wav"),
            "the declared fire_sound must be snapshotted onto the turret"
        );
        assert_eq!(
            app.world()
                .entity(with_sound)
                .get::<TurretSectionDryFireSound>()
                .and_then(|s| s.0.as_ref())
                .and_then(|r| r.path()),
            Some("base/sounds/dry_fire.wav"),
            "the declared dry_fire_sound must be snapshotted onto the turret"
        );
        // The snapshot is unconditional (None passes through), so the audio side
        // reads one component shape whether or not a sound was authored.
        assert_eq!(
            app.world()
                .entity(without_sound)
                .get::<TurretSectionFireSound>()
                .map(|s| s.0.is_none()),
            Some(true),
            "a turret without a fire_sound still carries the component as None"
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

    /// The muzzle joint entity of `turret`.
    fn muzzle_entity(app: &App, turret: Entity) -> Entity {
        **app
            .world()
            .get::<TurretSectionMuzzleEntity>(turret)
            .expect("the turret records its muzzle")
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
            TurretSectionMuzzles(vec![muzzle]),
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
                // The muzzle child below carries the fire timer directly; the
                // config's per-muzzle rate is unused by this rig.
                TurretSectionConfigHelper(TurretSectionConfig {
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
        app.world_mut().entity_mut(turret).insert((
            TurretSectionMuzzleEntity(muzzle),
            TurretSectionMuzzles(vec![muzzle]),
        ));
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

    // -- joint tree (spike 20260717-214834) --

    /// Collect every joint entity of `turret` in DFS order (root first).
    fn joint_entities(app: &App) -> Vec<Entity> {
        let world = app.world();
        world
            .iter_entities()
            .filter(|e| world.get::<TurretJointMarker>(e.id()).is_some())
            .map(|e| e.id())
            .collect()
    }

    #[test]
    fn every_turret_joint_render_child_is_parented_to_its_joint() {
        // BASE-FLOATING REGRESSION: the base (and every unmeshed fixed joint)
        // renders a default primitive as a CHILD of the joint entity. If that
        // render child is not actually parented (ChildOf == joint), it drifts to
        // world origin instead of riding the ship - the "base floats" report.
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_observer(insert_turret_section);
        app.add_observer(insert_turret_joint_render);

        // Place the turret far from the world origin, like a section on a flying
        // ship. If a render child is detached, its GlobalTransform stays near the
        // origin - hundreds of units from where the turret actually is.
        let ship_pos = Vec3::new(100.0, 50.0, 200.0);
        let turret = app
            .world_mut()
            .spawn((turret_section(TurretSectionConfig::default()),))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert(Transform::from_translation(ship_pos));
        app.world_mut().flush();
        app.update(); // propagate transforms

        for joint in joint_entities(&app) {
            let render_children: Vec<Entity> = app
                .world()
                .get::<Children>(joint)
                .map(|c| {
                    c.iter()
                        .filter(|&e| app.world().get::<SectionRenderOf>(e).is_some())
                        .collect()
                })
                .unwrap_or_default();
            // A muzzle is an invisible fire point (no render child); every other
            // joint renders and must be parented on the ship.
            if app
                .world()
                .get::<TurretSectionBarrelMuzzleMarker>(joint)
                .is_some()
            {
                assert!(
                    render_children.is_empty(),
                    "muzzle joint {joint:?} should be invisible but has a render child"
                );
                continue;
            }
            assert!(
                !render_children.is_empty(),
                "joint {joint:?} has no render child"
            );
            for rc in render_children {
                let parent = app.world().get::<ChildOf>(rc).map(|c| c.0);
                assert_eq!(
                    parent,
                    Some(joint),
                    "render child {rc:?} is not parented to its joint {joint:?}"
                );
                // The whole turret assembly spans ~2 units; a correctly mounted
                // render child sits within that of the turret's world position.
                let world = app
                    .world()
                    .get::<GlobalTransform>(rc)
                    .map(|g| g.translation())
                    .unwrap_or(Vec3::ZERO);
                assert!(
                    world.distance(ship_pos) < 5.0,
                    "render child {rc:?} of joint {joint:?} is at {world:?}, {} units \
                     from the turret at {ship_pos:?} - it floats",
                    world.distance(ship_pos)
                );
            }
        }

        // FOLLOW CHECK: move the turret (like a flying ship) and confirm every
        // render child rode along instead of staying behind at the old spot - a
        // detached child "floats" in place while the ship flies off.
        let moved = Vec3::new(-400.0, 900.0, -50.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(Transform::from_translation(moved));
        app.update();
        for joint in joint_entities(&app) {
            let render_children: Vec<Entity> = app
                .world()
                .get::<Children>(joint)
                .map(|c| {
                    c.iter()
                        .filter(|&e| app.world().get::<SectionRenderOf>(e).is_some())
                        .collect()
                })
                .unwrap_or_default();
            for rc in render_children {
                let world = app
                    .world()
                    .get::<GlobalTransform>(rc)
                    .map(|g| g.translation())
                    .unwrap_or(Vec3::ZERO);
                assert!(
                    world.distance(moved) < 5.0,
                    "render child {rc:?} of joint {joint:?} did not follow the turret \
                     to {moved:?} (it is at {world:?}) - it floats"
                );
            }
        }
    }

    /// A meshed joint's render child carries the authored
    /// `render_mesh_transform` (task 20260718-113307), and a meshed joint that
    /// omits it gets an identity transform - the pre-feature behavior. This is
    /// the load-bearing wiring: the transform must land on the mesh CHILD, not
    /// the joint entity (whose transform is the kinematic frame).
    #[test]
    fn render_mesh_transform_positions_the_meshed_render_child() {
        use bevy::asset::AssetPlugin;

        // A one-joint turret (fixed root + a muzzle leaf so it is a valid
        // turret) whose root carries a mesh and the given transform.
        let turret_with = |xf: Option<RenderMeshTransform>| TurretSectionConfig {
            root: TurretJoint {
                offset: Vec3::ZERO,
                axis: None,
                speed: default_joint_speed(),
                min: None,
                max: None,
                render_mesh: Some(AssetRef::from("gltf/turret-yaw-01.glb#Scene0".to_string())),
                render_mesh_transform: xf,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: Vec3::new(0.0, 0.0, -0.5),
                    axis: None,
                    speed: default_joint_speed(),
                    min: None,
                    max: None,
                    render_mesh: None,
                    render_mesh_transform: None,
                    muzzle: Some(MuzzleConfig {
                        fire_rate: 100.0,
                        muzzle_effect: None,
                    }),
                    children: vec![],
                }],
            },
            ..Default::default()
        };

        // The single WorldAssetRoot (meshed) render child's local Transform.
        let meshed_child_transform = |xf: Option<RenderMeshTransform>| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_asset::<WorldAsset>();
            app.add_observer(insert_turret_section);
            app.add_observer(insert_turret_joint_render);
            app.world_mut().spawn((turret_section(turret_with(xf)),));
            app.world_mut().flush();
            app.update();

            let world = app.world_mut();
            let mut q =
                world.query_filtered::<&Transform, (With<SectionRenderOf>, With<WorldAssetRoot>)>();
            let found: Vec<Transform> = q.iter(world).copied().collect();
            assert_eq!(found.len(), 1, "exactly one meshed render child expected");
            found[0]
        };

        // Authored transform lands verbatim on the mesh child.
        let authored = RenderMeshTransform {
            position: Vec3::new(0.1, 0.2, 0.3),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        };
        let got = meshed_child_transform(Some(authored));
        assert_eq!(got.translation, authored.position);
        assert!(
            got.rotation.abs_diff_eq(authored.rotation, 1e-5),
            "render child rotation {:?} != authored {:?}",
            got.rotation,
            authored.rotation
        );
        assert_eq!(
            got.scale,
            Vec3::ONE,
            "render transform must not touch scale"
        );

        // No authored transform => identity child (unchanged pre-feature look).
        let got = meshed_child_transform(None);
        assert_eq!(got, Transform::IDENTITY);
    }

    #[test]
    fn render_mesh_transform_type_defaults_and_round_trips() {
        // Default is identity: an omitted field must reproduce the old look.
        assert_eq!(
            RenderMeshTransform::default().to_transform(),
            Transform::IDENTITY
        );
        let xf = RenderMeshTransform {
            position: Vec3::new(1.0, -2.0, 0.5),
            rotation: Quat::from_rotation_x(0.5),
        };
        let t = xf.to_transform();
        assert_eq!(t.translation, xf.position);
        assert!(t.rotation.angle_between(xf.rotation) < 1e-6);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn render_mesh_transform_serde_round_trips_and_omits_defaults() {
        // Full round-trip.
        let xf = RenderMeshTransform {
            position: Vec3::new(0.1, 0.2, 0.3),
            rotation: Quat::from_rotation_z(0.25),
        };
        let ron = ron::ser::to_string(&xf).expect("serialize");
        let back: RenderMeshTransform = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back, xf);

        // Rotation-only authoring: the zero position is not serialized, and a
        // string with only `rotation` still deserializes (position defaults).
        let rot_only = RenderMeshTransform {
            position: Vec3::ZERO,
            rotation: Quat::from_rotation_y(0.3),
        };
        let ron = ron::ser::to_string(&rot_only).expect("serialize");
        assert!(
            !ron.contains("position"),
            "zero position must be omitted: {ron}"
        );
        let back: RenderMeshTransform = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back, rot_only);

        // A joint that omits render_mesh_transform entirely does not serialize
        // the field (keeps authored turrets and RON parity unchanged).
        let joint = TurretJoint {
            offset: Vec3::ZERO,
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![],
        };
        let ron = ron::ser::to_string(&joint).expect("serialize");
        assert!(
            !ron.contains("render_mesh_transform"),
            "unset render_mesh_transform must not serialize: {ron}"
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
        assert_eq!(pitch_rot.min, Some(-std::f32::consts::FRAC_PI_6));
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

    /// An app that runs the full aim + controller + sync + propagation loop on a
    /// manual clock, so a stepped turret converges its muzzle onto a target.
    fn aim_convergence_app() -> App {
        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::SmoothLookRotationPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SmoothLookRotationPlugin));
        app.add_observer(insert_turret_section);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        // The aim chain runs in PostUpdate BEFORE the controller's Sync (matching
        // production ordering is not required - either order is a stable servo).
        app.add_systems(
            PostUpdate,
            (update_turret_aim_point, update_turret_target_joints_system)
                .chain()
                .before(SmoothLookRotationSystems::Sync),
        );
        app.add_systems(Update, sync_turret_joint_rotation);
        app
    }

    /// The world-space muzzle forward (-Z) after propagation.
    fn muzzle_aim_error_deg(app: &mut App, muzzle: Entity, target: Vec3) -> f32 {
        let gt = *app.world().get::<GlobalTransform>(muzzle).unwrap();
        let pos = gt.translation();
        let forward: Vec3 = gt.forward().into();
        let to_target = (target - pos).normalize();
        forward.dot(to_target).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_default_turret_converges_its_muzzle_onto_a_target() {
        // AIM CONVERGENCE (behavioral CCD parity): a default turret steered at a
        // reachable target must point its muzzle forward (-Z) at the target
        // within a few degrees after stepping frames. This is the parity
        // guarantee for the CCD swap - proven behaviorally, not by theta match.
        for target in [
            Vec3::new(10.0, 5.0, -30.0),
            Vec3::new(-15.0, 8.0, -25.0),
            Vec3::new(0.0, 3.0, -40.0),
        ] {
            let mut app = aim_convergence_app();
            let ship = app
                .world_mut()
                .spawn((SpaceshipRootMarker, Transform::IDENTITY))
                .id();
            let turret = app
                .world_mut()
                .spawn(turret_section(TurretSectionConfig {
                    muzzle_speed: 1000.0, // near-straight lead so aim ~= target dir
                    ..Default::default()
                }))
                .id();
            app.world_mut()
                .entity_mut(turret)
                .insert((ChildOf(ship), Transform::IDENTITY));
            app.world_mut().flush();
            app.world_mut()
                .entity_mut(turret)
                .insert(TurretSectionTargetInput(Some(target)));
            let muzzle = muzzle_entity(&app, turret);

            for _ in 0..240 {
                app.update();
            }

            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "muzzle should converge onto {target:?}, aim error {error} deg"
            );
        }
    }

    #[test]
    fn a_turret_swings_around_to_a_target_directly_behind() {
        // STUCK-BEHIND REGRESSION: a target dead behind the barrel is a rotation
        // singularity (either way is a valid 180); the naive solve dithered and
        // the turret froze facing forward. It must commit and swing around.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: 1000.0,
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        // Barrel rest points -Z; put the target dead behind at +Z (a hair off the
        // axis so it is a true 180, level so pitch can hold it).
        let target = Vec3::new(0.0, 0.0, 30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..240 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 5.0,
            "turret must swing around to a target behind, not freeze forward: \
             aim error {error} deg"
        );
    }

    #[test]
    fn an_aimed_turret_holds_steady_without_shaking() {
        // NO-SHAKE REGRESSION (task 20260717-214834 follow-up): an undamped CCD
        // step (`target = output + delta`) made the coupled yaw+pitch overshoot
        // and settle into a ~4-5 deg limit cycle - the barrel visibly shook
        // around the aim. With the damping gain + deadband the muzzle must HOLD:
        // after it converges, its aim error must barely move frame to frame.
        let target = Vec3::new(8.0, 6.0, -28.0);
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig {
                muzzle_speed: 1000.0,
                ..Default::default()
            }))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        // Converge.
        for _ in 0..240 {
            app.update();
        }

        // Then watch the aim error for 120 more frames: its peak-to-peak swing is
        // the shake amplitude. A limit cycle would read several degrees; a held
        // turret reads a fraction of one.
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for _ in 0..120 {
            app.update();
            let e = muzzle_aim_error_deg(&mut app, muzzle, target);
            min = min.min(e);
            max = max.max(e);
        }
        let peak_to_peak = max - min;
        assert!(
            peak_to_peak < 0.5,
            "an aimed turret must hold steady, not shake: aim error swings \
             {peak_to_peak} deg (min {min}, max {max})"
        );
    }

    #[test]
    fn a_three_hinge_tree_reaches_a_target_a_two_dof_turret_cannot() {
        // MULTI-HINGE: a hand-built 3-hinge arm (Y at base, then X, then Y two
        // down) converges onto a target, sanity that arbitrary chains solve.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // Y -> X -> Y -> muzzle, each a link one unit long, generous limits.
        let hinge = |axis: Vec3, offset: Vec3, children: Vec<TurretJoint>| TurretJoint {
            offset,
            axis: Some(axis),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children,
        };
        let root = hinge(
            Vec3::Y,
            Vec3::ZERO,
            vec![hinge(
                Vec3::X,
                Vec3::new(0.0, 1.0, 0.0),
                vec![hinge(
                    Vec3::Y,
                    Vec3::new(0.0, 1.0, 0.0),
                    vec![TurretJoint {
                        offset: Vec3::new(0.0, 0.0, -1.0),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: Some(MuzzleConfig {
                            fire_rate: 10.0,
                            muzzle_effect: None,
                        }),
                        children: vec![],
                    }],
                )],
            )],
        );

        let config = TurretSectionConfig {
            root,
            muzzle_speed: 1000.0,
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(6.0, -4.0, 8.0); // behind + below: needs the extra DOF
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzle = muzzle_entity(&app, turret);

        for _ in 0..600 {
            app.update();
        }

        let error = muzzle_aim_error_deg(&mut app, muzzle, target);
        assert!(
            error < 8.0,
            "a 3-hinge arm must converge onto {target:?}, aim error {error} deg"
        );
    }

    #[test]
    fn a_shared_chain_twin_barrel_points_both_muzzles_at_the_target() {
        // MULTI-MUZZLE AIM (task 20260717-215857): a twin barrel that shares its
        // yaw+pitch chain (two muzzles hanging off one common barrel) steers BOTH
        // muzzles onto the target - the shared joints are written by each muzzle's
        // CCD pass and agree for the symmetric barrels.
        let mut app = aim_convergence_app();
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, Transform::IDENTITY))
            .id();

        // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) with TWO muzzles.
        let muzzle = |x: f32| TurretJoint {
            offset: Vec3::new(x, 0.0, -0.5),
            axis: None,
            speed: default_joint_speed(),
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        let yaw = TurretJoint {
            offset: Vec3::ZERO,
            axis: Some(Vec3::Y),
            speed: std::f32::consts::TAU,
            min: Some(-std::f32::consts::PI),
            max: Some(std::f32::consts::PI),
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: None,
            children: vec![TurretJoint {
                offset: Vec3::ZERO,
                axis: Some(Vec3::X),
                speed: std::f32::consts::TAU,
                min: Some(-std::f32::consts::PI),
                max: Some(std::f32::consts::PI),
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children: vec![muzzle(0.1), muzzle(-0.1)],
            }],
        };

        let config = TurretSectionConfig {
            root: yaw,
            muzzle_speed: 1000.0, // near-straight lead so aim ~= target dir
            ..default()
        };
        let turret = app.world_mut().spawn(turret_section(config)).id();
        app.world_mut()
            .entity_mut(turret)
            .insert((ChildOf(ship), Transform::IDENTITY));
        app.world_mut().flush();

        let target = Vec3::new(10.0, 5.0, -30.0);
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetInput(Some(target)));
        let muzzles = app
            .world()
            .entity(turret)
            .get::<TurretSectionMuzzles>()
            .expect("the turret records its muzzles")
            .0
            .clone();
        assert_eq!(muzzles.len(), 2, "the twin barrel must record two muzzles");

        for _ in 0..240 {
            app.update();
        }

        for muzzle in muzzles {
            let error = muzzle_aim_error_deg(&mut app, muzzle, target);
            assert!(
                error < 5.0,
                "both muzzles must converge onto {target:?}, muzzle {muzzle:?} \
                 aim error {error} deg"
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_turret_joint_tree_survives_a_ron_round_trip() {
        // RON ROUND-TRIP: a tree config serializes + deserializes back to an
        // equal tree (the authored content path).
        let config = TurretSectionConfig::default();
        let ron = ron::ser::to_string(&config.root).expect("serialize");
        let back: TurretJoint = ron::from_str(&ron).expect("deserialize");
        // Compare structurally via a re-serialize (TurretJoint has no PartialEq).
        let ron_back = ron::ser::to_string(&back).expect("re-serialize");
        assert_eq!(ron, ron_back, "the tree must round-trip unchanged");
    }
}
