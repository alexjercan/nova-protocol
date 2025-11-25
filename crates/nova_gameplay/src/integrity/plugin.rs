use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_common_systems::prelude::*;

use super::{blast::*, components::*};
use crate::prelude::{SectionInactiveMarker, SectionMarker};

pub mod prelude {
    pub use super::IntegrityPlugin;
}

const RESTITUTION_COEFFICIENT: f32 = 0.5;
const IMPULSE_DAMAGE_MODIFIER: f32 = 0.1;
const ENERGY_DAMAGE_MODIFIER: f32 = 0.05;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegritySystems;

pub struct IntegrityPlugin;

impl Plugin for IntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("IntegrityPlugin: build");

        // Handle explosion on destruction
        app.add_plugins(super::explode::ExplodablePlugin);

        app.add_observer(on_collider_of_spawn);
        app.add_observer(on_impact_collision_event);
        app.add_observer(on_blast_collision_event);
        app.add_observer(on_health_depleted_disable);
        app.add_observer(handle_destroy);
        app.add_observer(handle_chain_destroy);
        app.add_observer(on_destroyed);

        // TODO: This should be probably moved to some glue.rs file to not make integrity too
        // dependent on sections
        app.add_observer(on_section_disable);
        // TODO: This should be probably moved to some glue.rs file to not make integrity too
        // dependent on sections
        app.add_observer(on_section_graph_create);
        // TODO: This should maybe be moved to somewhere else, but it is the generic case where we
        // only have on rigidbody and one collider (e.g. for asteroids)
        app.add_observer(on_rigidbody_graph_create);

        app.add_systems(Update, on_changed_graph.in_set(IntegritySystems));
    }
}

fn on_collider_of_spawn(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_collider: Query<Entity, (With<ColliderOf>, With<Health>)>,
) {
    let entity = add.entity;
    trace!("on_collider_of_spawn: entity {:?}", entity);

    let Ok(_) = q_collider.get(entity) else {
        trace!(
            "on_collider_of_spawn: entity {:?} not found in q_collider",
            entity
        );
        return;
    };

    debug!(
        "on_collider_of_spawn: adding CollisionEventsEnabled to entity {:?}",
        entity
    );
    commands.entity(entity).insert(CollisionEventsEnabled);
}

fn on_impact_collision_event(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_body: Query<(&LinearVelocity, &ComputedMass), With<RigidBody>>,
    // NOTE: We exclude BlastDamageMarker here to avoid double-dipping damage from blast collisions
    q_other: Query<(&LinearVelocity, &ComputedMass), (With<RigidBody>, Without<BlastDamageMarker>)>,
) {
    trace!(
        "on_impact_collision_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let collider1 = collision.collider1;
    let collider2 = collision.collider2;

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };

    let Ok((velocity1, mass1)) = q_body.get(body) else {
        return;
    };
    let Ok((velocity2, mass2)) = q_other.get(other) else {
        return;
    };

    let relative_velocity = **velocity1 - **velocity2;
    if relative_velocity.length_squared() < 0.1 {
        return;
    }

    let effective_mass = (mass1.value() * mass2.value()) / (mass1.value() + mass2.value());
    let impulse = effective_mass * (1.0 + RESTITUTION_COEFFICIENT) * relative_velocity.length();
    let energy_lost = 0.5
        * effective_mass
        * (1.0 - RESTITUTION_COEFFICIENT.powi(2))
        * relative_velocity.length_squared();

    let damage = impulse * IMPULSE_DAMAGE_MODIFIER + energy_lost * ENERGY_DAMAGE_MODIFIER;
    if damage <= f32::EPSILON {
        return;
    }
    debug!(
        "on_impact_collision_event: collider {:?} (body {:?}) hit by collider {:?} (other {:?}) for damage {:.2}",
        collider1, body, collider2, other, damage
    );
    commands.trigger(HealthApplyDamage {
        target: collider1,
        source: Some(collider2),
        amount: damage,
    });
}

