//! Shared scenario fixtures for the loader submodule tests.

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;

use crate::prelude::*;

pub(crate) fn spawn_object_action() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "obj".to_string(),
            name: "Obj".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: None,
            destroy_sound: None,
            radius: 1.0,
            texture: AssetRef::default(),
            health: 1.0,
            mass: None,
            invulnerable: false,
            lock_signature: None,
        }),
    })
}

pub(crate) fn event_with(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
    ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions,
    }
}

pub(crate) fn scenario_with(id: &str, events: Vec<ScenarioEventConfig>) -> ScenarioConfig {
    ScenarioConfig {
        description: "For tests".to_string(),
        events,
        ..ScenarioConfig::new(
            id.to_string(),
            "Test Scenario".to_string(),
            AssetRef::default(),
        )
    }
}
