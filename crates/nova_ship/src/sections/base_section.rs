//! The section type system shared by every ship part. A ship is a tree of
//! sections, and every section pairs a [`BaseSectionConfig`] (health, collider,
//! sounds - the data every kind carries) with a kind-specific config
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
use nova_gameplay::{
    asset_ref::AssetRef,
    markers::prelude::*,
    prelude::{destructible_body, ConnectedTo, ExplodableEntity},
};

use super::prelude::*;

/// The section and preview spawners, `SectionConfig` and `GameSections`, and the section markers,
/// collider and render-mesh transforms.
pub mod prelude {
    pub use super::{
        base_section, preview_section, BaseSectionConfig, GameSections, ImpactDestroySounds,
        RenderMeshTransform, SectionCollider, SectionConfig, SectionKind,
        SectionRenderMeshTransform, SectionRenderOf,
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
/// Physical note: a section is solid ship, so [`base_section`] hands avian a
/// density of 1 and the volume of the shape here IS the section's mass. A
/// larger collider therefore makes a heavier section - intended, but worth
/// knowing when tuning handling, and the reason the box is authored rather than
/// taken from the render mesh.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SectionCollider {
    /// Axis-aligned box; `size` is the full side length on each axis.
    Cuboid {
        /// Full side length on each axis.
        size: Vec3,
    },
    /// Sphere of the given radius.
    Sphere {
        /// Sphere radius.
        radius: f32,
    },
    /// Capsule (radius + a cylindrical segment of `length`) along local Y.
    Capsule {
        /// Capsule radius.
        radius: f32,
        /// Length of the cylindrical segment along local Y.
        length: f32,
    },
    /// Cylinder of the given radius and height along local Y.
    Cylinder {
        /// Cylinder radius.
        radius: f32,
        /// Cylinder height along local Y.
        height: f32,
    },
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

/// Skip serializing an unscaled mesh - the common case, since art is normally
/// modelled at the size it is used.
#[cfg(feature = "serde")]
fn is_unit_scale(v: &Vec3) -> bool {
    *v == Vec3::ONE
}

/// A missing `scale` means "as modelled", not "scaled to nothing".
#[cfg(feature = "serde")]
fn unit_scale() -> Vec3 {
    Vec3::ONE
}

/// An authored transform (position, rotation and scale) applied to a section's
/// RENDER MESH only, relative to the section's own frame. It never touches the
/// section's physics/kinematic transform, so art can be nudged, reoriented or
/// resized without moving the collider or (for turrets) disturbing the joint
/// tree. Each field is authored independently and defaults out, so a mesh that
/// only needs a small rotation writes just `rotation`. Shared by every section
/// kind (turret joints carry it per-joint; hull/thruster/controller/torpedo
/// carry it per-section).
///
/// `scale` resizes the ART, and only the art. A whole ASSEMBLY - a turret's
/// joint tree - is resized by scaling every joint's mesh AND every joint offset
/// by the same factor; scaling the meshes alone leaves the parts spaced for the
/// unscaled size. See `turret_joint_tree` in `nova_authoring` for the shipped
/// example.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
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
    /// Local scale of the render mesh. `Vec3::ONE` is the modelled size.
    #[cfg_attr(
        feature = "serde",
        serde(default = "unit_scale", skip_serializing_if = "is_unit_scale")
    )]
    pub scale: Vec3,
}

impl Default for RenderMeshTransform {
    /// The identity: art where it was modelled, at the size it was modelled.
    /// Hand-written because a derived `Default` would scale every mesh to
    /// nothing.
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl RenderMeshTransform {
    /// The bevy [`Transform`] this describes. Used as the render-mesh child
    /// entity's local transform.
    pub fn to_transform(self) -> Transform {
        Transform {
            translation: self.position,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
}

/// A section's authored render-mesh transform, snapshotted from its config so
/// the kind-specific render observer can apply it to the mesh child without
/// re-reading the config. `None` = identity (unchanged behavior). Hull,
/// thruster and controller sections carry this; turret joints use their own
/// per-joint carrier, and the torpedo body reads it straight off the config.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
pub struct SectionRenderMeshTransform(pub Option<RenderMeshTransform>);

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
    /// Section hit points; reaching zero destroys the section.
    pub health: f32,
    /// The sound a hit on THIS section plays - per-target, so the target IS
    /// the material (a rock, a light hull and a reinforced hull can each sound
    /// different). Authorable asset ref like the meshes; AUTHORED-OR-SILENT.
    /// Snapshotted into [`ImpactDestroySounds`] by [`base_section`].
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
    /// Structural sockets in section-local space. Empty means the section has
    /// no structural attachment points; collider geometry never supplies them.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub link_points: Vec<LinkPoint>,
    /// When true this section is hidden from the editor sandbox's section
    /// palette - it can still be authored and spawned, it just does not clutter
    /// the picker. Used for the cut-cube spaceship prototypes (racer/cargob/
    /// cargoa), which are dozens of near-identical hull tiles that only make
    /// sense assembled into a ship, not placed one at a time. Serde-defaulted to
    /// false, so ordinary sections omit it; author a hidden one as
    /// `hide_in_editor: true`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    pub hide_in_editor: bool,
    /// The damage looks this section wears, fitted into components by
    /// [`DamageEffectsPlugin`](super::damage_effects::DamageEffectsPlugin).
    /// Omitted means `[Cracks]` - what every section wears unless it was
    /// authorable - so unchanged content and third-party sections keep their
    /// behaviour. Author the empty list for a section that should never show
    /// damage.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "DamageEffects::is_default")
    )]
    pub damage_effects: DamageEffects,
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
    /// Sound played when this target is hit but not destroyed.
    #[reflect(ignore)]
    pub impact: Option<AssetRef<AudioSource>>,
    /// Sound played when this target is destroyed.
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
    trace!("base_section: config {:?}", config);

