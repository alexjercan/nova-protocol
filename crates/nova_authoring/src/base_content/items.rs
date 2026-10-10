//! Built-in ITEM content: every item the base game's holds, trades, reloads,
//! repairs and mining use.
//!
//! An item is content like a section is content, so this file is a builder -
//! `content gen` serializes it into `assets/base/items/base.content.ron`. A
//! mod adds items under its own ids, and a mod that depends on another pack
//! may replace that pack's whole item.
//!
//! The ids of [`ITEM_ROLES`](nova_gameplay::prelude::ITEM_ROLES) are the ones
//! the game itself uses - weapons reload from them, repair spends one, mining
//! yields them, generated sector ship holds draw them - so the base game must
//! define each under its role's category. A dependent mod may replace one of
//! them whole, with new mass and prices, under the same category.
//! Prices are provisional values, not a balance decision; each bid is below
//! its ask, so buying and selling back loses credits.

use nova_gameplay::prelude::{
    ItemCategoryType, ItemDesign, ItemDesignId, ITEM_CARBON_ORE, ITEM_HULL_PLATE, ITEM_IRON_ORE,
    ITEM_PDC_ROUND, ITEM_RAIL_SLUG, ITEM_RATIONS, ITEM_SALVAGED_PARTS, ITEM_STONE_ORE,
    ITEM_TORPEDO, ITEM_WATER_ICE,
};

/// The base game's items, in the order the file carries them.
pub(crate) fn item_catalog() -> Vec<ItemDesign> {
    use ItemCategoryType::{Ammo, Food, Parts, Raw, Repair};
    [
        (
            ITEM_HULL_PLATE,
            "Hull plate",
            "Structural plating for hull sections.",
            Repair,
            10_000,
            40,
            30,
        ),
        (
            ITEM_PDC_ROUND,
            "PDC round",
            "Point-defense round. Kinetic and Pierce mounts reload from it.",
            Ammo,
            200,
            4,
            3,
        ),
        (
            ITEM_RAIL_SLUG,
            "Rail slug",
            "Railgun slug. Railgun mounts reload from it.",
            Ammo,
            20_000,
            40,
            30,
        ),
        (
            ITEM_TORPEDO,
            "Torpedo",
            "Torpedo. Every torpedo bay reloads from it.",
            Ammo,
            150_000,
            400,
            300,
        ),
        (
            ITEM_STONE_ORE,
            "Stone ore",
            "Silicate rock, mined from rock asteroids.",
            Raw,
            10_000,
            4,
            3,
        ),
        (
            ITEM_IRON_ORE,
            "Iron ore",
            "Metal-rich ore, mined from metal asteroids.",
            Raw,
            10_000,
            16,
            12,
        ),
        (
            ITEM_WATER_ICE,
            "Water ice",
            "Frozen volatiles, mined from ice asteroids.",
            Raw,
            10_000,
            12,
            9,
        ),
        (
            ITEM_CARBON_ORE,
            "Carbon ore",
            "Carbon-rich ore, mined from carbon asteroids.",
            Raw,
            10_000,
            8,
            6,
        ),
        (
            ITEM_RATIONS,
            "Rations",
            "Packed crew food. Traders buy and sell it.",
            Food,
            2_000,
            8,
            6,
        ),
        (
            ITEM_SALVAGED_PARTS,
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
