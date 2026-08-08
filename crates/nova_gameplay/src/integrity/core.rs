//! The destruction pipeline over the [`components`](super::components) graph:
//! disable at zero health, destroy leaves, prune, cascade - plus the collision
//! that starts it.
//!
//! The lifecycle, all observer-driven:
//!
//! - a fast impact between two rigid bodies deals kinetic damage scaled by
//!   relative velocity and effective mass (`on_impact_collision_deal_damage`);
//! - a node whose health hits zero is disabled;
//! - a disabled LEAF - or a disabled [`IntegrityRoot`] - is destroyed, which is
//!   the [`IntegrityDestroyMarker`] seam nova's [`explode`](super::explode)
//!   reacts to;
//! - destroying a node prunes it from its neighbours' [`ConnectedTo`] lists,
//!   re-deriving leaf markers and cascading through the structure.
//!
//! Impact damage goes through [`apply_typed_damage`] as [`DamageType::Kinetic`],
//! so ramming respects the same per-section resistance table every weapon does.
//! Blast damage is the [`NovaBlast`](crate::damage::NovaBlast) volume's job and
//! lives in [`crate::damage`] beside the table.

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{components::prelude::*, health::prelude::*};
use crate::damage::{apply_typed_damage, DamageType, ProjectileDamage, SectionDamageClass};

/// `IntegrityCorePlugin` and `IntegritySystems`.
pub mod prelude {
    pub use super::{IntegrityCorePlugin, IntegritySystems};
}

/// Feel constants for ramming, not physical ones. The restitution coefficient
/// splits an impact between the impulse term and the energy the collision
/// absorbs (0.5 is half-elastic, so both contribute); the two modifiers convert
/// those into hit points.
///
/// These are the reference numbers [`representative_kinetic_damage`](crate::damage::representative_kinetic_damage)
/// authors turret damage against, so a slug at a typical engagement speed hurts
/// about as much as it did when damage was purely emergent. Changing one means
/// re-authoring the other.
const RESTITUTION_COEFFICIENT: f32 = 0.5;
const IMPULSE_DAMAGE_MODIFIER: f32 = 0.1;
const ENERGY_DAMAGE_MODIFIER: f32 = 0.05;

/// Below this squared relative speed an impact is a nudge, not a ram, and deals
/// nothing. Without it two docked hulls grind each other down at rest.
const MIN_IMPACT_SPEED_SQUARED: f32 = 0.1;

/// System set for the leaf derivation, so gameplay can order graph edits around
/// it. `glue` builds the section graph inside it; `neutralize` reads the result
/// after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegritySystems;

/// The generic destruction core: collision damage in, [`IntegrityDestroyMarker`]
/// out. Requires [`NovaHealthPlugin`] and avian's `PhysicsPlugins`.
pub struct IntegrityCorePlugin;

impl Plugin for IntegrityCorePlugin {
    fn build(&self, app: &mut App) {
        debug!("IntegrityCorePlugin: build");

        app.register_type::<IntegrityRoot>();
        app.register_type::<ConnectedTo>();
        app.register_type::<IntegrityLeafMarker>();
        app.register_type::<IntegrityDisabledMarker>();
        app.register_type::<IntegrityDestroyMarker>();

        app.add_observer(on_collider_of_spawn_insert_collision_events);
        app.add_observer(on_impact_collision_deal_damage);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(destroy_a_disabled_leaf);
        app.add_observer(destroy_a_disabled_node_that_became_a_leaf);
        app.add_observer(destroy_the_structure_of_a_disabled_root);
        app.add_observer(prune_a_destroyed_node_from_its_neighbours);

        app.add_systems(Update, derive_integrity_leaves.in_set(IntegritySystems));
    }
}

/// Opt a health-bearing collider into collision events, so impacts against it
/// are reported at all.
fn on_collider_of_spawn_insert_collision_events(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_collider: Query<Entity, (With<ColliderOf>, With<Health>)>,
) {
    let entity = add.entity;
    if q_collider.get(entity).is_err() {
        return;
    }

    trace!(
        "on_collider_of_spawn: enabling collision events on {:?}",
        entity
    );
    commands.entity(entity).insert(CollisionEventsEnabled);
}

