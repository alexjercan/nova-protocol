//! The port's body: its authored scene, or the placeholder block a section
//! with no art wears.
//!
//! Nothing here drives the sleeve. The sleeve is a named node inside that
//! scene, moved by the shared section-animation rig from the
//! [`SectionAnimationCue::DockTube`] target the port's state writes - so a
//! docking port gets its motion from the same machinery as a bay door and a
//! turret lid, and adds no second animator of its own.

use bevy::prelude::*;

use super::{DockingSectionMarker, DockingSectionRenderMesh};
use crate::prelude::*;

/// Spawn a docking port's body.
pub(super) fn insert_docking_section_render(
    add: On<Add, DockingSectionMarker>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
    asset_server: Res<AssetServer>,
    q_ports: Query<
        (&DockingSectionRenderMesh, &SectionRenderMeshTransform),
        With<DockingSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_docking_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform)) = q_ports.get(entity) else {
        error!("insert_docking_section_render: entity {entity:?} not found in q_ports");
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert(children![(
                Name::new("Docking Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            )]);
        }
        None => {
            commands.entity(entity).insert(children![(
                Name::new("Docking Section Body"),
                SectionRenderOf(entity),
                Mesh3d(placeholder.body.clone()),
                MeshMaterial3d(placeholder.structure_material.clone()),
            )]);
        }
    }
}
