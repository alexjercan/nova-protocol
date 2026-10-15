//! Generated ships and wrecks from the real base catalog: the snapshot the
//! content lint checks, laid out by the world's own `generate_ship` and
//! ruined by its `generate_wreck`.

use bevy::math::Vec3;
use nova_authoring::lint_walk::repo_ship_part_packs;
use nova_scenario::prelude::SpaceshipSectionConfig;
use nova_ship::prelude::{intake_in_dock_approach, RefitPlacedSection, SectionKind, GRID_EPSILON};
use nova_world::prelude::{CivilizationId, ShipRoleType};
use nova_world_base::prelude::{
    generate_ship, generate_wreck, ShipLayoutRequest, ShipPartSnapshot, MAX_SHIP_SECTIONS,
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

/// The base request every sweep below names, at `seed`, `role` and
/// `advancement`.
fn request(seed: u32, role: ShipRoleType, advancement: f32) -> ShipLayoutRequest {
    ShipLayoutRequest {
        seed,
        civilization: CivilizationId {
            world_seed: 3,
            node: [0, 0, 0],
        },
        role,
        advancement,
    }
}

/// The base capital drive is a top-tier part: no ship below advancement 0.5
/// mounts one, and from 0.6 up some ships of every sweep do.
#[test]
fn the_capital_drive_mounts_from_mid_advancement_only() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    for advancement in [0.0, 0.25, 0.45, 0.6, 0.8, 1.0] {
        let mut capital = 0;
        for seed in 0..32 {
            for role in snapshot.eligible_roles(advancement) {
                let context = format!("seed {seed}, {role:?}, advancement {advancement}");
                let ship = generate_ship(&snapshot, request(seed, role, advancement))
                    .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                capital += ship
                    .design
                    .sections
                    .iter()
                    .filter(|section| section.source.prototype_id() == "capital_thruster_section")
                    .count();
            }
        }
        assert_eq!(
            capital > 0,
            advancement >= 0.6,
            "advancement {advancement}: {capital} capital drives"
        );
    }
}

/// An armored hull at full advancement fights with both weapon kinds: every
/// one mounts turrets beside its torpedo bays, though the base torpedo bays
/// alone reach the top part tier.
#[test]
fn every_armored_ship_at_full_advancement_mounts_turrets_and_torpedo_bays() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let kind = |id: &str| {
        &snapshot
            .parts()
            .iter()
            .find(|part| part.id() == id)
            .unwrap_or_else(|| panic!("'{id}' is a base part"))
            .config
            .kind
    };
    for seed in 0..12 {
        let ship = generate_ship(&snapshot, request(seed, ShipRoleType::Armored, 1.0))
            .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
        let kinds: Vec<&SectionKind> = ship
            .design
            .sections
            .iter()
            .map(|section| kind(section.source.prototype_id()))
            .collect();
        let turrets = kinds
            .iter()
            .filter(|kind| matches!(kind, SectionKind::Turret(_)))
            .count();
        let bays = kinds
            .iter()
            .filter(|kind| matches!(kind, SectionKind::Torpedo(_)))
            .count();
        assert!(
            turrets > 0 && bays > 0,
            "seed {seed}: {turrets} turrets, {bays} torpedo bays"
        );
    }
}

/// Every generated ship at full advancement, the largest it grows, stays
/// within the per-ship section budget.
#[test]
fn generated_ships_at_full_advancement_stay_within_the_section_budget() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    for role in snapshot.eligible_roles(1.0) {
        for seed in 1..=32 {
            let ship = generate_ship(&snapshot, request(seed, role, 1.0))
                .unwrap_or_else(|failure| panic!("seed {seed}, {role:?}: {failure}"));
            let sections = ship.design.sections.len();
            assert!(
                sections <= MAX_SHIP_SECTIONS,
                "seed {seed}, {role:?}: {sections} sections"
            );
        }
    }
}
