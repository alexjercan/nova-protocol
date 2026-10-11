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
//! its ask, so buying and selling back loses credits.

use nova_gameplay::prelude::{ItemCategoryType, ItemDesign, ItemDesignId, ItemRoleType};

/// The base game's items, in the order the file carries them.
pub(crate) fn item_catalog() -> Vec<ItemDesign> {
    use ItemCategoryType::{Ammo, Food, Parts, Raw, Repair};
    [
        (
            ItemRoleType::HullPlate,
            "Hull plate",
            "Structural plating for hull sections.",
            Repair,
            10_000,
            40,
            30,
        ),
        (
            ItemRoleType::PdcRound,
            "PDC round",
            "Point-defense round. Kinetic and Pierce mounts reload from it.",
            Ammo,
            200,
            4,
            3,
        ),
        (
            ItemRoleType::RailSlug,
            "Rail slug",
            "Railgun slug. Railgun mounts reload from it.",
            Ammo,
            20_000,
            40,
            30,
        ),
        (
            ItemRoleType::Torpedo,
            "Torpedo",
            "Torpedo. Every torpedo bay reloads from it.",
            Ammo,
            150_000,
            400,
            300,
        ),
        (
            ItemRoleType::StoneOre,
            "Stone ore",
            "Silicate rock, mined from rock asteroids.",
            Raw,
            10_000,
            4,
            3,
        ),
        (
            ItemRoleType::IronOre,
            "Iron ore",
            "Metal-rich ore, mined from metal asteroids.",
            Raw,
            10_000,
            16,
            12,
        ),
        (
            ItemRoleType::WaterIce,
            "Water ice",
            "Frozen volatiles, mined from ice asteroids.",
            Raw,
            10_000,
            12,
            9,
        ),
        (
            ItemRoleType::CarbonOre,
            "Carbon ore",
            "Carbon-rich ore, mined from carbon asteroids.",
            Raw,
            10_000,
            8,
            6,
        ),
        (
            ItemRoleType::Rations,
            "Rations",
            "Packed crew food. Traders buy and sell it.",
            Food,
            2_000,
            8,
            6,
        ),
        (
            ItemRoleType::SalvagedParts,
            "Salvaged parts",
            "Reusable ship components. Traders buy and sell them.",
            Parts,
            25_000,
            120,
            90,
        ),
    ]
    .into_iter()
    .map(
        |(id, name, about, category, mass_g, ask_cr, bid_cr)| ItemDesign {
            id: ItemDesignId::from(id),
            name: name.to_string(),
            about: about.to_string(),
            category,
            mass_g,
            ask_cr,
            bid_cr,
        },
    )
    .collect()
}
