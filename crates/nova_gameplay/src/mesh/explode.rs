//! A Bevy plugin that makes entities explode into pieces when they are destroyed.
//!
//! This plugin listens for `ExplodeMesh` components being added to entities and
//! generates fragments from their meshes. Fragments are stored in an `ExplodeFragments` component
//! and can be used for visual effects or physics simulations.
//!
//! Nova owns this because it is the geometry half of destruction: fragment
//! counts and slice planes are tuned against [`crate::integrity`]'s debris
//! spawning, which is what reads [`ExplodeFragments`].

use std::collections::VecDeque;

use bevy::prelude::*;
use rand::RngExt;

use super::builder::TriangleMeshBuilder;

/// Glob-import surface: `use nova_gameplay::mesh::explode::prelude::*`
/// re-exports the public API of this module.
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

    let mut fragment_meshes = Vec::new();
    for (mesh_entity, mesh3d) in mesh_entities.into_iter() {
        // One bad mesh among many must not abort the whole explosion: the
        // fragment handler is the wreck's only despawn path
        // (`integrity/explode.rs` skips anything `With<Mesh3d>`), so returning
        // early left a zero-health wreck lingering with its collider live.
        let Some(mesh) = meshes.get(&**mesh3d) else {
            error!(
                "handle_explosion: mesh_entity {:?} has no mesh data.",
                mesh_entity
            );
            continue;
        };

        trace!(
            "handle_explosion: mesh_entity {:?} fragment_count {}",
            mesh_entity,
            fragment_count
        );

        let Some(fragments) = explode_mesh(&mesh.clone(), fragment_count, MAX_ITERATIONS) else {
            error!(
                "explode_mesh: mesh_entity {:?} failed to slice mesh into fragments.",
                mesh_entity
            );
            continue;
        };

        for (mesh, normal) in fragments {
            // NOTE: a carried (never-sliced) fragment has a zero direction and a bad
            // normal may be non-finite, so fall back to a fixed axis to keep Dir3 valid.
            let direction = Dir3::new(normal.normalize_or_zero()).unwrap_or(Dir3::Y);
            fragment_meshes.push(ExplodeFragment {
                origin: mesh_entity,
                mesh: meshes.add(mesh.clone()),
                direction,
            });
        }
    }

    commands
        .entity(entity)
        .insert(ExplodeFragments(fragment_meshes));
}

/// Generate a random unit vector, uniformly distributed on the sphere.
fn random_unit_vector(rng: &mut impl RngExt) -> Vec3 {
    let u: f32 = rng.random_range(-1.0..1.0);
    let theta: f32 = rng.random_range(0.0..std::f32::consts::TAU);
    let r = (1.0 - u * u).sqrt();
    Vec3::new(r * theta.cos(), r * theta.sin(), u).normalize()
}

/// Slice a mesh into fragments using random planes through the origin.
///
/// Returns `Some(Vec<(Mesh, Vec3)>)` pairing each fragment mesh with its
/// explosion direction, or `None` if the input mesh cannot be interpreted as
/// a triangle list or no non-empty fragment survives.
///
/// The mesh is converted once up front with the fallible
/// [`TriangleMeshBuilder::try_from_mesh`] and all slicing happens on builders,
/// so an untrusted input mesh can never panic the conversion. A fragment that
/// a given plane fails to split is carried forward intact rather than dropped.
fn explode_mesh(
    original: &Mesh,
    fragment_count: usize,
    max_iterations: usize,
) -> Option<Vec<(Mesh, Vec3)>> {
    let builder = TriangleMeshBuilder::try_from_mesh(original)?;
    let mut rng = rand::rng();

    // NOTE: the Vec3 in each entry is the normal of that fragment's last cut, which
    // becomes its explosion direction.
    let mut queue: Vec<(TriangleMeshBuilder, Vec3)> = vec![(builder, Vec3::ZERO)];

    for _ in 0..max_iterations {
        if queue.len() >= fragment_count {
            break;
        }

        let mut next: Vec<(TriangleMeshBuilder, Vec3)> = Vec::with_capacity(queue.len() * 2);
        for (mesh_builder, direction) in queue.drain(..) {
            let plane_normal = random_unit_vector(&mut rng);

            match mesh_builder.slice(plane_normal, Vec3::ZERO) {
                Some((pos, neg)) => {
                    next.push((pos, plane_normal));
                    next.push((neg, -plane_normal));
                }
                None => {
                    // NOTE: the plane missed this fragment (one side came out empty);
                    // keep it intact so no geometry is lost.
                    next.push((mesh_builder, direction));
                }
            }
        }

        queue = next;
    }

    let fragments: Vec<(Mesh, Vec3)> = queue
        .into_iter()
        .filter(|(builder, _)| !builder.is_empty())
        .map(|(builder, direction)| (builder.build(), direction))
        .collect();

    if fragments.is_empty() {
        error!("explode_mesh: no fragments generated after slicing.");
        return None;
    }

    Some(fragments)
}

