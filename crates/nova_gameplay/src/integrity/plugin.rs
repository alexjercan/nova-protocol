use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use super::{blast::*, components::*};

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

        // Section-specific systems (graph construction, section disable, aggregate health)
        // live in glue.rs so this core stays independent of the ship sections.
        app.add_plugins(super::glue::IntegrityGluePlugin);

        app.add_observer(on_collider_of_spawn_insert_collision_events);
        app.add_observer(on_impact_collision_deal_damage);
        app.add_observer(on_blast_collision_deal_damage);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(handle_destroy);
        app.add_observer(handle_chain_destroy);
        app.add_observer(handle_parent_destroy);
        app.add_observer(on_destroyed);

        app.add_systems(Update, derive_integrity_leaves.in_set(IntegritySystems));
    }
}

fn on_collider_of_spawn_insert_collision_events(
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

fn on_impact_collision_deal_damage(
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
        entity: collider1,
        source: Some(collider2),
        amount: damage,
    });
}

fn on_blast_collision_deal_damage(
    collision: On<CollisionStart>,
    mut commands: Commands,
    // NOTE: Maybe we want the distance between the colliders
    q_body: Query<&Transform, With<RigidBody>>,
    q_blast: Query<(&Transform, &BlastDamageConfig), (With<RigidBody>, With<BlastDamageMarker>)>,
) {
    // FIXME(20260706-162912): For some reason, this event is not fired consistently. I don't know what the problem
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
        entity: collider1,
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

fn on_health_depleted_insert_disabled(add: On<Add, HealthZeroMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!(
        "on_health_depleted_disable: entity {:?} health depleted, disabling",
        entity
    );

    commands.entity(entity).insert(IntegrityDisabledMarker);
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

fn handle_parent_destroy(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityRoot>)>,
) {
    let entity = add.entity;
    trace!("handle_parent_destroy: entity {:?}", entity);

    let Ok(_) = q_destroyed.get(entity) else {
        return;
    };

    commands.entity(entity).insert(IntegrityDestroyMarker);
}

/// When a node is destroyed, prune it from its neighbors' [`ConnectedTo`] lists. Mutating a
/// neighbor's list marks it `Changed`, so `derive_integrity_leaves` re-evaluates whether the
/// neighbor has become a leaf (which, if it is also disabled, drives the chain reaction via
/// `handle_chain_destroy`).
///
/// The destroyed node carries `IntegrityDestroyMarker`; its neighbors do not (a neighbor that
/// happens to be destroyed the same frame is skipped, which is harmless - it is going away
/// anyway). The disjoint `With`/`Without` filters keep the two `ConnectedTo` accesses sound.
fn on_destroyed(
    add: On<Add, IntegrityDestroyMarker>,
    q_destroyed: Query<&ConnectedTo, With<IntegrityDestroyMarker>>,
    mut q_neighbors: Query<&mut ConnectedTo, Without<IntegrityDestroyMarker>>,
) {
    let entity = add.entity;
    trace!("on_destroyed: entity {:?}", entity);

    let Ok(connected) = q_destroyed.get(entity) else {
        return;
    };

    let neighbors = connected.0.clone();
    for neighbor in neighbors {
        if let Ok(mut neighbor_connections) = q_neighbors.get_mut(neighbor) {
            neighbor_connections.retain(|&node| node != entity);
        }
    }
}

/// Re-derive leaf markers whenever a node's [`ConnectedTo`] changes (on initial build, or
/// when a neighbor is pruned by `on_destroyed`). A node with one or zero neighbors is a leaf.
fn derive_integrity_leaves(
    mut commands: Commands,
    q_nodes: Query<(Entity, &ConnectedTo), Changed<ConnectedTo>>,
) {
    for (entity, connected) in &q_nodes {
        if connected.len() <= 1 {
            commands.entity(entity).try_insert(IntegrityLeafMarker);
        } else {
            commands.entity(entity).try_remove::<IntegrityLeafMarker>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal app wired with the avian-free core of the integrity pipeline plus
    /// the bcs health machinery, so tests can drive it from real damage.
    fn integrity_core_app() -> App {
        let mut app = App::new();
        app.add_plugins(HealthPlugin);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(handle_destroy);
        app.add_observer(handle_chain_destroy);
        app.add_observer(handle_parent_destroy);
        app.add_observer(on_destroyed);
        app.add_systems(Update, derive_integrity_leaves);
        app
    }

    #[test]
    fn blast_damage_falls_off_linearly_to_the_radius() {
        let config = BlastDamageConfig {
            radius: 10.0,
            max_damage: 100.0,
        };
        assert_eq!(calculate_blast_damage(0.0, &config), 100.0);
        assert!((calculate_blast_damage(5.0, &config) - 50.0).abs() < 1e-3);
        assert_eq!(calculate_blast_damage(10.0, &config), 0.0);
        assert_eq!(calculate_blast_damage(20.0, &config), 0.0);
    }

    #[test]
    fn leaves_are_derived_from_the_connection_count() {
        let mut app = App::new();
        app.add_systems(Update, derive_integrity_leaves);

        let a = app.world_mut().spawn_empty().id();
        let b = app.world_mut().spawn_empty().id();
        let leaf = app.world_mut().spawn(ConnectedTo(vec![a])).id(); // 1 neighbor
        let hub = app.world_mut().spawn(ConnectedTo(vec![a, b])).id(); // 2 neighbors

        app.update();

        assert!(app.world().get::<IntegrityLeafMarker>(leaf).is_some());
        assert!(app.world().get::<IntegrityLeafMarker>(hub).is_none());

        // Dropping the hub to one neighbor makes it a leaf.
        app.world_mut().get_mut::<ConnectedTo>(hub).unwrap().0 = vec![a];
        app.update();
        assert!(app.world().get::<IntegrityLeafMarker>(hub).is_some());
    }

    #[test]
    fn a_disabled_leaf_is_marked_for_destruction() {
        let mut app = integrity_core_app();
        let node = app.world_mut().spawn(IntegrityLeafMarker).id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_some());
    }

    #[test]
    fn a_disabled_non_leaf_is_not_destroyed() {
        // A disabled interior node is deactivated, not destroyed - only leaves are.
        let mut app = integrity_core_app();
        let node = app.world_mut().spawn_empty().id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_none());
    }

    #[test]
    fn becoming_a_leaf_while_disabled_triggers_destruction() {
        let mut app = integrity_core_app();
        let node = app.world_mut().spawn(IntegrityDisabledMarker).id();

        app.world_mut().entity_mut(node).insert(IntegrityLeafMarker);
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_some());
    }

    #[test]
    fn a_disabled_root_is_destroyed_whole() {
        // The whole body dies when its root is disabled (ship at zero health).
        let mut app = integrity_core_app();
        let root = app.world_mut().spawn(IntegrityRoot).id();

        app.world_mut()
            .entity_mut(root)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(root).is_some());
    }

    #[test]
    fn damage_drives_a_leaf_from_full_health_to_destruction() {
        // The headline sequence: damage -> health zero -> disabled -> destroyed.
        let mut app = integrity_core_app();
        let node = app
            .world_mut()
            .spawn((Health::new(50.0), ConnectedTo(vec![])))
            .id();

        // Derive the leaf marker (no neighbors -> leaf).
        app.update();
        assert!(app.world().get::<IntegrityLeafMarker>(node).is_some());
        assert!(app.world().get::<IntegrityDisabledMarker>(node).is_none());

        // Fatal damage.
        app.world_mut().trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount: 60.0,
        });
        app.update();

        assert!(app.world().get::<HealthZeroMarker>(node).is_some());
        assert!(app.world().get::<IntegrityDisabledMarker>(node).is_some());
        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_some());
    }

    #[test]
    fn destruction_chains_through_a_connected_structure() {
        // A line A-B-C, all disabled. A and C are leaves (1 neighbor) and are
        // destroyed; pruning them from B leaves B a leaf, so it is destroyed too -
        // the whole structure comes apart.
        let mut app = integrity_core_app();

        let a = app.world_mut().spawn(IntegrityDisabledMarker).id();
        let b = app.world_mut().spawn(IntegrityDisabledMarker).id();
        let c = app.world_mut().spawn(IntegrityDisabledMarker).id();
        app.world_mut()
            .entity_mut(a)
            .insert(ConnectedTo(vec![b]));
        app.world_mut()
            .entity_mut(b)
            .insert(ConnectedTo(vec![a, c]));
        app.world_mut()
            .entity_mut(c)
            .insert(ConnectedTo(vec![b]));

        // Let the leaf derivation and chain reaction settle.
        for _ in 0..5 {
            app.update();
        }

        for node in [a, b, c] {
            assert!(
                app.world().get::<IntegrityDestroyMarker>(node).is_some(),
                "node {node:?} should have been destroyed in the chain"
            );
        }
    }
}
