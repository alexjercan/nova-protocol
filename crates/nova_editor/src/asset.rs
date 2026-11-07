//! The bench: one item of the mod that is not a scenario, open in the shell.
//!
//! A scenario is a tree of ships and objects. A section, an item or a style is
//! one config, so it stands in the document as [`AssetNode`] on the root, beside
//! the root's [`ScenarioNode`](crate::node::ScenarioNode). The root stays a
//! scenario node on purpose: history paths, the edit context, the sky and the
//! framing all start there, and an undo across the two is one diff on one root.
//!
//! A section's parts are nodes under the root, so the Hierarchy lists them and
//! the gizmo, delete and undo reach them the way they reach a ship:
//!
//! ```text
//! root (ScenarioNode + AssetNode: the section, link points taken out)
//!   collider (AssetPartType)   selection handle, edits the root
//!   mesh     (AssetPartType)   selection handle, edits the root
//!   socket   (SocketNode)      one per link point, its pose in cells
//! ```
//!
//! [`lowered_item`] folds that back into one [`Content`]. What the file holds is
//! the only truth, so a socket keeps the normal it was read with until a turn
//! moves it, and an untouched section lowers equal to what was lifted.

pub(crate) mod session;
pub(crate) mod verbs;
pub(crate) mod view;

use bevy::prelude::*;
use nova_assets::mod_refs::prelude::SELF_SCHEME;
use nova_gameplay::prelude::AssetRef;
use nova_modding::prelude::Content;
use nova_ship::prelude::{
    LinkPoint, LinkPointSkinType, RenderMeshTransform, SectionConfig, SectionKind,
};

use self::verbs::glb_fault;
use crate::{
    config::EditorCamera,
    frame::{view_stage, CameraFraming, ViewAngle},
    node::{EditContext, EditorNode, NodeId},
    project::{store, ProjectData},
};

/// How close a socket's rotation must stay to the normal it was read with to
/// keep that normal. A turn moves it by 15 degrees at the least.
const SAME_FACING: f32 = 1.0e-4;

/// The grid a turned socket's normal is written to. A rotation holds a normal
/// to about seven digits, and the file should not read `0.99999994`.
const NORMAL_GRID: f32 = 1.0e5;

/// The item on the bench, held on the document root. A section's link points
/// are out of it and stand as [`SocketNode`]s.
#[derive(Component, Clone, Debug)]
pub(crate) struct AssetNode(pub(crate) Content);

/// Which part of a section a handle under the root stands for.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssetPartType {
    /// The authored collider.
    Collider,
    /// The render mesh and its transform.
    Mesh,
}

impl AssetPartType {
    /// Both, in tree order.
    pub(crate) const ALL: [Self; 2] = [Self::Collider, Self::Mesh];

    /// The node's id and tree label.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Collider => "collider",
            Self::Mesh => "mesh",
        }
    }

    /// What the tree and the Inspector call it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Collider => "Collider",
            Self::Mesh => "Mesh",
        }
    }
}

/// One link point of the section on the bench. Its id is the node's
/// [`NodeId`], and its position and facing are the node's `Transform`, in cells.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SocketNode {
    /// Its place in the file's list. Children reorder across an undo.
    pub(crate) order: u32,
    /// The normal it was read with. See [`SocketNode::normal`].
    pub(crate) authored: Vec3,
    /// What the skin does with its face.
    pub(crate) skin: LinkPointSkinType,
}

impl SocketNode {
    /// The outward normal a node with this rotation has.
    ///
    /// The authored value until a turn moves the socket: `rotation * Z` differs
    /// from it in the last digit, and a socket nobody turned must lower equal.
    pub(crate) fn normal(&self, rotation: Quat) -> Vec3 {
        if socket_rotation(self.authored).angle_between(rotation) < SAME_FACING {
            return self.authored;
        }
        let turned = rotation * Vec3::Z;
        ((turned * NORMAL_GRID).round() / NORMAL_GRID).normalize_or(Vec3::Z)
    }
}

/// The rotation that faces a node's `Z` along `normal`.
pub(crate) fn socket_rotation(normal: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Z))
}

