//! Generated stations from the real base catalog: the snapshot the content
//! lint checks, laid out by the world's own `generate_station` and stocked by
//! its `generated_station`, pass the station lint at every advancement and
//! stock the ammunition of every weapon their civilization knows.

use std::collections::BTreeSet;

use nova_authoring::{
    generation::{build_items, build_section_catalog, build_styles},
    lint_walk::repo_ship_part_packs,
};
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::{
    CivilizationId, GameItems, ItemCategoryType, ItemRoleType, StanceType,
};
use nova_scenario::prelude::{
    lint_station_design, resolve_ship_design, GameShipDesigns, KnownSections, LintSeverity,
    ShipDesignSource,
};
use nova_ship::prelude::{GameSections, GameStyles, RAILGUN_LANCE_SECTION_ID};
use nova_world_base::prelude::{
    generate_station, generated_station, station_ceiling, Civilization, CivilizationStatusType,
    ShipPartSnapshot, StationArchetypeType, StationLayoutRequest,
};

/// The civilizations every sweep builds for. The first knew no structural
/// cube hull at advancement 0.5 before `known` guaranteed one, so its ring
/// failed with "no eligible station hull part".
const CIVILIZATIONS: [CivilizationId; 3] = [
    CivilizationId {
        world_seed: 11,
        node: [2, 0, -1],
    },
    CivilizationId {
        world_seed: 3,
        node: [0, 0, 0],
    },
    CivilizationId {
        world_seed: 20_260_922,
        node: [1, -1, 0],
    },
];

/// Every archetype at every quarter of advancement, over eight seeds and
/// three civilizations, lays out from the base parts inside its ceiling and
/// passes the station lint with no error.
#[test]
fn every_base_station_passes_the_station_lint_at_every_advancement() {
    let parts = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let catalog = build_section_catalog();
    let sections = KnownSections::from_configs(catalog.iter());
    let items = GameItems::new(build_items());
    let styles = GameStyles(build_styles());
    for id in CIVILIZATIONS {
        for archetype in StationArchetypeType::ALL {
            for advancement in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let civilization = Civilization {
                    id,
                    centroid: Meters3::ZERO,
                    status: CivilizationStatusType::Living,
                    advancement,
                    role_preference: [1.0; 4],
                    stance: StanceType::Neutral,
                };
                for seed in 0..8 {
                    let context =
                        format!("{id}, {archetype}, advancement {advancement}, seed {seed}");
                    let request = StationLayoutRequest {
                        seed,
                        civilization: id,
                        archetype,
                        advancement,
                    };
                    let layout = generate_station(&parts, request)
                        .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    assert!(
                        layout.clearance.0 <= station_ceiling(advancement).0,
                        "{context}: {:?}",
                        layout.clearance
                    );
                    let design = generated_station(&parts, &items, &layout, &civilization)
                        .unwrap_or_else(|constraint| panic!("{context}: {constraint}"));
                    let errors: Vec<String> = lint_station_design(
                        &context,
                        &design,
                        &sections,
                        &items,
                        &styles,
                        "generated",
                    )
                    .into_iter()
                    .filter(|issue| issue.severity == LintSeverity::Error)
                    .map(|issue| issue.message)
                    .collect();
                    assert!(errors.is_empty(), "{context}: {errors:#?}");
                }
            }
        }
    }
}

