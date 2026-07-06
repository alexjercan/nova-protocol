use bevy::prelude::*;
use bevy_common_systems::prelude::*;

pub mod prelude {
    pub use super::{
        health_hud, HealthHudConfig, HealthHudMarker, HealthHudPlugin, HealthHudTargetEntity,
    };
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct HealthHudMarker;

#[derive(Clone, Debug, Default)]
pub struct HealthHudConfig {
    pub target: Option<Entity>,
}

pub fn health_hud(config: HealthHudConfig) -> impl Bundle {
    debug!("health_hud: config {:?}", config);

    (
        Name::new("HealthHUD"),
        HealthHudMarker,
        HealthHudTargetEntity(config.target),
        Text::new("Health: 100%"),
        TextShadow::default(),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            right: px(5),
            ..default()
        },
    )
}

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct HealthHudTargetEntity(Option<Entity>);

#[derive(Default)]
pub struct HealthHudPlugin;

impl Plugin for HealthHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_text_hud.in_set(super::NovaHudSystems));
    }
}

fn update_text_hud(
    mut q_hud: Query<(&mut Text, &HealthHudTargetEntity), With<HealthHudMarker>>,
    q_target: Query<&Health>,
) {
    for (mut hud_input, target) in &mut q_hud {
        let Some(target) = **target else {
            **hud_input = "Health: 0%".to_string();
            continue;
        };

        let Ok(health) = q_target.get(target) else {
            **hud_input = "Health: 0%".to_string();
            continue;
        };

        let health_percent = (health.current / health.max * 100.0).round();
        **hud_input = format!("Health: {}%", health_percent);
    }
}
