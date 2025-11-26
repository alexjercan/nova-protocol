/// A Bevy plugin that makes entities explode into pieces when they are destroyed.
///
/// This plugin listens for `ExplodeMesh` components being added to entities and
/// generates fragments from their meshes. Fragments are stored in an `ExplodeFragments` component
/// and can be used for visual effects or physics simulations.
use std::collections::VecDeque;

use bevy::prelude::*;
use rand::Rng;

use super::builder::TriangleMeshBuilder;

pub mod prelude {
    pub use super::{ExplodeFragments, ExplodeMesh, ExplodeMeshPlugin};
}

/// Maximum iterations for recursive mesh slicing.
const MAX_ITERATIONS: usize = 10;

/// A single fragment of an exploded mesh.
///
/// Contains a reference to the original entity, the mesh for this fragment, and a
/// normalized direction vector for movement or physics effects.
#[derive(Clone, Debug, Reflect)]
pub struct ExplodeFragment {
    /// The original entity from which this fragment was created.
    pub origin: Entity,
    /// The mesh of the fragment.
    pub mesh: Handle<Mesh>,
    /// The explosion direction (normalized).
    pub direction: Dir3,
}

/// Component storing the generated fragments from an exploded entity.
///
/// This component is added after the explosion is processed and can be used
/// to spawn visual fragments, apply physics, or perform further effects.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect)]
pub struct ExplodeFragments(pub Vec<ExplodeFragment>);

/// Component that triggers an explosion of a mesh into fragments.
///
/// Adding this component to an entity will cause the `ExplodeMeshPlugin` to
/// generate fragments for the entity's mesh and its children (recursively).
#[derive(Component, Clone, Debug, Reflect)]
pub struct ExplodeMesh {
    /// The number of fragments to generate for this explosion.
    pub fragment_count: usize,
}

impl Default for ExplodeMesh {
    fn default() -> Self {
        Self { fragment_count: 4 }
    }
}

/// Bevy plugin that handles mesh explosions for entities with `ExplodeMesh`.
pub struct ExplodeMeshPlugin;

impl Plugin for ExplodeMeshPlugin {
    fn build(&self, app: &mut App) {
        debug!("ExplodeMeshPlugin: build");

        // Observe when an ExplodeMesh component is added and handle explosion.
        app.add_observer(handle_explosion);
    }
}

/// Handle the explosion of an entity with `ExplodeMesh`.
///
/// This function recursively collects all mesh entities, slices their meshes
/// into fragments using random planes, and adds an `ExplodeFragments` component
/// to store the resulting fragments.
fn handle_explosion(
    add: On<Add, ExplodeMesh>,
    mut commands: Commands,
    q_explode: Query<(&ExplodeMesh, Option<&Children>)>,
    q_mesh: Query<(Entity, &Mesh3d), (With<Mesh3d>, With<MeshMaterial3d<StandardMaterial>>)>,
    q_children: Query<&Children>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let entity = add.entity;
    trace!("handle_explosion: entity {:?}", entity);

    let Ok((explode, children)) = q_explode.get(entity) else {
        error!(
            "handle_explosion: entity {:?} not found in q_explode.",
            entity
        );
        return;
    };

    let fragment_count = explode.fragment_count;

    // Collect all mesh entities, including children recursively
    let mut mesh_entities = Vec::new();
    if let Ok(mesh_entity) = q_mesh.get(entity) {
        mesh_entities.push(mesh_entity);
    }

    if let Some(children) = children {
        for child in children.iter() {
            let mut queue: VecDeque<Entity> = VecDeque::from([child]);
            while let Some(child) = queue.pop_front() {
                if let Ok(mesh_entity) = q_mesh.get(child) {
                    mesh_entities.push(mesh_entity);
                }

                if let Ok(child_children) = q_children.get(child) {
                    for grandchild in child_children {
                        queue.push_back(*grandchild);
                    }
                }
            }
        }
    }

    // Generate fragments for each mesh entity
    let mut fragment_meshes = Vec::new();
    for (mesh_entity, mesh3d) in mesh_entities.into_iter() {
        let Some(mesh) = meshes.get(&**mesh3d) else {
            error!(
                "handle_explosion: mesh_entity {:?} has no mesh data.",
                mesh_entity
            );
            return;
        };

        trace!(
            "handle_explosion: mesh_entity {:?} fragment_count {}",
            mesh_entity,
            fragment_count
        );

        let Some(fragments) = explode_mesh(&mesh.clone(), fragment_count, MAX_ITERATIONS) else {
            error!(
                "explode_mesh: entity {:?} failed to slice mesh into fragments.",
                entity
            );
            return;
        };

        for (mesh, normal) in fragments {
            fragment_meshes.push(ExplodeFragment {
                origin: mesh_entity,
                mesh: meshes.add(mesh.clone()),
                direction: Dir3::new_unchecked(normal.normalize()),
            });
        }
    }

    // Attach the generated fragments to the entity
    commands
        .entity(entity)
        .insert(ExplodeFragments(fragment_meshes));
}

/// Slice a mesh into fragments using random planes.
///
/// Returns `Some(Vec<(Mesh, Vec3)>)` containing the fragment mesh and its explosion direction.
/// Returns `None` if slicing fails or no fragments are generated.
fn explode_mesh(
    original: &Mesh,
    fragment_count: usize,
    max_iterations: usize,
) -> Option<Vec<(Mesh, Vec3)>> {
    let mut queue = VecDeque::from([(original.clone(), Vec3::ZERO)]);
    let mut rng = rand::rng();

    for _ in 0..max_iterations {
        let mut fragments = vec![];

        while let Some((mesh, _)) = queue.pop_front() {
            let plane_point = Vec3::ZERO;
            let plane_normal = {
                // Generate a random unit vector as plane normal
                let u: f32 = rng.random_range(-1.0..1.0);
                let theta: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                let r = (1.0 - u * u).sqrt();
                Vec3::new(r * theta.cos(), r * theta.sin(), u).normalize()
            };

            let Some((pos, neg)) = TriangleMeshBuilder::from(mesh).slice(plane_normal, plane_point)
            else {
                error!(
                    "explode_mesh: could not slice mesh with plane normal {:?} at point {:?}.",
                    plane_normal, plane_point
                );
                continue;
            };

            fragments.push((pos.build(), plane_normal));
            fragments.push((neg.build(), -plane_normal));
        }

        if fragments.len() >= fragment_count {
            return Some(fragments);
        } else if fragments.is_empty() {
            error!("explode_mesh: no fragments generated after slicing.");
            return None;
        } else {
            queue = VecDeque::from(fragments);
        }
    }

    None
}
