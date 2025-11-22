//! A Bevy plugin that handles damage.

pub mod prelude {
    pub use super::{DamagePlugin, MeshFragmentMarker, blast_damage, BlastDamageConfig};
}

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_rand::prelude::*;
use nova_events::prelude::*;
use rand::Rng;

const DAMAGE_MODIFIER: f32 = 1.00;

#[derive(Component, Debug, Clone, Reflect)]
pub struct MeshFragmentMarker;

/// A plugin that handles damage.
pub struct DamagePlugin;

impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        debug!("DamagePlugin: build");

        app.add_observer(on_collider_of_spawn);
        app.add_observer(on_collision_hit_to_damage);
        app.add_observer(on_blast_collision_start_event);

        app.add_observer(on_destroyed_entity);
        app.add_observer(on_explode_entity);
        app.add_observer(handle_entity_explosion);
    }
}

/// Add CollisionImpactMarker to entities with ColliderOf that have Health.
fn on_collider_of_spawn(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_collider: Query<&ColliderOf>,
    q_health: Query<(), (With<Health>, With<RigidBody>, With<CollisionImpactMarker>)>,
) {
    let entity = add.entity;
    trace!("on_collider_of_spawn: entity {:?}", entity);

    let Ok(collider) = q_collider.get(entity) else {
        error!(
            "on_collider_of_spawn: entity {:?} not found in q_collider",
            entity
        );
        return;
    };

    let Ok(_) = q_health.get(collider.body) else {
        return;
    };

    debug!(
        "on_collider_of_spawn: adding CollisionImpactMarker to entity {:?}",
        entity
    );
    commands.entity(entity).insert(CollisionImpactMarker);
}

fn on_collision_hit_to_damage(
    hit: On<CollisionImpactEvent>,
    mut commands: Commands,
    q_mass: Query<&ComputedMass>,
) {
    let amount = hit.relative_velocity.length() * DAMAGE_MODIFIER;
    let mass = q_mass.get(hit.other).map(|m| m.value()).unwrap_or(1.0);
    let amount = amount * mass;
    if amount <= f32::EPSILON {
        return;
    }

    debug!(
        "on_collision_hit_to_damage: entity {:?} hit by {:?} for damage {:.2}",
        hit.entity, hit.other, amount
    );
    commands.trigger(HealthApplyDamage {
        target: hit.entity,
        source: Some(hit.other),
        amount,
    });
}

fn on_blast_collision_start_event(
    collision: On<CollisionStart>,
    q_blast: Query<(&Transform, &BlastDamageConfig), With<BlastDamageMarker>>,
    q_health: Query<&Transform, With<Health>>,
    mut commands: Commands,
) {
    // FIXME: For some reason, this event is not fired consistently. I don't know what the problem
    // might be, but this needs further investigation. The event fires only for ("object", "blast")
    // but not for ("blast", "object"). Which is weird, because the `area.rs` module works also
    // with sensors, and it fires both ways.
    trace!(
        "on_blast_collision_start_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };

    let Ok((blast_transform, blast_config)) = q_blast.get(other) else {
        return;
    };
    let Ok(other_transform) = q_health.get(body) else {
        return;
    };

    let distance = blast_transform
        .translation
        .distance(other_transform.translation);

    let damage = calculate_blast_damage(distance, blast_config);
    if damage <= f32::EPSILON {
        return;
    };
    debug!(
        "on_blast_collision_start_event: applying blast damage {:.2} to entity {:?}",
        damage, body
    );
    commands.trigger(HealthApplyDamage {
        target: body,
        source: None,
        amount: damage,
    });
}

fn on_destroyed_entity(
    add: On<Add, DestroyedMarker>,
    mut commands: Commands,
    q_info: Query<(&EntityId, &EntityTypeName), With<DestroyedMarker>>,
) {
    let entity = add.entity;
    trace!("on_destroyed_entity: entity {:?}", entity);

    let Ok((id, type_name)) = q_info.get(entity) else {
        return;
    };

    debug!(
        "on_destroyed_entity: entity {:?} destroyed (id: {:?}, type: {:?})",
        entity, id, type_name
    );
    commands.fire::<OnDestroyedEvent>(OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: type_name.to_string(),
    });
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

// TODO: Refactor this module into smaller modules for better organization.
// E.g., collision.rs, destruction.rs, explosion.rs, etc.

// NOTE: We will do linear falloff for now, but we might consider other falloff models later.
#[derive(Component, Debug, Clone, Reflect)]
pub struct BlastDamageConfig {
    pub radius: f32,
    pub max_damage: f32,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct BlastDamageMarker;

pub fn blast_damage(config: BlastDamageConfig) -> impl Bundle {
    debug!(
        "blast_damage: radius {:.2}, max_damage {:.2}",
        config.radius, config.max_damage
    );

    (
        Name::new("BlastDamageArea"),
        BlastDamageMarker,
        config.clone(),
        RigidBody::Static,
        Collider::sphere(config.radius),
        Sensor,
        Visibility::Visible,
    )
}

fn calculate_blast_damage(distance: f32, config: &BlastDamageConfig) -> f32 {
    if distance >= config.radius {
        0.0
    } else {
        let falloff = 1.0 - (distance / config.radius);
        config.max_damage * falloff
    }
}