    let collider = config.collider.unwrap_or_default();
    (
        Name::new(config.name.clone()),
        SectionMarker,
        SectionLinkPoints(config.link_points),
        ConnectedTo::default(),
        collider.to_collider(),
        // Keep the authored collider shape ON the section so the NOVA OS ship app
        // can build its schematic blocks from exact authored extents
        // (`aabb_half_extents`) without decoding the avian collider.
        collider,
        // Density 1, never authorable: a section is solid ship, so its mass IS
        // the volume of the authored collider above. Exactly, not roughly - the
        // shape is the authored box, never the render mesh - so nothing can
        // make one part denser than another.
        destructible_body(config.health, 1.0),
        // destructible_body is the generic Health + density + visibility bundle;
        // ExplodableEntity is what puts the section into the explode pipeline.
        ExplodableEntity,
        // The authored look list. Carried as data rather than fitted here,
        // because the components it names live in the render half and a
        // headless server builds this bundle too.
        config.damage_effects,
        ImpactDestroySounds {
            impact: config.impact_sound.clone(),
            destroy: config.destroy_sound,
        },
    )
}

/// A lightweight, pickable stand-in for a section, used by the editor to preview a ship
/// configuration without spawning a live combat ship.
///
/// It renders (via the kind-specific `*_section` bundle inserted alongside it) and can be
/// clicked to place adjacent sections, but unlike [`base_section`] it carries no `Health`,
/// `ColliderDensity` or `ExplodableEntity`, so it never enters the integrity/damage
/// pipeline. Its root uses the editor-only preview marker rather than
/// `SpaceshipRootMarker`, so no integrity graph is built for the preview ship and none of
/// the gameplay/health systems act on it.
pub fn preview_section(config: BaseSectionConfig) -> impl Bundle {
    trace!("preview_section: config {:?}", config);

    let collider = config.collider.unwrap_or_default();
    (
        Name::new(config.name.clone()),
        SectionMarker,
        SectionLinkPoints(config.link_points),
        collider.to_collider(),
        // The authored shape rides along, as it does on a live section: editor
        // placement needs those extents for its overlap refusal, and reading
        // them back out of an avian collider is not the same number.
        collider,
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

    #[test]
    fn live_and_preview_sections_snapshot_link_points_but_only_live_has_a_graph_node() {
        let config = BaseSectionConfig {
            name: "section".to_string(),
            health: 10.0,
            link_points: unit_cube_link_points(),
            ..default()
        };
        let mut world = World::new();
        let live = world.spawn(base_section(config.clone())).id();
        let preview = world.spawn(preview_section(config)).id();

        assert_eq!(world.get::<SectionLinkPoints>(live).unwrap().len(), 6);
        assert_eq!(world.get::<SectionLinkPoints>(preview).unwrap().len(), 6);
        assert!(world.get::<ConnectedTo>(live).is_some());
        assert!(world.get::<ConnectedTo>(preview).is_none());
        // Both carry the authored shape: editor placement measures overlap
        // against a preview section exactly as the lint measures a live one.
        assert_eq!(
            world.get::<SectionCollider>(preview).copied(),
            Some(SectionCollider::default())
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn collider_field_round_trips_and_is_omitted_when_unset() {
        // Authored collider survives a RON round-trip.
        let authored = BaseSectionConfig {
            id: "s".to_string(),
            name: "s".to_string(),
            description: String::new(),
            health: 100.0,
            impact_sound: None,
            destroy_sound: None,
            collider: Some(SectionCollider::Cuboid {
                size: Vec3::new(0.8, 0.8, 0.8),
            }),
            link_points: Vec::new(),
            hide_in_editor: false,
            damage_effects: DamageEffects::default(),
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
            health: 100.0,
            impact_sound: None,
            destroy_sound: None,
            collider: None,
            link_points: Vec::new(),
            hide_in_editor: false,
            damage_effects: DamageEffects::default(),
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
