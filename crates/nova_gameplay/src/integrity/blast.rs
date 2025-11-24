use bevy::prelude::*;
use avian3d::prelude::*;

pub mod prelude {
    pub use super::{blast_damage, BlastDamageConfig, BlastDamageMarker};
}

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
