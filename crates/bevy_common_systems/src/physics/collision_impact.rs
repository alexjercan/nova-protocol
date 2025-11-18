//! A Bevy plugin to detect collision impacts and generate impact events for entities.

use avian3d::prelude::*;
use bevy::prelude::*;

pub mod prelude {
    pub use super::{
        CollisionImpactEvent, CollisionImpactMarker, CollisionImpactPlugin, CollisionImpactSystems,
    };
}

/// Marker component for entities that should generate collision impact events.
#[derive(Component, Clone, Debug, Reflect)]
pub struct CollisionImpactMarker;

/// Event triggered when an entity is impacted by a collision.
#[derive(Event, Clone, Debug)]
pub struct CollisionImpactEvent {
    /// The entity that took the impact.
    pub entity: Entity,
    /// The other entity involved in the collision.
    pub other: Entity,
    // /// The point of impact in world space.
    // pub hit_point: Vec3,
    // /// The surface normal at the impact point.
    // pub hit_normal: Vec3,
    /// The relative velocity between the two entities at the point of impact.
    pub relative_velocity: Vec3,
}

/// System sets used by the CollisionImpactPlugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CollisionImpactSystems {
    Sync,
}

/// Plugin to manage collision impact events for entities with `CollisionImpactMarker`.
pub struct CollisionImpactPlugin;

impl Plugin for CollisionImpactPlugin {
    fn build(&self, app: &mut App) {
        debug!("CollisionImpactPlugin: build");

        app.add_observer(insert_collision_events);
        app.add_observer(on_collision_event);
    }
}

/// Insert collision event tracking for newly added entities with `CollisionImpactMarker`.
fn insert_collision_events(add: On<Add, CollisionImpactMarker>, mut commands: Commands) {
    let entity = add.entity;
    trace!("insert_collision_events: entity {:?}", entity);

    commands.entity(entity).insert(CollisionEventsEnabled);
}

/// Handle collision events and trigger `CollisionImpactEvent`s for impacted entities.
fn on_collision_event(
    collision: On<CollisionStart>,
    mut commands: Commands,
    q_body: Query<&LinearVelocity, (With<RigidBody>, With<CollisionImpactMarker>)>,
    q_velocity: Query<&LinearVelocity, With<RigidBody>>,
) {
    trace!(
        "on_collision_event: collision between {:?} and {:?}",
        collision.body1,
        collision.body2
    );

    let Some(body) = collision.body1 else {
        return;
    };
    let Some(other) = collision.body2 else {
        return;
    };

    let Ok(velocity1) = q_body.get(body) else {
        return;
    };
    let Ok(velocity2) = q_velocity.get(other) else {
        return;
    };

    let relative_velocity = **velocity1 - **velocity2;
    if relative_velocity.length_squared() < 0.1 {
        return;
    }

    commands.trigger(CollisionImpactEvent {
        entity: body,
        other,
        // hit_point: collision.contact_point,
        // hit_normal: collision.contact_normal,
        relative_velocity,
    });
}
