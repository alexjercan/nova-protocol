//! The base game's shipped stations and the station CONTENT entries a
//! scenario spawns them by.
//!
//! One station for now: the waystation, a parked block hull a ship docks
//! with to rebuild, refit, trade and store. Its hull is built the way the
//! block fleet is (`ships::block`), with a bridge and a collar and no drive.

use nova_gameplay::prelude::{ItemRoleType, ShipInventoryStock};
use nova_scenario::prelude::{StationDesign, StationServiceType};
use nova_world_base::prelude::BLOCK_WAYSTATION_STATION_ID;

use super::{
    assets::BaseContentAssets,
    items::{
        ACTUATOR_PACK, DRIVE_BELL, FLIGHT_CORE, FRAME_SPAR, SECTION_PANEL, TURRET_BARREL,
        TURRET_BODY,
    },
    ships,
};

/// Grams the waystation stores for the ships that dock with it: ten hull
/// plates' worth, a tenth of its own 1.7 t hold.
const WAYSTATION_STORAGE_CAP_G: u32 = 100_000;

/// Every shipped station, in stable generated-content order.
pub(crate) fn station_catalog(assets: &BaseContentAssets) -> Vec<StationDesign> {
    vec![StationDesign {
        id: BLOCK_WAYSTATION_STATION_ID.into(),
        name: "Waystation".to_string(),
        about: "A plated two-deck block with a bridge set into the top deck and a docking \
                collar off the port flank. Ships dock to rebuild, refit, trade and leave \
                cargo in its store."
            .to_string(),
        hull: ships::waystation_hull(assets),
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
    }]
}