/// Damage a body from the impulse and energy lost in a fast impact against
/// another body.
///
/// Routed through the typed layer as [`DamageType::Kinetic`]: a ram is a kinetic
/// hit, so it scales by the struck section's resistance exactly as a slug does.
fn on_impact_collision_deal_damage(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_body: Query<(&LinearVelocity, &ComputedMass), With<RigidBody>>,
    // NOTE: excluding a blast volume keeps a blast overlap from ALSO dealing
    // impact damage - it is a massless static sensor, and its damage is the
    // NovaBlast observer's job.
    q_other: Query<
        (&LinearVelocity, &ComputedMass),
        (With<RigidBody>, Without<crate::damage::NovaBlast>),
    >,
    q_class: Query<&SectionDamageClass>,
) {
    let collider1 = collision.collider1;
    let collider2 = collision.collider2;

    let (Some(body), Some(other)) = (collision.body1, collision.body2) else {
        return;
    };
    let (Ok((velocity1, mass1)), Ok((velocity2, mass2))) = (q_body.get(body), q_other.get(other))
    else {
        return;
    };

    let relative_velocity = **velocity1 - **velocity2;
    if relative_velocity.length_squared() < MIN_IMPACT_SPEED_SQUARED {
        return;
    }

    let effective_mass = (mass1.value() * mass2.value()) / (mass1.value() + mass2.value());
    let amount = impact_damage(effective_mass, relative_velocity.length());
    if amount <= f32::EPSILON {
        return;
    }

    debug!(
        "on_impact_collision: collider {:?} (body {:?}) rammed by {:?} (body {:?}) for {amount:.2}",
        collider1, body, collider2, other
    );
    apply_typed_damage(
        &mut commands,
        collider1,
        Some(collider2),
        q_class.get(collider1).ok().copied(),
        ProjectileDamage {
            amount,
            kind: DamageType::Kinetic,
        },
    );
}

/// Hit points a ram deals: the impulse term plus the energy the collision
/// absorbs, each scaled into health units. Pure, so the turret's damage
/// authoring can call the same formula.
pub fn impact_damage(effective_mass: f32, relative_speed: f32) -> f32 {
    let impulse = effective_mass * (1.0 + RESTITUTION_COEFFICIENT) * relative_speed;
    let energy_lost = 0.5
        * effective_mass
        * (1.0 - RESTITUTION_COEFFICIENT.powi(2))
        * relative_speed
        * relative_speed;
    impulse * IMPULSE_DAMAGE_MODIFIER + energy_lost * ENERGY_DAMAGE_MODIFIER
}

/// Disable a node the moment its health reaches zero.
fn on_health_depleted_insert_disabled(add: On<Add, HealthZeroMarker>, mut commands: Commands) {
    trace!("integrity: entity {:?} depleted, disabling", add.entity);
    commands.entity(add.entity).insert(IntegrityDisabledMarker);
}