/// Stand `item` on the bench under `root`.
pub(crate) fn lift_item(world: &mut World, root: Entity, item: &Content) {
    let mut held = item.clone();
    let sockets = match &mut held {
        Content::Section(config) => Some(std::mem::take(&mut config.base.link_points)),
        _ => None,
    };
    world.entity_mut(root).insert(AssetNode(held));
    let Some(sockets) = sockets else {
        return;
    };
    for part in AssetPartType::ALL {
        world.spawn((
            EditorNode,
            NodeId(part.id().to_string()),
            Name::new(format!("Asset Part {}", part.label())),
            part,
            Transform::default(),
            Visibility::Inherited,
            ChildOf(root),
        ));
    }
    for (order, socket) in sockets.iter().enumerate() {
        spawn_socket(world, root, socket, order as u32);
    }
}

/// Stand one socket under `root`.
pub(crate) fn spawn_socket(
    world: &mut World,
    root: Entity,
    socket: &LinkPoint,
    order: u32,
) -> Entity {
    world
        .spawn((
            EditorNode,
            NodeId(socket.id.clone()),
            Name::new(format!("Socket Node {}", socket.id)),
            SocketNode {
                order,
                authored: socket.normal,
                skin: socket.skin,
            },
            Transform::from_translation(socket.position)
                .with_rotation(socket_rotation(socket.normal)),
            Visibility::Inherited,
            ChildOf(root),
        ))
        .id()
}

/// The bench as the file will hold it, from the root's item and its sockets in
/// any order.
pub(crate) fn lower_bench<'a>(
    node: &AssetNode,
    sockets: impl Iterator<Item = (&'a NodeId, &'a Transform, &'a SocketNode)>,
) -> Content {
    let mut item = node.0.clone();
    if let Content::Section(config) = &mut item {
        let mut held: Vec<_> = sockets.collect();
        held.sort_by_key(|(_, _, socket)| socket.order);
        config.base.link_points = held
            .into_iter()
            .map(|(id, pose, socket)| LinkPoint {
                id: id.0.clone(),
                position: pose.translation,
                normal: socket.normal(pose.rotation),
                skin: socket.skin,
            })
            .collect();
    }
    item
}

/// The item on the bench under `root`, as the file will hold it.
pub(crate) fn lowered_item(world: &World, root: Entity) -> Option<Content> {
    let node = world.get::<AssetNode>(root)?;
    let sockets = world
        .get::<Children>(root)
        .into_iter()
        .flatten()
        .filter_map(|child| {
            Some((
                world.get::<NodeId>(*child)?,
                world.get::<Transform>(*child)?,
                world.get::<SocketNode>(*child)?,
            ))
        });
    Some(lower_bench(node, sockets))
}

/// Whether the document holds an item rather than a scenario.
pub(crate) fn on_bench(world: &World) -> bool {
    world
        .resource::<EditContext>()
        .scenario()
        .is_some_and(|root| world.get::<AssetNode>(root).is_some())
}

/// Face the editor's camera at the section under `root`: a section is about one
/// cell, which the stage's framing would leave a speck.
pub(crate) fn frame_bench(world: &mut World, root: Entity) {
    let Some(Content::Section(config)) = world.get::<AssetNode>(root).map(|node| &node.0) else {
        return;
    };
    let spread = config
        .base
        .collider
        .unwrap_or_default()
        .aabb_half_extents()
        .length();
    let pose = view_stage(Vec3::ZERO, spread, ViewAngle::default());
    let mut cameras = world.query_filtered::<&mut Transform, With<EditorCamera>>();
    let Some(mut camera) = cameras.iter_mut(world).next() else {
        return;
    };
    *camera = pose;
    *world.resource_mut::<CameraFraming>() = CameraFraming::on(Vec3::ZERO, pose.translation);
}

/// The section this item is, for a caller that reads only sections.
pub(crate) fn section_of(item: &Content) -> Option<&SectionConfig> {
    match item {
        Content::Section(config) => Some(config),
        _ => None,
    }
}

