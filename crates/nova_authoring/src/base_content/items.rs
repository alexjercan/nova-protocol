//! Built-in ITEM content: every item the base game's holds, trades, reloads,
//! repairs and mining use.
//!
//! An item is content like a section is content, so this file is a builder -
//! `content gen` serializes it into `assets/base/items/base.content.ron`. A
//! mod adds items under its own ids, and a mod that depends on another pack
//! may replace that pack's whole item.
//!
//! The ids of [`ItemRoleType`](nova_gameplay::prelude::ItemRoleType) are the
//! ones the game itself uses - weapons reload from them, repair spends one,
//! mining yields them, generated sector ship holds draw them - so the base
//! game must define each under its role's category. A dependent mod may replace one of
//! them whole, with new mass and prices, under the same category.
//! Prices are provisional values, not a balance decision; each bid is below
//! its ask, so buying and selling back loses credits. A PDC round prices by
//! the lot of 100: one round is worth less than a credit.
//!
//! The section parts below have no role: no system spends them by id. Section
//! build recipes name them, so a lost section is rebuilt from them at a
//! station, and the recipe lint holds each recipe above its strip value.

use nova_gameplay::prelude::{ItemCategoryType, ItemDesign, ItemDesignId, ItemRoleType};

/// Light structural panel: every section recipe spends some.
pub(crate) const SECTION_PANEL: &str = "SectionPanel";
/// Load-bearing frame member for large sections.
pub(crate) const FRAME_SPAR: &str = "FrameSpar";
/// Motor and gearing for anything that moves.
pub(crate) const ACTUATOR_PACK: &str = "ActuatorPack";
/// Turret housing and traverse ring.
pub(crate) const TURRET_BODY: &str = "TurretBody";
/// One gun barrel.
pub(crate) const TURRET_BARREL: &str = "TurretBarrel";
/// Thrust chamber and nozzle.
pub(crate) const DRIVE_BELL: &str = "DriveBell";
/// Flight computer core for a controller.
pub(crate) const FLIGHT_CORE: &str = "FlightCore";
/// Energy store for railguns and beams.
pub(crate) const CAPACITOR_BANK: &str = "CapacitorBank";
/// Launch tube for a torpedo bay.
pub(crate) const TORPEDO_TUBE: &str = "TorpedoTube";
/// Seal ring and latches for a docking port.
pub(crate) const DOCKING_COLLAR: &str = "DockingCollar";

/// The base game's items, in the order the file carries them.
pub(crate) fn item_catalog() -> Vec<ItemDesign> {
    use ItemCategoryType::{Ammo, Food, Parts, Raw, Repair};
    [
        (
            ItemRoleType::HullPlate.id(),
            "Hull plate",
            "Structural plating for hull sections.",
            Repair,
            10_000,
            1,
            40,
            30,
        ),
        (
            ItemRoleType::PdcRound.id(),
            "PDC round",
            "Point-defense round. Kinetic and Pierce mounts reload from it.",
            Ammo,
            200,
            100,
            10,
            7,
        ),
        (
            ItemRoleType::RailSlug.id(),
            "Rail slug",
            "Railgun slug. Railgun mounts reload from it.",
            Ammo,
            20_000,
            1,
            40,
            30,
        ),
        (
            ItemRoleType::Torpedo.id(),
            "Torpedo",
            "Torpedo. Every torpedo bay reloads from it.",
            Ammo,
            150_000,
            1,
            400,
            300,
        ),
        (
            ItemRoleType::StoneOre.id(),
            "Stone ore",
            "Silicate rock, mined from rock asteroids.",
            Raw,
            10_000,
            1,
            4,
            3,
        ),
        (
            ItemRoleType::IronOre.id(),
            "Iron ore",
            "Metal-rich ore, mined from metal asteroids.",
            Raw,
            10_000,
            1,
            16,
            12,
        ),
        (
            ItemRoleType::WaterIce.id(),
            "Water ice",
            "Frozen volatiles, mined from ice asteroids.",
            Raw,
            10_000,
            1,
            12,
            9,
        ),
        (
            ItemRoleType::CarbonOre.id(),
            "Carbon ore",
            "Carbon-rich ore, mined from carbon asteroids.",
            Raw,
            10_000,
            1,
            8,
            6,
        ),
        (
            ItemRoleType::Rations.id(),
            "Rations",
            "Packed crew food. Traders buy and sell it.",
            Food,
            2_000,
            1,
            8,
            6,
        ),
        (
            ItemRoleType::SalvagedParts.id(),
            "Salvaged parts",
            "Reusable ship components. Traders buy and sell them.",
            Parts,
            25_000,
            1,
            120,
            90,
        ),
        (
            SECTION_PANEL,
            "Section panel",
            "Light structural panel. Every section build spends some.",
            Parts,
            10_000,
            1,
            20,
            15,
        ),
        (
            FRAME_SPAR,
            "Frame spar",
            "Load-bearing frame member for large sections.",
            Parts,
            40_000,
            1,
            60,
            45,
        ),
        (
            ACTUATOR_PACK,
            "Actuator pack",
            "Motor and gearing for mounts, doors and intakes.",
            Parts,
            15_000,
            1,
            80,
            60,
        ),
        (
            TURRET_BODY,
            "Turret body",
            "Turret housing and traverse ring.",
            Parts,
            60_000,
            1,
            200,
            150,
        ),
        (
            TURRET_BARREL,
            "Turret barrel",
            "One gun barrel for a point-defense mount.",
            Parts,
            20_000,
            1,
            60,
            45,
        ),
        (
            DRIVE_BELL,
            "Drive bell",
            "Thrust chamber and nozzle for a drive section.",
            Parts,
            80_000,
            1,
            160,
            120,
        ),
        (
            FLIGHT_CORE,
            "Flight computer core",
            "Flight computer for a controller section.",
            Parts,
            10_000,
            1,
            400,
            300,
        ),
        (
            CAPACITOR_BANK,
            "Capacitor bank",
            "Energy store for railguns and mining beams.",
            Parts,
            40_000,
            1,
            240,
            180,
        ),
        (
            TORPEDO_TUBE,
            "Torpedo tube",
            "Launch tube for a torpedo bay.",
            Parts,
            80_000,
            1,
            240,
            180,
        ),
        (
            DOCKING_COLLAR,
            "Docking collar",
            "Seal ring and latches for a docking port.",
            Parts,
            80_000,
            1,
            200,
            150,
        ),
    ]
    .into_iter()
    .map(
        |(id, name, about, category, mass_g, price_lot, ask_cr, bid_cr)| ItemDesign {
            id: ItemDesignId::from(id),
            name: name.to_string(),
            about: about.to_string(),
            category,
            mass_g,
            price_lot,
            ask_cr,
            bid_cr,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use nova_gameplay::test_support::test_items;

    use super::*;

    /// The gameplay test fixture restates the base rows, so a retune in this
    /// builder that leaves the fixture behind fails here instead of leaving
    /// every price and mass test green on old numbers.
    #[test]
    fn the_test_item_fixture_matches_the_base_catalog() {
        let fixture = test_items();
        let catalog = item_catalog();
        assert_eq!(fixture.iter().count(), catalog.len());
        for design in &catalog {
            let row = |item: &ItemDesign| {
                (
                    item.name.clone(),
                    item.category,
                    item.mass_g,
                    item.price_lot,
                    item.ask_cr,
                    item.bid_cr,
                )
            };
            let mirrored = fixture
                .get(&design.id)
                .unwrap_or_else(|| panic!("test_items lacks '{}'", design.id));
            assert_eq!(row(mirrored), row(design), "'{}'", design.id);
        }
    }
}