/// Destroy a node that is disabled while ALREADY a leaf.
fn destroy_a_disabled_leaf(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_disabled: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    if q_disabled.get(add.entity).is_err() {
        return;
    }

    debug!("integrity: disabled leaf {:?} destroyed", add.entity);
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Destroy an already-disabled node that has just BECOME a leaf - the chain
/// reaction.
fn destroy_a_disabled_node_that_became_a_leaf(
    add: On<Add, IntegrityLeafMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    if q_destroyed.get(add.entity).is_err() {
        return;
    }

    debug!(
        "integrity: {:?} became a disabled leaf, cascading",
        add.entity
    );
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Destroy a disabled [`IntegrityRoot`] outright: the whole structure dies with
/// its root, leaf or not.
fn destroy_the_structure_of_a_disabled_root(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityRoot>)>,
) {
    if q_destroyed.get(add.entity).is_err() {
        return;
    }

    debug!("integrity: disabled root {:?} destroyed", add.entity);
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Prune a destroyed node from its neighbours' [`ConnectedTo`] lists. Mutating a
/// neighbour's list marks it `Changed`, so [`derive_integrity_leaves`]
/// re-evaluates whether it has become a leaf - which, if it is also disabled,
/// drives the chain reaction.
///
/// The disjoint `With`/`Without` filters keep the two `ConnectedTo` accesses
/// sound. A neighbour destroyed in the same frame is skipped, which is harmless:
/// it is going away anyway.
fn prune_a_destroyed_node_from_its_neighbours(
    add: On<Add, IntegrityDestroyMarker>,
    q_destroyed: Query<&ConnectedTo, With<IntegrityDestroyMarker>>,
    mut q_neighbors: Query<&mut ConnectedTo, Without<IntegrityDestroyMarker>>,
) {
    let entity = add.entity;
    let Ok(connected) = q_destroyed.get(entity) else {
        return;
    };

    for neighbor in connected.0.clone() {
        if let Ok(mut neighbor_connections) = q_neighbors.get_mut(neighbor) {
            neighbor_connections.retain(|&node| node != entity);
        }
    }
}

/// Re-derive leaf markers whenever a node's [`ConnectedTo`] changes - on the
/// initial build, or when a neighbour is pruned. One or zero neighbours is a
/// leaf.
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

    /// The avian-free core plus the health store, so tests drive destruction
    /// from real damage without stepping physics.
    fn integrity_core_app() -> App {
        let mut app = App::new();
        app.add_plugins(NovaHealthPlugin);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(destroy_a_disabled_leaf);
        app.add_observer(destroy_a_disabled_node_that_became_a_leaf);
        app.add_observer(destroy_the_structure_of_a_disabled_root);
        app.add_observer(prune_a_destroyed_node_from_its_neighbours);
        app.add_systems(Update, derive_integrity_leaves);
        app
    }

    /// A lone node has no neighbours, so it is a leaf from the first update and
    /// dies the moment its health runs out.
    #[test]
    fn a_lone_node_is_a_leaf_and_dies_when_depleted() {
        let mut app = integrity_core_app();
        let node = app
            .world_mut()
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        app.update();
        assert!(app.world().get::<IntegrityLeafMarker>(node).is_some());

        app.world_mut().trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDisabledMarker>(node).is_some());
        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_some());
    }

    /// Leafness is DERIVED, not latched: a node that gains a second neighbour
    /// loses the marker again. Without the removal branch a node welded into
    /// the middle of a structure would stay destroyable as a leaf.
    #[test]
    fn a_leaf_that_gains_a_second_neighbour_stops_being_one() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let hub = world.spawn(ConnectedTo(vec![])).id();
        let first = world.spawn(ConnectedTo(vec![hub])).id();
        world.entity_mut(hub).insert(ConnectedTo(vec![first]));
        app.update();
        assert!(
            app.world().get::<IntegrityLeafMarker>(hub).is_some(),
            "one neighbour is a leaf"
        );

        let second = app.world_mut().spawn(ConnectedTo(vec![hub])).id();
        app.world_mut()
            .entity_mut(hub)
            .insert(ConnectedTo(vec![first, second]));
        app.update();

        assert!(
            app.world().get::<IntegrityLeafMarker>(hub).is_none(),
            "two neighbours is not a leaf any more"
        );
    }

    /// An INTERIOR node is disabled but not destroyed: a dead section still
    /// holds the structure together until it is pruned into a leaf.
    #[test]
    fn a_depleted_interior_node_is_disabled_but_survives() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let left = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let middle = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let right = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        world.entity_mut(left).insert(ConnectedTo(vec![middle]));
        world
            .entity_mut(middle)
            .insert(ConnectedTo(vec![left, right]));
        world.entity_mut(right).insert(ConnectedTo(vec![middle]));
        app.update();

        assert!(
            app.world().get::<IntegrityLeafMarker>(middle).is_none(),
            "two neighbours is not a leaf"
        );

        app.world_mut().trigger(HealthApplyDamage {
            entity: middle,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDisabledMarker>(middle).is_some());
        assert!(
            app.world().get::<IntegrityDestroyMarker>(middle).is_none(),
            "an interior node is deactivated, not destroyed"
        );
    }

    /// Destroying a leaf prunes it from its neighbour, which turns the
    /// neighbour into a leaf - and if that neighbour is already disabled, it
    /// dies too. This is the chain reaction.
    #[test]
    fn destroying_a_leaf_cascades_into_a_disabled_neighbour() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let outer = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let inner = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let anchor = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        world.entity_mut(outer).insert(ConnectedTo(vec![inner]));
        world
            .entity_mut(inner)
            .insert(ConnectedTo(vec![outer, anchor]));
        world.entity_mut(anchor).insert(ConnectedTo(vec![inner]));
        app.update();

        // Kill the interior node first: disabled, but it holds.
        app.world_mut().trigger(HealthApplyDamage {
            entity: inner,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();
        assert!(app.world().get::<IntegrityDestroyMarker>(inner).is_none());

        // Now kill the leaf outside it. The prune leaves `inner` with one
        // neighbour, so it becomes a leaf - and it is already disabled.
        app.world_mut().trigger(HealthApplyDamage {
            entity: outer,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(outer).is_some());
        assert!(
            app.world().get::<IntegrityDestroyMarker>(inner).is_some(),
            "pruning the outer leaf made the disabled interior node a leaf, so it cascaded"
        );
    }

    /// A disabled root takes the whole structure with it, leaf or not.
    #[test]
    fn a_disabled_root_is_destroyed_even_with_neighbours() {
        let mut app = integrity_core_app();
        let root = app
            .world_mut()
            .spawn((Health::new(10.0), IntegrityRoot))
            .id();
        app.update();

        app.world_mut().trigger(HealthApplyDamage {
            entity: root,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDestroyMarker>(root).is_some());
    }

    /// The ram formula is monotone in both inputs and zero at rest - the
    /// property the turret's damage authoring leans on.
    #[test]
    fn impact_damage_grows_with_mass_and_speed_and_is_zero_at_rest() {
        assert_eq!(impact_damage(100.0, 0.0), 0.0);
        assert!(impact_damage(100.0, 20.0) > impact_damage(100.0, 10.0));
        assert!(impact_damage(200.0, 10.0) > impact_damage(100.0, 10.0));
    }
}
