//! Generated wrecks from the real base catalog at the lowest advancement: the
//! snapshot the content lint checks, ruined by the world's own
//! `generate_wreck`.

use bevy::math::Vec3;
use nova_authoring::lint_walk::repo_ship_part_packs;
use nova_ship::prelude::GRID_EPSILON;
use nova_world_base::prelude::{
    generate_wreck, CivilizationId, ShipLayoutConstraintType, ShipLayoutRequest, ShipPartSnapshot,
};

#[test]
fn a_base_wreck_at_advancement_zero_breaks_its_mirror_or_fails_as_unruinable() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let roles = snapshot.eligible_roles(0.0);
    let mut ruined = Vec::new();
    for seed in 0..12 {
        for role in roles.iter().copied() {
            let request = ShipLayoutRequest {
                seed,
                civilization: CivilizationId {
                    world_seed: 7,
                    node: [0, 0, 0],
                },
                role,
                advancement: 0.0,
            };
            match generate_wreck(&snapshot, request) {
                Ok(wreck) => {
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
                    ruined.push(role);
                }
                // Only a hull that cannot lose one off-centre outer cube
                // fails.
                Err(failure) => assert_eq!(
                    failure.constraint,
                    ShipLayoutConstraintType::Unruined {
                        omitted: 0,
                        needed: 1
                    },
                    "{failure}"
                ),
            }
        }
    }
    for role in roles {
        assert!(ruined.contains(&role), "no {role:?} wreck at advancement 0");
    }
}
