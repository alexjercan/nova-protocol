//! Freeze and thaw of a detached section piece: the wreck that comes off a
//! section when it dies, and what it looks like when a save brings it back.
//!
//! A piece carries no gameplay state of its own once it detaches - no
//! `Health`, no weapon, nothing that can fire or take another hit - only the
//! art it already wore. A freeze walks that art tree and classifies every
//! node into a durable record; a thaw rebuilds the same tree through the same
//! builders a live section uses, so a resumed piece looks exactly like the
//! one that was saved.

use avian3d::prelude::{
    AngularVelocity, CenterOfMass, LinearVelocity, NoAutoCenterOfMass, RigidBody,
};
use bevy::{platform::collections::HashMap, prelude::*, world_serialization::WorldInstanceReady};
use bevy_hanabi::ParticleEffect;
use nova_gameplay::prelude::{
    AssetRef, ChunkGrace, DetachedPieceMarker, GravityAffected, TransientFreezeFault,
};

use crate::sections::{
    damage_cracks::{mark_wreck_cracks, mesh_wears_cracks},
    prelude::{
        GameStyles, PlaceholderArt, PlaceholderArtMarker, PlaceholderArtType, SectionCollider,
        ShellShape, ShipDecorMarker, ShipStyle, ShipStyleConfig, SkinAssets, ThrusterExhaustConfig,
    },
    shell_skin::{hang_surfaces, ShipSkinMarker, SkinSurfaceMarker},
};

/// `FrozenDetachedPiece`, `FrozenArtNode`, `FrozenArtType`,
/// `DetachedPieceSource`, `freeze_detached_piece`, and `thaw_detached_piece`.
pub mod prelude {
    pub use super::{
        freeze_detached_piece, thaw_detached_piece, DetachedPieceSource, FrozenArtNode,
        FrozenArtType, FrozenDetachedPiece,
    };
}

/// A detached piece's durable state: its physics pose and drift, its
/// collider, how long until its grace period ends (if it has not already),
/// the style it wears, and every art node it carries.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenDetachedPiece {
    name: String,
    translation: Vec3,
    rotation: Quat,
    linear: Vec3,
    angular: Vec3,
    center_of_mass: Vec3,
    collider: SectionCollider,
    grace: Option<f32>,
    style: Option<String>,
    nodes: Vec<FrozenArtNode>,
}

impl FrozenDetachedPiece {
    /// The style id this piece wears, or `None` for the unstyled look. A
    /// caller checks a saved id against its loaded styles before a thaw, so a
    /// miss can be refused early instead of surfacing as a thaw-time panic.
    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }

    /// The saved world translation of the piece's body, so a Load can check
    /// the thawed body against its record.
    pub fn translation(&self) -> Vec3 {
        self.translation
    }

    /// Rejects a record a thaw cannot act on safely: a non-finite pose,
    /// drift, or center of mass; a grace that is not finite and not negative;
    /// a node whose parent comes after it; a non-finite node transform; or a
    /// non-finite light value. Every fault names the field or node at fault
    /// in plain words.
    ///
    /// # Errors
    ///
    /// The first fault found, as a plain-words message.
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("translation", self.translation),
            ("linear", self.linear),
            ("angular", self.angular),
            ("center_of_mass", self.center_of_mass),
        ] {
            if !value.is_finite() {
                return Err(format!(
                    "a detached piece's {field} is {value}, which is not finite"
                ));
            }
        }
        if !self.rotation.is_finite() {
            return Err(format!(
                "a detached piece's rotation is {:?}, which is not finite",
                self.rotation
            ));
        }
        if let Some(grace) = self.grace {
            if !grace.is_finite() || grace < 0.0 {
                return Err(format!(
                    "a detached piece's grace remaining is {grace}, which is not finite and not negative"
                ));
            }
        }
        for (index, node) in self.nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                if parent as usize >= index {
                    return Err(format!(
                        "a detached piece's node {index} names the parent {parent}, which comes after it"
                    ));
                }
            }
            if !node.transform.translation.is_finite()
                || !node.transform.rotation.is_finite()
                || !node.transform.scale.is_finite()
            {
                return Err(format!(
                    "a detached piece's node {index} has a transform that is not finite"
                ));
            }
            if let FrozenArtType::Light {
                intensity,
                range,
                radius,
                ..
            } = &node.art
            {
                for (field, value) in [
                    ("intensity", *intensity),
                    ("range", *range),
                    ("radius", *radius),
                ] {
                    if !value.is_finite() {
                        return Err(format!(
                            "a detached piece's node {index} has a light {field} of {value}, which is not finite"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// One node of a piece's art tree: where it sits (relative to its parent),
/// whether it is drawn, and what it is.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenArtNode {
    /// The index of this node's parent in the piece's own node list, or
    /// `None` for a node that sits directly under the piece root. Always
    /// less than this node's own index: a parent is always recorded before
    /// its children.
    parent: Option<u32>,
    name: Option<String>,
    transform: Transform,
    #[cfg_attr(feature = "serde", serde(with = "visibility_serde"))]
    visibility: Visibility,
    art: FrozenArtType,
}

/// `Visibility` has no serde form of its own; a record writes its variant
/// name.
#[cfg(feature = "serde")]
mod visibility_serde {
    use bevy::prelude::Visibility;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "Visibility")]
    enum VisibilityDef {
        Inherited,
        Hidden,
        Visible,
    }

    pub(super) fn serialize<S: Serializer>(
        visibility: &Visibility,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        VisibilityDef::serialize(visibility, serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Visibility, D::Error> {
        VisibilityDef::deserialize(deserializer)
    }
}

/// What one art node draws.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FrozenArtType {
    /// A plain transform node: a turret joint, a torpedo body or spawner, or
    /// any other wrapper that draws nothing on its own.
    Group,
    /// A gltf scene. `poses` holds the local transform of every named
    /// descendant the scene spawns, keyed by its name path from the scene
    /// root (same-named siblings get an index suffix), so a door or arm that
    /// had moved off its authored rest pose comes back the same way.
    /// `cracked` is true when the scene was already showing section cracks
    /// at the moment it froze.
    Scene {
        /// The scene's authored asset path.
        asset: String,
        /// Whether the scene already wore section cracks when it froze.
        cracked: bool,
        /// Every named descendant's local transform, keyed by its name path.
        poses: Vec<(Vec<String>, Transform)>,
    },
    /// A stand-in mesh, drawn when a section authors no render mesh of its
    /// own.
    Placeholder {
        /// Which stand-in it is.
        art: PlaceholderArtType,
        /// Whether it wore section cracks, or was about to, when it froze.
        cracked: bool,
    },
    /// A hull skin plate, re-hung from its shape and the piece's style.
    SkinPlate(ShellShape),
    /// A piece of decoration, by its authored asset path.
    Decor {
        /// The decoration's authored asset path.
        asset: String,
    },
    /// A thruster's exhaust cone. Its inner cone is rebuilt by the same
    /// observer that built it the first time, so only the outer config is
    /// saved.
    Exhaust(ThrusterExhaustConfig),
    /// A point light: a railgun's charge glow.
    Light {
        /// The light's color.
        color: Color,
        /// The light's intensity.
        intensity: f32,
        /// The light's range.
        range: f32,
        /// The light's radius.
        radius: f32,
    },
}

/// What a dying section's body carried that dies WITH it: the inputs a
/// resumed piece needs but cannot read off its own tree, because the section
/// that authored them is gone by the time anything saves the piece.
///
/// Stamped once, at detach, by [`stamp_detached_piece_source`], while the
/// dying section is still alive to read.
#[derive(Component, Clone, Debug)]
pub struct DetachedPieceSource {
    collider: SectionCollider,
    style: Option<String>,
}

/// Copies a dying section's [`SectionCollider`] and [`ShipStyle`] onto the
/// piece that just detached from it, while the section is still alive to
/// read - the piece spawn is queued before the section's own despawn, so
/// both run in the same command flush.
///
/// A no-op when the piece already carries [`DetachedPieceSource`] (a resumed
/// piece stamps its own, directly, and inserts this marker only to keep the
/// piece's own shape consistent with a live one) or when the named source no
/// longer exists (same reason, covering `Entity::PLACEHOLDER`).
pub(crate) fn stamp_detached_piece_source(
    add: On<Add, DetachedPieceMarker>,
    q_marker: Query<&DetachedPieceMarker>,
    q_stamped: Query<&DetachedPieceSource>,
    q_collider: Query<&SectionCollider>,
    q_child_of: Query<&ChildOf>,
    q_style: Query<&ShipStyle>,
    mut commands: Commands,
) {
    let piece = add.entity;
    if q_stamped.get(piece).is_ok() {
        return;
    }
    let Ok(DetachedPieceMarker(source)) = q_marker.get(piece) else {
        return;
    };
    let Ok(collider) = q_collider.get(*source) else {
        return;
    };
    let style = worn_detached_style(*source, &q_child_of, &q_style);
    commands.entity(piece).insert(DetachedPieceSource {
        collider: *collider,
        style,
    });
}

/// The style id the ship a detached section's body sat on wears, found by
/// walking its ancestors, the same way a skin plate finds the style it is
/// dressed in.
fn worn_detached_style(
    entity: Entity,
    q_child_of: &Query<&ChildOf>,
    q_style: &Query<&ShipStyle>,
) -> Option<String> {
    let mut current = entity;
    loop {
        if let Ok(ShipStyle(style)) = q_style.get(current) {
            return style.clone();
        }
        current = q_child_of.get(current).ok()?.0;
    }
}

/// Captures `piece`'s durable physics state and every art node it carries.
///
/// # Panics
///
/// Fails loudly, naming the entity, node, or component at fault: `piece`
/// carries no [`DetachedPieceSource`] (every live piece is stamped the
/// instant it detaches, so a missing stamp means this is not a detached
/// piece); an art node this module cannot classify; a scene node whose
/// handle names no asset path; or a mesh or material with no
/// [`PlaceholderArtMarker`].
///
/// # Errors
///
/// Never returns [`TransientFreezeFault::Unsettled`]: a detached piece has no
/// multi-frame destruction process of its own left to finish. The `Result`
/// shape matches every other transient freeze.
pub fn freeze_detached_piece(
    world: &World,
    piece: Entity,
) -> Result<FrozenDetachedPiece, TransientFreezeFault> {
    let Some(name) = world.get::<Name>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no Name");
    };
    let Some(transform) = world.get::<Transform>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no Transform");
    };
    let Some(LinearVelocity(linear)) = world.get::<LinearVelocity>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no LinearVelocity");
    };
    let Some(AngularVelocity(angular)) = world.get::<AngularVelocity>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no AngularVelocity");
    };
    let Some(CenterOfMass(center_of_mass)) = world.get::<CenterOfMass>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no CenterOfMass");
    };
    let Some(source) = world.get::<DetachedPieceSource>(piece) else {
        panic!("freeze_detached_piece: piece {piece:?} carries no DetachedPieceSource");
    };
    let grace = world.get::<ChunkGrace>(piece).map(ChunkGrace::remaining);

    let mut nodes = Vec::new();
    collect_art_nodes(world, piece, None, &mut nodes);

    Ok(FrozenDetachedPiece {
        name: name.as_str().to_string(),
        translation: transform.translation,
        rotation: transform.rotation,
        linear: *linear,
        angular: *angular,
        center_of_mass: *center_of_mass,
        collider: source.collider,
        grace,
        style: source.style.clone(),
        nodes,
    })
}