/// Every seeded station of every archetype, at every quarter of advancement
/// and for three civilizations, stocks at least one unit of each item its
/// civilization's known reloading weapons draw, no other ammunition, and no
/// more than its hull holds; between them they stock every kind, the rare
/// torpedo too.
#[test]
fn a_seeded_station_stocks_its_known_weapons_ammunition_within_its_hold() {
    let parts = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let sections = GameSections(build_section_catalog());
    let items = GameItems::new(build_items());
    let mut stocked = BTreeSet::new();
    for id in CIVILIZATIONS {
        for archetype in StationArchetypeType::ALL {
            for advancement in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let civilization = Civilization {
                    id,
                    centroid: Meters3::ZERO,
                    status: CivilizationStatusType::Living,
                    advancement,
                    role_preference: [1.0; 4],
                    stance: StanceType::Neutral,
                };
                let known: BTreeSet<ItemRoleType> = parts
                    .known(id, advancement)
                    .into_iter()
                    .filter_map(|part| part.config.kind.reload_item())
                    .collect();
                for seed in 0..2 {
                    let context =
                        format!("{id}, {archetype}, advancement {advancement}, seed {seed}");
                    let request = StationLayoutRequest {
                        seed,
                        civilization: id,
                        archetype,
                        advancement,
                    };
                    let layout = generate_station(&parts, request)
                        .unwrap_or_else(|failure| panic!("{context}: {failure}"));
                    let design = generated_station(&parts, &items, &layout, &civilization)
                        .unwrap_or_else(|constraint| panic!("{context}: {constraint}"));
                    for ammunition in &known {
                        assert!(
                            design
                                .stock
                                .stacks()
                                .any(|(item, count)| item.as_str() == ammunition.id() && count > 0),
                            "{context}: no {ammunition:?} for a known weapon"
                        );
                    }
                    for (item, _) in design.stock.stacks() {
                        if let Some(ammunition) = ItemRoleType::ALL.into_iter().find(|role| {
                            role.id() == item.as_str() && role.category() == ItemCategoryType::Ammo
                        }) {
                            assert!(
                                known.contains(&ammunition),
                                "{context}: stocks {ammunition:?}, its weapons reload {known:?}"
                            );
                            stocked.insert(ammunition);
                        }
                    }
                    let (resolved, _) = resolve_ship_design(
                        &ShipDesignSource::Inline(design.hull.clone()),
                        &GameShipDesigns::default(),
                        &sections,
                    );
                    assert!(
                        design.stock.mass_g(&items) <= u64::from(resolved.cargo_capacity_g()),
                        "{context}: the stock overfills the hold"
                    );
                }
            }
        }
    }
    assert_eq!(
        stocked,
        BTreeSet::from([
            ItemRoleType::PdcRound,
            ItemRoleType::RailSlug,
            ItemRoleType::Torpedo,
        ]),
        "the stations between them stock every kind"
    );
}

/// A generated station builds exactly what its civilization knows, never
/// what its stock could pay for: a ring-0 station lists no railgun lance,
/// whose recipe shares its panels and spars with parts it does build, and a
/// civilization at advancement 0.5 that knows the lance lists it.
#[test]
fn a_generated_station_builds_what_its_civilization_knows_and_no_more() {
    let parts = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let items = GameItems::new(build_items());
    let station = |id: CivilizationId, advancement: f32| {
        let civilization = Civilization {
            id,
            centroid: Meters3::ZERO,
            status: CivilizationStatusType::Living,
            advancement,
            role_preference: [1.0; 4],
            stance: StanceType::Neutral,
        };
        let request = StationLayoutRequest {
            seed: 1,
            civilization: id,
            archetype: StationArchetypeType::Ring,
            advancement,
        };
        let layout = generate_station(&parts, request)
            .unwrap_or_else(|failure| panic!("{id} at {advancement}: {failure}"));
        generated_station(&parts, &items, &layout, &civilization)
            .unwrap_or_else(|constraint| panic!("{id} at {advancement}: {constraint}"))
    };

    let ring0 = station(CIVILIZATIONS[0], 0.0);
    let known: BTreeSet<&str> = parts
        .known(CIVILIZATIONS[0], 0.0)
        .into_iter()
        .map(|part| part.id())
        .collect();
    let built: BTreeSet<&str> = ring0.builds.iter().map(String::as_str).collect();
    assert_eq!(
        built, known,
        "a ring-0 station builds what its civilization knows"
    );
    assert_eq!(
        ring0.builds.len(),
        built.len(),
        "each id once: {:?}",
        ring0.builds
    );
    assert!(
        !built.contains(RAILGUN_LANCE_SECTION_ID),
        "a ring-0 station does not build the railgun lance: {built:?}"
    );

    let advanced = CIVILIZATIONS
        .into_iter()
        .find(|id| {
            parts
                .known(*id, 0.5)
                .iter()
                .any(|part| part.id() == RAILGUN_LANCE_SECTION_ID)
        })
        .expect("a sweep civilization knows the railgun lance at advancement 0.5");
    let advanced_station = station(advanced, 0.5);
    let built: BTreeSet<&str> = advanced_station.builds.iter().map(String::as_str).collect();
    assert!(
        built.contains(RAILGUN_LANCE_SECTION_ID),
        "{advanced} at 0.5 builds the railgun lance it knows: {built:?}"
    );
}
