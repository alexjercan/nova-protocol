//! Generated wrecks from the real base catalog at the lowest advancement: the
//! snapshot the content lint checks, ruined by the world's own
//! `generate_wreck`.

use bevy::math::Vec3;
use nova_authoring::lint_walk::repo_ship_part_packs;
use nova_ship::prelude::GRID_EPSILON;
use nova_world::prelude::{CivilizationId, ShipRoleType};
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

#[test]
fn base_industrial_spines_that_fit_no_intake_carry_an_intake_pair_and_a_dock() {
    // These world-seed-1 requests failed to fit an intake on a short spine.
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    for seed in [92, 471, 1451] {
        let request = ShipLayoutRequest {
            seed,
            civilization: CivilizationId {
                world_seed: 1,
                node: [0, 0, 0],
            },
            role: ShipRoleType::Industrial,
            advancement: 0.0,
        };
        let ship = generate_ship(&snapshot, request)
            .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
        let fitted = |slot: &str| {
            ship.design
                .sections
                .iter()
                .filter(|section| section.id.starts_with(slot))
                .count()
        };
        assert_eq!(fitted("intake_"), 2, "seed {seed}");
        assert!(fitted("dock") >= 1, "seed {seed}");
        generate_wreck(&snapshot, request)
            .unwrap_or_else(|failure| panic!("seed {seed}: {failure}"));
    }
}
