//! What the bench shows: the section alone on the stage, and its sockets.

use bevy::{picking::Pickable, prelude::*};
use nova_assets::mod_refs::prelude::rewrite_refs;
use nova_modding::prelude::Content;
use nova_ship::prelude::{draw_socket, SectionCollider, SectionConfig, SectionKind};

use super::{mesh_fault, mesh_is_optional, set_mesh, AssetNode, SocketNode};
use crate::{
    asset_index::AssetIndex,
    config::{EditorCamera, EditorGizmos, EditorOverlays, SelectedNode},
    node::{spawn_node_view, NodeView},
    palette,
    project::ModProject,
    ExampleStates,
};

/// The radius of a socket's marker, in cells: a target for a click, sized so a
/// marker on a face does not cover the face.
const MARKER_RADIUS: f32 = 0.09;

/// The parts of a section its body is drawn from. A scrub of its health or its
/// price must not throw the body away.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BodyKey {
    collider: Option<SectionCollider>,
    kind: SectionKind,
}

impl BodyKey {
    fn of(config: &SectionConfig) -> Self {
        Self {
            collider: config.base.collider,
            kind: config.kind.clone(),
        }
    }
}

/// `config` as the stage can load it. A mesh file the mod does not hold loads
/// nothing, so it draws the kind's own mesh while the Problems tab says why.
fn staged(
    config: &SectionConfig,
    index: &AssetIndex,
    project: Option<&ModProject>,
) -> SectionConfig {
    let mut held = config.clone();
    let mesh = super::mesh_of(&held.kind);
    let missing = !index.resolves(&mesh)
        || project.is_some_and(|project| {
            let own = project.id.as_deref().unwrap_or_default();
            mesh_fault(own, &project.data, &mesh).is_some()
        });
    if missing && mesh_is_optional(&held.kind) {
        // Only the empty ref is written, and an optional field takes that.
        let _ = set_mesh(&mut held.kind, "");
    }
    let item = Content::Section(Box::new(held));
    match index.scoped(|scope| rewrite_refs(&item, scope)) {
        Content::Section(config) => *config,
        _ => unreachable!("a rewrite keeps the content's kind"),
    }
}

/// Keep the section's body on the document root.
#[expect(
    clippy::too_many_arguments,
    reason = "a body reads the node, the index and what it replaced"
)]
pub(crate) fn sync_bench_view(
    mut commands: Commands,
    benches: Query<(Entity, &AssetNode, Option<&Children>)>,
    views: Query<(), With<NodeView>>,
    children: Query<&Children>,
    index: AssetIndex,
    project: Option<Res<ModProject>>,
    mut gone: RemovedComponents<AssetNode>,
    mut shown: Local<Option<(Entity, BodyKey)>>,
) {
    for root in gone.read() {
        let Ok(held) = children.get(root) else {
            continue;
        };
        for view in held.iter().filter(|child| views.contains(*child)) {
            commands.entity(view).despawn();
        }
        if shown.as_ref().is_some_and(|(held, _)| *held == root) {
            *shown = None;
        }
    }
    let Some((root, node, held)) = benches.iter().next() else {
        return;
    };
    let Content::Section(config) = &node.0 else {
        return;
    };
    let key = BodyKey::of(config);
    let bodied = held.is_some_and(|held| held.iter().any(|child| views.contains(child)));
    if bodied && shown.as_ref() == Some(&(root, key.clone())) {
        return;
    }
    if let Some(held) = held {
        for view in held.iter().filter(|child| views.contains(*child)) {
            commands.entity(view).despawn();
        }
    }
    spawn_node_view(
        &mut commands,
        root,
        &staged(config, &index, project.as_deref()),
    );
    *shown = Some((root, key));
}

/// The mesh and material every socket marker shares.
#[derive(Clone)]
pub(crate) struct MarkerArt {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Give each socket the marker a click lands on. The ring around it is
/// [`draw_bench_sockets`], which the Link Points view turns off.
pub(crate) fn sync_socket_views(
    mut commands: Commands,
    sockets: Query<(Entity, Option<&Children>), With<SocketNode>>,
    views: Query<(), With<NodeView>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut art: Local<Option<MarkerArt>>,
) {
    let (Some(meshes), Some(materials)) = (meshes.as_mut(), materials.as_mut()) else {
        return;
    };
    for (socket, children) in &sockets {
        if children.is_some_and(|held| held.iter().any(|child| views.contains(child))) {
            continue;
        }
        let art = art
            .get_or_insert_with(|| MarkerArt {
                mesh: meshes.add(Sphere::new(MARKER_RADIUS)),
                material: materials.add(StandardMaterial {
                    base_color: palette::GO,
                    unlit: true,
                    ..default()
                }),
            })
            .clone();
        commands.entity(socket).with_children(|parent| {
            parent.spawn((
                DespawnOnExit(ExampleStates::Editor),
                NodeView,
                Name::new("Socket Marker"),
                Mesh3d(art.mesh),
                MeshMaterial3d(art.material),
                Transform::default(),
                Visibility::Inherited,
                Pickable::default(),
            ));
        });
    }
}

/// Draw each socket, facing out, the selected one aimed.
pub(crate) fn draw_bench_sockets(
    overlays: Res<EditorOverlays>,
    selected: Res<SelectedNode>,
    sockets: Query<(Entity, &GlobalTransform), With<SocketNode>>,
    cameras: Query<&GlobalTransform, With<EditorCamera>>,
    mut gizmos: Gizmos<EditorGizmos>,
) {
    if !overlays.link_points {
        return;
    }
    let Some(eye) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    for (socket, pose) in &sockets {
        let aimed = selected.iter().any(|node| node == socket);
        draw_socket(
            &mut gizmos,
            eye,
            pose.translation(),
            (pose.rotation() * Vec3::Z).normalize_or(Vec3::Z),
            if aimed { palette::AIMED } else { palette::GO },
            aimed,
        );
    }
}
