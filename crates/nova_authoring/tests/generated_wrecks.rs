//! Generated ships and wrecks from the real base catalog: the snapshot the
//! content lint checks, laid out by the world's own `generate_ship` and
//! ruined by its `generate_wreck`.

use bevy::math::Vec3;
use nova_authoring::lint_walk::repo_ship_part_packs;
use nova_scenario::prelude::SpaceshipSectionConfig;
use nova_ship::prelude::{intake_in_dock_approach, RefitPlacedSection, SectionKind, GRID_EPSILON};
use nova_world::prelude::CivilizationId;
use nova_world_base::prelude::{
    generate_ship, generate_wreck, ShipLayoutRequest, ShipPartSnapshot,
};

#[test]
fn every_base_wreck_at_advancement_zero_breaks_its_mirror() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    for seed in 0..12 {
        for role in snapshot.eligible_roles(0.0) {
            let request = ShipLayoutRequest {
                seed,
                civilization: CivilizationId {
                    world_seed: 7,
                    node: [0, 0, 0],
                },
                role,
                advancement: 0.0,
            };
            let wreck = generate_wreck(&snapshot, request)
                .unwrap_or_else(|failure| panic!("seed {seed}, {role:?}: {failure}"));
            let sections = &wreck.design.sections;
            let unmirrored = sections
                .iter()
                .filter(|section| {
                    let image = section.position * Vec3::new(-1.0, 1.0, 1.0);
                    !sections
                        .iter()
                        .any(|other| other.position.abs_diff_eq(image, GRID_EPSILON))
                })
                .count();
            assert!(unmirrored >= 1, "seed {seed}, {role:?}");
        }
    }
}

/// Every open-world ship takes cargo in: each base ship, at both ends of the
/// advancement range, carries a cargo intake that stays behind its docking
/// ports' faces, so a berthed hull never parks on it.
#[test]
fn every_base_ship_carries_an_intake_clear_of_its_ports() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let config = |section: &SpaceshipSectionConfig| {
        let id = section.source.prototype_id();
        &snapshot
            .parts()
            .iter()
            .find(|part| part.id() == id)
            .unwrap_or_else(|| panic!("'{id}' is a base part"))
            .config
    };
    for advancement in [0.0, 1.0] {
        for seed in 0..12 {
            for role in snapshot.eligible_roles(advancement) {
                let request = ShipLayoutRequest {
                    seed,
                    civilization: CivilizationId {
                        world_seed: 1,
                        node: [0, 0, 0],
                    },
                    role,
                    advancement,
                };
                let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                let ship = generate_ship(&snapshot, request)
                    .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                let placed: Vec<RefitPlacedSection<'_>> = ship
                    .design
                    .sections
                    .iter()
                    .map(|section| RefitPlacedSection {
                        position: section.position,
                        rotation: section.rotation,
                        config: config(section),
                    })
                    .collect();
                assert!(
                    placed
                        .iter()
                        .any(|section| matches!(section.config.kind, SectionKind::CargoIntake(_))),
                    "{context}"
                );
                assert_eq!(intake_in_dock_approach(&placed), None, "{context}");
            }
        }
    }
}
