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

/// `ExplodeMesh`, `ExplodeFragments` and `ExplodeMeshPlugin`.
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
/// Recursively collects the entity's render geometry - its own mesh and every
/// meshed descendant - slices it, and reports the result as `ExplodeFragments`.
///
/// THE ONLY PLACE that decides whether a destroyed body has geometry at all.
/// The component is inserted on every path, EMPTY when the walk found nothing,
/// because the integrity finale branches on that answer to choose between real
/// fragments and its fallback burst. Returning early instead would leave the
/// finale waiting on a component that never arrives and the wreck standing.
///
/// The walk is why the finale needs no `Mesh3d` on the gameplay entity: a ship
/// section holds its health and collider on a root whose art hangs several
/// levels below under a gltf `WorldAssetRoot`, and this reaches it.
fn handle_explosion(
    add: On<Add, ExplodeMesh>,
    mut commands: Commands,
    q_explode: Query<(&ExplodeMesh, Option<&Children>)>,
    // GEOMETRY ONLY. This used to require `MeshMaterial3d<StandardMaterial>`
    // as well, because the finale cloned the source material onto every
    // fragment - so a body drawn with any other material type was invisible to
    // the walk and fell through to the fallback burst. An asteroid moving to a
    // triplanar material was enough to do it, and the same would happen to
    // every section the moment one wears a shader of its own.
    //
    // What a fragment is DRAWN with is `FragmentMaterial`'s business now, and
    // the walk asks about nothing but geometry.
    q_mesh: Query<(Entity, &Mesh3d)>,
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

    // The budget belongs to the BODY, not to each mesh it happens to draw
    // with. An authored gltf section is many mesh entities and a procedural one
    // is a single mesh, so charging it per mesh made a multi-part turret cost
    // several times a hull cube for the same death - and a capital collapse
    // multiplies that across every section at once.
    //
    // More meshes than budget is not an error: each simply comes off whole,
    // which is what a turret parting at its joints should look like anyway.
    // Meshes past the budget are dropped rather than emitted uncounted, so the
    // cap is a real cap; they are the smallest detail on a body being
    // destroyed.
    let considered = mesh_entities.len().min(fragment_count.max(1));
    if mesh_entities.len() > considered {
        debug!(
            "handle_explosion: entity {:?} draws with {} meshes, budget covers {}",
            entity,
            mesh_entities.len(),
            considered
        );
    }
    let per_mesh = (fragment_count / considered.max(1)).max(1);

    let mut fragment_meshes = Vec::new();
    for (mesh_entity, mesh3d) in mesh_entities.into_iter().take(considered) {
        // One bad mesh among many must not abort the whole explosion: the
        // fragment handler is the wreck's only despawn path, so returning early
        // left a zero-health wreck lingering with its collider live.
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
            per_mesh
        );

        let Some(fragments) = explode_mesh(mesh, per_mesh, MAX_ITERATIONS) else {
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
                mesh: meshes.add(mesh),
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
        for (mesh_builder, direction) in queue {
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

    /// Spawn a body that draws with `parts` separate meshes, the way an
    /// authored gltf section does, and explode it with `budget`.
    fn fragments_of_a_body_with(parts: usize, budget: usize) -> usize {
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

        let body = app.world_mut().spawn_empty().id();
        for _ in 0..parts {
            app.world_mut().spawn((
                ChildOf(body),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
            ));
        }
        app.world_mut().entity_mut(body).insert(ExplodeMesh {
            fragment_count: budget,
        });
        app.update();

        app.world()
            .entity(body)
            .get::<ExplodeFragments>()
            .expect("the walk reports its result either way")
            .len()
    }

    /// The budget belongs to the BODY. Charging it per mesh made a multi-part
    /// turret cost several times a hull cube for the same death, and a capital
    /// collapse multiplies that across every section at once.
    #[test]
    fn the_fragment_budget_is_spent_across_the_whole_body() {
        // One mesh spends the whole budget on itself.
        assert_eq!(fragments_of_a_body_with(1, 4), 4);
        // Many meshes divide it rather than each claiming it: six parts at a
        // budget of four used to yield twenty-four pieces.
        assert!(
            fragments_of_a_body_with(6, 4) <= 4,
            "six parts must not out-spend the budget, got {}",
            fragments_of_a_body_with(6, 4)
        );
        // And a body that draws with nothing reports so, rather than staying
        // silent - the integrity finale needs that answer to run its fallback.
        assert_eq!(fragments_of_a_body_with(0, 4), 0);
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
