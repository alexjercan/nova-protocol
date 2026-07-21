//! The section type system shared by every ship part. A ship is a tree of
//! sections, and every section pairs a [`BaseSectionConfig`] (health, mass,
//! collider, sounds - the data every kind carries) with a kind-specific config
//! selected by the [`SectionKind`] enum. [`SectionConfig`] bundles the two, and
//! the loaded set of authorable sections lives in the [`GameSections`] resource.
//!
//! Touch this module when adding a field common to all sections, a new physics
//! [`SectionCollider`] shape, or a new [`SectionKind`] variant; the per-kind
//! configs (hull/thruster/controller/turret/torpedo) live in their own sibling
//! modules. The [`base_section`] / [`preview_section`] bundle factories turn a
//! config into the live (or editor-preview) section entity, snapshotting the
//! authored collider and sounds into runtime components. See the sections wiki
//! page for the authoring model.

use std::fmt::Debug;

use avian3d::prelude::*;
use bevy::prelude::*;

use super::prelude::*;
use crate::{
    asset_ref::AssetRef,
    prelude::{destructible_body, ExplodableEntity},
};

pub mod prelude {
    pub use super::{
        base_section, preview_section, BaseSectionConfig, GameSections, ImpactDestroySounds,
        RenderMeshTransform, SectionCollider, SectionConfig, SectionInactiveMarker, SectionKind,
        SectionMarker, SectionRenderMeshTransform, SectionRenderOf,
    };
}

/// Authorable physics collider for a section. Content omits it and gets the unit
/// cube every section carried before this was configurable, so existing files
/// stay byte-for-byte unchanged (see [`BaseSectionConfig::collider`]).
///
/// The scalar fields use the exact units avian's constructors take, so what is
/// authored is what avian builds: `Cuboid.size` is the FULL side length on each
/// axis (not half-extents), and `Capsule`/`Cylinder` extend along local Y.
///
/// Physical note: [`base_section`] feeds the section's `mass` field to avian as
/// DENSITY (`destructible_body(health, density)`), and avian derives real mass
/// from `density * collider_volume`. A larger collider therefore makes a heavier
/// section - intended, but worth knowing when tuning handling.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionCollider {
    /// Axis-aligned box; `size` is the full side length on each axis.
    Cuboid { size: Vec3 },
    /// Sphere of the given radius.
    Sphere { radius: f32 },
    /// Capsule (radius + a cylindrical segment of `length`) along local Y.
    Capsule { radius: f32, length: f32 },
    /// Cylinder of the given radius and height along local Y.
    Cylinder { radius: f32, height: f32 },
}

impl Default for SectionCollider {
    /// The unit cube - the shape every section had before colliders were
    /// authorable, so a `None` collider field resolves to exactly this.
    fn default() -> Self {
        Self::Cuboid { size: Vec3::ONE }
    }
}

impl SectionCollider {
    /// Build the avian [`Collider`] this describes.
    pub fn to_collider(self) -> Collider {
        match self {
            Self::Cuboid { size } => Collider::cuboid(size.x, size.y, size.z),
            Self::Sphere { radius } => Collider::sphere(radius),
            Self::Capsule { radius, length } => Collider::capsule(radius, length),
            Self::Cylinder { radius, height } => Collider::cylinder(radius, height),
        }
    }

    /// Half-extents of the axis-aligned bounding box, ignoring rotation. The
    /// section-overlap lint is rotation-agnostic by design (all shipped content
    /// uses quarter-turns), so an AABB is the right, conservative primitive.
    pub fn aabb_half_extents(self) -> Vec3 {
        match self {
            Self::Cuboid { size } => size * 0.5,
            Self::Sphere { radius } => Vec3::splat(radius),
            Self::Capsule { radius, length } => Vec3::new(radius, radius + length * 0.5, radius),
            Self::Cylinder { radius, height } => Vec3::new(radius, height * 0.5, radius),
        }
    }
}

/// Skip serializing a zero translation - the common case for a render-mesh
/// transform that only reorients (or is authored purely for symmetry with a
/// sibling). Keeps `render_mesh_transform` blocks minimal.
#[cfg(feature = "serde")]
fn is_zero_translation(v: &Vec3) -> bool {
    *v == Vec3::ZERO
}

/// Skip serializing an identity rotation - the common case for a render-mesh
/// transform that only translates.
#[cfg(feature = "serde")]
fn is_identity_rotation(q: &Quat) -> bool {
    *q == Quat::IDENTITY
}

