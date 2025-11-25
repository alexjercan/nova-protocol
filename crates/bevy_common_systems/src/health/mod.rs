//! Health component and related systems for Bevy games.
//!
//! This plugin provides a simple health system for game entities.
//!
//! Features:
//! - `Health` component to track current and maximum health.
//! - `HealthApplyDamage` event to apply damage to entities.
//! - `HealthZeroMarker` component added when an entity's health reaches zero.
//!
//! Usage:
//! ```rust
//! commands.spawn((
//!     Health::new(100.0),
//! ));
//!
//! // Apply damage from some system
//! commands.trigger(HealthApplyDamage {
//!     target: entity,
//!     source: Some(player_entity),
//!     amount: 25.0,
//! });
//! ```

use bevy::prelude::*;

pub mod prelude {
    pub use super::{
        HealthZeroMarker, Health, HealthApplyDamage, HealthPlugin, HealthPluginSystems,
    };
}

/// Component representing the health of an entity.
///
/// Contains current and maximum health values. Health cannot exceed `max`
/// and should not drop below 0.
#[derive(Component, Clone, Debug, Reflect)]
pub struct Health {
    /// Current health value.
    pub current: f32,

    /// Maximum health value.
    pub max: f32,
}

impl Health {
    /// Create a new Health component with `current` equal to `max`.
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

/// Marker component indicating that an entity has been destroyed.
///
/// This is automatically added by the `on_damage` system when an entity's
/// health reaches zero. You can use this marker to trigger destruction logic
/// like removing the entity, playing effects, or spawning loot.
#[derive(Component, Clone, Debug, Reflect)]
pub struct HealthZeroMarker;

/// Event to apply damage to a target entity.
///
/// `amount` is subtracted from the target's current health. If health reaches
/// zero or below, the `HealthZeroMarker` is added.
#[derive(Event, Clone, Debug)]
pub struct HealthApplyDamage {
    /// The entity receiving damage.
    pub target: Entity,

    /// TODO: Maybe make this `source` more configurable? - what if we can also specify stuff like
    /// damage type, critical hit, etc.?
    /// Optional source entity causing the damage.
    pub source: Option<Entity>,

    /// Amount of damage to apply.
    pub amount: f32,
}

/// System set for the Health plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HealthPluginSystems {
    /// Systems responsible for syncing health and applying damage.
    Sync,
}

/// Plugin that enables the Health component and related systems.
#[derive(Default)]
pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        debug!("HealthPlugin: build");

        // Listen for damage events and apply them to entities
        app.add_observer(on_damage);
    }
}

/// System to handle `HealthApplyDamage` events.
///
/// Reduces the target's current health by the damage amount. If health
/// reaches zero, adds `HealthZeroMarker`.
fn on_damage(
    damage: On<HealthApplyDamage>,
    mut commands: Commands,
    mut q_health: Query<(Entity, &mut Health, Has<HealthZeroMarker>)>,
) {
    let target = damage.target;
    trace!("on_damage: target {:?}, damage {:?}", target, damage.amount);

    let Ok((entity, mut health, destroyed)) = q_health.get_mut(target) else {
        trace!("on_damage: entity {:?} not found in q_health", target);
        return;
    };

    if destroyed {
        trace!("on_damage: entity {:?} is already destroyed", entity);
        return;
    }

    if health.current <= 0.0 {
        trace!("on_damage: entity {:?} health is already zero", entity);
        return;
    }

    health.current -= damage.amount;
    if health.current <= 0.0 {
        health.current = 0.0;
        commands.entity(entity).insert(HealthZeroMarker);
    }
}
