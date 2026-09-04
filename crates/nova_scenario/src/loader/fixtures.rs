//! Shared scenario fixtures for the loader submodule tests.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::AssetRef;

use crate::prelude::*;

pub(crate) fn spawn_object_action() -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "obj".to_string(),
            name: "Obj".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: KIND_ROCK.to_string(),
            destroy_sound: None,
            radius: Meters(10.0),
            texture: AssetRef::default(),
            mass: None,
            invulnerable: false,
            seed: None,
            lock_signature: None,
        }),
    })
}

pub(crate) fn event_with(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
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
