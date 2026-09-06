//! Shared config fixtures for the lint submodule tests.

use std::collections::HashSet;

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::{AssetRef, CHANNEL_COMMS, CHANNEL_CREW, CHANNEL_GUARD};
use nova_ship::prelude::{
    GrammarGrid, GrammarKeel, GrammarPart, GrammarVacuum, SectionConfig, ShipGrammarConfig,
};

use crate::{
    lint::{KnownSections, KnownShips},
    prelude::*,
};

pub(crate) fn known(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// The channel ids base content authors, so a fixture cue naming one of them
/// resolves the way it does in the shipped game.
pub(crate) fn base_channels() -> HashSet<String> {
    known(&[CHANNEL_COMMS, CHANNEL_CREW, CHANNEL_GUARD])
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

/// A catalog whose prototypes carry an authored CELL SPAN, for the checks that
/// read a seeded part's footprint rather than just its id.
pub(crate) fn sized_sections(spans: &[(&str, UVec3)]) -> KnownSections {
    let configs: Vec<_> = spans
        .iter()
        .map(|(id, span)| SectionConfig {
            base: nova_ship::prelude::BaseSectionConfig {
                id: (*id).to_string(),
                collider: Some(nova_ship::prelude::SectionCollider::Cuboid {
                    size: span.as_vec3(),
                }),
                link_points: nova_ship::prelude::unit_cube_link_points(),
                ..default()
            },
            kind: nova_ship::prelude::SectionKind::Hull(default()),
        })
        .collect();
    KnownSections::from_configs(&configs)
}

/// The catalog [`grammar`] draws from: the four unit-cube roles it seeds, plus
/// the oversized parts the seed-fit checks need something to name.
pub(crate) fn grammar_sections() -> KnownSections {
    sized_sections(&[
        ("hull", UVec3::ONE),
        ("bridge", UVec3::ONE),
        ("deck", UVec3::ONE),
        ("drive", UVec3::ONE),
        ("capital_drive", UVec3::new(5, 5, 3)),
        ("lance", UVec3::new(1, 1, 4)),
        ("broad_lance", UVec3::new(3, 1, 2)),
    ])
}

/// One well-formed grammar over [`grammar_sections`], on the shipped hull's own
/// grid - the shape each `lint_grammar_config` test bends exactly one field of.
pub(crate) fn grammar() -> ShipGrammarConfig {
    ShipGrammarConfig {
        id: "test_hull".to_string(),
        name: "Test Hull".to_string(),
        grid: GrammarGrid {
            half_width: 4,
            height: 5,
            length: 11,
        },
        vacuum: GrammarVacuum {
            base: 1.0,
            taper: 0.5,
            stern: 2.0,
            bow_taper: 1.0,
        },
        keel: GrammarKeel {
            hull: "hull".to_string(),
            bridge: "bridge".to_string(),
            stern_deck: "deck".to_string(),
            stern_drive: "drive".to_string(),
            bow_gun: None,
        },
        parts: vec![GrammarPart {
            prototype: "hull".to_string(),
            weight: 1.0,
            aim: None,
            zone: None,
        }],
    }
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
                    ..default()
                },
                kind,
            }),
            modifications: vec![],
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
            hull: ShipSource::Inline(ShipHull {
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
