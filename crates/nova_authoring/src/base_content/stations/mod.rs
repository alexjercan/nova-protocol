//! The base game's shipped stations and the station CONTENT entries a
//! scenario spawns them by.
//!
//! Four stations ship. The waystation is a parked block hull a ship docks
//! with to rebuild, refit, trade and store; its hull is built the way the
//! block fleet is (`ships::block`), with two flight computers and a collar
//! and no drive.
//! The ring, spire and yard stations (`layouts`) are the armed stations
//! generated worlds place, one look each.

use nova_gameplay::prelude::{ItemRoleType, ShipInventoryStock};
use nova_scenario::prelude::{
    ShipDesign, ShipIntegrityConfig, SpaceshipSectionConfig, StationDesign, StationServiceType,
    STATION_COLLAPSE_THRESHOLD,
};
use nova_world_base::prelude::{
    ARMOURED_STYLE_ID, BLOCK_WAYSTATION_STATION_ID, INDUSTRIAL_STYLE_ID, RING_STATION_STATION_ID,
    SPIRE_STATION_STATION_ID, STATION_STYLE_ID, YARD_STATION_STATION_ID,
};

use super::{
    assets::BaseContentAssets,
    items::{
        ACTUATOR_PACK, DRIVE_BELL, FLIGHT_CORE, FRAME_SPAR, SECTION_PANEL, TURRET_BARREL,
        TURRET_BODY,
    },
    ships,
};

mod layouts;

/// Grams the waystation stores for the ships that dock with it: ten hull
/// plates' worth, a tenth of its own 1.7 t hold.
const WAYSTATION_STORAGE_CAP_G: u32 = 100_000;

/// Grams an armed station stores for the ships that dock with it: the
/// waystation's cap, so a docked ship reads the same store at every station.
const ARMED_STATION_STORAGE_CAP_G: u32 = WAYSTATION_STORAGE_CAP_G;

/// Every shipped station, in stable generated-content order.
pub(crate) fn station_catalog(assets: &BaseContentAssets) -> Vec<StationDesign> {
    vec![
        waystation(assets),
        armed_station(
            assets,
            RING_STATION_STATION_ID,
            "Ring Station",
            "A plated hub on four truss spokes inside a ring, about 230 m across, with a \
             docking collar at the end of every spoke and twelve point-defense guns on the \
             hub and the ring.",
            layouts::ring_station(),
            STATION_STYLE_ID,
        ),
        armed_station(
            assets,
            SPIRE_STATION_STATION_ID,
            "Spire Station",
            "Three armoured decks stacked on truss columns, about 205 m tall, with four \
             collars round the middle deck, eight point-defense guns on the top and the \
             bottom deck, and solar wings.",
            layouts::spire_station(),
            ARMOURED_STYLE_ID,
        ),
        armed_station(
            assets,
            YARD_STATION_STATION_ID,
            "Yard Station",
            "A working yard about 270 m long: a truss spine between two end blocks, rows \
             of solar panels either side, collars at both ends and on the core, and eight \
             point-defense guns on the end blocks.",
            layouts::yard_station(),
            INDUSTRIAL_STYLE_ID,
        ),
    ]
}

/// The waystation: a parked block, unarmed, with one collar.
fn waystation(assets: &BaseContentAssets) -> StationDesign {
    StationDesign {
        id: BLOCK_WAYSTATION_STATION_ID.into(),
        name: "Waystation".to_string(),
        about: "A plated two-deck block with a bridge set into the top deck, a second flight \
                computer in the lower deck and a docking collar off the port flank. Ships \
                dock to rebuild, refit, trade and leave cargo in its store."
            .to_string(),
        hull: ShipDesign {
            integrity: ShipIntegrityConfig {
                collapse_threshold: Some(STATION_COLLAPSE_THRESHOLD),
            },
            ..ships::waystation_hull(assets)
        },
        services: vec![
            StationServiceType::Rebuild,
            StationServiceType::Refit,
            StationServiceType::Trade,
            StationServiceType::Storage,
        ],
        storage_cap_g: WAYSTATION_STORAGE_CAP_G,
        // Repair stock, the two trade goods and enough section parts to
        // rebuild a PDC, a small drive, a controller and a few hull blocks:
        // 80 kg of plates, 40 kg of food, 250 kg of salvage and 500 kg of
        // section parts in a 1.7 t hold.
        stock: ShipInventoryStock::new([
            (ItemRoleType::HullPlate.into(), 8),
            (ItemRoleType::Rations.into(), 20),
            (ItemRoleType::SalvagedParts.into(), 10),
            (SECTION_PANEL.into(), 20),
            (FRAME_SPAR.into(), 2),
            (ACTUATOR_PACK.into(), 2),
            (TURRET_BODY.into(), 1),
            (TURRET_BARREL.into(), 2),
            (DRIVE_BELL.into(), 1),
            (FLIGHT_CORE.into(), 1),
        ]),
        credits: 5_000,
    }
}

/// One of the armed stations generated worlds place: `sections` clad in
/// `style`, with the four dock services, the shared stock and the station
/// collapse threshold.
fn armed_station(
    assets: &BaseContentAssets,
    id: &str,
    name: &str,
    about: &str,
    sections: Vec<SpaceshipSectionConfig>,
    style: &str,
) -> StationDesign {
    let mut presentation = ships::default_presentation(assets);
    presentation.skin = true;
    presentation.style = Some(style.to_string());
    StationDesign {
        id: id.into(),
        name: name.to_string(),
        about: format!("{about} Ships dock to rebuild, refit, trade and leave cargo in its store."),
        hull: ShipDesign {
            sections,
            integrity: ShipIntegrityConfig {
                collapse_threshold: Some(STATION_COLLAPSE_THRESHOLD),
            },
            presentation,
        },
        services: vec![
            StationServiceType::Rebuild,
            StationServiceType::Refit,
            StationServiceType::Trade,
            StationServiceType::Storage,
        ],
        storage_cap_g: ARMED_STATION_STORAGE_CAP_G,
        // The waystation's goods at about four times the depth, plus the
        // rounds its own guns reload from: 1000 PDC rounds are 200 kg.
        stock: ShipInventoryStock::new([
            (ItemRoleType::PdcRound.into(), 1_000),
            (ItemRoleType::HullPlate.into(), 30),
            (ItemRoleType::Rations.into(), 80),
            (ItemRoleType::SalvagedParts.into(), 40),
            (SECTION_PANEL.into(), 80),
            (FRAME_SPAR.into(), 8),
            (ACTUATOR_PACK.into(), 8),
            (TURRET_BODY.into(), 4),
            (TURRET_BARREL.into(), 8),
            (DRIVE_BELL.into(), 4),
            (FLIGHT_CORE.into(), 4),
        ]),
        credits: 20_000,
    }
}
