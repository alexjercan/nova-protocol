//! Authoring types for a turret: the kinematic joint tree, its muzzles, and
//! the section config the tree hangs off.

use bevy::prelude::*;
use bevy_hanabi::prelude::EffectAsset;
use nova_gameplay::prelude::*;

use crate::prelude::*;

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
pub(super) fn default_joint_speed() -> f32 {
    std::f32::consts::PI
}

/// How far the default pitch hinge may DEPRESS below level (10 deg). A turret
/// stands on a hull, so a deeper floor only aims the barrel back into its own
/// ship; elevation is unclamped to 90 deg for the point-defense arc.
const DEFAULT_TURRET_DEPRESSION_LIMIT: f32 = std::f32::consts::PI / 18.0;

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
    /// old flat yaw/pitch/offset/render-mesh fields.
    pub root: TurretJoint,
    /// The muzzle speed of the turret in units per second.
    pub muzzle_speed: f32,
    /// The projectile lifetime
    pub projectile_lifetime: f32,
    /// Authored Kinetic damage per hit (pre-resistance). Weapon damage is
    /// AUTHORED here, not emergent from bullet mass x velocity: the bullet is
    /// spawned at a near-zero physical
    /// mass ([`NEUTRALIZED_BULLET_MASS`]) so bcs's kinetic term vanishes, and
    /// this fixed amount (scaled by the section resistance table) is the only
    /// weapon damage. Kinetic resistance is 1.0 everywhere, so authoring this to
    /// the old emergent per-hit (via [`representative_kinetic_damage`]) keeps the
    /// turret's feel unchanged.
    pub bullet_damage: f32,
    /// Damage TYPE of the round this turret is loaded with. The authoring
    /// default for the turret's [`LoadedBullet`] slot; the fired
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
    /// the same `self://`/`dep://` scheme pipeline. AUTHORED-OR-SILENT: `None`
    /// means the turret fires silently - base turrets author
    /// `self://sounds/turret_fire.wav` via
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
    /// magazine. Authorable like [`Self::fire_sound`]: snapshotted onto the
    /// turret as `TurretSectionDryFireSound`, resolved by
    /// the audio cue. `None` means no click (authored-or-silent).
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dry_fire_sound: Option<AssetRef<AudioSource>>,
    /// Magazine size in rounds. `None` fires without limit (the pre-ammo
    /// behavior); `Some(n)` gives the turret a [`SectionAmmo`] of `n` rounds
    /// that depletes one per bullet and blocks firing once empty.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ammo_capacity: Option<u32>,
    /// Auto-reload for the magazine. `None` = no reload (a spent magazine stays
    /// empty - the pre-reload behavior). `Some` attaches a [`SectionReload`]
    /// alongside the `SectionAmmo`, so it only applies when `ammo_capacity` is
    /// also `Some`; an unlimited turret never reloads.
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
                        // Turrets mount ON a hull, so the depression floor
                        // stops the barrel aiming back into it; elevation
                        // stays at 90 for the point-defense arc.
                        min: Some(-DEFAULT_TURRET_DEPRESSION_LIMIT),
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

#[cfg(test)]
mod tests {
    use super::*;

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
            scale: Vec3::splat(0.4),
        };
        let t = xf.to_transform();
        assert_eq!(t.translation, xf.position);
        assert!(t.rotation.angle_between(xf.rotation) < 1e-6);
        assert_eq!(t.scale, xf.scale);

        // The default SCALE is one, not zero. A derived `Default` would scale
        // every unauthored mesh in the game to nothing.
        assert_eq!(RenderMeshTransform::default().scale, Vec3::ONE);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn render_mesh_transform_serde_round_trips_and_omits_defaults() {
        // Full round-trip.
        let xf = RenderMeshTransform {
            position: Vec3::new(0.1, 0.2, 0.3),
            rotation: Quat::from_rotation_z(0.25),
            scale: Vec3::splat(0.5),
        };
        let ron = ron::ser::to_string(&xf).expect("serialize");
        let back: RenderMeshTransform = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back, xf);

        // An unscaled mesh omits the field, and a file written before `scale`
        // existed still reads as "as modelled" rather than as scaled away.
        let unscaled = RenderMeshTransform {
            position: Vec3::X,
            ..default()
        };
        let ron = ron::ser::to_string(&unscaled).expect("serialize");
        assert!(!ron.contains("scale"), "unit scale must be omitted: {ron}");
        let legacy: RenderMeshTransform =
            ron::from_str("(position: (1.0, 0.0, 0.0))").expect("deserialize");
        assert_eq!(legacy, unscaled);

        // Rotation-only authoring: the zero position is not serialized, and a
        // string with only `rotation` still deserializes (position defaults).
        let rot_only = RenderMeshTransform {
            position: Vec3::ZERO,
            rotation: Quat::from_rotation_y(0.3),
            scale: Vec3::ONE,
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