/// An authored transform (position + rotation) applied to a section's RENDER
/// MESH only, relative to the section's own frame. It never touches the
/// section's physics/kinematic transform, so art can be nudged or reoriented
/// without moving the collider or (for turrets) disturbing the joint tree.
/// Position and rotation are authored independently (each defaults out), so a
/// mesh that only needs a small rotation writes just `rotation`, and a nudge
/// writes just `position`. Shared by every section kind (turret joints carry it
/// per-joint; hull/thruster/controller/torpedo carry it per-section).
#[derive(Clone, Copy, Debug, PartialEq, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RenderMeshTransform {
    /// Local translation of the render mesh, relative to the section origin.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "is_zero_translation")
    )]
    pub position: Vec3,
    /// Local rotation of the render mesh, relative to the section frame.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "is_identity_rotation")
    )]
    pub rotation: Quat,
}

impl RenderMeshTransform {
    /// The bevy [`Transform`] this describes (scale left at 1). Used as the
    /// render-mesh child entity's local transform.
    pub fn to_transform(self) -> Transform {
        Transform::from_translation(self.position).with_rotation(self.rotation)
    }
}

/// A section's authored render-mesh transform, snapshotted from its config so
/// the kind-specific render observer can apply it to the mesh child without
/// re-reading the config. `None` = identity (unchanged behavior). Hull,
/// thruster and controller sections carry this; turret joints use their own
/// per-joint carrier, and the torpedo body reads it straight off the config.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SectionRenderMeshTransform(pub Option<RenderMeshTransform>);

/// Marks a live section entity in a ship tree. Present on every spawned
/// section (added by [`base_section`]); its absence marks the editor preview
/// ([`preview_section`] omits it, carrying [`SectionInactiveMarker`] instead).
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionMarker;

/// Marks a section that is present in the world but not simulated - the editor
/// preview / palette section. Added by [`preview_section`] in place of
/// [`SectionMarker`] so gameplay systems skip it.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionInactiveMarker;

/// Back-reference from a section's render-mesh child to the section entity it
/// draws, so render observers can look up their owning section.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, PartialEq, Eq)]
pub struct SectionRenderOf(pub Entity);

/// The data every section carries regardless of kind: identity, physics and the
/// authored hit/destroy sounds and collider. Authored in the section RON as the
/// `base` of a [`SectionConfig`]; snapshotted into runtime components (collider,
/// [`ImpactDestroySounds`]) by [`base_section`] / [`preview_section`].
#[derive(Component, Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseSectionConfig {
    /// Stable content id used to look the section up in [`GameSections`].
    pub id: String,
    /// Display name shown in the editor palette and HUD.
    pub name: String,
    /// Longer editor/tooltip description.
    pub description: String,
    /// Fed to avian as DENSITY (not absolute mass): real mass is
    /// `mass * collider_volume`, so a bigger collider is a heavier section.
    /// See [`SectionCollider`].
    pub mass: f32,
    /// Section hit points; reaching zero destroys the section.
    pub health: f32,
    /// The sound a hit on THIS section plays - per-target, so the target IS
    /// the material (a rock, a light hull and a reinforced hull can each sound
    /// different; spike 20260717-101524, task 20260717-101641). Authorable
    /// asset ref like the meshes; AUTHORED-OR-SILENT. Snapshotted into
    /// [`ImpactDestroySounds`] by [`base_section`].
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub impact_sound: Option<AssetRef<AudioSource>>,
    /// The sound this section's destruction plays; same rules as
    /// [`Self::impact_sound`].
    #[reflect(ignore)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub destroy_sound: Option<AssetRef<AudioSource>>,
    /// Authored physics collider shape/size. Omitted (`None`) means the unit
    /// cube that every section carried before this was configurable, so content
    /// that does not set it stays byte-for-byte unchanged. See
    /// [`SectionCollider`] for the shapes and units. Snapshotted into a real
    /// avian collider by [`base_section`] / [`preview_section`].
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collider: Option<SectionCollider>,
    /// When true this section is hidden from the editor sandbox's section
    /// palette - it can still be authored and spawned, it just does not clutter
    /// the picker. Used for the cut-cube spaceship prototypes (racer/cargob/
    /// cargoa), which are dozens of near-identical hull tiles that only make
    /// sense assembled into a ship, not placed one at a time. Serde-defaulted to
    /// false, so ordinary sections omit it; author a hidden one as
    /// `hide_in_editor: true`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub hide_in_editor: bool,
}

