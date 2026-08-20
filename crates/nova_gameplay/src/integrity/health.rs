//! Nova's hit-point store: the [`Health`] pool, the [`HealthApplyDamage`] event
//! that spends it, and the observer that applies a hit and marks a depleted node.
//!
//! Nova owns this rather than importing it because the damage layer needs it to:
//! [`crate::damage`] decides a hit's final magnitude at the weapon and this
//! observer subtracts exactly that. Bevy 0.19 gives no ordering between two
//! observers of one event, so a nova observer that tried to adjust a third
//! party's event would race its subtractor and lose half the time.
//!
//! The overkill rule is the other reason. Damage bubbles up `ChildOf` so an
//! aggregate hull reacts to a hit on a section, and only the amount that
//! ACTUALLY LANDED propagates: a 1000-damage hit on a 100 hp section costs the
//! ship root 100, not 1000.
//!
//! [`HealthIsolated`] opts a node out of that bubbling entirely, for pools that
//! stand IN FRONT of the thing they hang off rather than being part of it.

use bevy::prelude::*;

/// `Health` with its damage message and zero marker, the `destructible_body` bundle and
/// `NovaHealthPlugin`.
pub mod prelude {
    pub use super::{
        destructible_body, Health, HealthApplyDamage, HealthIsolated, HealthZeroMarker,
        NovaHealthPlugin,
    };
}

/// A hit-point pool. `current` never exceeds `max` and is clamped at zero.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Health {
    /// Current health.
    pub current: f32,
    /// Maximum health.
    pub max: f32,
}

impl Health {
    /// A full pool of `max` hit points.
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

/// Marks a node whose health has reached zero. Inserted by `on_damage`; the
/// integrity core turns it into [`IntegrityDisabledMarker`](super::components::IntegrityDisabledMarker).
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct HealthZeroMarker;

/// Marks a pool that absorbs its damage instead of passing it up `ChildOf`.
///
/// The default bubbling says "a hit on my child is a hit on me", which is right
/// for a section of a ship: the ship really is the sum of its parts. It is wrong
/// for anything that stands IN FRONT of its parent rather than being part of it.
/// Cladding is the case that forced this: a plate bolted to a hull would charge
/// that hull for every hit it soaked, so armour made a ship take MORE damage
/// than going bare, and a piercing round that spent budget on the plate and then
/// hit the hull charged it twice over.
///
/// A node marked this way still takes and spends its own damage normally. Only
/// the ancestors stop hearing about it.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct HealthIsolated;

/// Spend `amount` hit points on `entity`.
///
/// `amount` is the FINAL number: the weapon decided it (see
/// [`apply_damage`](crate::damage::apply_damage), the one path in) and nothing
/// between there and here reinterprets it. Trigger this directly only where
/// there is no weapon behind the hit (a test rig, a scripted scenario beat).
///
/// Propagates up `ChildOf` carrying only what landed - see the module docs.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct HealthApplyDamage {
    /// The entity receiving the damage.
    pub entity: Entity,
    /// The collider that dealt it (a bullet, a blast, a rammed hull), for threat
    /// attribution. `None` for unattributed damage.
    pub source: Option<Entity>,
    /// Hit points to spend: the final number, as the weapon computed it.
    pub amount: f32,
}

/// Registers the health store: the `on_damage` observer and the reflected types.
#[derive(Default)]
pub struct NovaHealthPlugin;

impl Plugin for NovaHealthPlugin {
    fn build(&self, app: &mut App) {
        trace!("NovaHealthPlugin: build");

        app.register_type::<Health>();
        app.register_type::<HealthZeroMarker>();
        app.register_type::<HealthIsolated>();
        app.add_observer(on_damage);
    }
}

/// Subtract a hit from a node's pool, mark it at zero, and propagate only what
/// landed.
///
/// The propagated amount is clamped to the node's remaining health, which is
/// what stops overkill on a child from teleporting into a parent aggregate.
/// Entity-event propagation reuses one event instance for every ancestor, so
/// mutating `amount` here is exactly what the next node up sees.
///
/// A node with no [`Health`] (an intermediate transform parent) leaves the amount
/// untouched so it keeps bubbling. A node already destroyed or already at zero
/// absorbs nothing and zeroes the amount: hitting a corpse must not charge its
/// parents. A [`HealthIsolated`] node stops the walk outright, whether or not it
/// had anything left to give - a spent plate is still a plate in the way, and
/// the round behind it is what carries damage inward, not the event.
fn on_damage(
    mut damage: On<HealthApplyDamage>,
    mut commands: Commands,
    mut q_health: Query<(
        Entity,
        &mut Health,
        Has<HealthZeroMarker>,
        Has<HealthIsolated>,
    )>,
) {
    let target = damage.entity;
    trace!("on_damage: target {:?}, damage {:?}", target, damage.amount);

    let Ok((entity, mut health, destroyed, isolated)) = q_health.get_mut(target) else {
        trace!("on_damage: entity {:?} has no Health, bubbling on", target);
        return;
    };

    if isolated {
        damage.propagate(false);
    }

    if destroyed || health.current <= 0.0 {
        trace!("on_damage: entity {:?} is already spent", entity);
        damage.amount = 0.0;
        return;
    }

    let applied = damage.amount.min(health.current);
    health.current -= applied;
    damage.amount = applied;
    if health.current <= 0.0 {
        health.current = 0.0;
        commands.entity(entity).insert(HealthZeroMarker);
    }
}

