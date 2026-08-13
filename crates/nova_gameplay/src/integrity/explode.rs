//! The visual side of destruction: when an [`ExplodableEntity`] is destroyed
//! its mesh is sliced into debris fragments (tagged [`MeshFragmentMarker`]) that
//! are spawned with physics and a bounded lifetime. Reacts to the destruction
//! events fired by the integrity glue rather than deciding when something dies.
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
    prelude::{SectionMarker, SpaceshipRootMarker},
};

/// `ExplodableEntity` and `MeshFragmentMarker`.
pub mod prelude {
    pub use super::{ExplodableEntity, MeshFragmentMarker};
}

/// How long a sliced mesh fragment survives before it despawns.
///
/// Fragments are dynamic rigid bodies with convex-hull colliders and nothing
/// despawns them otherwise until scenario teardown; an unattended scene that
/// keeps destroying rocks (a menu backdrop, a long mission) accumulates
/// physics bodies without bound. Long enough to watch the wreck drift apart,
/// short enough to keep the body count flat.
const MESH_FRAGMENT_LIFETIME_SECS: f32 = 30.0;

/// Marker component to indicate that an entity can be exploded.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ExplodableEntity;

/// Tags a debris fragment spawned when an [`ExplodableEntity`] is destroyed -
/// one piece of the sliced mesh, given physics and a fade-out lifetime.
#[derive(Component, Debug, Clone, Reflect)]
pub struct MeshFragmentMarker;

pub(super) struct ExplodablePlugin;

impl Plugin for ExplodablePlugin {
    fn build(&self, app: &mut App) {
        debug!("ExplodablePlugin: build");

        app.add_observer(on_add_explodable_entity);
        app.add_observer(on_destroyed_entity);
        app.add_observer(on_explode_entity);
        app.add_observer(handle_entity_explosion);
        app.add_observer(spawn_section_debris);
        app.add_observer(despawn_destroyed_without_mesh);
    }
}

/// Scatter a short-lived burst of physics debris when a section is destroyed, so the
/// section visually breaks apart instead of vanishing silently.
///
/// Sections render via a gltf `WorldAssetRoot` scene and carry no `Mesh3d` of their own,
/// so they cannot go through the mesh-slicer fragment path (`handle_entity_explosion`,
/// used by asteroids). Instead we spawn a handful of small cubes at the section's world
/// position, launched outward, that fall under the scene's zero gravity and auto-despawn
/// via `TempEntity`. Only `SectionMarker` entities are handled here (the meshless ship
/// root is excluded - its sections have already burst by the time it dies).
fn spawn_section_debris(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_section: Query<
        &GlobalTransform,
        (
            With<IntegrityDestroyMarker>,
            With<SectionMarker>,
            Without<Mesh3d>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let entity = add.entity;
    let Ok(transform) = q_section.get(entity) else {
        return;
    };
    let origin = transform.translation();

    trace!("spawn_section_debris: bursting section {:?}", entity);

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
            Name::new("Section Debris"),
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

/// Despawn destroyed entities that have no `Mesh3d` of their own to explode.
///
/// Mesh-bearing entities (asteroids) are despawned by `handle_entity_explosion` after the
/// slicer produces their fragments. But sections render via a `WorldAssetRoot` gltf scene
/// and have no `Mesh3d`, so they are skipped by `on_explode_entity` (which requires a mesh
/// to slice) and would otherwise never be despawned - they would linger, still colliding
/// and functioning at zero health. Despawn them here (recursively, so their gltf children
/// go too). `try_despawn` is used in case the entity is already gone.
fn despawn_destroyed_without_mesh(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_meshless: Query<(), (With<IntegrityDestroyMarker>, Without<Mesh3d>)>,
) {
    let entity = add.entity;
    if !q_meshless.contains(entity) {
        return;
    }

    trace!("despawn_destroyed_without_mesh: despawning {:?}", entity);
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

fn on_explode_entity(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    // Require a Mesh3d: the mesh slicer can only fragment an entity that actually has a
    // mesh. ExplodableEntity is propagated to parent roots (see on_add_explodable_entity),
    // and those roots (ship/section roots that render via a WorldAssetRoot scene) have no
    // Mesh3d of their own. Handing the slicer a meshless entity is an edge case that can
    // crash it, so we simply do not trigger a slice on entities with nothing to slice.
    q_explode: Query<
        (),
        (
            With<ExplodableEntity>,
            With<IntegrityDestroyMarker>,
            With<Mesh3d>,
        ),
    >,
) {
    let entity = add.entity;
    trace!("on_explode_entity: entity {:?}", entity);

    let Ok(_) = q_explode.get(entity) else {
        return;
    };

    debug!("on_explode_entity: entity {:?} will explode", entity);
    commands
        .entity(entity)
        .insert(ExplodeMesh { fragment_count: 4 });
}

fn handle_entity_explosion(
    add: On<Add, ExplodeFragments>,
    mut commands: Commands,
    q_explode: Query<&ExplodeFragments, With<ExplodableEntity>>,
    q_mesh: Query<(&GlobalTransform, &MeshMaterial3d<StandardMaterial>), With<Mesh3d>>,
    meshes: ResMut<Assets<Mesh>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let entity = add.entity;
    trace!("handle_entity_explosion: entity {:?}", entity);

    let Ok(fragments) = q_explode.get(entity) else {
        error!(
            "handle_entity_explosion: entity {:?} not found in q_explode.",
            entity,
        );
        return;
    };

    for fragment in fragments.iter() {
        let Ok((transform, mesh_material)) = q_mesh.get(fragment.origin) else {
            error!(
                "handle_entity_explosion: mesh_entity {:?} not found in q_mesh.",
                fragment.origin,
            );
            continue;
        };

        let transform = transform.compute_transform();
        let offset = fragment.direction * 0.5;
        let velocity = fragment.direction * rng.random_range(2.0..5.0);
        let transform = transform.with_translation(transform.translation + offset);
        let Some(mesh) = meshes.get(&fragment.mesh) else {
            error!(
                "handle_entity_explosion: mesh_entity {:?} has no mesh data.",
                fragment.origin,
            );
            continue;
        };

        commands.spawn((
            MeshFragmentMarker,
            Name::new(format!("Explosion Fragment of {:?}", entity)),
            Mesh3d(fragment.mesh.clone()),
            mesh_material.clone(),
            transform,
            RigidBody::Dynamic,
            Collider::convex_hull_from_mesh(mesh).unwrap_or(Collider::sphere(0.5)),
            LinearVelocity(velocity),
            TempEntity(MESH_FRAGMENT_LIFETIME_SECS),
        ));
    }

    commands.entity(entity).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::explode::ExplodeFragment;

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

    #[test]
    fn a_meshless_destroyed_entity_is_despawned() {
        // Sections (and the ship root) have no Mesh3d of their own to slice, so the
        // destruction path despawns them here rather than leaving them lingering.
        let mut app = App::new();
        app.add_observer(despawn_destroyed_without_mesh);

        let entity = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(entity)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert!(!app.world().entities().contains(entity));
    }
}
