//! A generated station for an example: the open world's station generator
//! run on the loaded parts, for one archetype, advancement and seed.
//!
//! Include with `#[path = "../shared/generated_station.rs"] mod
//! generated_station;`.

use nova_gameplay::prelude::CivilizationId;
use nova_protocol::prelude::*;

/// The layout and the station design a living Neutral civilization at
/// `advancement` builds in `archetype` from the parts of `loaded`, as the
/// open world builds one.
///
/// # Panics
///
/// When the loaded parts do not build a snapshot or the generator fails:
/// the example cannot stand its station.
pub fn generated_station_design(
    loaded: &LoadedSectionPacks,
    items: &GameItems,
    archetype: StationArchetypeType,
    advancement: f32,
    seed: u32,
) -> (StationLayout, StationDesign) {
    let packs: Vec<ShipPartPack> = loaded
        .packs
        .iter()
        .map(|pack| ShipPartPack {
            id: pack.id.clone(),
            dependencies: pack.dependencies.clone(),
            sections: pack.sections.clone(),
        })
        .collect();
    let snapshot = ShipPartSnapshot::build(&packs).unwrap_or_else(|faults| {
        panic!("generated station: the loaded parts are refused: {faults:?}")
    });
    let civilization = Civilization {
        id: CivilizationId {
            world_seed: seed,
            node: [0, 0, 0],
        },
        centroid: Meters3::ZERO,
        status: CivilizationStatusType::Living,
        advancement,
        role_preference: [1.0; 4],
        stance: StanceType::Neutral,
    };
    let layout = generate_station(
        &snapshot,
        StationLayoutRequest {
            seed,
            civilization: civilization.id,
            archetype,
            advancement,
        },
    )
    .unwrap_or_else(|failure| panic!("generated station: {failure}"));
    let design = generated_station(&snapshot, items, &layout, &civilization);
    (layout, design)
}
