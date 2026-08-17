//! The visual side of destruction: when an [`ExplodableEntity`] is destroyed
//! its render geometry is sliced into debris fragments (tagged
//! [`MeshFragmentMarker`]) that are spawned with physics and a bounded
//! lifetime. Reacts to the destruction events fired by the integrity glue
//! rather than deciding when something dies.
//!
//! # The finale is decided ONCE
//!
//! Destructibility is SEMANTIC - it is [`ExplodableEntity`] plus a destroy
//! marker, and never where a `Mesh3d` happens to sit. A section carries health,
//! a collider and the marker on a gameplay root while its art hangs off
//! `SectionRenderOf` descendants under a gltf `WorldAssetRoot`, so a finale
//! gated on `Mesh3d` at the gameplay entity found nothing on every section ever
//! shipped and sent all of them to the generic cube burst.
//!
//! The fix is not a wider filter. Two observers agreeing about whether geometry
//! exists is what let one death emit both real fragments and fallback cubes, so
//! the walk for geometry happens in exactly one place
//! ([`handle_explosion`](crate::mesh::explode)) and its RESULT - a possibly
//! empty [`ExplodeFragments`] - is the only thing the finale branches on.
//!
//! Touch this module to change how wrecks come apart (fragment count, spread,
//! lifetime). Health, disable, and destroy bookkeeping lives in the generic
//! [`core`](super::core) integrity layer; structure owners publish its graph.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::*;
use nova_events::prelude::*;
use rand::RngExt;

use super::components::prelude::*;
use crate::{
    integrity::neutralize::prelude::DefeatedMarker,
    lifetime::TempEntity,
    mesh::prelude::{ExplodeFragments, ExplodeMesh},
    prelude::SpaceshipRootMarker,
};

/// `ExplodableEntity`, `FragmentMaterial`, `MeshFragmentMarker` and the
/// per-body fragment budget.
pub mod prelude {
    pub use super::{ExplodableEntity, FragmentMaterial, MeshFragmentMarker, BODY_FRAGMENT_BUDGET};
}

/// How long a sliced mesh fragment survives before it despawns.
///
/// Fragments are dynamic rigid bodies with convex-hull colliders and nothing
/// despawns them otherwise until scenario teardown; an unattended scene that
/// keeps destroying rocks (a menu backdrop, a long mission) accumulates
/// physics bodies without bound. Long enough to watch the wreck drift apart,
/// short enough to keep the body count flat.
const MESH_FRAGMENT_LIFETIME_SECS: f32 = 30.0;

/// How many pieces one destroyed body may come apart into, WHOLE BODY.
///
/// Spent across everything the body draws with rather than per mesh: an
/// authored gltf section is many mesh entities and a procedural one is a single
/// mesh, and charging the budget per mesh made a multi-part turret cost several
/// times a hull cube for the same death. See
/// [`handle_explosion`](crate::mesh::explode) for how it is divided.
///
/// Public because it is a CONTRACT, not just a tuning number: a capital-scale
/// collapse multiplies it by every section dying at once, so the
/// `destruction_finale` range asserts real deaths against it rather than
/// against a copy that could drift.
pub const BODY_FRAGMENT_BUDGET: usize = 4;

/// The thinnest a fragment's collider may be, in the mesh's own units.
///
/// Ship art is full of FLAT panels, and a cut across one leaves coplanar
/// vertices. parry hulls those into a shape with no volume, avian gives a
/// volumeless dynamic body zero mass AND zero inertia, and the solver divides
/// by it: the body's swept AABB comes back NaN and avian asserts on it frames
/// later, deep inside `update_solver_body_aabbs`
/// (`assertion failed: b.min.cmple(b.max).all()`). That took down a
/// capital-scale fight the first time sections fragmented for real.
///
/// Asteroids never found this. A rock is a blob and every piece of one has
/// volume; it took section art to produce a flat shard.
const FRAGMENT_MIN_THICKNESS: f32 = 0.02;

/// Marker component to indicate that an entity can be exploded.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ExplodableEntity;

/// Tags a debris fragment spawned when an [`ExplodableEntity`] is destroyed -
/// one piece of the sliced mesh, given physics and a fade-out lifetime.
#[derive(Component, Debug, Clone, Reflect)]
pub struct MeshFragmentMarker;

