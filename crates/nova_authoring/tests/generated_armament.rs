//! Armament of generated ships from the real base catalog: what the open
//! world's armed ships carry early and late, laid out by the world's own
//! `generate_ship` over the snapshot the content lint checks, and the
//! ammunition `ship_stock` holds for it.

use std::collections::BTreeSet;

use nova_authoring::{generation::build_items, lint_walk::repo_ship_part_packs};
use nova_gameplay::prelude::{
    escort_raider_size, CivilizationId, GameItems, ItemCategoryType, ItemRoleType, ShipRoleType,
    ShipSizeType,
};
use nova_ship::prelude::SectionKind;
use nova_world::prelude::SectorShipConditionType;
use nova_world_base::prelude::{
    generate_ship, ship_stock, ShipLayoutRequest, ShipPartSnapshot, ShipSpareDriveType,
};

/// Weapon sections mounted across eight civilizations, `seeds` seeds and
/// every armed role at `advancement`: turrets, torpedo bays and railgun
/// lances.
fn census(snapshot: &ShipPartSnapshot, advancement: f32, seeds: u32) -> (usize, usize, usize) {
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
            for seed in 0..seeds {
                let request = ShipLayoutRequest {
                    seed,
                    civilization,
                    role,
                    advancement,
                    size: ShipSizeType::Advancement,
                    spare_drive: ShipSpareDriveType::Without,
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
///
/// Every part open at advancement 0 is known by every civilization outright
/// (`ShipPartSnapshot::known`), so at advancement 0 the eight civilizations
/// draw the same weapon set and the seed is the census's only real axis; 32
/// seeds is what it takes to land the torpedo bay's low `draw_weight`
/// reliably. Above advancement 0 a civilization's known set is still a
/// seeded draw, so eight seeds across eight civilizations keeps catching the
/// lance.
#[test]
fn early_ships_carry_rare_torpedoes_and_no_lance_and_late_ships_carry_lances() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));

    let (turrets, bays, lances) = census(&snapshot, 0.0, 32);
    assert!(bays >= 1, "no early torpedo bay: {turrets} turrets");
    assert!(
        bays * 2 < turrets,
        "early bays {bays} against turrets {turrets}"
    );
    assert_eq!(lances, 0, "an early lance mounted");

    let (_, _, lances) = census(&snapshot, 0.45, 8);
    assert_eq!(lances, 0, "a lance mounted below its advancement");

    let (_, _, lances) = census(&snapshot, 1.0, 8);
    assert!(lances >= 1, "no lance at full advancement");
}

/// Every intact armed ship across eight civilizations and eight seeds, early
/// and late, holds ammunition only of the items its own reloading weapons
/// draw, and some of them hold some.
#[test]
fn a_generated_hold_carries_only_the_ammunition_its_weapons_reload_from() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let items = GameItems::new(build_items());
    let mut stocked = BTreeSet::new();
    for advancement in [0.0, 1.0] {
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
                    let context = format!("{civilization:?}, seed {seed}, {role:?}, {advancement}");
                    let request = ShipLayoutRequest {
                        seed,
                        civilization,
                        role,
                        advancement,
                        size: ShipSizeType::Advancement,
                        spare_drive: ShipSpareDriveType::Without,
                    };
                    let ship = generate_ship(&snapshot, request)
                        .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    let reloads: BTreeSet<ItemRoleType> = ship
                        .design
                        .sections
                        .iter()
                        .filter_map(|section| {
                            let id = section.source.prototype_id();
                            snapshot
                                .parts()
                                .iter()
                                .find(|part| part.id() == id)
                                .unwrap_or_else(|| panic!("'{id}' is a base part"))
                                .config
                                .kind
                                .reload_item()
                        })
                        .collect();
                    let stock = ship_stock(
                        &snapshot,
                        &items,
                        &ship.design,
                        role,
                        SectorShipConditionType::Intact,
                        advancement,
                        seed,
                    )
                    .unwrap_or_else(|| panic!("{context}: the hold fits its lightest item"));
                    for (item, _) in stock.stacks() {
                        let Some(ammunition) = ItemRoleType::ALL.into_iter().find(|role| {
                            role.id() == item.as_str() && role.category() == ItemCategoryType::Ammo
                        }) else {
                            continue;
                        };
                        assert!(
                            reloads.contains(&ammunition),
                            "{context}: holds {ammunition:?}, its weapons reload {reloads:?}"
                        );
                        stocked.insert(ammunition);
                    }
                }
            }
        }
    }
    assert!(!stocked.is_empty(), "no armed ship holds ammunition");
}

/// An escort raider posted below advancement 0.25 is about the player's
/// 27-section start ship with one turret and no other weapon, across eight civilizations and
/// sixteen seeds through the band; a normal scavenger at the same advancement
/// can be four times that with two turrets.
#[test]
fn an_escort_raider_below_advancement_a_quarter_stays_within_the_small_budget_with_one_turret() {
    let snapshot = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    for advancement in [0.0, 0.008, 0.12, 0.249] {
        let size = escort_raider_size(advancement);
        assert_eq!(
            size,
            ShipSizeType::Budget {
                sections: 40,
                turrets: 1
            },
            "the band below 0.25 at {advancement}"
        );
        for node in 0..8 {
            let civilization = CivilizationId {
                world_seed: 37,
                node: [node, 0, 0],
            };
            if !snapshot
                .eligible_roles(civilization, advancement)
                .contains(&ShipRoleType::Scavenger)
            {
                continue;
            }
            for seed in 0..16 {
                let context = format!("{civilization:?}, seed {seed}, {advancement}");
                let request = ShipLayoutRequest {
                    seed,
                    civilization,
                    role: ShipRoleType::Scavenger,
                    advancement,
                    size,
                    spare_drive: ShipSpareDriveType::Without,
                };
                let ship = generate_ship(&snapshot, request)
                    .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                let sections = ship.design.sections.len();
                assert!(
                    (20..=40).contains(&sections),
                    "{context}: {sections} sections"
                );
                let weapons: Vec<&SectionKind> = ship
                    .design
                    .sections
                    .iter()
                    .filter_map(|section| {
                        let id = section.source.prototype_id();
                        let part = snapshot.parts().iter().find(|part| part.id() == id)?;
                        matches!(
                            part.config.kind,
                            SectionKind::Turret(_)
                                | SectionKind::Torpedo(_)
                                | SectionKind::Railgun(_)
                        )
                        .then_some(&part.config.kind)
                    })
                    .collect();
                assert!(
                    matches!(weapons.as_slice(), [SectionKind::Turret(_)]),
                    "{context}: weapons {weapons:?}"
                );
            }
        }
    }
}