/// Walks every direct child of `parent` (a piece root or an art node),
/// skipping a [`ParticleEffect`] entirely and a [`SkinSurfaceMarker`] (a skin
/// plate's own re-derived render mesh), and appends one [`FrozenArtNode`] per
/// surviving child in depth-first, parent-before-child order.
fn collect_art_nodes(
    world: &World,
    parent: Entity,
    parent_index: Option<u32>,
    out: &mut Vec<FrozenArtNode>,
) {
    let Some(children) = world.get::<Children>(parent) else {
        return;
    };
    for child in children.iter() {
        if world.get::<ParticleEffect>(child).is_some() {
            continue;
        }
        if world.get::<SkinSurfaceMarker>(child).is_some() {
            continue;
        }
        let art = classify_art_node(world, child);
        let recurse = !matches!(
            art,
            FrozenArtType::Scene { .. } | FrozenArtType::Decor { .. } | FrozenArtType::Exhaust(_)
        );
        let transform = world.get::<Transform>(child).copied().unwrap_or_default();
        let visibility = world.get::<Visibility>(child).copied().unwrap_or_default();
        let name = world
            .get::<Name>(child)
            .map(|name| name.as_str().to_string());
        let my_index = out.len() as u32;
        out.push(FrozenArtNode {
            parent: parent_index,
            name,
            transform,
            visibility,
            art,
        });
        if recurse {
            collect_art_nodes(world, child, Some(my_index), out);
        }
    }
}

/// Classifies one art node, in the fixed order every node is checked against:
/// decoration, skin plate, thruster exhaust, point light, gltf scene, then a
/// [`PlaceholderArtMarker`]. A node that is none of these and draws nothing
/// is a plain [`FrozenArtType::Group`].
fn classify_art_node(world: &World, node: Entity) -> FrozenArtType {
    if let Some(ShipDecorMarker(asset_ref)) = world.get::<ShipDecorMarker>(node) {
        let Some(path) = asset_ref.path() else {
            panic!(
                "freeze_detached_piece: decor node {node:?} names a resolved handle, not an \
                 authored path, and cannot be saved"
            );
        };
        return FrozenArtType::Decor {
            asset: path.to_string(),
        };
    }
    if let Some(ShipSkinMarker(shape)) = world.get::<ShipSkinMarker>(node) {
        return FrozenArtType::SkinPlate(*shape);
    }
    if let Some(exhaust) = world.get::<ThrusterExhaustConfig>(node) {
        return FrozenArtType::Exhaust(exhaust.clone());
    }
    if let Some(light) = world.get::<PointLight>(node) {
        return FrozenArtType::Light {
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            radius: light.radius,
        };
    }
    if let Some(WorldAssetRoot(handle)) = world.get::<WorldAssetRoot>(node) {
        let asset_server = world.resource::<AssetServer>();
        let Some(path) = asset_server.get_path(handle.id()) else {
            panic!(
                "freeze_detached_piece: scene node {node:?} carries a WorldAssetRoot handle with \
                 no authored path, and cannot be saved"
            );
        };
        let mut poses = Vec::new();
        collect_scene_poses(world, node, &mut Vec::new(), &mut poses);
        let cracked = any_descendant_cracked(world, node);
        return FrozenArtType::Scene {
            asset: path.to_string(),
            cracked,
            poses,
        };
    }
    if let Some(&PlaceholderArtMarker(art)) = world.get::<PlaceholderArtMarker>(node) {
        return FrozenArtType::Placeholder {
            art,
            cracked: mesh_wears_cracks(world, node),
        };
    }
    if world.get::<Mesh3d>(node).is_some()
        || world
            .get::<MeshMaterial3d<StandardMaterial>>(node)
            .is_some()
    {
        panic!("freeze_detached_piece: node {node:?} draws a mesh no saved art type names");
    }
    FrozenArtType::Group
}

/// True when any descendant of `root` (a scene node) wears section cracks or
/// is about to (see [`mesh_wears_cracks`]), so the thaw must burn the scene
/// back to the same look.
fn any_descendant_cracked(world: &World, root: Entity) -> bool {
    let Some(children) = world.get::<Children>(root) else {
        return false;
    };
    for child in children.iter() {
        if mesh_wears_cracks(world, child) || any_descendant_cracked(world, child) {
            return true;
        }
    }
    false
}

/// Builds the name-path label of one child among `children`: its own `Name`
/// when no sibling shares it, or `"name#index"` when one or more do, counted
/// in child order. Shared by the freeze-side walk and the thaw-side lookup,
/// so both agree on the same key for the same node.
fn labelled_children<'a>(
    children: &'a Children,
    q_name: impl Fn(Entity) -> Option<&'a Name>,
) -> Vec<(Entity, String)> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for child in children.iter() {
        if let Some(name) = q_name(child) {
            *counts.entry(name.as_str()).or_insert(0) += 1;
        }
    }
    let mut seen: HashMap<&str, u32> = HashMap::new();
    let mut labelled = Vec::new();
    for child in children.iter() {
        let Some(name) = q_name(child) else {
            continue;
        };
        let total = counts[name.as_str()];
        let label = if total > 1 {
            let index = seen.entry(name.as_str()).or_insert(0);
            let label = format!("{}#{}", name.as_str(), index);
            *index += 1;
            label
        } else {
            name.as_str().to_string()
        };
        labelled.push((child, label));
    }
    labelled
}

/// Captures every named descendant's local `Transform` under `root` (a scene
/// node), keyed by its name path from `root`. A descendant with no `Name`,
/// and everything under it, is skipped: there is no path to key it by.
fn collect_scene_poses(
    world: &World,
    parent: Entity,
    prefix: &mut Vec<String>,
    out: &mut Vec<(Vec<String>, Transform)>,
) {
    let Some(children) = world.get::<Children>(parent) else {
        return;
    };
    for (child, label) in labelled_children(children, |entity| world.get::<Name>(entity)) {
        prefix.push(label);
        if let Some(transform) = world.get::<Transform>(child) {
            out.push((prefix.clone(), *transform));
        }
        collect_scene_poses(world, child, prefix, out);
        prefix.pop();
    }
}

/// Resolves a name-path `path` (as [`collect_scene_poses`] built it) down
/// from `root` against a freshly spawned scene, re-deriving the same
/// same-named-sibling index suffix.
fn resolve_scene_path(
    root: Entity,
    path: &[String],
    q_children: &Query<&Children>,
    q_name: &Query<&Name>,
) -> Option<Entity> {
    let mut current = root;
    for label in path {
        let children = q_children.get(current).ok()?;
        let labelled = labelled_children(children, |entity| q_name.get(entity).ok());
        current = labelled
            .into_iter()
            .find(|(_, candidate)| candidate == label)
            .map(|(entity, _)| entity)?;
    }
    Some(current)
}