/// What the pieces of this body should be DRAWN with when it comes apart.
///
/// Carried by a render entity whose own material its fragments must not
/// inherit. Two reasons, and the second is the one that matters:
///
/// - the finale can only clone a `StandardMaterial`, so a body wearing any
///   other material type had nothing to hand its fragments and fell through the
///   geometry walk entirely;
/// - and it should not hand them its own anyway. The rock and section shaders
///   sample by the body's OWN LOCAL POSITION - which is what stops the pattern
///   swimming as the body tumbles - so a fragment, being a new body with a new
///   origin, would draw that pattern from the wrong place. A chunk is not a
///   small copy of the thing it broke off.
///
/// Absent means the fragments inherit the entity's own `StandardMaterial`,
/// which is what every plainly-materialled body already did and still does.
#[derive(Component, Debug, Clone, Reflect)]
pub struct FragmentMaterial(pub Handle<StandardMaterial>);

/// Bare debris: what a piece is drawn with when nothing better is on offer.
///
/// The last resort of the MATERIAL question, which is a different question from
/// the geometry one - a body can have perfectly good geometry to come apart
/// into and still wear a material its fragments cannot inherit. Anonymous grey
/// metal is a poor look and a much better one than an undrawn chunk.
fn debris_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.9,
        metallic: 0.4,
        ..default()
    }
}

pub(super) struct ExplodablePlugin;

impl Plugin for ExplodablePlugin {
    fn build(&self, app: &mut App) {
        debug!("ExplodablePlugin: build");

        app.add_observer(on_add_explodable_entity);
        app.add_observer(on_destroyed_entity);
        app.add_observer(on_explode_entity);
        app.add_observer(handle_entity_explosion);
        app.add_observer(despawn_destroyed_without_finale);
    }
}

/// Scatter a short-lived burst of generic cubes, for a body that had no render
/// geometry to come apart into.
///
/// The LAST RESORT, not the section path. It runs when the geometry walk came
/// back empty: art that has not finished loading (a gltf `WorldAssetRoot` scene
/// instantiates asynchronously, and a section can die before it lands), a body
/// that draws nothing, or a mesh the slicer could not interpret. Anything with
/// real geometry breaks into that geometry instead.
///
/// Called from [`handle_entity_explosion`] rather than observing destruction on
/// its own, because "no geometry" is only knowable AFTER the walk - and two
/// observers each deciding it separately is how one death used to emit both
/// cubes and fragments.
fn spawn_fallback_burst(
    commands: &mut Commands,
    entity: Entity,
    origin: Vec3,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rng: &mut WyRand,
) {
    trace!(
        "spawn_fallback_burst: bursting {:?} with no geometry",
        entity
    );

    let mesh = meshes.add(Cuboid::new(0.25, 0.25, 0.25));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.9,
        metallic: 0.4,
        ..default()
    });

    for _ in 0..8 {
        let direction = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
            rng.random_range(-1.0..1.0),
        )
        .normalize_or_zero();
        let velocity = direction * rng.random_range(3.0..7.0);

        commands.spawn((
            MeshFragmentMarker,
            Name::new("Fallback Debris"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(origin + direction * 0.3),
            RigidBody::Dynamic,
            Collider::cuboid(0.25, 0.25, 0.25),
            LinearVelocity(velocity),
            AngularVelocity(Vec3::new(
                rng.random_range(-6.0..6.0),
                rng.random_range(-6.0..6.0),
                rng.random_range(-6.0..6.0),
            )),
            TempEntity(2.0),
        ));
    }
}

/// A collider avian can actually simulate for a fragment of `mesh`, or `None`
/// when the mesh has no bounds to work from and there is nothing to spawn.
///
/// Prefers the true convex hull, and falls back to the mesh's own bounding box
/// padded to [`FRAGMENT_MIN_THICKNESS`] whenever that hull would leave the body
/// without mass. MASS is the property tested rather than the vertex layout,
/// because it is the one the solver divides by - a hull can be judged
/// non-degenerate by its bounds and still come back with none.
///
/// A slightly boxy shard inside a debris burst that lasts a few seconds is not
/// something a player can see. A NaN rigid body is a crash.
fn fragment_collider(mesh: &Mesh) -> Option<Collider> {
    let (centre, half) = mesh_bounds(mesh)?;

    if half.min_element() > FRAGMENT_MIN_THICKNESS {
        if let Some(hull) = Collider::convex_hull_from_mesh(mesh) {
            if hull.mass_properties(1.0).mass > 0.0 {
                return Some(hull);
            }
        }
    }

    // Offset by the bounds' centre: a fragment is a piece cut off a bigger
    // mesh, so its geometry sits wherever it sat in the original and a box
    // centred on the entity would not cover it.
    let padded = half.max(Vec3::splat(FRAGMENT_MIN_THICKNESS));
    Some(Collider::compound(vec![(
        centre,
        Quat::IDENTITY,
        Collider::cuboid(padded.x * 2.0, padded.y * 2.0, padded.z * 2.0),
    )]))
}