/// Where a section kind keeps its render mesh.
enum MeshSlot<'a> {
    /// Absent means the kind's built-in mesh.
    Optional(&'a mut Option<AssetRef<WorldAsset>>),
    /// The kind has no built-in mesh.
    Required(&'a mut AssetRef<WorldAsset>),
}

/// The render mesh field of `kind`. `None` for a turret, which holds one mesh
/// per joint of its tree.
fn mesh_slot(kind: &mut SectionKind) -> Option<MeshSlot<'_>> {
    Some(match kind {
        SectionKind::Hull(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::Thruster(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::Controller(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::Torpedo(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::Railgun(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::Docking(config) => MeshSlot::Optional(&mut config.render_mesh),
        SectionKind::CargoIntake(config) => MeshSlot::Required(&mut config.render_mesh),
        SectionKind::Mining(config) => MeshSlot::Required(&mut config.render_mesh),
        SectionKind::Turret(_) => return None,
    })
}

/// Whether `kind` has one render mesh field.
pub(crate) fn has_mesh(kind: &SectionKind) -> bool {
    !matches!(kind, SectionKind::Turret(_))
}

/// Whether `kind` can stand with no mesh ref.
pub(crate) fn mesh_is_optional(kind: &SectionKind) -> bool {
    !matches!(
        kind,
        SectionKind::CargoIntake(_) | SectionKind::Mining(_) | SectionKind::Turret(_)
    )
}

/// The ref in `kind`'s mesh field, or the empty string.
pub(crate) fn mesh_of(kind: &SectionKind) -> String {
    let mut kind = kind.clone();
    let held = match mesh_slot(&mut kind) {
        Some(MeshSlot::Optional(slot)) => slot.clone(),
        Some(MeshSlot::Required(slot)) => Some(slot.clone()),
        None => None,
    };
    held.and_then(|mesh| mesh.path().map(str::to_string))
        .unwrap_or_default()
}

/// Write `text` into `kind`'s mesh field, or say why it cannot go there.
pub(crate) fn set_mesh(kind: &mut SectionKind, text: &str) -> Result<(), String> {
    let text = text.trim();
    match mesh_slot(kind) {
        Some(MeshSlot::Optional(slot)) => {
            *slot = (!text.is_empty()).then(|| AssetRef::from(text.to_string()));
            Ok(())
        }
        Some(MeshSlot::Required(_)) if text.is_empty() => Err("this kind needs a mesh".to_string()),
        Some(MeshSlot::Required(slot)) => {
            *slot = AssetRef::from(text.to_string());
            Ok(())
        }
        None => Err("a turret holds a mesh per joint".to_string()),
    }
}

/// The `render_mesh_transform` field of `kind`. `None` for a turret, whose
/// joints each hold their own.
pub(crate) fn mesh_pose_mut(kind: &mut SectionKind) -> Option<&mut Option<RenderMeshTransform>> {
    Some(match kind {
        SectionKind::Hull(config) => &mut config.render_mesh_transform,
        SectionKind::Thruster(config) => &mut config.render_mesh_transform,
        SectionKind::Controller(config) => &mut config.render_mesh_transform,
        SectionKind::Torpedo(config) => &mut config.render_mesh_transform,
        SectionKind::Railgun(config) => &mut config.render_mesh_transform,
        SectionKind::Docking(config) => &mut config.render_mesh_transform,
        SectionKind::CargoIntake(config) => &mut config.render_mesh_transform,
        SectionKind::Mining(config) => &mut config.render_mesh_transform,
        SectionKind::Turret(_) => return None,
    })
}

/// What the tree and the Inspector call `item`: a section's name, else its id.
pub(crate) fn item_name(item: &Content) -> String {
    match item {
        Content::Section(config) => config.base.name.clone(),
        other => other.id().to_string(),
    }
}

/// What is wrong with `mesh` as a file `own` holds, or nothing. A ref into
/// another mod is the ref check's to judge, and an empty one draws no file.
pub(crate) fn mesh_fault(own: &str, data: &ProjectData, mesh: &str) -> Option<String> {
    let file = mesh.strip_prefix(SELF_SCHEME)?;
    let file = file.split('#').next().unwrap_or(file);
    if !data.resources.iter().any(|held| held == file) {
        return Some(format!("mesh {file} is not a resource of this mod"));
    }
    let Some(bytes) = store::mod_file(own, file) else {
        return Some(format!("mesh {file} is missing from the mod's folder"));
    };
    glb_fault(&bytes).map(|fault| format!("mesh {file} cannot be drawn: {fault}"))
}

/// What a section's body weighs: its collider's volume, at the one density a
/// section has.
pub(crate) fn section_mass(config: &SectionConfig) -> f32 {
    nova_ship::prelude::hull_mass_properties(std::iter::once((
        Vec3::ZERO,
        Quat::IDENTITY,
        config.base.collider.unwrap_or_default().to_collider(),
        1.0,
    )))
    .mass
}

#[cfg(test)]
mod tests;