/// `skip_serializing_if` predicate for a `bool` that defaults to false: omit it
/// from the serialized RON when false so unflagged sections stay clean.
#[cfg(feature = "serde")]
fn is_false(b: &bool) -> bool {
    !*b
}

/// A damage target's authored impact/destroy sounds, snapshotted UNRESOLVED
/// from its config ([`BaseSectionConfig`] via [`base_section`]; the asteroid
/// bundle and the torpedo projectile snapshot their own). The audio observers
/// find it by walking up from the hit/destroyed entity (asteroids keep their
/// Health on a child node), resolve, and play - authored-or-silent. `pub`
/// because nova_scenario's asteroid bundle constructs it.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ImpactDestroySounds {
    #[reflect(ignore)]
    pub impact: Option<AssetRef<AudioSource>>,
    #[reflect(ignore)]
    pub destroy: Option<AssetRef<AudioSource>>,
}

/// Which kind of section this is, tagging the matching kind-specific config.
/// The discriminant that selects a section's behavior plugin and the config it
/// reads: hull (structure only), thruster (thrust), controller (attitude PD),
/// turret (guns), torpedo (bay). Add a variant here (plus its config module and
/// plugin) to introduce a new section kind.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant)]
pub enum SectionKind {
    /// Passive structural block; see [`HullSectionConfig`].
    Hull(HullSectionConfig),
    /// Directional thrust; see [`ThrusterSectionConfig`].
    Thruster(ThrusterSectionConfig),
    /// Attitude control via a PD controller; see [`ControllerSectionConfig`].
    Controller(ControllerSectionConfig),
    /// Aimed gun; see [`TurretSectionConfig`].
    Turret(TurretSectionConfig),
    /// Guided-torpedo launch bay; see [`TorpedoSectionConfig`].
    Torpedo(TorpedoSectionConfig),
}

/// A complete authorable section: the shared [`BaseSectionConfig`] plus its
/// kind-specific [`SectionKind`] config. This is the unit stored in
/// [`GameSections`] and placed by the editor.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionConfig {
    /// Fields common to every section kind.
    pub base: BaseSectionConfig,
    /// The kind-specific config selected by [`SectionKind`].
    pub kind: SectionKind,
}

/// The loaded catalog of authorable sections (the editor palette / lookup
/// table), populated from the section content. Look a section up by its
/// [`BaseSectionConfig::id`] with [`get_section`](GameSections::get_section).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameSections(pub Vec<SectionConfig>);

impl GameSections {
    /// The section whose [`BaseSectionConfig::id`] matches `id`, if loaded.
    pub fn get_section(&self, id: &str) -> Option<&SectionConfig> {
        self.iter().find(|section| section.base.id == id)
    }
}

/// Bundle factory for a live (simulated) section from its [`BaseSectionConfig`]:
/// resolves the authored collider and sounds into runtime components and tags it
/// [`SectionMarker`]. See [`preview_section`] for the editor-preview counterpart.
pub fn base_section(config: BaseSectionConfig) -> impl Bundle {
    debug!("base_section: config {:?}", config);

    (
        Name::new(config.name.clone()),
        SectionMarker,
        config.collider.unwrap_or_default().to_collider(),
        destructible_body(config.health, config.mass),
        // bevy_common_systems' destructible_body is the generic Health + density + visibility
        // bundle; nova adds ExplodableEntity so the section enters the explode pipeline.
        ExplodableEntity,
        ImpactDestroySounds {
            impact: config.impact_sound.clone(),
            destroy: config.destroy_sound.clone(),
        },
    )
}