/// A mesh's local centre and half extents, or `None` when it carries no finite
/// positions to measure.
///
/// Read straight off the position attribute rather than through bevy's
/// `compute_aabb`, so the finite check is on the numbers this module is about
/// to hand to the physics engine.
fn mesh_bounds(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    use bevy::mesh::VertexAttributeValues;
    let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for position in positions {
        let point = Vec3::from_array(*position);
        if !point.is_finite() {
            return None;
        }
        min = min.min(point);
        max = max.max(point);
    }
    (min.cmple(max).all()).then(|| ((min + max) * 0.5, (max - min) * 0.5))
}

/// Despawn a destroyed entity that the finale does not own.
///
/// [`handle_entity_explosion`] despawns everything it runs on, which is every
/// destroyed [`ExplodableEntity`] that is not an [`IntegrityRoot`]. This reaper
/// covers the rest, and it must: a zero-health node left standing keeps its
/// collider live and its capabilities working.
///
/// Two kinds reach it. Destructible things that are not explodable - skin
/// plates and other fixtures, which come off rather than break apart - and
/// integrity roots, which [`on_explode_entity`] deliberately skips. Recursive,
/// so a section's gltf children go with it; `try_despawn` because a sibling
/// reaction can have taken the entity already.
fn despawn_destroyed_without_finale(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_destroyed: Query<(Has<ExplodableEntity>, Has<IntegrityRoot>), With<IntegrityDestroyMarker>>,
) {
    let entity = add.entity;
    let Ok((explodable, is_root)) = q_destroyed.get(entity) else {
        return;
    };
    if explodable && !is_root {
        // The finale owns this one and despawns it once its fragments exist.
        return;
    }

    trace!("despawn_destroyed_without_finale: despawning {:?}", entity);
    commands.entity(entity).try_despawn();
}

fn on_add_explodable_entity(
    add: On<Add, ExplodableEntity>,
    mut commands: Commands,
    q_explode: Query<&ChildOf, With<ExplodableEntity>>,
) {
    let entity = add.entity;
    trace!("on_add_explodable_entity: entity {:?}", entity);

    let Ok(ChildOf(parent)) = q_explode.get(entity) else {
        return;
    };

    debug!(
        "on_add_explodable_entity: entity {:?} is child of {:?}, adding ExplodableEntity to parent",
        entity, *parent
    );

    commands.entity(*parent).insert(ExplodableEntity);
}

fn on_destroyed_entity(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_info: Query<
        (
            &EntityId,
            &EntityTypeName,
            Has<SpaceshipRootMarker>,
            Has<DefeatedMarker>,
        ),
        With<IntegrityDestroyMarker>,
    >,
) {
    let entity = add.entity;
    trace!("on_destroyed_entity: entity {:?}", entity);

    let Ok((id, type_name, is_ship, already_defeated)) = q_info.get(entity) else {
        return;
    };

    debug!(
        "on_destroyed_entity: entity {:?} destroyed (id: {:?}, type: {:?})",
        entity, id, type_name
    );
    let id = id.to_string();
    let type_name = type_name.to_string();
    if is_ship && !already_defeated {
        // Persist the exact-once guard through the rest of this frame. This
        // also prevents neutralization detection from reporting a second
        // defeat if destruction and section loss converge in one update.
        // try_insert: `despawn_destroyed_without_mesh` observes this same Add
        // event and observer order is unspecified, so its queued despawn can
        // land before this insert; a plain insert then panics at apply time.
        // A root despawned mid-batch cannot be defeated twice, so losing the
        // marker is harmless - the events below still fire either way.
        // A sibling destruction reaction can despawn the root in this command flush.
        commands.entity(entity).try_insert(DefeatedMarker);
        commands.fire::<OnDefeatedEvent>(OnDefeatedEventInfo {
            id: id.clone(),
            type_name: type_name.clone(),
        });
    }
    commands.fire::<OnDestroyedEvent>(OnDestroyedEventInfo { id, type_name });
}

