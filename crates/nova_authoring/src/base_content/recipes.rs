//! Built-in RECIPE content: what a ship crafts from items in its hold.
//!
//! `content gen` serializes this builder into
//! `assets/base/recipes/base.content.ron`. Ship recipes run on every player
//! hull, anywhere; station recipes run docked at a living station that
//! offers Craft, for a fee per batch. The inputs and outputs are the base
//! catalog's (`super::items`); three ship recipes make the consumables
//! (`super::consumables`), and two make the dock parts a lost docking port
//! rebuilds from in the field.
//!
//! Durations, counts and fees are provisional, like the item prices. Each
//! recipe passes the shared rule (`recipe_faults`): its inputs at ask plus
//! the fee are worth at least its output at bid, and its output weighs at
//! most its inputs, so it always fits the hold they leave.

use nova_gameplay::prelude::{
    CraftSiteType, ItemDesignId, ItemRoleType, RecipeDesign, RecipeDesignId,
};

use super::items::{
    ACTUATOR_PACK, ALLOY_PLATE, BOOST_CELL, DECOY_FLARE, DOCKING_COLLAR, REPAIR_KIT, SECTION_PANEL,
};

/// The base game's recipes, in the order the file carries them: the ship
/// recipes, then the station recipes.
pub(crate) fn recipe_catalog() -> Vec<RecipeDesign> {
    use CraftSiteType::{Ship, Station};
    use ItemRoleType::{
        CarbonOre, HullPlate, IronOre, PdcRound, RailSlug, StoneOre, Torpedo, WaterIce,
    };
    let recipe = |id: &str,
                  inputs: &[(&str, u32)],
                  output: (&str, u32),
                  duration_secs: f32,
                  site: CraftSiteType| RecipeDesign {
        id: RecipeDesignId::from(id),
        inputs: inputs
            .iter()
            .map(|(item, count)| (ItemDesignId::from(*item), *count))
            .collect(),
        output: (ItemDesignId::from(output.0), output.1),
        duration_secs,
        site,
    };
    vec![
        // The ammo supply away from a station: 100 rounds per 8 s is one
        // eighth of one PDC's fire rate, so it refills between fights.
        recipe(
            "base/pdc_round",
            &[(IronOre.id(), 1), (StoneOre.id(), 1)],
            (PdcRound.id(), 100),
            8.0,
            Ship,
        ),
        // Plates and panels feed the field repair and field rebuild jobs.
        recipe(
            "base/hull_plate",
            &[(IronOre.id(), 2)],
            (HullPlate.id(), 1),
            8.0,
            Ship,
        ),
        recipe(
            "base/section_panel",
            &[(CarbonOre.id(), 2)],
            (SECTION_PANEL, 1),
            6.0,
            Ship,
        ),
        // The consumables are field supply, so all three craft on the ship.
        recipe(
            "base/repair_kit",
            &[(HullPlate.id(), 2), (SECTION_PANEL, 1)],
            (REPAIR_KIT, 1),
            12.0,
            Ship,
        ),
        recipe(
            "base/boost_cell",
            &[(WaterIce.id(), 1), (CarbonOre.id(), 1)],
            (BOOST_CELL, 1),
            8.0,
            Ship,
        ),
        recipe(
            "base/decoy_flare",
            &[(IronOre.id(), 1), (CarbonOre.id(), 1)],
            (DECOY_FLARE, 1),
            6.0,
            Ship,
        ),
        // A docking port field rebuild takes a collar and an actuator pack;
        // these let a ship make both away from a station, from ore and one
        // carried alloy plate.
        recipe(
            "base/docking_collar",
            &[(ALLOY_PLATE, 1), (IronOre.id(), 5)],
            (DOCKING_COLLAR, 1),
            20.0,
            Ship,
        ),
        recipe(
            "base/actuator_pack",
            &[(IronOre.id(), 4)],
            (ACTUATOR_PACK, 1),
            10.0,
            Ship,
        ),
        recipe(
            "base/alloy_plate",
            &[(IronOre.id(), 2), (CarbonOre.id(), 1)],
            (ALLOY_PLATE, 1),
            10.0,
            Station { fee_cr: 30 },
        ),
        recipe(
            "base/rail_slug",
            &[(IronOre.id(), 2)],
            (RailSlug.id(), 1),
            10.0,
            Station { fee_cr: 10 },
        ),
        // The deliberate, expensive ammo stays station-made.
        recipe(
            "base/torpedo",
            &[(ALLOY_PLATE, 2), (ACTUATOR_PACK, 1), (CarbonOre.id(), 9)],
            (Torpedo.id(), 1),
            30.0,
            Station { fee_cr: 40 },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use nova_gameplay::{prelude::recipe_faults, test_support::test_items};

    use super::*;

    #[test]
    fn base_recipes_have_no_faults() {
        let items = test_items();
        for recipe in recipe_catalog() {
            assert_eq!(
                recipe_faults(&recipe, &items),
                Vec::<String>::new(),
                "{}",
                recipe.id
            );
        }
    }
}
