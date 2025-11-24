use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_rand::prelude::*;
use rand::Rng;

pub mod prelude {
    pub use super::MeshFragmentMarker;
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct MeshFragmentMarker;

pub(super) struct ExplodablePlugin;

impl Plugin for ExplodablePlugin {
    fn build(&self, app: &mut App) {
        debug!("ExplodablePlugin: build");

        app.add_observer(on_explode_entity);
        app.add_observer(handle_entity_explosion);
    }
}

fn on_explode_entity(
    add: On<Add, DestroyedMarker>,
    mut commands: Commands,
    q_explode: Query<(), (With<ExplodableEntity>, With<DestroyedMarker>)>,
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
        ));
    }

    commands.entity(entity).despawn();
}