/// The common makeup of a destructible physics body: a health pool and a physics
/// density, visible by default.
///
/// Pair it with a [`Collider`](avian3d::prelude::Collider) (whose shape varies
/// per object) on an entity parented to a `RigidBody`. This is the physics/health
/// half; the destruction behaviour is the other half - add
/// [`ConnectedTo`](super::components::ConnectedTo), mark the body
/// [`IntegrityRoot`](super::components::IntegrityRoot), and react to
/// `On<Add, IntegrityDestroyMarker>`.
pub fn destructible_body(health: f32, density: f32) -> impl Bundle {
    (
        Health::new(health),
        avian3d::prelude::ColliderDensity(density),
        Visibility::Inherited,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health_app() -> App {
        let mut app = App::new();
        app.add_plugins(NovaHealthPlugin);
        app
    }

    /// Overkill on a child must not teleport into its parent aggregate: the
    /// parent is charged only what the child could actually absorb.
    #[test]
    fn overkill_on_a_child_only_costs_the_parent_the_childs_remaining_health() {
        let mut app = health_app();
        let parent = app.world_mut().spawn(Health::new(200.0)).id();
        let child = app
            .world_mut()
            .spawn((Health::new(100.0), ChildOf(parent)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: child,
            source: None,
            amount: 1000.0,
        });
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(child).unwrap().current, 0.0);
        assert!(app.world().get::<HealthZeroMarker>(child).is_some());
        assert_eq!(
            app.world().get::<Health>(parent).unwrap().current,
            100.0,
            "the parent is charged the child's remaining health, not the raw hit"
        );
    }

    /// Cladding is armour, so it must not double as a damage relay: a plate
    /// bolted to a hull absorbs the hit and the hull hears nothing. Without
    /// this, armour makes a ship take MORE damage than going bare.
    #[test]
    fn an_isolated_child_absorbs_its_hit_instead_of_charging_its_parent() {
        let mut app = health_app();
        let hull = app.world_mut().spawn(Health::new(200.0)).id();
        let plate = app
            .world_mut()
            .spawn((Health::new(100.0), HealthIsolated, ChildOf(hull)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: plate,
            source: None,
            amount: 40.0,
        });
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(plate).unwrap().current, 60.0);
        assert_eq!(
            app.world().get::<Health>(hull).unwrap().current,
            200.0,
            "the hull behind the plate is untouched"
        );
    }

    /// Isolation survives the plate's own death. A spent plate is still a plate
    /// in the way; what reaches the hull is the ROUND carrying on, not the
    /// event walking up.
    #[test]
    fn a_spent_isolated_child_still_shields_its_parent_from_overkill() {
        let mut app = health_app();
        let hull = app.world_mut().spawn(Health::new(200.0)).id();
        let plate = app
            .world_mut()
            .spawn((Health::new(20.0), HealthIsolated, ChildOf(hull)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: plate,
            source: None,
            amount: 1000.0,
        });
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(plate).unwrap().current, 0.0);
        assert!(app.world().get::<HealthZeroMarker>(plate).is_some());
        assert_eq!(
            app.world().get::<Health>(hull).unwrap().current,
            200.0,
            "overkill on cladding must not leak into the hull behind it"
        );
    }

    /// A hit on a corpse charges nobody: the dead node absorbs nothing AND
    /// zeroes the propagated amount, so its parents are untouched.
    #[test]
    fn hitting_a_spent_node_charges_neither_it_nor_its_parents() {
        let mut app = health_app();
        let parent = app.world_mut().spawn(Health::new(200.0)).id();
        let child = app
            .world_mut()
            .spawn((Health::new(100.0), ChildOf(parent)))
            .id();

        for _ in 0..2 {
            app.world_mut().trigger(HealthApplyDamage {
                entity: child,
                source: None,
                amount: 100.0,
            });
            app.world_mut().flush();
        }

        assert_eq!(
            app.world().get::<Health>(parent).unwrap().current,
            100.0,
            "the second hit landed on a corpse and cost the parent nothing"
        );
    }

    /// A node with no `Health` is transparent: the hit passes through it to the
    /// next ancestor that has a pool, at full strength.
    #[test]
    fn a_health_less_node_passes_the_hit_through_undiminished() {
        let mut app = health_app();
        let root = app.world_mut().spawn(Health::new(200.0)).id();
        let joint = app.world_mut().spawn(ChildOf(root)).id();
        let node = app.world_mut().spawn(ChildOf(joint)).id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount: 30.0,
        });
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(root).unwrap().current, 170.0);
    }

    /// The overkill clamp must not become a shield: when the parent's pool
    /// equals the child's, a lethal hit on the child still zeroes BOTH. The
    /// clamp caps the propagated amount at what landed, and what landed is
    /// exactly enough.
    #[test]
    fn a_lethal_hit_still_bubbles_to_zero_a_matching_parent() {
        let mut app = health_app();
        let parent = app.world_mut().spawn(Health::new(100.0)).id();
        let child = app
            .world_mut()
            .spawn((Health::new(100.0), ChildOf(parent)))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: child,
            source: None,
            amount: 1000.0,
        });
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(child).unwrap().current, 0.0);
        assert_eq!(app.world().get::<Health>(parent).unwrap().current, 0.0);
        assert!(app.world().get::<HealthZeroMarker>(child).is_some());
        assert!(app.world().get::<HealthZeroMarker>(parent).is_some());
    }
}