/// Start the finale for a destroyed [`ExplodableEntity`].
///
/// The entity needs NO `Mesh3d` of its own. Requiring one here is what sent
/// every ship section ever shipped to the generic cube burst: a section carries
/// the gameplay components and draws through `SectionRenderOf` descendants, so
/// the gameplay entity never has a mesh no matter how much art hangs off it.
/// Whether real geometry exists is settled by the walk in
/// [`handle_explosion`](crate::mesh::explode), which can answer honestly.
///
/// An [`IntegrityRoot`] is excluded. `ExplodableEntity` propagates to parents
/// (see [`on_add_explodable_entity`]), so a root carries it too - and a root is
/// by definition the entity whose descendants are its integrity NODES rather
/// than its own art. Fragmenting them together would spend one body's budget on
/// a whole structure and bypass every node's own finale.
///
/// The test is `IntegrityRoot` rather than `SpaceshipRootMarker` because there
/// is more than one kind of root and the others are not spaceships: a severed
/// wreck body carries `ShipWreckFragmentMarker` and no ship marker at all, and
/// an asteroid's root is a husk whose collider, health and mesh live on a child
/// node. Naming the concept instead of one of its cases also keeps this layer
/// free of anything `nova_ship` owns.
///
/// Nothing is lost by skipping roots: a structure comes apart node by node,
/// each bursting as itself, and the root is only taken once they are gone.
fn on_explode_entity(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_explode: Query<
        (),
        (
            With<ExplodableEntity>,
            With<IntegrityDestroyMarker>,
            Without<IntegrityRoot>,
        ),
    >,
) {
    let entity = add.entity;
    trace!("on_explode_entity: entity {:?}", entity);

    let Ok(_) = q_explode.get(entity) else {
        return;
    };

    debug!("on_explode_entity: entity {:?} will explode", entity);
    commands.entity(entity).insert(ExplodeMesh {
        fragment_count: BODY_FRAGMENT_BUDGET,
    });
}