#[cfg(test)]
mod test {
    use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};

    use super::*;

    /// Assert every position, normal and UV in the mesh is finite (no NaN /
    /// inf), which is what keeps Bevy from panicking on the AABB / GPU upload.
    fn assert_mesh_finite(mesh: &Mesh) {
        for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
            if let Some(VertexAttributeValues::Float32x3(vals)) = mesh.attribute(attr) {
                for v in vals {
                    assert!(v.iter().all(|c| c.is_finite()), "non-finite vertex {v:?}");
                }
            }
        }

        if let Some(VertexAttributeValues::Float32x2(vals)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            for v in vals {
                assert!(v.iter().all(|c| c.is_finite()), "non-finite uv {v:?}");
            }
        }
    }

    /// Plane normals are random, so this runs many times to hammer the RNG
    /// paths that produce degenerate slivers and parallel edges.
    #[test]
    fn test_explode_mesh_produces_finite_geometry() {
        let mesh = TriangleMeshBuilder::new_octahedron(2).build();

        for _ in 0..50 {
            let fragments = explode_mesh(&mesh, 8, MAX_ITERATIONS)
                .expect("an octahedron centered at the origin always splits");

            assert!(
                fragments.len() >= 2,
                "expected at least the first split, got {}",
                fragments.len()
            );

            for (fragment, direction) in &fragments {
                assert_mesh_finite(fragment);
                assert!(direction.is_finite(), "direction must be finite");
            }
        }
    }

    /// An index-less mesh must decline gracefully, not panic.
    #[test]
    fn test_explode_mesh_without_indices_returns_none() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );

        assert!(explode_mesh(&mesh, 4, MAX_ITERATIONS).is_none());
    }

    /// Drive the full `ExplodeMeshPlugin` observer path in a headless app -
    /// the same interaction `examples/05_explode.rs` triggers, on the same
    /// component shape its target uses - without a window or renderer. This is
    /// the integration test the graphical example cannot be for CI.
    #[test]
    fn test_explode_mesh_plugin_produces_fragments() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(ExplodeMeshPlugin);

        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(TriangleMeshBuilder::new_octahedron(2).build());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let entity = app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                ExplodeMesh { fragment_count: 8 },
            ))
            .id();

        app.update();

        let fragments = app
            .world()
            .entity(entity)
            .get::<ExplodeFragments>()
            .expect("ExplodeFragments should be inserted by the plugin");

        assert!(
            fragments.len() >= 2,
            "expected fragments, got {}",
            fragments.len()
        );

        let meshes = app.world().resource::<Assets<Mesh>>();
        for fragment in fragments.iter() {
            let mesh = meshes
                .get(&fragment.mesh)
                .expect("fragment mesh handle must resolve");
            assert_mesh_finite(mesh);
            assert!(fragment.direction.is_finite());
        }
    }
}