/// Re-applies a resumed scene's saved node poses once its instance is ready,
/// and, if the scene was cracked when it froze, marks every one of its
/// meshes as wreck art so grading burns it back to the same look instead of
/// reading it as pristine.
///
/// A `WorldInstanceReady` whose entity carries no [`ResumedScenePoses`] is
/// some other scene entirely, and is left alone.
pub(crate) fn apply_resumed_scene_poses(
    ready: On<WorldInstanceReady>,
    q_pending: Query<&ResumedScenePoses>,
    q_children: Query<&Children>,
    q_name: Query<&Name>,
    mut q_transform: Query<&mut Transform>,
    q_mesh: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
    mut commands: Commands,
) {
    let Ok(ResumedScenePoses(poses, cracked)) = q_pending.get(ready.entity) else {
        return;
    };
    for (path, pose) in poses {
        if let Some(node) = resolve_scene_path(ready.entity, path, &q_children, &q_name) {
            if let Ok(mut transform) = q_transform.get_mut(node) {
                *transform = *pose;
            }
        }
    }
    if *cracked {
        for descendant in q_children.iter_descendants(ready.entity) {
            if q_mesh.get(descendant).is_ok() {
                mark_wreck_cracks(&mut commands, descendant);
            }
        }
    }
    commands.entity(ready.entity).remove::<ResumedScenePoses>();
}

/// Marks a thawed scene node for [`apply_resumed_scene_poses`] to finish once
/// its `WorldInstanceReady` fires: the poses it must write back, and whether
/// to burn its meshes to wreck art.
#[derive(Component)]
pub(crate) struct ResumedScenePoses(Vec<(Vec<String>, Transform)>, bool);

