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

/// Apply radial blast damage to a body that overlaps a blast sensor.
///
/// The blast sensor is the "self" side of the event (`collider1`/`body1`): it carries
/// `CollisionEventsEnabled` (see `blast_damage`), so avian raises `CollisionStart` with the
/// blast as `body1` against every collider it overlaps. This is why the blast owns its events
/// rather than relying on each target - a body only takes blast damage if *some* collider in
/// the pair has events enabled, and keying that on the blast means it reaches every overlapped
/// body regardless of the target's own configuration (mirroring `area.rs`).
///
/// avian raises the swapped `{body1 = target, body2 = blast}` event too whenever the target
/// also has events enabled (e.g. a ship section, for impact damage). We ignore that ordering
/// here - `q_blast.get(body1)` fails when `body1` is the target - so each overlap deals damage
/// exactly once and never double-dips.
fn on_blast_collision_deal_damage(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_blast: Query<(&Transform, &BlastDamageConfig), With<BlastDamageMarker>>,
    // NOTE: Maybe we want the distance between the colliders
    q_body: Query<&Transform, With<RigidBody>>,
) {
    trace!(
        "on_blast_collision_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let blast_collider = collision.collider1;
    let target_collider = collision.collider2;

    let Some(blast) = collision.body1 else {
        return;
    };
    let Some(target) = collision.body2 else {
        return;
    };

    // Only act when this side of the event is the blast; the swapped ordering is handled by
    // its own event (or ignored entirely).
    let Ok((blast_transform, blast_config)) = q_blast.get(blast) else {
        return;
    };
    let Ok(target_transform) = q_body.get(target) else {
        return;
    };

    let distance = blast_transform
        .translation
        .distance(target_transform.translation);
    let damage = calculate_blast_damage(distance, blast_config);
    if damage <= f32::EPSILON {
        return;
    };

    debug!(
        "on_blast_collision_start_event: applying blast damage {:.2} to collider {:?} (body {:?}) from blast collider {:?} (blast {:?})",
        damage, target_collider, target, blast_collider, blast
    );
    commands.trigger(HealthApplyDamage {
        entity: target_collider,
        source: Some(blast_collider),
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

/// Physics-level tests for the collision-driven damage observers. Unlike the `tests` module
/// above (which drives the avian-free core by hand), these run a real headless avian world so
/// the observers read genuine `ComputedMass` / `Transform` / `ColliderOf` state.
#[cfg(test)]
mod physics_tests {
    use super::*;
    use crate::integrity::test_support::{integrity_physics_app, settle};

    /// Spawn a dynamic rigid body with a unit-sphere collider child that carries `Health`.
    /// Returns `(body, collider)`. Placement is far from the origin so bodies never actually
    /// touch - the tests inject the `CollisionStart` themselves for determinism.
    fn spawn_body(app: &mut App, at: Vec3) -> (Entity, Entity) {
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::from_translation(at)))
            .id();
        let collider = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Collider::sphere(1.0),
                ColliderDensity(1.0),
                Health::new(1000.0),
            ))
            .id();
        (body, collider)
    }

    fn health(app: &App, entity: Entity) -> f32 {
        app.world().get::<Health>(entity).unwrap().current
    }

    #[test]
    fn an_impact_applies_damage_from_relative_velocity_and_mass() {
        // A real collision is left to the solver, which zeroes the contact velocity before the
        // observer can read it (making sim-driven damage timing-dependent). Instead we let
        // avian compute real masses, then inject the contact event with a known impact
        // velocity and check the observer's impulse/energy formula against that real mass.
        let mut app = integrity_physics_app();
        let (b1, c1) = spawn_body(&mut app, Vec3::new(-100.0, 0.0, 0.0));
        let (b2, c2) = spawn_body(&mut app, Vec3::new(100.0, 0.0, 0.0));
        settle(&mut app);

        // Head-on closing velocity of 40 (=20 - -20).
        app.world_mut().get_mut::<LinearVelocity>(b1).unwrap().0 = Vec3::new(20.0, 0.0, 0.0);
        app.world_mut().get_mut::<LinearVelocity>(b2).unwrap().0 = Vec3::new(-20.0, 0.0, 0.0);

        let m1 = app.world().get::<ComputedMass>(b1).unwrap().value();
        let m2 = app.world().get::<ComputedMass>(b2).unwrap().value();
        assert!(m1.is_finite() && m1 > 0.0, "mass should be finalized: {m1}");

        app.world_mut().trigger(CollisionStart {
            collider1: c1,
            collider2: c2,
            body1: Some(b1),
            body2: Some(b2),
        });
        app.update();

        // Recompute the expected damage from the real mass and the module's own constants, so
        // this checks the wiring + real physics state rather than a hard-coded magic number.
        let rel = 40.0_f32;
        let effective_mass = (m1 * m2) / (m1 + m2);
        let impulse = effective_mass * (1.0 + RESTITUTION_COEFFICIENT) * rel;
        let energy = 0.5 * effective_mass * (1.0 - RESTITUTION_COEFFICIENT.powi(2)) * rel * rel;
        let expected = impulse * IMPULSE_DAMAGE_MODIFIER + energy * ENERGY_DAMAGE_MODIFIER;

        // Damage lands on collider1 (the target), and only there - collider2 is the source.
        assert!((health(&app, c1) - (1000.0 - expected)).abs() < 1e-2);
        assert_eq!(health(&app, c2), 1000.0);
    }

    #[test]
    fn a_near_stationary_contact_applies_no_impact_damage() {
        // Two bodies barely moving relative to each other: below the velocity gate, so a
        // graze deals nothing (otherwise resting stacks of debris would grind each other away).
        let mut app = integrity_physics_app();
        let (b1, c1) = spawn_body(&mut app, Vec3::new(-100.0, 0.0, 0.0));
        let (b2, c2) = spawn_body(&mut app, Vec3::new(100.0, 0.0, 0.0));
        settle(&mut app);

        app.world_mut().get_mut::<LinearVelocity>(b1).unwrap().0 = Vec3::new(0.01, 0.0, 0.0);
        app.world_mut().get_mut::<LinearVelocity>(b2).unwrap().0 = Vec3::ZERO;

        app.world_mut().trigger(CollisionStart {
            collider1: c1,
            collider2: c2,
            body1: Some(b1),
            body2: Some(b2),
        });
        app.update();

        assert_eq!(health(&app, c1), 1000.0);
    }

    /// Spawn a blast sensor via the production `blast_damage` bundle at `at`.
    fn spawn_blast(app: &mut App, at: Vec3, radius: f32, max_damage: f32) -> Entity {
        app.world_mut()
            .spawn((
                blast_damage(BlastDamageConfig { radius, max_damage }),
                Transform::from_translation(at),
            ))
            .id()
    }

    #[test]
    fn a_blast_sensor_overlap_applies_falloff_damage() {
        // Unlike the impact case, a sensor overlap fires a real, deterministic `CollisionStart`
        // (no solver to zero out), so this drives the whole path through the physics engine:
        // avian detects the overlap, the observer reads both transforms, and the linear falloff
        // yields damage at the body's distance from the blast centre.
        let mut app = integrity_physics_app();
        let (_body, target_collider) = spawn_body(&mut app, Vec3::ZERO);
        // Blast centred 4.0 away, radius 10.0 -> falloff factor 1 - 4/10 = 0.6 -> 60 damage.
        spawn_blast(&mut app, Vec3::new(4.0, 0.0, 0.0), 10.0, 100.0);

        settle(&mut app);

        let expected =
            calculate_blast_damage(4.0, &BlastDamageConfig { radius: 10.0, max_damage: 100.0 });
        assert!((expected - 60.0).abs() < 1e-3, "sanity: {expected}");
        assert!((health(&app, target_collider) - (1000.0 - expected)).abs() < 1e-2);
    }

    #[test]
    fn a_blast_reaches_a_target_that_has_no_collision_events() {
        // Regression for the ordering bug (task 20260706-162912): the blast must not depend on
        // the target having `CollisionEventsEnabled`. Here the target opts out (it had no
        // `Health` when its `ColliderOf` was added, so `on_collider_of_spawn_...` skipped it),
        // yet the blast - which owns its events - still reaches it. Before the fix the only
        // event raised was the target's, which this target never raises, so no damage landed.
        let mut app = integrity_physics_app();
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default()))
            .id();
        // Collider spawned without Health, so no collision events get enabled on it.
        let target_collider = app
            .world_mut()
            .spawn((ChildOf(body), Collider::sphere(1.0), ColliderDensity(1.0)))
            .id();
        settle(&mut app);
        assert!(
            app.world()
                .get::<CollisionEventsEnabled>(target_collider)
                .is_none(),
            "target must not have opted into collision events for this regression to be meaningful"
        );
        // Now give it Health to take damage (this does not enable events - that observer keys on
        // ColliderOf, not Health).
        app.world_mut()
            .entity_mut(target_collider)
            .insert(Health::new(1000.0));

        spawn_blast(&mut app, Vec3::new(4.0, 0.0, 0.0), 10.0, 100.0);
        settle(&mut app);

        assert!(
            (health(&app, target_collider) - 940.0).abs() < 1e-2,
            "blast should reach a target that has no collision events of its own"
        );
    }

    #[test]
    fn a_blast_deals_damage_only_once_when_the_target_also_has_events() {
        // When the target has its own events (a normal ship section / asteroid), avian raises
        // both orderings of the pair. The observer acts only on the blast-as-self ordering, so
        // the target is damaged exactly once - 60, not 120.
        let mut app = integrity_physics_app();
        let (_body, target_collider) = spawn_body(&mut app, Vec3::ZERO);
        settle(&mut app);
        assert!(
            app.world()
                .get::<CollisionEventsEnabled>(target_collider)
                .is_some(),
            "a Health-bearing target should have its own collision events"
        );

        spawn_blast(&mut app, Vec3::new(4.0, 0.0, 0.0), 10.0, 100.0);
        settle(&mut app);

        assert!((health(&app, target_collider) - 940.0).abs() < 1e-2);
    }

    #[test]
    fn a_body_outside_the_blast_takes_no_damage() {
        // A body beyond the sensor's reach never overlaps it, so no collision fires and no
        // damage is dealt - the blast only hits what it physically contains.
        let mut app = integrity_physics_app();
        let (_body, target_collider) = spawn_body(&mut app, Vec3::ZERO);
        // Blast radius 5 centred 8 away; the target (unit sphere) sits ~7 out, clear of it.
        spawn_blast(&mut app, Vec3::new(8.0, 0.0, 0.0), 5.0, 100.0);

        settle(&mut app);

        assert_eq!(health(&app, target_collider), 1000.0);
    }
}