/// THE finale, and the only place it is decided.
///
/// Branches on what the geometry walk actually produced rather than on a
/// predicate about what it might produce. A non-empty [`ExplodeFragments`]
/// becomes physics debris; an empty one - unloaded art, a body that draws
/// nothing, a mesh the slicer declined - becomes the fallback burst. One
/// destroyed body therefore emits one or the other and never both, which two
/// independent observers could not guarantee.
///
/// Despawns the body either way, on every path including the error ones. It is
/// the only thing that despawns a destroyed explodable, so an early return
/// leaves a zero-health wreck standing with a live collider.
fn handle_entity_explosion(
    add: On<Add, ExplodeFragments>,
    mut commands: Commands,
    q_explode: Query<(&ExplodeFragments, Option<&GlobalTransform>), With<ExplodableEntity>>,
    q_mesh: Query<
        (
            &GlobalTransform,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&FragmentMaterial>,
        ),
        With<Mesh3d>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let entity = add.entity;
    trace!("handle_entity_explosion: entity {:?}", entity);

    let Ok((fragments, body)) = q_explode.get(entity) else {
        error!(
            "handle_entity_explosion: entity {:?} not found in q_explode.",
            entity,
        );
        return;
    };
    // Where the body was, for the fallback burst and for aiming whole pieces
    // away from the middle of the wreck.
    let centre = body.map(|transform| transform.translation());

    if fragments.is_empty() {
        spawn_fallback_burst(
            &mut commands,
            entity,
            centre.unwrap_or(Vec3::ZERO),
            &mut meshes,
            &mut materials,
            &mut rng,
        );
        commands.entity(entity).despawn();
        return;
    }

    // Minted at most once for the whole body, and only if something actually
    // needs it: a body wearing an exotic material that never named a fragment
    // material still has to come apart as SOMETHING, and bare debris grey is a
    // better answer than not drawing the piece at all.
    let mut anonymous: Option<Handle<StandardMaterial>> = None;

    for fragment in fragments.iter() {
        let Ok((transform, own_material, fragment_material)) = q_mesh.get(fragment.origin) else {
            error!(
                "handle_entity_explosion: mesh_entity {:?} not found in q_mesh.",
                fragment.origin,
            );
            continue;
        };
        // A named fragment material wins, then the entity's own standard one.
        // See `FragmentMaterial` for why a body that names one is not just
        // working around a type mismatch.
        let mesh_material = match (fragment_material, own_material) {
            (Some(FragmentMaterial(handle)), _) => MeshMaterial3d(handle.clone()),
            (None, Some(material)) => material.clone(),
            (None, None) => MeshMaterial3d(
                anonymous
                    .get_or_insert_with(|| materials.add(debris_material()))
                    .clone(),
            ),
        };

        let transform = transform.compute_transform();
        // A piece that was never cut carries no meaningful cut normal, so it
        // flies off the way it already sat: outward from the body's middle.
        // That is what makes a multi-part turret come apart at its joints
        // instead of every piece sliding the same way. A single-mesh body has
        // no offset to read, and keeps its slice normal.
        let direction = centre
            .and_then(|centre| Dir3::new(transform.translation - centre).ok())
            .unwrap_or(fragment.direction);
        let offset = direction * 0.5;
        let velocity = direction * rng.random_range(2.0..5.0);
        let transform = transform.with_translation(transform.translation + offset);
        // A collider is scaled by the entity's transform, so a degenerate scale
        // flattens even a good hull back to no volume - the same NaN body
        // FRAGMENT_MIN_THICKNESS exists to prevent. Authored art is the source
        // of this scale, so it is not this module's to trust.
        if !transform.scale.is_finite() || transform.scale.min_element() <= f32::EPSILON {
            debug!(
                "handle_entity_explosion: fragment of {:?} has an unusable scale {:?}",
                fragment.origin, transform.scale
            );
            continue;
        }
        let Some(mesh) = meshes.get(&fragment.mesh) else {
            error!(
                "handle_entity_explosion: mesh_entity {:?} has no mesh data.",
                fragment.origin,
            );
            continue;
        };
        let Some(collider) = fragment_collider(mesh) else {
            debug!(
                "handle_entity_explosion: fragment of {:?} has no usable bounds",
                fragment.origin
            );
            continue;
        };

        commands.spawn((
            MeshFragmentMarker,
            Name::new(format!("Explosion Fragment of {:?}", entity)),
            Mesh3d(fragment.mesh.clone()),
            mesh_material,
            transform,
            RigidBody::Dynamic,
            collider,
            LinearVelocity(velocity),
            TempEntity(MESH_FRAGMENT_LIFETIME_SECS),
        ));
    }

    commands.entity(entity).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{builder::TriangleMeshBuilder, explode::ExplodeFragment};

    #[derive(Resource, Default)]
    struct FiredEvents(Vec<&'static str>);

    fn destruction_event_app() -> App {
        let mut app = App::new();
        app.init_resource::<FiredEvents>();
        app.add_observer(on_destroyed_entity);
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<FiredEvents>| {
            fired.0.push(event.name());
        });
        app
    }

    #[test]
    fn direct_ship_destruction_defeats_before_it_destroys() {
        let mut app = destruction_event_app();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("ship"),
                EntityTypeName::new("spaceship"),
            ))
            .id();

        app.world_mut()
            .entity_mut(ship)
            .insert(IntegrityDestroyMarker);

        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDefeatedEvent::name(), OnDestroyedEvent::name()]
        );
        assert!(
            app.world().entity(ship).contains::<DefeatedMarker>(),
            "direct destruction persists the exact-once guard"
        );
    }

    #[test]
    fn defeat_survives_a_sibling_observer_despawning_the_root() {
        // Bevy gives no ordering between two observers of one Add event, so
        // `despawn_destroyed_without_mesh` can queue the root's despawn BEFORE
        // this observer's DefeatedMarker insert. Model that exact queue shape:
        // a despawn already queued ahead of the observer pass. The marker
        // insert must tolerate the dead root (a plain insert panics at
        // command-apply time and takes down the whole frame - the 2026-08-13
        // "The Raid" crash), and the defeat/destroy events must still reach
        // the scenario.
        let mut app = destruction_event_app();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("raider"),
                EntityTypeName::new("spaceship"),
            ))
            .id();

        app.world_mut().commands().entity(ship).try_despawn();
        app.world_mut()
            .entity_mut(ship)
            .insert(IntegrityDestroyMarker);

        assert!(!app.world().entities().contains(ship));
        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDefeatedEvent::name(), OnDestroyedEvent::name()],
            "the scenario still hears a kill whose root died mid-batch"
        );
    }

    #[test]
    fn destroying_an_already_defeated_wreck_does_not_defeat_it_twice() {
        let mut app = destruction_event_app();
        let wreck = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                DefeatedMarker,
                EntityId::new("wreck"),
                EntityTypeName::new("spaceship"),
            ))
            .id();

        app.world_mut()
            .entity_mut(wreck)
            .insert(IntegrityDestroyMarker);

        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDestroyedEvent::name()]
        );
    }

    #[test]
    fn mesh_fragments_carry_a_bounded_lifetime() {
        // Fragments are the only destruction debris nothing else cleans up:
        // an unattended scene (menu backdrop) must not accumulate physics
        // bodies without bound, so every fragment must ride TempEntity.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.add_observer(handle_entity_explosion);

        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let origin = app
            .world_mut()
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().spawn((
            ExplodableEntity,
            ExplodeFragments(vec![ExplodeFragment {
                origin,
                mesh,
                direction: Dir3::X,
            }]),
        ));
        app.update();

        let mut q_fragments = app
            .world_mut()
            .query_filtered::<Has<TempEntity>, With<MeshFragmentMarker>>();
        let lifetimes: Vec<bool> = q_fragments.iter(app.world()).collect();
        assert!(!lifetimes.is_empty(), "explosion must spawn fragments");
        assert!(
            lifetimes.iter().all(|has| *has),
            "every fragment must carry a despawn lifetime"
        );
    }

    /// The whole finale, wired the way the game wires it: the geometry walk
    /// and the integrity reaction together. Anything less lets the two halves
    /// disagree, which is the failure this module is built to prevent.
    fn finale_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.add_plugins(crate::mesh::prelude::ExplodeMeshPlugin);
        app.add_plugins(ExplodablePlugin);
        app
    }

    /// A body that draws the way every shipped section draws: nothing on the
    /// gameplay entity, art two levels down. Returns the gameplay root.
    fn body_drawing_through_descendants(app: &mut App, parts: usize) -> Entity {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(TriangleMeshBuilder::new_octahedron(2).build());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());

        let root = app
            .world_mut()
            .spawn((
                ExplodableEntity,
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        // The `SectionRenderOf` child a section spawns, standing in for the
        // gltf scene root; the meshes hang under IT, not under the root.
        let art = app
            .world_mut()
            .spawn((
                ChildOf(root),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        for part in 0..parts {
            let offset = Vec3::X * part as f32;
            app.world_mut().spawn((
                ChildOf(art),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(offset),
                GlobalTransform::from_translation(offset),
            ));
        }
        root
    }

    /// Debris on the ground, split by which path produced it: real mesh
    /// fragments, and generic fallback cubes.
    fn debris(app: &mut App) -> (usize, usize) {
        let mut q_debris = app
            .world_mut()
            .query_filtered::<&Name, With<MeshFragmentMarker>>();
        let names: Vec<String> = q_debris
            .iter(app.world())
            .map(|name| name.to_string())
            .collect();
        let fallback = names
            .iter()
            .filter(|name| name.starts_with("Fallback"))
            .count();
        (names.len() - fallback, fallback)
    }

    /// THE regression. Every section ever shipped holds its health and collider
    /// on a gameplay root and draws through descendants, so a finale gated on
    /// `Mesh3d` at that root found nothing on all of them and burst eight gray
    /// cubes no matter what art they had.
    #[test]
    fn a_body_drawing_through_descendants_breaks_into_its_own_art() {
        let mut app = finale_app();
        let body = body_drawing_through_descendants(&mut app, 1);

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        let (fragments, fallback) = debris(&mut app);
        assert!(
            fragments > 0,
            "a section's art must become its debris, got {fragments} fragments"
        );
        assert_eq!(fallback, 0, "and the generic burst must not run as well");
        assert!(
            !app.world().entities().contains(body),
            "the wreck is despawned once its fragments exist"
        );
    }

    /// The fallback is now exactly what it claims to be: what happens when
    /// there was nothing to break. A section can die before its gltf scene has
    /// finished instantiating, and that is the case this covers.
    #[test]
    fn a_body_with_no_geometry_falls_back_to_the_generic_burst() {
        let mut app = finale_app();
        let body = app
            .world_mut()
            .spawn((
                ExplodableEntity,
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        let (fragments, fallback) = debris(&mut app);
        assert_eq!(fragments, 0, "there was no art to fragment");
        assert!(fallback > 0, "so the body still visibly comes apart");
        assert!(
            !app.world().entities().contains(body),
            "and is despawned rather than left standing at zero health"
        );
    }

    /// A root carries `ExplodableEntity` by propagation, and its descendants
    /// are whole integrity NODES rather than its own art. Fragmenting them
    /// together would spend one body's budget on an entire structure and rob
    /// every node of its own finale.
    ///
    /// Run over BOTH kinds of root, because the exclusion used to name
    /// `SpaceshipRootMarker` and there is more than one root: a severed wreck
    /// body carries `ShipWreckFragmentMarker` and no ship marker at all, so a
    /// ship-specific test passed while a cut-loose wreck still fragmented every
    /// section hanging off it at once.
    #[test]
    fn no_kind_of_root_fragments_the_structure_hanging_off_it() {
        for ship in [true, false] {
            let mut app = finale_app();
            let root = body_drawing_through_descendants(&mut app, 2);
            // An IntegrityRoot either way; only one of the two is a spaceship.
            app.world_mut().entity_mut(root).insert(IntegrityRoot);
            if ship {
                app.world_mut().entity_mut(root).insert(SpaceshipRootMarker);
            }

            app.world_mut()
                .entity_mut(root)
                .insert(IntegrityDestroyMarker);
            app.update();

            assert_eq!(
                debris(&mut app),
                (0, 0),
                "a root emits neither fragments nor cubes (spaceship: {ship})"
            );
            assert!(
                !app.world().entities().contains(root),
                "but the reaper still takes it - nothing else would (spaceship: {ship})"
            );
        }
    }

    /// A FLAT panel - the shape ship art is full of and an asteroid never is.
    /// Two triangles in the y=0 plane, so its convex hull has no volume.
    fn flat_panel() -> Mesh {
        Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-1.0f32, 0.0, -1.0],
                [1.0, 0.0, -1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; 4])
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    }

    /// THE crash. A flat fragment's convex hull has no volume, so avian gives
    /// the dynamic body zero mass and zero inertia, the solver divides by it,
    /// and the NaN swept AABB trips `b.min.cmple(b.max).all()` several frames
    /// later inside `update_solver_body_aabbs` - a panic with nothing in its
    /// stack pointing back here.
    ///
    /// Asserted on MASS at the spawned collider rather than on the panic,
    /// because mass is the property the solver actually divides by and the
    /// panic is both distant and non-deterministic.
    #[test]
    fn a_flat_fragment_still_gets_a_collider_with_mass() {
        // The bare hull is the thing that fails, so pin that too: without it
        // this test could pass on a mesh that was never degenerate.
        let bare = Collider::convex_hull_from_mesh(&flat_panel())
            .expect("parry hulls a flat panel rather than declining");
        assert_eq!(
            bare.mass_properties(1.0).mass,
            0.0,
            "delivery guard: a coplanar hull is exactly the zero-mass case"
        );

        let mut app = finale_app();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(flat_panel());
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let origin = app
            .world_mut()
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        app.world_mut().spawn((
            ExplodableEntity,
            GlobalTransform::IDENTITY,
            ExplodeFragments(vec![ExplodeFragment {
                origin,
                mesh,
                direction: Dir3::X,
            }]),
        ));
        app.update();

        let mut q_debris = app
            .world_mut()
            .query_filtered::<&Collider, With<MeshFragmentMarker>>();
        let masses: Vec<f32> = q_debris
            .iter(app.world())
            .map(|collider| collider.mass_properties(1.0).mass)
            .collect();

        assert!(!masses.is_empty(), "flat art must still produce debris");
        for mass in masses {
            assert!(
                mass > 0.0,
                "a fragment body with no mass makes the solver produce NaN, \
                 which avian asserts on frames later"
            );
        }
    }

    /// Destructible but not explodable: a skin plate comes off rather than
    /// breaking apart, and nothing in the finale claims it. The reaper must,
    /// or it stands at zero health with a live collider.
    #[test]
    fn a_destructible_that_is_not_explodable_is_still_reaped() {
        let mut app = finale_app();
        let plate = app
            .world_mut()
            .spawn((Transform::default(), GlobalTransform::IDENTITY))
            .id();

        app.world_mut()
            .entity_mut(plate)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert!(!app.world().entities().contains(plate));
        assert_eq!(debris(&mut app), (0, 0), "and it does not burst");
    }
}
