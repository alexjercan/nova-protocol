//! Shared config fixtures for the lint submodule tests.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;

use crate::{lint::KnownSections, prelude::*};

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
/// A catalog of known prototype ids, none of them mounts (the shape
/// every pre-mount-check test wants: the adjacency arm stays silent).
pub(crate) fn sections(ids: &[&str]) -> KnownSections {
    KnownSections {
        ids: known(ids),
        mounts: HashSet::new(),
    }
}

/// A catalog where `mounts` are mount-kind prototypes (also known).
pub(crate) fn sections_with_mounts(ids: &[&str], mounts: &[&str]) -> KnownSections {
    let mut catalog = sections(ids);
    for id in mounts {
        catalog.ids.insert(id.to_string());
        catalog.mounts.insert(id.to_string());
    }
    catalog
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
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            sections: vec![SpaceshipSectionConfig {
                id: "hull".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Prototype(proto.to_string()),
                modifications: vec![],
            }],
        }),
    })
}

pub(crate) fn scenario(
    actions: Vec<EventActionConfig>,
    filters: Vec<EventFilterConfig>,
) -> ScenarioConfig {
    ScenarioConfig {
        id: "test_scenario".to_string(),
        name: "Test".to_string(),
        description: "Test".to_string(),
        cubemap: AssetRef::default(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters,
            actions,
        }],
        ..Default::default()
    }
}

pub(crate) fn errors(issues: &[LintIssue]) -> Vec<&LintIssue> {
    issues
        .iter()
        .filter(|i| i.severity == LintSeverity::Error)
        .collect()
}