fn on_blast_collision_event(
    collision: On<CollisionStart>,
    mut commands: Commands,
    // NOTE: Maybe we want the distance between the colliders
    q_body: Query<&Transform, With<RigidBody>>,
    q_blast: Query<(&Transform, &BlastDamageConfig), (With<RigidBody>, With<BlastDamageMarker>)>,
) {
    // FIXME: For some reason, this event is not fired consistently. I don't know what the problem
    // might be, but this needs further investigation. The event fires only for ("object", "blast")
    // but not for ("blast", "object"). Which is weird, because the `area.rs` module works also
    // with sensors, and it fires both ways.
    trace!(
        "on_blast_collision_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let collider1 = collision.collider1;
    let collider2 = collision.collider2;

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(blast) = collision.body2 else {
        return;
    };

    let Ok(body_transform) = q_body.get(body) else {
        return;
    };
    let Ok((blast_transform, blast_config)) = q_blast.get(blast) else {
        return;
    };

    let distance = blast_transform
        .translation
        .distance(body_transform.translation);
    let damage = calculate_blast_damage(distance, blast_config);
    if damage <= f32::EPSILON {
        return;
    };

    debug!(
        "on_blast_collision_start_event: applying blast damage {:.2} to collider {:?} (body {:?}) from collider {:?} (blast {:?})",
        damage, collider1, body, collider2, blast
    );
    commands.trigger(HealthApplyDamage {
        target: collider1,
        source: Some(collider2),
        amount: damage,
    });
}

fn calculate_blast_damage(distance: f32, config: &BlastDamageConfig) -> f32 {
    if distance >= config.radius {
        0.0
    } else {
        let falloff = 1.0 - (distance / config.radius);
        config.max_damage * falloff
    }
}

fn on_health_depleted_disable(add: On<Add, HealthZeroMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!(
        "on_health_depleted_disable: entity {:?} health depleted, disabling",
        entity
    );

    commands.entity(entity).insert(IntegrityDisabledMarker);
}

fn on_section_disable(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    // NOTE: If it is already a leaf, no need to disable the section since it will be destroyed
    // anyway
    q_section: Query<
        Entity,
        (
            With<SectionMarker>,
            With<IntegrityDisabledMarker>,
            Without<IntegrityLeafMarker>,
        ),
    >,
) {
    let entity = add.entity;
    if !q_section.contains(entity) {
        return;
    }

    trace!(
        "on_section_disable: entity {:?} integrity disabled, disabling section",
        entity
    );

    commands.entity(entity).insert(SectionInactiveMarker);
}

fn handle_destroy(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_disabled: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    let entity = add.entity;
    trace!("handle_destroy: entity {:?}", entity);

    let Ok(_) = q_disabled.get(entity) else {
        return;
    };

    debug!("handle_destroy: entity {:?} will explode", entity);
    commands.entity(entity).insert(IntegrityDestroyMarker);
}

fn handle_chain_destroy(
    add: On<Add, IntegrityLeafMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    let entity = add.entity;
    trace!("handle_chain_destroy: entity {:?}", entity);

    let Ok(_) = q_destroyed.get(entity) else {
        return;
    };

    debug!(
        "handle_chain_destroy: entity {:?} parent destroyed, destroying",
        entity
    );
    commands.entity(entity).insert(IntegrityDestroyMarker);
}

fn on_destroyed(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_destroyed: Query<&ChildOf, (With<IntegrityDestroyMarker>, Without<IntegrityGraph>)>,
    mut q_graph: Query<&mut IntegrityGraph>,
) {
    let entity = add.entity;
    trace!("on_destroyed: entity {:?}", entity);

    let Ok(ChildOf(parent)) = q_destroyed.get(entity) else {
        return;
    };

    let Ok(mut graph) = q_graph.get_mut(*parent) else {
        error!(
            "on_destroyed: entity {:?} parent {:?} not found in q_graph",
            entity, parent
        );
        return;
    };

    // remove_entity_from_graph
    graph.remove(&entity);
    for (_parent, children) in graph.iter_mut() {
        children.retain(|&child| child != entity);
    }

    // update_graph_leafs
    let leafs: Vec<Entity> = graph
        .iter()
        .filter(|(_parent, children)| children.iter().len() == 1)
        .map(|(parent, _children)| *parent)
        .collect();

    for leaf in leafs {
        commands.entity(leaf).insert(IntegrityLeafMarker);
    }
}

fn on_section_graph_create(
    add: On<Add, SectionMarker>,
    mut q_graph: Query<&mut IntegrityGraph>,
    q_sections: Query<(Entity, &Transform, &ChildOf), With<SectionMarker>>,
) {
    let entity = add.entity;
    trace!("on_section_graph_create: entity {:?}", entity);

    let Ok((section, section_transform, ChildOf(parent))) = q_sections.get(entity) else {
        error!(
            "on_section_graph_create: entity {:?} not found in q_sections",
            entity
        );
        return;
    };

    let Ok(mut graph) = q_graph.get_mut(*parent) else {
        return;
    };

    let section_position = section_transform.translation;
    let mut neighbors: Vec<Entity> = Vec::new();
    for &child in graph.keys() {
        let Ok((_, child_transform, _)) = q_sections.get(child) else {
            continue;
        };

        let child_position = child_transform.translation;
        let distance = section_position.distance(child_position);
        if (distance - 1.0).abs() < f32::EPSILON {
            neighbors.push(child);
        }
    }

    graph.insert(section, neighbors.clone());
    for neighbor in &neighbors {
        if let Some(neighbor_children) = graph.get_mut(neighbor) {
            neighbor_children.push(section);
        }
    }
}

fn on_rigidbody_graph_create(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_graph: Query<(), With<IntegrityGraph>>,
    q_collider: Query<(Entity, &ChildOf), With<ColliderOf>>,
) {
    let entity = add.entity;
    trace!("on_rigidbody_graph_create: entity {:?}", entity);

    let Ok((collider, ChildOf(rigidbody))) = q_collider.get(entity) else {
        return;
    };

    if q_graph.contains(*rigidbody) {
        return;
    }

    let mut graph: HashMap<Entity, Vec<Entity>> = HashMap::new();
    graph.insert(collider, Vec::new());

    commands.entity(*rigidbody).insert(IntegrityGraph(graph));
}

fn on_changed_graph(
    mut commands: Commands,
    q_graph: Query<&IntegrityGraph, Changed<IntegrityGraph>>,
) {
    for graph in &q_graph {
        for (entity, neighbors) in graph.iter() {
            if neighbors.iter().len() <= 1 {
                commands.entity(*entity).try_insert(IntegrityLeafMarker);
            } else {
                commands.entity(*entity).try_remove::<IntegrityLeafMarker>();
            }
        }
    }
}
