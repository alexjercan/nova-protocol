//! Shared config fixtures for the lint submodule tests.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;
use nova_ship::prelude::SectionConfig;

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

/// A catalog where `mounts` are mount-kind prototypes (also known).
pub(crate) fn sections_with_mounts(ids: &[&str], mounts: &[&str]) -> KnownSections {
    let mut all = ids.to_vec();
    all.extend(mounts.iter().copied());
    let configs: Vec<_> = all
        .iter()
        .map(|id| SectionConfig {
            base: nova_ship::prelude::BaseSectionConfig {
                id: (*id).to_string(),
                link_points: nova_ship::prelude::unit_cube_link_points(),
                ..default()
            },
            kind: if mounts.contains(id) {
                nova_ship::prelude::SectionKind::Torpedo(default())
            } else {
                nova_ship::prelude::SectionKind::Hull(default())
            },
        })
        .collect();
    KnownSections::from_configs(&configs)
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
        description: "Test".to_string(),
        events: vec![ScenarioEventConfig {
            name: EventConfig::OnStart,
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