/// Rebuilds a detached piece from `record`: its root's physics state, and
/// every art node, in parent-before-child order, through the same builders a
/// live section's own render observers use.
///
/// No `lifetime` argument: the caller inserts `resumed_lifetime` on the
/// returned entity afterward, as it does for every other resumed transient.
///
/// # Panics
///
/// `record.style()` names an id that `styles` does not resolve, or `styles`
/// is `None` while `record.style()` is `Some` - both programming errors, since
/// a caller must prevalidate a saved style before any transient thaw runs.
pub fn thaw_detached_piece(
    commands: &mut Commands,
    asset_server: &AssetServer,
    placeholder: &PlaceholderArt,
    styles: Option<&GameStyles>,
    skin_assets: &mut SkinAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    record: &FrozenDetachedPiece,
) -> Entity {
    let style_config: Option<&ShipStyleConfig> = match (record.style(), styles) {
        (None, _) => None,
        (Some(id), None) => {
            panic!("thaw_detached_piece: piece names style '{id}', but no styles are loaded")
        }
        (Some(id), Some(styles)) => {
            let Some(config) = ShipStyle(Some(id.to_string())).resolve(styles) else {
                panic!("thaw_detached_piece: piece names style '{id}', which does not exist");
            };
            Some(config)
        }
    };

    let name = Name::new(record.name.clone());
    let transform = Transform::from_translation(record.translation).with_rotation(record.rotation);
    let source = DetachedPieceSource {
        collider: record.collider,
        style: record.style.clone(),
    };
    let root = match record.grace {
        Some(remaining) => commands
            .spawn((
                DetachedPieceMarker(Entity::PLACEHOLDER),
                source,
                name,
                transform,
                Visibility::Visible,
                RigidBody::Kinematic,
                CenterOfMass(record.center_of_mass),
                NoAutoCenterOfMass,
                LinearVelocity(record.linear),
                AngularVelocity(record.angular),
                ChunkGrace::resumed(record.collider.to_collider(), remaining),
            ))
            .id(),
        None => commands
            .spawn((
                DetachedPieceMarker(Entity::PLACEHOLDER),
                source,
                name,
                transform,
                Visibility::Visible,
                RigidBody::Dynamic,
                CenterOfMass(record.center_of_mass),
                NoAutoCenterOfMass,
                LinearVelocity(record.linear),
                AngularVelocity(record.angular),
                record.collider.to_collider(),
                GravityAffected,
            ))
            .id(),
    };

    let mut spawned: Vec<Entity> = Vec::with_capacity(record.nodes.len());
    for node in &record.nodes {
        let parent = match node.parent {
            Some(index) => spawned[index as usize],
            None => root,
        };
        let entity = commands
            .spawn((node.transform, node.visibility, ChildOf(parent)))
            .id();
        if let Some(name) = &node.name {
            commands.entity(entity).insert(Name::new(name.clone()));
        }
        match &node.art {
            FrozenArtType::Group => {}
            FrozenArtType::Scene {
                asset,
                cracked,
                poses,
            } => {
                commands.entity(entity).insert((
                    WorldAssetRoot(asset_server.load(asset.clone())),
                    ResumedScenePoses(poses.clone(), *cracked),
                ));
            }
            FrozenArtType::Placeholder { art, cracked } => {
                commands.entity(entity).insert(placeholder.bundle(*art));
                if *cracked {
                    mark_wreck_cracks(commands, entity);
                }
            }
            FrozenArtType::SkinPlate(shape) => {
                hang_surfaces(
                    commands,
                    entity,
                    *shape,
                    style_config,
                    skin_assets,
                    meshes,
                    materials,
                );
            }
            FrozenArtType::Decor { asset } => {
                commands
                    .entity(entity)
                    .insert(ShipDecorMarker(AssetRef::Path(asset.clone())));
            }
            FrozenArtType::Exhaust(config) => {
                commands.entity(entity).insert(config.clone());
            }
            FrozenArtType::Light {
                color,
                intensity,
                range,
                radius,
            } => {
                commands.entity(entity).insert(PointLight {
                    color: *color,
                    intensity: *intensity,
                    range: *range,
                    radius: *radius,
                    shadow_maps_enabled: false,
                    ..default()
                });
            }
        }
        spawned.push(entity);
    }

    root
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{Collider, Gravity, PhysicsPlugins};
    use bevy::{
        asset::AssetMetaCheck, ecs::system::SystemState, gltf::GltfPlugin, mesh::MeshPlugin,
        time::TimeUpdateStrategy, world_serialization::WorldSerializationPlugin,
    };
    use bevy_hanabi::EffectAsset;
    use bevy_rand::prelude::*;
    use nova_events::units::prelude::{Meters, MetersPerSecond};
    use nova_gameplay::prelude::{
        resumed_lifetime, CargoCanisterIdAllocator, Health, HealthApplyDamage, NovaIntegrityPlugin,
        SavedLifetime, SectionMarker, SmoothLookRotation, SpaceshipRootMarker, TempEntityPlugin,
    };

    use super::*;
    use crate::sections::{
        base_section::{section_body, BaseSectionConfig, SectionConfig, SectionKind},
        cargo_intake_section::CargoIntakeSectionConfig,
        controller_section::ControllerSectionConfig,
        damage_cracks::{SectionCracks, SectionCracksMaterial},
        docking_section::DockingSectionConfig,
        fixture::SectionFixture,
        hull_section::HullSectionConfig,
        mining_section::MiningSectionConfig,
        prelude::SpaceshipSectionPlugin,
        railgun_section::{RailgunCharge, RailgunSectionConfig},
        shell_shape::prelude::FULL,
        shell_skin::{plate_body, SkinPlate},
        skin_decor::decor_body,
        skin_style::{FixturePlacement, StyleFixtureConfig, StylePalette, SurfaceFinish},
        thruster_section::ThrusterSectionConfig,
        torpedo_section::{TorpedoSectionConfig, TorpedoSectionSpawnerFireState},
        turret_section::{TurretSectionConfig, TurretSectionMuzzleEntity},
    };

    /// A dyed finish neither `ShellSurface::colour()` default ever produces,
    /// so finding it on a thawed plate's material proves the STYLE was read,
    /// not the built-in look.
    fn dyed_finish() -> SurfaceFinish {
        SurfaceFinish {
            color: Color::srgb(0.9, 0.2, 0.05),
            roughness: 0.4,
            metallic: 0.8,
        }
    }

    /// A headless app that can load a real `.glb` through the `AssetServer`
    /// and run a section through its own real destruction: `AssetPlugin`,
    /// `WorldSerializationPlugin`, `MeshPlugin` and `GltfPlugin` cover a real
    /// scene load without a window or a renderer; `PhysicsPlugins` and
    /// `NovaIntegrityPlugin` cover the real health/explode pipeline;
    /// `SpaceshipSectionPlugin { render: true }` is the one master plugin
    /// that wires every section kind's builder and render observers, the
    /// skin, the ship integrity adapter, the frozen-piece observers and
    /// section cracks - the same plugin set a real game boots.
    fn real_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: "../../assets".to_string(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            },
            TransformPlugin,
            WorldSerializationPlugin,
            MeshPlugin,
            GltfPlugin::default(),
            PhysicsPlugins::default(),
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        // `bevy_world_serialization`'s spawner copies a `WorldAsset`'s
        // components by reflection (`write_to_world_with`); without a real
        // `MaterialPlugin<StandardMaterial>` (not added here - it needs a
        // render world) nothing ever registers `MeshMaterial3d<StandardMaterial>`,
        // so a scene's node meshes would spawn with their `Mesh3d` but silently
        // skip the material the glb loader also baked into them.
        app.register_type::<MeshMaterial3d<StandardMaterial>>();
        // The turret muzzle flash and torpedo launch puff observers need the
        // asset storage `bevy_hanabi::HanabiPlugin` would otherwise provide;
        // that plugin itself needs a render world this test does not build
        // (the same reason `frozen_rounds.rs` and `railgun_section/wake.rs`
        // init this asset type directly instead of adding the plugin). The
        // same observers also read `SoftDot`'s backing `Assets<Image>`.
        app.init_asset::<EffectAsset>();
        app.init_asset::<Image>();
        app.add_plugins(NovaIntegrityPlugin);
        app.add_plugins(TempEntityPlugin);
        app.add_plugins(SpaceshipSectionPlugin { render: true });
        app.init_resource::<CargoCanisterIdAllocator>();
        // The destroy path's debris observers need the global rng even in a
        // headless run.
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.insert_resource(Gravity(Vec3::ZERO));
        // A fixed, manual step: without it, FixedUpdate (and so avian's own
        // integration) only advances when enough REAL wall-clock time has
        // passed between `app.update()` calls, which makes a piece's kicked
        // drift depend on how fast the test loop itself runs, not on frame
        // count (`nova_gameplay::test_support` uses the same fix).
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ));
        app.insert_resource(GameStyles(vec![ShipStyleConfig {
            id: "raider".to_string(),
            name: "Raider".to_string(),
            palette: StylePalette {
                top: dyed_finish(),
                wall: dyed_finish(),
            },
            fixtures: Vec::new(),
        }]));
        app.finish();
        app.cleanup();
        app
    }

    /// Every descendant of `root`, in no particular order.
    fn descendants(world: &World, root: Entity) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut queue = vec![root];
        while let Some(entity) = queue.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                for child in children.iter() {
                    out.push(child);
                    queue.push(child);
                }
            }
        }
        out
    }

    /// The first descendant of `root` named `name`, breadth-first.
    fn find_named_descendant(world: &World, root: Entity, name: &str) -> Option<Entity> {
        let mut queue = vec![root];
        while let Some(entity) = queue.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                for child in children.iter() {
                    if world.get::<Name>(child).map(Name::as_str) == Some(name) {
                        return Some(child);
                    }
                    queue.push(child);
                }
            }
        }
        None
    }

    /// How many of `root`'s direct children carry a [`SkinSurfaceMarker`].
    fn surface_count(world: &World, root: Entity) -> usize {
        world
            .get::<Children>(root)
            .map(|children| {
                children
                    .iter()
                    .filter(|&child| world.get::<SkinSurfaceMarker>(child).is_some())
                    .count()
            })
            .unwrap_or(0)
    }

    /// Fails the case when `root` or any of its descendants carries gameplay
    /// state a resumed piece must never bring back: a dying section's own
    /// `Health`, `SectionFixture`, `SectionMarker`, `Collider` or
    /// `TorpedoSectionSpawnerFireState`, or (by component name, since the real
    /// type is private to `turret_section`) a `TurretJointMarker`.
    fn assert_tree_carries_no_gameplay_state(world: &World, root: Entity, case: &str) {
        let mut queue = vec![root];
        while let Some(entity) = queue.pop() {
            assert!(
                world.get::<Health>(entity).is_none(),
                "case {case}: {entity:?} carries Health"
            );
            assert!(
                world.get::<SectionFixture>(entity).is_none(),
                "case {case}: {entity:?} carries SectionFixture"
            );
            assert!(
                world.get::<SectionMarker>(entity).is_none(),
                "case {case}: {entity:?} carries SectionMarker"
            );
            assert!(
                world.get::<Collider>(entity).is_none(),
                "case {case}: {entity:?} carries a Collider"
            );
            assert!(
                world
                    .get::<TorpedoSectionSpawnerFireState>(entity)
                    .is_none(),
                "case {case}: {entity:?} carries TorpedoSectionSpawnerFireState"
            );
            // `TurretJointMarker` itself is private to `turret_section` and
            // unreachable by type from here; `SmoothLookRotation` (public,
            // `nova_gameplay`) is what a real joint's own setup pairs with
            // it whenever the joint has a hinge axis
            // (`turret_section/setup.rs:60-70`), and nothing else in this
            // fixture's tree ever wears one, so its absence is the same
            // proof without a component-name match (`ComponentInfo::name()`
            // needs bevy_ecs's `debug` feature to return a real name, which
            // this workspace does not enable).
            assert!(
                world.get::<SmoothLookRotation>(entity).is_none(),
                "case {case}: {entity:?} carries a SmoothLookRotation, the same as a live turret joint"
            );
            if let Some(children) = world.get::<Children>(entity) {
                queue.extend(children.iter());
            }
        }
    }

    /// What one case's section builds, by default, and the expectations that
    /// follow from it.
    enum CaseArt {
        /// Stand-in meshes a real [`PlaceholderArt`] entry must match,
        /// anywhere in the section's live or thawed tree.
        Placeholder(&'static [PlaceholderArtType]),
        /// A real authored scene, forced via `render_mesh`, and the name of a
        /// real node in it to pose off its authored rest before the freeze.
        Scene {
            path: &'static str,
            door: &'static str,
        },
    }

    struct Case {
        kind: &'static str,
        section: SectionKind,
        art: CaseArt,
    }

    /// One fixture per [`SectionKind`] variant, built with that kind's own
    /// real section builder: the six that draw no render mesh of their own,
    /// by default, wear a [`PlaceholderArt`] stand-in; `CargoIntake` and
    /// `Mining` require a real scene; `Torpedo` is given one via
    /// `render_mesh`, the same field a real ship authors to do it.
    fn cases() -> Vec<Case> {
        vec![
            Case {
                kind: "hull",
                section: SectionKind::Hull(HullSectionConfig::default()),
                art: CaseArt::Placeholder(&[PlaceholderArtType::Body]),
            },
            Case {
                kind: "thruster",
                section: SectionKind::Thruster(ThrusterSectionConfig::default()),
                art: CaseArt::Placeholder(&[
                    PlaceholderArtType::Barrel,
                    PlaceholderArtType::Nozzle,
                ]),
            },
            Case {
                kind: "controller",
                section: SectionKind::Controller(ControllerSectionConfig::default()),
                art: CaseArt::Placeholder(&[
                    PlaceholderArtType::ControllerBody,
                    PlaceholderArtType::Window,
                ]),
            },
            Case {
                kind: "turret",
                section: SectionKind::Turret(TurretSectionConfig::default()),
                art: CaseArt::Placeholder(&[PlaceholderArtType::TurretPlate]),
            },
            Case {
                kind: "railgun",
                // `charge_seconds: 0.0` (the default) is an instant-fire
                // test rig (`railgun_section/mod.rs:194-198`): a manually
                // set `Charging` state would fire and reset to `Ready`
                // on the very next tick, before this test ever observes
                // the glow lit. A real aiming delay keeps it charging.
                section: SectionKind::Railgun(RailgunSectionConfig {
                    charge_seconds: 5.0,
                    ..RailgunSectionConfig::default()
                }),
                art: CaseArt::Placeholder(&[PlaceholderArtType::Body]),
            },
            Case {
                kind: "docking",
                section: SectionKind::Docking(DockingSectionConfig::default()),
                art: CaseArt::Placeholder(&[PlaceholderArtType::Body]),
            },
            Case {
                kind: "cargo_intake",
                section: SectionKind::CargoIntake(CargoIntakeSectionConfig {
                    render_mesh: AssetRef::from("base/gltf/intake_accordion_3x2x1.glb#Scene0"),
                    render_mesh_transform: None,
                    canister_mesh: AssetRef::from("base/gltf/intake_accordion_3x2x1.glb#Scene0"),
                    door_sound: AssetRef::default(),
                    eject_sound: AssetRef::default(),
                    take_sound: AssetRef::default(),
                    detection_range: Meters(40.0),
                    capture_gap: Meters(1.0),
                    aperture_width: Meters(22.2),
                    aperture_height: Meters(15.3),
                    eject_speed: MetersPerSecond(3.0),
                }),
                art: CaseArt::Scene {
                    path: "base/gltf/intake_accordion_3x2x1.glb#Scene0",
                    door: "intake_slat_l0",
                },
            },
            Case {
                kind: "mining",
                section: SectionKind::Mining(MiningSectionConfig {
                    render_mesh: AssetRef::from("base/gltf/mining_beam_compact.glb#Scene0"),
                    render_mesh_transform: None,
                    pulse_sound: AssetRef::default(),
                    refusal_sound: AssetRef::default(),
                    door_open_sound: AssetRef::default(),
                    door_close_sound: AssetRef::default(),
                    reach: Meters(100.0),
                    pulse_interval_seconds: 1.0,
                    carve_radius_cells: 1.5,
                }),
                art: CaseArt::Scene {
                    path: "base/gltf/mining_beam_compact.glb#Scene0",
                    door: "stow_lid_right",
                },
            },
            Case {
                kind: "torpedo",
                section: SectionKind::Torpedo(TorpedoSectionConfig {
                    render_mesh: Some(AssetRef::from("base/gltf/bay_tube.glb#Scene0")),
                    ..Default::default()
                }),
                art: CaseArt::Scene {
                    path: "base/gltf/bay_tube.glb#Scene0",
                    door: "door_petal_0",
                },
            },
        ]
    }

    /// A detached piece, frozen and thawed, wears the exact art a live one
    /// did: the same stand-in mesh (or the same scene at the same pose and
    /// crack state), the same skin plate (re-dressed in the same style), the
    /// same decoration, and - on the railgun case - the same charge-glow
    /// light. Every case is built and killed through the real section
    /// builder and the real health/explode pipeline, not a hand-built
    /// stand-in, so a turret case also proves it wore a real turret joint
    /// before it died and none of the gameplay state that died with the
    /// section it came off. A headless app gives a file-loaded scene no
    /// material to crack, so a last case cracks an in-memory scene.
    #[test]
    fn a_detached_piece_resumes_as_the_art_it_wore() {
        for case in cases() {
            run_case(case);
        }
        run_in_memory_scene_crack_case();
    }

    fn run_case(case: Case) {
        let mut app = real_app();

        let ship = app
            .world_mut()
            .spawn((
                Name::new("ship"),
                ShipStyle(Some("raider".to_string())),
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();

        let source_transform = Transform::from_translation(Vec3::new(3.0, -1.0, 5.0))
            .with_rotation(Quat::from_rotation_y(0.4));
        let fixture_collider = SectionCollider::Cuboid {
            size: Vec3::new(2.0, 1.0, 1.0),
        };
        let config = SectionConfig {
            base: BaseSectionConfig {
                id: format!("{}_fixture", case.kind),
                name: format!("{} section", case.kind),
                health: 123.0,
                collider: Some(fixture_collider),
                ..Default::default()
            },
            kind: case.section,
        };
        let source = {
            let mut state: SystemState<Commands> = SystemState::new(app.world_mut());
            let entity = {
                let Ok(mut commands) = state.get_mut(app.world_mut()) else {
                    panic!("case {}: the spawn's Commands are not available", case.kind);
                };
                let mut section_entity = commands.spawn((ChildOf(ship), source_transform));
                section_body(&mut section_entity, &config, false);
                section_entity.id()
            };
            state.apply(app.world_mut());
            entity
        };

        // Let the real builders' render observers, the scene loader and the
        // skin dressing settle before anything is posed or damaged.
        for _ in 0..30 {
            app.update();
        }

        if case.kind == "turret" {
            // `TurretJointMarker` is private to `turret_section`, and
            // `ComponentInfo::name()` only returns a real name under
            // bevy_ecs's `debug` feature, which this workspace does not
            // enable - a name-string match always reads every component as
            // the same placeholder here, so it cannot tell a joint apart.
            // `TurretSectionMuzzleEntity`, public, is only ever written once
            // `insert_turret_section` has walked the real joint tree and
            // found its primary muzzle (`turret_section/setup.rs:121-145`),
            // so a live one naming a real, spawned descendant is the same
            // proof.
            let muzzle = app
                .world()
                .get::<TurretSectionMuzzleEntity>(source)
                .unwrap_or_else(|| {
                    panic!("case turret: the live section built no TurretSectionMuzzleEntity")
                })
                .0;
            assert!(
                descendants(app.world(), source).contains(&muzzle),
                "case turret: the recorded muzzle joint must be a real descendant of the live section"
            );
        }

        let shape = ShellShape::new([FULL; 4], [FULL; 4]).expect("a legal shape");
        let plate = SkinPlate {
            cell: IVec3::ZERO,
            anchor: IVec3::NEG_Y,
            shape,
            rotation: Quat::IDENTITY,
        };
        let plate_pose = Transform::from_translation(Vec3::new(0.5, 0.2, -0.3));
        let plate_entity = app
            .world_mut()
            .spawn((plate_body(&plate, plate_pose), ChildOf(source)))
            .id();

        let decor_path = "base/gltf/greebles/civilian_door.glb#Scene0".to_string();
        let decor_cfg = StyleFixtureConfig {
            id: "greeble".to_string(),
            model: AssetRef::from(decor_path.clone()),
            health: 10.0,
            collider: Vec3::new(0.3, 0.2, 0.1),
            placement: FixturePlacement::default(),
        };
        let decor_pose = Transform::from_translation(Vec3::new(-0.4, 0.1, 0.2));
        app.world_mut()
            .spawn((decor_body(&decor_cfg, decor_pose), ChildOf(source)));

        for _ in 0..20 {
            app.update();
        }

        let door_pose = if let CaseArt::Scene { door, .. } = &case.art {
            let door_entity =
                find_named_descendant(app.world(), source, door).unwrap_or_else(|| {
                    panic!(
                        "case {}: no real node named {door} spawned under the live section",
                        case.kind
                    )
                });
            let rest = *app.world().get::<Transform>(door_entity).unwrap();
            let mut opened = rest;
            opened.rotate_local_y(0.6);
            *app.world_mut().get_mut::<Transform>(door_entity).unwrap() = opened;
            assert_ne!(
                opened, rest,
                "case {}: the posed door must differ from rest",
                case.kind
            );
            Some((door.to_string(), opened))
        } else {
            None
        };

        let source_surface_count = surface_count(app.world(), plate_entity);
        assert!(
            source_surface_count > 0,
            "case {}: the live plate wears no surfaces",
            case.kind
        );

        // A real charge: the fixture's `charge_seconds: 5.0` (not the
        // default 0, an instant-fire test rig per
        // `railgun_section/mod.rs:194-198`) keeps `RailgunCharge::Charging`
        // short of full progress long enough for `drive_railgun_charge_glow`
        // to light the glow before the kill clones it onto the piece.
        let expected_light = if case.kind == "railgun" {
            app.world_mut()
                .entity_mut(source)
                .insert(RailgunCharge::Charging { elapsed: 0.0 });
            app.update();
            let glow = find_named_descendant(app.world(), source, "Railgun Charge Glow")
                .expect("case railgun: the real body always wears a charge glow");
            Some(*app.world().get::<PointLight>(glow).unwrap())
        } else {
            None
        };

        // Damage the section before the kill, through the real integrity
        // plugins, so a scene case shows real section cracks at freeze time.
        let expect_cracked = matches!(case.art, CaseArt::Scene { .. });
        if expect_cracked {
            app.world_mut().trigger(HealthApplyDamage {
                entity: source,
                source: None,
                amount: 70.0,
            });
            for _ in 0..15 {
                app.update();
            }
        }

        // Kill it for real: `HealthApplyDamage` to zero, through the real
        // `ExplodablePlugin`/integrity plugins, so `detach_destroyed_body`
        // spawns the piece exactly as a live collapse does. The kill is an
        // observer chain (zero health, destroy marker, detach), so the piece
        // exists once the trigger's commands flush, before any `Update`
        // system runs. That is the pre-grade window: the crack systems run
        // in `Update`, unordered against the save, so a save can see it.
        app.world_mut().trigger(HealthApplyDamage {
            entity: source,
            source: None,
            amount: 10_000.0,
        });
        app.world_mut().flush();

        let is_placeholder = matches!(case.art, CaseArt::Placeholder(_));
        if is_placeholder {
            let piece_now = {
                let mut pieces = app
                    .world_mut()
                    .query_filtered::<Entity, With<DetachedPieceMarker>>();
                pieces.iter(app.world()).next().unwrap_or_else(|| {
                    panic!("case {}: no piece exists right after the kill", case.kind)
                })
            };
            let mesh_now = descendants(app.world(), piece_now)
                .into_iter()
                .find(|&e| app.world().get::<PlaceholderArtMarker>(e).is_some())
                .unwrap_or_else(|| {
                    panic!(
                        "case {}: no placeholder mesh right after the kill",
                        case.kind
                    )
                });
            assert!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(mesh_now)
                    .is_some()
                    && app
                        .world()
                        .get::<MeshMaterial3d<SectionCracksMaterial>>(mesh_now)
                        .is_none(),
                "case {}: right after the kill, grading has not swapped the material yet",
                case.kind
            );
            assert!(
                mesh_wears_cracks(app.world(), mesh_now),
                "case {}: right after the kill, before any grading system has run, the \
                 placeholder mesh must already read as cracked",
                case.kind
            );
            let pre_grade = freeze_detached_piece(app.world(), piece_now)
                .expect("a detached piece has no unsettled process");
            let node = pre_grade
                .nodes
                .iter()
                .find(|n| matches!(n.art, FrozenArtType::Placeholder { .. }))
                .unwrap_or_else(|| {
                    panic!(
                        "case {}: no placeholder node in the pre-grade freeze",
                        case.kind
                    )
                });
            assert!(
                matches!(node.art, FrozenArtType::Placeholder { cracked: true, .. }),
                "case {}: a freeze taken before any grading system has run must still read \
                 cracked: true off the dead-section pending mark",
                case.kind
            );
        }

        for _ in 0..15 {
            app.update();
        }

        assert!(
            app.world().get_entity(source).is_err(),
            "case {}: the dead section must be gone",
            case.kind
        );
        let piece = {
            let mut pieces = app
                .world_mut()
                .query_filtered::<Entity, With<DetachedPieceMarker>>();
            let found: Vec<Entity> = pieces.iter(app.world()).collect();
            assert_eq!(
                found.len(),
                1,
                "case {}: exactly one piece must detach: {found:?}",
                case.kind
            );
            found[0]
        };

        let stamped = app
            .world()
            .get::<DetachedPieceSource>(piece)
            .cloned()
            .unwrap_or_else(|| panic!("case {}: no DetachedPieceSource stamp", case.kind));
        assert_eq!(stamped.style, Some("raider".to_string()));
        assert_eq!(stamped.collider, fixture_collider);

        let grace_before = app
            .world()
            .get::<ChunkGrace>(piece)
            .unwrap_or_else(|| panic!("case {}: the real piece carries no grace", case.kind))
            .remaining();
        assert!(grace_before > 0.0);
        let saved_lifetime = SavedLifetime::of(app.world(), piece)
            .unwrap_or_else(|| panic!("case {}: the real piece carries no lifetime", case.kind));

        // The orphaned, settled state: enough frames have passed that
        // grading finished burning the mesh to the top bucket and dropped the
        // dead `SectionCracks` link (`grade_section_cracks`'s `orphaned`
        // branch), so neither component remains, only the cracked material.
        if is_placeholder {
            let mesh = descendants(app.world(), piece)
                .into_iter()
                .find(|&e| app.world().get::<PlaceholderArtMarker>(e).is_some())
                .unwrap_or_else(|| panic!("case {}: no placeholder mesh on the piece", case.kind));
            assert!(
                app.world().get::<SectionCracks>(mesh).is_none(),
                "case {}: an orphaned placeholder mesh must carry no SectionCracks",
                case.kind
            );
            assert!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                    .is_none(),
                "case {}: an orphaned placeholder mesh must wear no MeshMaterial3d<StandardMaterial>",
                case.kind
            );
        }

        let record = freeze_detached_piece(app.world(), piece)
            .expect("a detached piece has no unsettled process");
        record
            .validate()
            .expect("a freshly frozen record is well-formed");
        assert_eq!(record.style(), Some("raider"));

        let decor_nodes: Vec<&FrozenArtNode> = record
            .nodes
            .iter()
            .filter(|n| matches!(n.art, FrozenArtType::Decor { .. }))
            .collect();
        assert_eq!(decor_nodes.len(), 1, "case {}: one decor node", case.kind);
        match &decor_nodes[0].art {
            FrozenArtType::Decor { asset } => assert_eq!(*asset, decor_path),
            other => panic!("case {}: decor node classified as {other:?}", case.kind),
        }

        let plate_nodes: Vec<&FrozenArtNode> = record
            .nodes
            .iter()
            .filter(|n| matches!(n.art, FrozenArtType::SkinPlate(_)))
            .collect();
        assert_eq!(
            plate_nodes.len(),
            1,
            "case {}: one skin plate node",
            case.kind
        );
        assert!(matches!(plate_nodes[0].art, FrozenArtType::SkinPlate(s) if s == shape));

        match &case.art {
            CaseArt::Placeholder(expected) => {
                let found: Vec<PlaceholderArtType> = record
                    .nodes
                    .iter()
                    .filter_map(|n| match &n.art {
                        FrozenArtType::Placeholder { art, .. } => Some(*art),
                        _ => None,
                    })
                    .collect();
                for kind in expected.iter() {
                    assert!(
                        found.contains(kind),
                        "case {}: expected a Placeholder({kind:?}) node, found {found:?}",
                        case.kind
                    );
                }
                let cracked: Vec<bool> = record
                    .nodes
                    .iter()
                    .filter_map(|n| match &n.art {
                        FrozenArtType::Placeholder { cracked, .. } => Some(*cracked),
                        _ => None,
                    })
                    .collect();
                assert!(
                    cracked.iter().all(|&c| c),
                    "case {}: a placeholder orphaned by its own destroyed section must freeze cracked: true, found {cracked:?}",
                    case.kind
                );
            }
            CaseArt::Scene { path, .. } => {
                let scene_nodes: Vec<&FrozenArtNode> = record
                    .nodes
                    .iter()
                    .filter(|n| matches!(n.art, FrozenArtType::Scene { .. }))
                    .collect();
                assert_eq!(scene_nodes.len(), 1, "case {}: one scene node", case.kind);
                match &scene_nodes[0].art {
                    FrozenArtType::Scene {
                        asset,
                        cracked,
                        poses,
                    } => {
                        assert_eq!(
                            asset, path,
                            "case {}: the scene's own asset path",
                            case.kind
                        );
                        // Not `expect_cracked`: a real `.glb`-sourced
                        // `WorldAsset`'s mesh node never gains a real
                        // `MeshMaterial3d<StandardMaterial>` in this headless
                        // app. `bevy_gltf`'s own loader only inserts `Mesh3d`
                        // (`bevy_gltf-0.19.0/src/loader/mod.rs:1670-1743`,
                        // "TODO: could add the `GltfMaterial` here"); the
                        // material comes from a `GltfExtensionHandlerPbr`
                        // this crate cannot see or construct (private,
                        // `bevy_pbr-0.19.1/src/gltf.rs:13,102`), pushed into
                        // `GltfExtensionHandlers` only by the full
                        // `PbrPlugin::build` (`bevy_pbr-0.19.1/src/lib.rs:
                        // 255-258`), which needs a render world this app does
                        // not build. Without that component,
                        // `mark_section_meshes` never tracks the mesh, so the
                        // pre-kill damage above never reaches it - `cracked`
                        // correctly reads the real (false) live state. The
                        // real crack pipeline on a glb-sourced scene is
                        // proven headless instead by
                        // `run_in_memory_scene_crack_case`,
                        // whose mesh node is built in memory with a real
                        // material handle instead of loaded from a file.
                        assert!(
                            !*cracked,
                            "case {}: a headless app can never crack a glb-sourced scene mesh",
                            case.kind
                        );
                        assert!(
                            !poses.is_empty(),
                            "case {}: a real scene names at least one posed node",
                            case.kind
                        );
                        let (door_name, opened) = door_pose.as_ref().unwrap();
                        let saved = poses
                            .iter()
                            .find(|(path, _)| path.last() == Some(door_name))
                            .unwrap_or_else(|| {
                                panic!("case {}: no saved pose for {door_name}", case.kind)
                            });
                        assert_eq!(
                            saved.1, *opened,
                            "case {}: the door's posed transform",
                            case.kind
                        );
                    }
                    other => panic!("case {}: scene node classified as {other:?}", case.kind),
                }
            }
        }

        if case.kind == "railgun" {
            let light_nodes: Vec<&FrozenArtNode> = record
                .nodes
                .iter()
                .filter(|n| matches!(n.art, FrozenArtType::Light { .. }))
                .collect();
            assert_eq!(light_nodes.len(), 1, "case railgun: one light node");
            let expected = expected_light.as_ref().unwrap();
            match &light_nodes[0].art {
                FrozenArtType::Light {
                    color,
                    intensity,
                    range,
                    radius,
                } => {
                    assert_eq!(*color, expected.color);
                    assert!((*intensity - expected.intensity).abs() < 1e-3);
                    assert_eq!(*range, expected.range);
                    assert_eq!(*radius, expected.radius);
                }
                other => panic!("case railgun: light node classified as {other:?}"),
            }
        }

        #[cfg(feature = "serde")]
        let record = {
            let written = ron::to_string(&record).expect("a frozen detached piece serializes");
            println!(
                "case {}: frozen piece RON is {} bytes",
                case.kind,
                written.len()
            );
            let parsed: FrozenDetachedPiece = ron::from_str(&written).expect("and parses back");
            assert_eq!(parsed.style(), record.style());
            assert_eq!(parsed.nodes.len(), record.nodes.len());
            parsed
        };

        let mut state: SystemState<(
            Commands,
            Res<AssetServer>,
            Res<PlaceholderArt>,
            Option<Res<GameStyles>>,
            ResMut<SkinAssets>,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
        )> = SystemState::new(app.world_mut());
        let thawed = {
            let Ok((
                mut commands,
                asset_server,
                placeholder,
                styles,
                mut skin_assets,
                mut meshes,
                mut materials,
            )) = state.get_mut(app.world_mut())
            else {
                panic!("case {}: the thaw's resources are not available", case.kind);
            };
            let thawed = thaw_detached_piece(
                &mut commands,
                &asset_server,
                &placeholder,
                styles.as_deref(),
                &mut skin_assets,
                &mut meshes,
                &mut materials,
                &record,
            );
            commands
                .entity(thawed)
                .insert(resumed_lifetime(saved_lifetime));
            thawed
        };
        state.apply(app.world_mut());

        // The intermediate state right after the thaw's own commands land,
        // before any `app.update()`. `dress_skin_decor` is an `On<Add,
        // ShipDecorMarker>` observer, not a scheduled system, so it already
        // fired within this same command flush: a thawed decor carries both
        // `ShipDecorMarker` and the `WorldAssetRoot` the observer derived
        // from it, resolving to the same authored path. A thawed plate, by
        // design, is never given `ShipSkinMarker` at all - `hang_surfaces`
        // dresses a bare node directly, because the live `dress_skin_plate`
        // path it would otherwise trigger needs a ship root this piece no
        // longer has.
        {
            let world = app.world();
            let thawed_children: Vec<Entity> =
                world.get::<Children>(thawed).unwrap().iter().collect();
            let decor_now = thawed_children
                .iter()
                .copied()
                .find(|&e| world.get::<ShipDecorMarker>(e).is_some())
                .unwrap_or_else(|| panic!("case {}: no decor right after thaw", case.kind));
            let decor_asset_root = world.get::<WorldAssetRoot>(decor_now).unwrap_or_else(|| {
                panic!(
                    "case {}: dress_skin_decor's Add observer must have already resolved WorldAssetRoot",
                    case.kind
                )
            });
            let asset_server = world.resource::<AssetServer>();
            assert_eq!(
                asset_server
                    .get_path(decor_asset_root.0.id())
                    .map(|p| p.to_string()),
                Some(decor_path.clone()),
                "case {}: the just-thawed decor's WorldAssetRoot names the authored path",
                case.kind
            );
            let plate_now = thawed_children
                .iter()
                .copied()
                .find(|&e| surface_count(world, e) > 0)
                .unwrap_or_else(|| panic!("case {}: no skin plate right after thaw", case.kind));
            assert!(
                world.get::<ShipSkinMarker>(plate_now).is_none(),
                "case {}: a just-thawed plate must not carry ShipSkinMarker",
                case.kind
            );

            // A real thaw of a cracked placeholder: the stand-in mesh and its
            // marker land at once (`PlaceholderArt::bundle`), and the cracked
            // flag queues the pending crack mark for grading to pick up.
            // `PendingSectionCracks` itself is private to `damage_cracks`, so
            // this reads its effect through `mesh_wears_cracks`, the same
            // crate-visible accessor `classify_art_node` uses, rather than a
            // component-name match - `ComponentInfo::name()` needs bevy_ecs's
            // `debug` feature to return a real name, which this workspace
            // does not enable (confirmed: every name reads as the
            // "<Enable the debug feature ...>" placeholder here).
            if is_placeholder {
                // A full descendant search, not just direct children: the
                // turret case's placeholder plate hangs off a joint Group,
                // not off the piece root directly.
                let mesh_now = descendants(world, thawed)
                    .into_iter()
                    .find(|&e| world.get::<PlaceholderArtMarker>(e).is_some())
                    .unwrap_or_else(|| {
                        panic!(
                            "case {}: no thawed placeholder mesh right after thaw",
                            case.kind
                        )
                    });
                assert!(
                    world.get::<Mesh3d>(mesh_now).is_some(),
                    "case {}: a just-thawed placeholder must carry Mesh3d",
                    case.kind
                );
                assert!(
                    world
                        .get::<MeshMaterial3d<StandardMaterial>>(mesh_now)
                        .is_some(),
                    "case {}: a just-thawed placeholder starts on its own StandardMaterial",
                    case.kind
                );
                assert!(
                    mesh_wears_cracks(world, mesh_now),
                    "case {}: a cracked placeholder's thaw must queue a pending crack mark",
                    case.kind
                );
            }

            // Against the FROZEN pose, not the section's original one:
            // `detach_destroyed_body` always gives a piece a real kick and
            // spin (`explode.rs:403-444`), so by the time it freezes it has
            // already drifted from where the section died, and the settle
            // loop below drifts it further still. Checked here, right after
            // the thaw's own commands land and before any further
            // integration, this proves the thaw's fidelity - that it lands
            // exactly where the record says - without that later drift in
            // the way.
            assert_eq!(
                world.get::<Transform>(thawed).unwrap().translation,
                record.translation,
                "case {}: a thawed piece's position matches what was frozen",
                case.kind
            );
            assert_eq!(
                world.get::<Transform>(thawed).unwrap().rotation,
                record.rotation,
                "case {}: a thawed piece's rotation matches what was frozen",
                case.kind
            );

            // Also checked with no settling elapsed yet: `ChunkGrace` ticks
            // down with real time once `TimeUpdateStrategy::ManualDuration`
            // makes every `app.update()` a real step, so this must be read
            // before any of them, the same reason the pose check above is.
            let grace_after = world.get::<ChunkGrace>(thawed).unwrap().remaining();
            assert_eq!(
                grace_after, grace_before,
                "case {}: grace survives the thaw",
                case.kind
            );
        }

        // One step, no more: `resumed_lifetime`'s `TempEntityState` is only
        // populated by `TempEntityPlugin`'s own `Added<TempEntity>` system,
        // which needs a step to run - but, like grace, it then counts down
        // with every further real step, so it is read right after this one.
        app.update();
        let lifetime_after = SavedLifetime::of(app.world(), thawed)
            .unwrap_or_else(|| panic!("case {}: no lifetime on the thawed piece", case.kind));
        assert_eq!(lifetime_after.total, saved_lifetime.total);
        // That one step also counts against the resumed countdown - its
        // remaining is the saved value, less exactly the one real tick this
        // check needed to let `TempEntityState` populate.
        assert!(
            (saved_lifetime.remaining - lifetime_after.remaining - 1.0 / 60.0).abs() < 1e-4,
            "case {}: lifetime survives the thaw: {} vs {}",
            case.kind,
            lifetime_after.remaining,
            saved_lifetime.remaining
        );

        for _ in 0..29 {
            app.update();
        }

        let world = app.world();
        assert_tree_carries_no_gameplay_state(world, thawed, case.kind);

        let thawed_children: Vec<Entity> = world.get::<Children>(thawed).unwrap().iter().collect();
        let thawed_decor = thawed_children
            .iter()
            .copied()
            .find(|&e| world.get::<ShipDecorMarker>(e).is_some())
            .unwrap_or_else(|| panic!("case {}: no decor among the thawed children", case.kind));
        let thawed_plate = thawed_children
            .iter()
            .copied()
            .find(|&e| surface_count(world, e) > 0)
            .unwrap_or_else(|| panic!("case {}: no plate among the thawed children", case.kind));

        match &case.art {
            CaseArt::Placeholder(expected) => {
                let thawed_tree = descendants(world, thawed);
                for kind in expected.iter() {
                    let found = thawed_tree.iter().any(|&e| {
                        world.get::<PlaceholderArtMarker>(e) == Some(&PlaceholderArtMarker(*kind))
                    });
                    assert!(
                        found,
                        "case {}: no thawed node wears Placeholder({kind:?})",
                        case.kind
                    );
                }

                // Proof part 3 (after frames): grading has finished burning
                // the cracked placeholder to its wreck material.
                let mesh_now = thawed_tree
                    .iter()
                    .copied()
                    .find(|&e| world.get::<PlaceholderArtMarker>(e).is_some())
                    .unwrap_or_else(|| panic!("case {}: no thawed placeholder mesh", case.kind));
                assert!(
                    world
                        .get::<MeshMaterial3d<SectionCracksMaterial>>(mesh_now)
                        .is_some(),
                    "case {}: a cracked placeholder must wear SectionCracksMaterial once grading settles",
                    case.kind
                );

                // Proof part 4: re-freezing the thawed piece gets the same
                // art record back.
                let re_frozen = freeze_detached_piece(world, thawed)
                    .expect("a thawed piece has no unsettled process");
                let re_node = re_frozen
                    .nodes
                    .iter()
                    .find(|n| matches!(n.art, FrozenArtType::Placeholder { .. }))
                    .unwrap_or_else(|| {
                        panic!("case {}: no placeholder node in the re-freeze", case.kind)
                    });
                assert!(
                    matches!(
                        &re_node.art,
                        FrozenArtType::Placeholder { art, cracked: true } if expected.contains(art)
                    ),
                    "case {}: re-freezing the thawed piece must give the same placeholder art \
                     and cracked: true, got {:?}",
                    case.kind,
                    re_node.art
                );
            }
            CaseArt::Scene { path, .. } => {
                let thawed_tree = descendants(world, thawed);
                let asset_server = world.resource::<AssetServer>();
                let scene_entity = thawed_tree
                    .iter()
                    .copied()
                    .find(|&e| {
                        world.get::<WorldAssetRoot>(e).is_some_and(|handle| {
                            asset_server
                                .get_path(handle.0.id())
                                .is_some_and(|p| p.to_string() == *path)
                        })
                    })
                    .unwrap_or_else(|| panic!("case {}: no thawed node loads {path}", case.kind));

                let (door_name, opened) = door_pose.as_ref().unwrap();
                let resumed_door = find_named_descendant(world, scene_entity, door_name)
                    .unwrap_or_else(|| {
                        panic!(
                            "case {}: the thawed scene never spawned {door_name}",
                            case.kind
                        )
                    });
                let resumed_pose = *world.get::<Transform>(resumed_door).unwrap();
                assert_eq!(
                    resumed_pose, *opened,
                    "case {}: the door's pose is written back after the real WorldInstanceReady",
                    case.kind
                );

                // Not `expect_cracked`: the same headless gap as the freeze-side
                // check above (`bevy_gltf-0.19.0/src/loader/mod.rs:1670-1743`;
                // `bevy_pbr-0.19.1/src/gltf.rs:13,102`;
                // `bevy_pbr-0.19.1/src/lib.rs:255-258`) means a real glb-sourced
                // scene mesh never carries `MeshMaterial3d<StandardMaterial>`,
                // so `apply_resumed_scene_poses` has nothing to hand
                // `mark_wreck_cracks` either. Thawing a real scene's resumed
                // cracked flag is proven instead by
                // `run_in_memory_scene_crack_case`.
                let scene_tree = descendants(world, scene_entity);
                let cracked_meshes = scene_tree
                    .iter()
                    .filter(|&&e| {
                        world
                            .get::<MeshMaterial3d<SectionCracksMaterial>>(e)
                            .is_some()
                    })
                    .count();
                assert_eq!(
                    cracked_meshes, 0,
                    "case {}: a headless app can never crack a glb-sourced scene mesh",
                    case.kind
                );
            }
        }

        if case.kind == "railgun" {
            let light_entity = descendants(world, thawed)
                .into_iter()
                .find(|&e| world.get::<PointLight>(e).is_some())
                .unwrap_or_else(|| panic!("case railgun: no thawed light node"));
            let light = world.get::<PointLight>(light_entity).unwrap();
            let expected = expected_light.as_ref().unwrap();
            assert_eq!(light.color, expected.color);
            assert!((light.intensity - expected.intensity).abs() < 1e-3);
            assert_eq!(light.range, expected.range);
            assert_eq!(light.radius, expected.radius);
            assert_eq!(
                world.get::<Visibility>(light_entity).copied(),
                Some(Visibility::Inherited),
                "case railgun: a charging glow is visible"
            );
        }

        let thawed_surface_count = surface_count(world, thawed_plate);
        assert_eq!(
            thawed_surface_count, source_surface_count,
            "case {}: the thawed plate wears the same surface count",
            case.kind
        );
        // Floor is never dressed (it is bolted against the structure, never
        // seen), so only SOME of the plate's surfaces read the style - one
        // reading the dyed colour is what proves the style was read at all.
        let standard_materials = world.resource::<Assets<StandardMaterial>>();
        let any_dyed = world
            .get::<Children>(thawed_plate)
            .unwrap()
            .iter()
            .filter_map(|child| world.get::<MeshMaterial3d<StandardMaterial>>(child))
            .filter_map(|material| standard_materials.get(&material.0))
            .any(|material| material.base_color == dyed_finish().color);
        assert!(
            any_dyed,
            "case {}: no thawed surface reads the raider style, not the built-in look",
            case.kind
        );

        assert_eq!(
            world
                .get::<ShipDecorMarker>(thawed_decor)
                .map(|ShipDecorMarker(r)| r.path()),
            Some(Some(decor_path.as_str()))
        );
        let decor_handle = world
            .get::<WorldAssetRoot>(thawed_decor)
            .unwrap_or_else(|| {
                panic!(
                    "case {}: dress_skin_decor never resolved the decor",
                    case.kind
                )
            })
            .0
            .clone();
        let asset_server = world.resource::<AssetServer>();
        assert_eq!(
            asset_server
                .get_path(decor_handle.id())
                .unwrap()
                .to_string(),
            decor_path
        );
    }

    /// The real crack pipeline on a scene-art section, proven without a file
    /// load. A headless app can never attach a real
    /// `MeshMaterial3d<StandardMaterial>` to a `.glb`-sourced `WorldAsset`'s
    /// mesh node (see the comment on `run_case`'s `CaseArt::Scene` cracked
    /// assertion), so this builds the one component a real section's scene
    /// mesh would already carry - a real material handle - directly in an
    /// in-memory `WorldAsset`, and drives it through the same real damage,
    /// freeze, kill, and thaw path every other case uses.
    fn run_in_memory_scene_crack_case() {
        let mut app = real_app();

        let ship = app
            .world_mut()
            .spawn((
                Name::new("ship"),
                ShipStyle(Some("raider".to_string())),
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();

        let config = SectionConfig {
            base: BaseSectionConfig {
                id: "scene_proof_fixture".to_string(),
                name: "scene proof section".to_string(),
                health: 100.0,
                collider: Some(SectionCollider::Cuboid {
                    size: Vec3::new(2.0, 1.0, 1.0),
                }),
                ..Default::default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        };
        let source = {
            let mut state: SystemState<Commands> = SystemState::new(app.world_mut());
            let entity = {
                let Ok(mut commands) = state.get_mut(app.world_mut()) else {
                    panic!("the spawn's Commands are not available");
                };
                let mut section_entity = commands.spawn((ChildOf(ship), Transform::default()));
                section_body(&mut section_entity, &config, false);
                section_entity.id()
            };
            state.apply(app.world_mut());
            entity
        };
        for _ in 0..10 {
            app.update();
        }

        // A real material handle, exactly what a loaded glb's mesh would
        // carry once `GltfExtensionHandlerPbr` ran - the piece a headless app
        // cannot build (see `run_case`'s `CaseArt::Scene` comment).
        let mesh_handle = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::default(),
            ));
        let material_handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let mut inner = World::new();
        inner.spawn((
            Name::new("proof mesh"),
            Transform::default(),
            Visibility::default(),
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
        ));
        // Loaded by path, not `Assets::add`, so `freeze_detached_piece`'s
        // `classify_art_node` (which reads the path straight off the handle,
        // `AssetServer::get_path`) has one to find - then pre-empted with
        // the in-memory content directly, since nothing on disk backs this
        // path and the real background load would only ever fail.
        let world_asset_handle = app
            .world()
            .resource::<AssetServer>()
            .load::<WorldAsset>("test/in_memory_scene.scn.ron");
        app.world_mut()
            .resource_mut::<Assets<WorldAsset>>()
            .insert(&world_asset_handle, WorldAsset::new(inner))
            .expect("a fresh handle always inserts");

        let scene_node = app
            .world_mut()
            .spawn((
                Name::new("Proof Scene Body"),
                Transform::default(),
                ChildOf(source),
                WorldAssetRoot(world_asset_handle),
            ))
            .id();

        for _ in 0..10 {
            app.update();
        }

        let mesh = descendants(app.world(), scene_node)
            .into_iter()
            .find(|&e| app.world().get::<Mesh3d>(e).is_some())
            .unwrap_or_else(|| panic!("the in-memory scene mesh never spawned"));
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                .is_some(),
            "the in-memory scene mesh must carry the real material WorldInstanceReady copied"
        );
        // `PendingSectionCracks` is private to `damage_cracks` and
        // unreachable by type from here - the same gap every other crack
        // check in this file works around - so `SectionCracks` (`pub`) is
        // the proof that the real tracker (`mark_section_meshes`) picked the
        // mesh up the moment it existed: a pristine mesh resolves straight
        // to `SectionCracks { bucket: 0, .. }` (`damage_cracks.rs:431-441`).
        assert!(
            app.world().get::<SectionCracks>(mesh).is_some(),
            "a pristine in-memory scene mesh must already be tracked by the real crack pipeline"
        );

        app.world_mut().trigger(HealthApplyDamage {
            entity: source,
            source: None,
            amount: 70.0,
        });
        for _ in 0..15 {
            app.update();
        }
        assert!(
            mesh_wears_cracks(app.world(), mesh),
            "a damaged in-memory scene mesh must read as cracked"
        );

        app.world_mut().trigger(HealthApplyDamage {
            entity: source,
            source: None,
            amount: 10_000.0,
        });
        for _ in 0..30 {
            app.update();
        }
        let piece = {
            let mut pieces = app
                .world_mut()
                .query_filtered::<Entity, With<DetachedPieceMarker>>();
            pieces
                .iter(app.world())
                .next()
                .unwrap_or_else(|| panic!("no piece exists after the kill"))
        };
        let record = freeze_detached_piece(app.world(), piece)
            .expect("a detached piece has no unsettled process");
        let scene_node_record = record
            .nodes
            .iter()
            .find(|n| matches!(n.art, FrozenArtType::Scene { .. }))
            .unwrap_or_else(|| panic!("no scene node in the detached piece's freeze"));
        assert!(
            matches!(
                scene_node_record.art,
                FrozenArtType::Scene { cracked: true, .. }
            ),
            "freezing a detached, damaged scene must read cracked: true, got {:?}",
            scene_node_record.art
        );

        let mut state: SystemState<(
            Commands,
            Res<AssetServer>,
            Res<PlaceholderArt>,
            Option<Res<GameStyles>>,
            ResMut<SkinAssets>,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
        )> = SystemState::new(app.world_mut());
        let thawed = {
            let Ok((
                mut commands,
                asset_server,
                placeholder,
                styles,
                mut skin_assets,
                mut meshes,
                mut materials,
            )) = state.get_mut(app.world_mut())
            else {
                panic!("the thaw's resources are not available");
            };
            let thawed = thaw_detached_piece(
                &mut commands,
                &asset_server,
                &placeholder,
                styles.as_deref(),
                &mut skin_assets,
                &mut meshes,
                &mut materials,
                &record,
            );
            thawed
        };
        state.apply(app.world_mut());

        let thawed_scene = descendants(app.world(), thawed)
            .into_iter()
            .find(|&e| app.world().get::<WorldAssetRoot>(e).is_some())
            .unwrap_or_else(|| panic!("no thawed scene node"));
        // Proof part 3: the thaw queues the resume - the same cracked flag
        // the freeze read - for `apply_resumed_scene_poses` to burn back in
        // once its own `WorldInstanceReady` fires (`ResumedScenePoses`'s
        // fields are private, not `pub(crate)`, but visible here: `mod
        // tests` is a descendant of the module that defines them).
        let resumed = app
            .world()
            .get::<ResumedScenePoses>(thawed_scene)
            .unwrap_or_else(|| panic!("thaw never queued a ResumedScenePoses"));
        assert!(
            resumed.1,
            "a thawed cracked scene must queue ResumedScenePoses with cracked: true"
        );

        for _ in 0..15 {
            app.update();
        }

        let thawed_mesh = descendants(app.world(), thawed_scene)
            .into_iter()
            .find(|&e| app.world().get::<Mesh3d>(e).is_some())
            .unwrap_or_else(|| panic!("the thawed in-memory scene mesh never spawned"));
        assert!(
            mesh_wears_cracks(app.world(), thawed_mesh),
            "a thawed cracked scene's mesh must read as cracked once WorldInstanceReady fires"
        );
    }
}
