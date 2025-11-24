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

pub struct IntegrityPlugin;

impl Plugin for IntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("IntegrityPlugin: build");

        // Handle explosion on destruction
        app.add_plugins(super::explode::ExplodablePlugin);

        app.add_observer(on_collider_of_spawn);
        app.add_observer(on_impact_collision_event);
        app.add_observer(on_blast_collision_event);
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
