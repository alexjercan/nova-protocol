//! Armament of generated ships from the real base catalog: what the open
//! world's armed ships carry early and late, laid out by the world's own
//! `generate_ship` over the snapshot the content lint checks.

use nova_authoring::lint_walk::repo_ship_part_packs;
use nova_gameplay::prelude::{CivilizationId, ShipRoleType};
use nova_ship::prelude::SectionKind;
use nova_world_base::prelude::{generate_ship, ShipLayoutRequest, ShipPartSnapshot};

/// Weapon sections mounted across eight civilizations, eight seeds and every
/// armed role at `advancement`: turrets, torpedo bays and railgun lances.
fn census(snapshot: &ShipPartSnapshot, advancement: f32) -> (usize, usize, usize) {
    let (mut turrets, mut bays, mut lances) = (0, 0, 0);
    for node in 0..8 {
        let civilization = CivilizationId {
            world_seed: 5,
            node: [node, 0, 0],
        };
        for role in snapshot.eligible_roles(civilization, advancement) {
            if !matches!(role, ShipRoleType::Scavenger | ShipRoleType::Armored) {
                continue;
            }
            for seed in 0..8 {
                let request = ShipLayoutRequest {
                    seed,
                    civilization,
                    role,
                    advancement,
                };
                let ship = generate_ship(snapshot, request).unwrap_or_else(|failure| {
                    panic!("{civilization:?}, seed {seed}, {role:?}: {failure}")
                });
                for section in &ship.design.sections {
                    let id = section.source.prototype_id();
                    let part = snapshot
                        .parts()
                        .iter()
                        .find(|part| part.id() == id)
                        .unwrap_or_else(|| panic!("'{id}' is a base part"));
                    match &part.config.kind {
                        SectionKind::Turret(_) => turrets += 1,
                        SectionKind::Torpedo(_) => bays += 1,
                        SectionKind::Railgun(_) => lances += 1,
                        _ => {}
                    }
                }
            }
        }
    }
    (turrets, bays, lances)
}

/// At advancement 0 the armed base ships mount torpedo bays on a few hulls,
/// fewer than half as often as turrets, and never a railgun lance; below the
/// lance's advancement no lance mounts either, and at full advancement some
/// ships carry one.
#[test]
fn early_ships_carry_rare_torpedoes_and_no_lance_and_late_ships_carry_lances() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));

    let (turrets, bays, lances) = census(&snapshot, 0.0);
    assert!(bays >= 1, "no early torpedo bay: {turrets} turrets");
    assert!(
        bays * 2 < turrets,
        "early bays {bays} against turrets {turrets}"
    );
    assert_eq!(lances, 0, "an early lance mounted");

    let (_, _, lances) = census(&snapshot, 0.45);
    assert_eq!(lances, 0, "a lance mounted below its advancement");

    let (_, _, lances) = census(&snapshot, 1.0);
    assert!(lances >= 1, "no lance at full advancement");
}
