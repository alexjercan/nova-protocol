//! Generated stations from the real base catalog: the snapshot the content
//! lint checks, laid out by the world's own `generate_station` and stocked by
//! its `generated_station`, pass the station lint at every advancement.

use nova_authoring::{
    generation::{build_items, build_section_catalog},
    lint_walk::repo_ship_part_packs,
};
use nova_events::prelude::Meters3;
use nova_gameplay::prelude::{CivilizationId, GameItems, StanceType};
use nova_scenario::prelude::{lint_station_design, KnownSections, LintSeverity};
use nova_world_base::prelude::{
    generate_station, generated_station, station_ceiling, Civilization, CivilizationStatusType,
    ShipPartSnapshot, StationArchetypeType, StationLayoutRequest,
};

/// Every archetype at every quarter of advancement, over eight seeds, lays
/// out from the base parts inside its ceiling and passes the station lint
/// with no error.
#[test]
fn every_base_station_passes_the_station_lint_at_every_advancement() {
    let parts = ShipPartSnapshot::build(&repo_ship_part_packs(&["base"]))
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));
    let catalog = build_section_catalog();
    let sections = KnownSections::from_configs(catalog.iter());
    let items = GameItems::new(build_items());
    let id = CivilizationId {
        world_seed: 11,
        node: [2, 0, -1],
    };
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
                let context = format!("{archetype}, advancement {advancement}, seed {seed}");
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
                let design = generated_station(&parts, &items, &layout, &civilization);
                let errors: Vec<String> =
                    lint_station_design(&context, &design, &sections, &items, "generated")
                        .into_iter()
                        .filter(|issue| issue.severity == LintSeverity::Error)
                        .map(|issue| issue.message)
                        .collect();
                assert!(errors.is_empty(), "{context}: {errors:#?}");
            }
        }
    }
}
