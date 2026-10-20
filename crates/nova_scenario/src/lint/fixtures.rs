//! Shared config fixtures for the lint submodule tests.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::{AssetRef, ShipInventoryStock};
use nova_ship::prelude::SectionConfig;

use crate::{
    lint::{KnownSections, KnownShipDesigns, KnownStationDesigns},
    prelude::*,
};

pub(crate) fn known(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// A plain chapter in a void, by id.
pub(crate) fn chapter(id: &str) -> ScenarioConfig {
    ScenarioConfig::new(id.to_string(), id.to_string(), AssetRef::default())
}

/// A chapter that declares `role`.
pub(crate) fn with_role(id: &str, role: ScenarioRole) -> ScenarioConfig {
    ScenarioConfig {
        role,
        ..chapter(id)
    }
}

/// A chapter that runs on `world`.
pub(crate) fn with_world(id: &str, world: ScenarioWorldType) -> ScenarioConfig {
    ScenarioConfig {
        world,
        ..chapter(id)
    }
}

/// A generated, unbounded world with `seed`, or the session's seed on `None`.
pub(crate) fn generated(seed: Option<u32>) -> ScenarioWorldType {
    ScenarioWorldType::Generated(GeneratedWorldConfig { seed, bounds: None })
}

/// The registry `lint_campaign` resolves members against: `scenarios` by id.
pub(crate) fn registry(scenarios: &[ScenarioConfig]) -> HashMap<String, &ScenarioConfig> {
    scenarios
        .iter()
        .map(|scenario| (scenario.id.clone(), scenario))
        .collect()
}

pub(crate) fn campaign(id: &str, members: &[&str]) -> CampaignConfig {
    CampaignConfig {
        id: id.to_string(),
        name: id.to_string(),
        scenarios: members.iter().map(|s| s.to_string()).collect(),
    }
}
/// A recipe every item catalog accepts: no parts, so it names nothing a
/// fixture catalog could lack, and some credits, so it is never free.
pub(crate) fn fixture_recipe() -> nova_gameplay::prelude::BuildRecipe {
    nova_gameplay::prelude::BuildRecipe {
        credits: 1,
        parts: Default::default(),
        strip_yield: 0.5,
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
                build: fixture_recipe(),
                advancement: 0.0,
                draw_weight: 1.0,
                ..default()
            },
            kind: nova_ship::prelude::SectionKind::Hull(default()),
        })
        .collect();
    KnownSections::from_configs(&configs)
}

/// A catalog of known ships, each one unit-cube hull section named `hull`.
pub(crate) fn ships(ids: &[&str]) -> KnownShipDesigns {
    let configs: Vec<_> = ids
        .iter()
        .map(|id| ShipDesignPrototype {
            id: (*id).into(),
            name: (*id).to_string(),
            design: ShipDesign {
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype("hull"),
                }],
                ..default()
            },
        })
        .collect();
    KnownShipDesigns::from_configs(&configs)
}

/// The station catalog a lint resolves `Station` spawns against: one design
/// per id over a one-cell hull, enough for an id to resolve.
pub(crate) fn stations(ids: &[&str]) -> KnownStationDesigns {
    let configs: Vec<_> = ids
        .iter()
        .map(|id| StationDesignPrototype {
            id: (*id).into(),
            name: (*id).to_string(),
            about: String::new(),
            design: StationDesign {
                hull: ShipDesign {
                    sections: vec![SpaceshipSectionConfig {
                        id: "hull".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        source: SectionSource::prototype("hull"),
                    }],
                    ..default()
                },
                services: vec![StationServiceType::Trade],
                builds: Vec::new(),
                storage_cap_g: 0,
                stock: ShipInventoryStock::default(),
                credits: 0,
            },
        })
        .collect();
    KnownStationDesigns::from_configs(&configs)
}

pub(crate) fn spawn_object(id: &str) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: id.to_uppercase(),
            radius: Meters(10.0),
            color: Color::WHITE,
            area_radius: Some(Meters(50.0)),
            lock_signature: None,
        }),
    })
}

pub(crate) fn spawn_ship(id: &str, proto: &str) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::AI(AIControllerConfig::default()),
            design: ShipDesignSource::Inline(ShipDesign {
                sections: vec![SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype(proto),
                }],
                ..default()
            }),
            ..default()
        }),
    })
}

/// A spawn action for one armed ship, with a section of every class a
/// section-addressed weapon action can name: `spinal` a railgun, `bay` a
/// torpedo bay, `nose` a plain hull block.
///
/// The controller is the caller's, because half of what the scripted actions
/// lint for is WHO drives the ship they address.
pub(crate) fn spawn_armed_ship(id: &str, controller: SpaceshipController) -> EventActionConfig {
    let section = |section_id: &str, kind: nova_ship::prelude::SectionKind, cell: f32| {
        SpaceshipSectionConfig {
            id: section_id.to_string(),
            position: Vec3::new(0.0, 0.0, cell),
            rotation: Quat::IDENTITY,
            source: SectionSource::Inline(SectionConfig {
                base: nova_ship::prelude::BaseSectionConfig {
                    id: section_id.to_string(),
                    link_points: nova_ship::prelude::unit_cube_link_points(),
                    build: fixture_recipe(),
                    advancement: 0.0,
                    draw_weight: 1.0,
                    ..default()
                },
                kind,
            }),
        }
    };
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: id.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            design: ShipDesignSource::Inline(ShipDesign {
                sections: vec![
                    section(
                        "nose",
                        nova_ship::prelude::SectionKind::Hull(default()),
                        0.0,
                    ),
                    section(
                        "spinal",
                        nova_ship::prelude::SectionKind::Railgun(default()),
                        1.0,
                    ),
                    section(
                        "bay",
                        nova_ship::prelude::SectionKind::Torpedo(default()),
                        -1.0,
                    ),
                ],
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
            label: None,
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
