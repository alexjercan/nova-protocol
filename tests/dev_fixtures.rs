//! Structural contracts for the example-only ship fixtures.

use std::collections::{HashMap, HashSet};

use bevy::prelude::IVec3;

#[path = "../examples/shared/dev_fixtures/mod.rs"]
mod dev_fixtures;

use nova_authoring::generation::prelude::*;
use nova_protocol::prelude::*;

fn fleet() -> Vec<(&'static str, ShipDesign)> {
    vec![
        ("raider", dev_fixtures::raider()),
        ("carrier", dev_fixtures::carrier()),
        ("warship", dev_fixtures::warship()),
        ("skiff", dev_fixtures::skiff()),
        ("tug", dev_fixtures::tug()),
        ("claw", dev_fixtures::claw()),
        ("cleanup leader", dev_fixtures::cleanup_leader()),
    ]
}

#[test]
fn every_development_fixture_names_each_section_once() {
    for (name, design) in fleet() {
        let ids: HashSet<_> = design
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect();
        assert_eq!(
            ids.len(),
            design.sections.len(),
            "'{name}' repeats a section id across {} sections",
            design.sections.len()
        );
    }
}

#[test]
fn no_two_development_fixture_sections_share_a_cell() {
    for (name, design) in fleet() {
        let mut seats: HashMap<IVec3, &str> = HashMap::new();
        for section in &design.sections {
            let rounded = section.position.round();
            if !section.position.abs_diff_eq(rounded, 1e-5) {
                continue;
            }
            if let Some(other) = seats.insert(rounded.as_ivec3(), &section.id) {
                panic!("'{name}' seats '{}' on top of '{other}'", section.id);
            }
        }
    }
}

#[test]
fn every_development_fixture_has_one_connected_link_graph() {
    let catalog = build_section_catalog();
    for (name, design) in fleet() {
        let configs: Vec<&SectionConfig> = design
            .sections
            .iter()
            .map(|section| match &section.source {
                SectionSource::Inline(config) => config,
                SectionSource::Prototype { id, .. } => catalog
                    .iter()
                    .find(|prototype| prototype.base.id == *id)
                    .unwrap_or_else(|| panic!("'{name}' references unknown prototype '{id}'")),
            })
            .collect();
        let placed: Vec<PlacedSectionLinkPoints<'_>> = design
            .sections
            .iter()
            .zip(configs)
            .map(|(section, config)| PlacedSectionLinkPoints {
                position: section.position,
                rotation: section.rotation,
                link_points: &config.base.link_points,
            })
            .collect();
        assert!(
            derive_link_point_graph(&placed).is_ok(),
            "'{name}' has a disconnected or invalid link graph: {:?}",
            derive_link_point_graph(&placed).unwrap_err()
        );
    }
}

#[test]
fn the_warship_carries_every_addressed_weapon_and_owns_its_siege_sections() {
    let design = dev_fixtures::warship();
    for weapon in dev_fixtures::block::WARSHIP_RAILGUN_IDS
        .iter()
        .chain(dev_fixtures::block::WARSHIP_BAY_IDS.iter())
        .chain(dev_fixtures::block::WARSHIP_TURRET_IDS.iter())
    {
        assert!(
            design.sections.iter().any(|section| section.id == *weapon),
            "the warship is missing '{weapon}'"
        );
    }

    for (prototype, expected) in [
        (
            "fixture_siege_railgun_lance",
            dev_fixtures::block::WARSHIP_RAILGUN_IDS.len(),
        ),
        (
            "fixture_siege_torpedo_bay",
            dev_fixtures::block::WARSHIP_BAY_IDS.len(),
        ),
    ] {
        let inline = design
            .sections
            .iter()
            .filter(|section| {
                matches!(
                    &section.source,
                    SectionSource::Inline(config) if config.base.id == prototype
                )
            })
            .count();
        assert_eq!(inline, expected, "unexpected '{prototype}' mount count");
    }
}
