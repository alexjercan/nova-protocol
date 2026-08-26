//! Shared config fixtures for the lint submodule tests.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_ship::prelude::SectionConfig;

use crate::{
    lint::{KnownSections, KnownShips},
    prelude::*,
};

pub(crate) fn known(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

pub(crate) fn campaign(id: &str, members: &[&str]) -> CampaignConfig {
    CampaignConfig {
        id: id.to_string(),
        name: id.to_string(),
        scenarios: members.iter().map(|s| s.to_string()).collect(),
    }
}
/// A catalog of known unit-cube hull prototypes.
pub(crate) fn sections(ids: &[&str]) -> KnownSections {
    let configs: Vec<_> = ids
        .iter()
        .map(|id| SectionConfig {
            base: nova_ship::prelude::BaseSectionConfig {
                id: (*id).to_string(),
                link_points: nova_ship::prelude::unit_cube_link_points(),
                ..default()
            },
            kind: nova_ship::prelude::SectionKind::Hull(default()),
        })
        .collect();
    KnownSections::from_configs(&configs)
}

/// A catalog of known ships, each one unit-cube hull section named `hull`.
pub(crate) fn ships(ids: &[&str]) -> KnownShips {
    let configs: Vec<_> = ids
        .iter()
        .map(|id| ShipConfig {
            id: (*id).to_string(),
            name: (*id).to_string(),
            hull: ShipHull {
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype("hull".to_string()),
                    modifications: vec![],
                }],
                ..default()
            },
        })
        .collect();
    KnownShips::from_configs(&configs)
}

pub(crate) fn spawn_object(id: &str) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: id.to_uppercase(),
            radius: 1.0,
            color: Color::WHITE,
            area_radius: Some(5.0),
            lock_signature: None,
        }),
    })
}

pub(crate) fn spawn_ship(id: &str, proto: &str) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            hull: ShipSource::Inline(ShipHull {
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Prototype(proto.to_string()),
                    modifications: vec![],
                }],
                ..default()
            }),
            ..default()
        }),
    })
}

pub(crate) fn scenario(
    actions: Vec<EventActionConfig>,
    filters: Vec<EventFilterConfig>,
) -> ScenarioConfig {
    ScenarioConfig {
        description: "Test".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            once: false,
            filters,
            actions,
        }],
        ..ScenarioConfig::new(
            "test_scenario".to_string(),
            "Test".to_string(),
            AssetRef::default(),
        )
    }
}

pub(crate) fn errors(issues: &[LintIssue]) -> Vec<&LintIssue> {
    issues
        .iter()
        .filter(|i| i.severity == LintSeverity::Error)
        .collect()
}