/// A lightweight, pickable stand-in for a section, used by the editor to preview a ship
/// configuration without spawning a live combat ship.
///
/// It renders (via the kind-specific `*_section` bundle inserted alongside it) and can be
/// clicked to place adjacent sections, but unlike [`base_section`] it carries no `Health`,
/// `ColliderDensity` or `ExplodableEntity`, so it never enters the integrity/damage
/// pipeline. As long as neither it nor its root has a `RigidBody`, avian keeps its collider
/// in the standalone spatial-query tree (still pickable) and never links it with
/// `ColliderOf`, so no integrity graph is built for the preview ship and none of the
/// gameplay/health systems act on it.
pub fn preview_section(config: BaseSectionConfig) -> impl Bundle {
    debug!("preview_section: config {:?}", config);

    (
        Name::new(config.name.clone()),
        SectionMarker,
        config.collider.unwrap_or_default().to_collider(),
        Visibility::Inherited,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_collider_is_the_unit_cube() {
        // A section that omits `collider` must resolve to exactly the shape
        // every section had before the field existed, so old content is
        // physically unchanged.
        assert_eq!(
            SectionCollider::default(),
            SectionCollider::Cuboid { size: Vec3::ONE }
        );
        assert_eq!(
            SectionCollider::default().aabb_half_extents(),
            Vec3::splat(0.5)
        );
    }

    #[test]
    fn aabb_half_extents_match_each_shape() {
        assert_eq!(
            SectionCollider::Cuboid {
                size: Vec3::new(2.0, 1.0, 0.5)
            }
            .aabb_half_extents(),
            Vec3::new(1.0, 0.5, 0.25)
        );
        assert_eq!(
            SectionCollider::Sphere { radius: 0.75 }.aabb_half_extents(),
            Vec3::splat(0.75)
        );
        // Capsule/Cylinder extend along local Y; radius bounds X and Z.
        assert_eq!(
            SectionCollider::Capsule {
                radius: 0.5,
                length: 2.0
            }
            .aabb_half_extents(),
            Vec3::new(0.5, 1.5, 0.5)
        );
        assert_eq!(
            SectionCollider::Cylinder {
                radius: 0.5,
                height: 3.0
            }
            .aabb_half_extents(),
            Vec3::new(0.5, 1.5, 0.5)
        );
    }

    #[test]
    fn to_collider_builds_every_shape_without_panicking() {
        // avian's constructors are pure; this pins that every variant maps to a
        // real collider (a bad radius/length would panic here).
        let _ = SectionCollider::default().to_collider();
        let _ = SectionCollider::Sphere { radius: 0.5 }.to_collider();
        let _ = SectionCollider::Capsule {
            radius: 0.3,
            length: 1.0,
        }
        .to_collider();
        let _ = SectionCollider::Cylinder {
            radius: 0.3,
            height: 1.0,
        }
        .to_collider();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn collider_field_round_trips_and_is_omitted_when_unset() {
        // Authored collider survives a RON round-trip.
        let authored = BaseSectionConfig {
            id: "s".to_string(),
            name: "s".to_string(),
            description: String::new(),
            mass: 1.0,
            health: 100.0,
            impact_sound: None,
            destroy_sound: None,
            collider: Some(SectionCollider::Cuboid {
                size: Vec3::new(0.8, 0.8, 0.8),
            }),
            hide_in_editor: false,
        };
        let ron = ron::ser::to_string(&authored).expect("serialize");
        let back: BaseSectionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.collider, authored.collider);

        // Omitting it keeps existing content byte-identical: the field is not
        // emitted, and it reads back as the unit-cube-resolving `None`.
        let plain = BaseSectionConfig {
            collider: None,
            ..authored.clone()
        };
        let ron = ron::ser::to_string(&plain).expect("serialize");
        assert!(
            !ron.contains("collider"),
            "unset collider must not serialize: {ron}"
        );
        let back: BaseSectionConfig = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.collider, None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn hide_in_editor_defaults_false_round_trips_and_is_omitted_when_unset() {
        // Defaults to false and is skipped, so ordinary content stays clean.
        let visible = BaseSectionConfig {
            id: "s".to_string(),
            name: "s".to_string(),
            description: String::new(),
            mass: 1.0,
            health: 100.0,
            impact_sound: None,
            destroy_sound: None,
            collider: None,
            hide_in_editor: false,
        };
        let ron = ron::ser::to_string(&visible).expect("serialize");
        assert!(
            !ron.contains("hide_in_editor"),
            "an unset hide_in_editor must not serialize: {ron}"
        );
        let back: BaseSectionConfig = ron::from_str(&ron).expect("deserialize");
        assert!(!back.hide_in_editor);

        // When flagged it survives the round-trip.
        let hidden = BaseSectionConfig {
            hide_in_editor: true,
            ..visible
        };
        let ron = ron::ser::to_string(&hidden).expect("serialize");
        assert!(ron.contains("hide_in_editor:true"), "flagged: {ron}");
        let back: BaseSectionConfig = ron::from_str(&ron).expect("deserialize");
        assert!(back.hide_in_editor);
    }
}
