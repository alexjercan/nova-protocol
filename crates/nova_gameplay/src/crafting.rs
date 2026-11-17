//! Crafting recipes: what a ship makes from items in its hold, where, and
//! the rules a craft job plans each batch by.
//!
//! A [`RecipeDesign`] is authored content keyed by a [`RecipeDesignId`]. It
//! names the items one batch takes, the one stack it makes, the game
//! seconds a batch runs and its site: [`CraftSiteType::Ship`] runs on any
//! player hull anywhere, [`CraftSiteType::Station`] only while the ship is
//! docked at a living station that offers
//! [`StationServiceType::Craft`](crate::markers::StationServiceType::Craft),
//! which takes the recipe's fee per batch. The mod merge publishes the
//! effective recipes as [`GameRecipes`]; [`recipe_faults`] is the rule the
//! merge and content lint share.
//!
//! A craft is a timed job of N batches (`nova_ship`'s `ShipCraftJob`).
//! [`plan_craft_start`] checks a start against the whole job and takes
//! nothing; [`plan_craft_batch`] plans the end of one batch, which takes the
//! inputs and the fee and adds the output. The mass rule of
//! [`recipe_faults`] means the output always fits the hold its inputs leave.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;

use crate::inventory::prelude::{GameItems, ItemDesign, ItemDesignId, ShipInventory};

/// Glob-import surface: `use nova_gameplay::crafting::prelude::*`.
pub mod prelude {
    pub use super::{
        craft_batches_max, plan_craft_batch, plan_craft_start, recipe_faults, recipe_shape_faults,
        recipe_value_faults, CraftBatch, CraftRefusalType, CraftSiteType, GameRecipes,
        RecipeDesign, RecipeDesignId, CRAFT_BATCHES_MAX,
    };
}

/// The most batches one craft job may queue. Bounds the batch count a
/// player types or a Max press picks, so a job's totals stay small.
pub const CRAFT_BATCHES_MAX: u32 = 999;

/// The authored id of a recipe, namespaced like a trade profile:
/// `base/pdc_round`. A running craft job and a save name it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct RecipeDesignId(pub String);

impl RecipeDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RecipeDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for RecipeDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Where a recipe runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CraftSiteType {
    /// On the ship itself: every player hull runs it, anywhere, docked or
    /// not. No fee.
    Ship,
    /// Docked at a living station that offers Craft, which takes `fee_cr`
    /// from the ship per batch.
    Station {
        /// Credits the station takes per finished batch.
        fee_cr: u32,
    },
}

impl CraftSiteType {
    /// The fee one batch pays: zero on the ship.
    pub fn fee_cr(self) -> u32 {
        match self {
            Self::Ship => 0,
            Self::Station { fee_cr } => fee_cr,
        }
    }

    /// The tag a tile and the focus card show: `SHIP` or `STATION`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Ship => "SHIP",
            Self::Station { .. } => "STATION",
        }
    }
}

/// One authored recipe. Every field is required.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RecipeDesign {
    /// Its id.
    pub id: RecipeDesignId,
    /// What one batch takes from the hold, as `(item, count)` pairs in
    /// authored order: at least one, each count above zero, no item twice.
    pub inputs: Vec<(ItemDesignId, u32)>,
    /// What one batch adds to the hold: an item that is not an input, and a
    /// count above zero.
    pub output: (ItemDesignId, u32),
    /// Game seconds one batch runs, finite and above zero.
    pub duration_secs: f32,
    /// Where it runs.
    pub site: CraftSiteType,
}

impl RecipeDesign {
    /// The output item.
    pub fn output_item(&self) -> &ItemDesignId {
        &self.output.0
    }

    /// Units of the output one batch makes.
    pub fn output_count(&self) -> u32 {
        self.output.1
    }
}

/// The merged recipes, by id. Inserted by the mod merge.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameRecipes(pub BTreeMap<RecipeDesignId, RecipeDesign>);

impl GameRecipes {
    /// The catalog of `recipes`; a later recipe with an id replaces an
    /// earlier one.
    pub fn new(recipes: impl IntoIterator<Item = RecipeDesign>) -> Self {
        Self(
            recipes
                .into_iter()
                .map(|recipe| (recipe.id.clone(), recipe))
                .collect(),
        )
    }

    /// The recipe `id`, if the catalog holds it.
    pub fn get(&self, id: &RecipeDesignId) -> Option<&RecipeDesign> {
        self.0.get(id)
    }

    /// The recipe `id`, for an id a save check or the pane already resolved.
    ///
    /// # Panics
    ///
    /// When no recipe has the id: a running job named a recipe the loaded
    /// content lacks, which the save check refuses first.
    pub fn design(&self, id: &RecipeDesignId) -> &RecipeDesign {
        self.get(id)
            .unwrap_or_else(|| panic!("recipe '{id}' is not in the loaded recipe catalog"))
    }

    /// Every recipe, in id order.
    pub fn iter(&self) -> impl Iterator<Item = &RecipeDesign> {
        self.0.values()
    }
}

/// Every fault of `recipe` against `items`: the
/// [`recipe_shape_faults`], then the [`recipe_value_faults`].
pub fn recipe_faults(recipe: &RecipeDesign, items: &GameItems) -> Vec<String> {
    let mut faults = recipe_shape_faults(recipe, items);
    faults.extend(recipe_value_faults(recipe, items));
    faults
}

/// The faults of `recipe` that the recipe alone causes, keyed in the
/// message by the recipe id: an empty id, no inputs, an input count of
/// zero, an input listed twice, an input that is the output, an unknown
/// input or output item, an output count of zero, and a duration that is
/// not finite or not above zero. The pack that defines the recipe owns them.
pub fn recipe_shape_faults(recipe: &RecipeDesign, items: &GameItems) -> Vec<String> {
    let id = &recipe.id;
    let mut faults = Vec::new();
    if id.0.trim().is_empty() {
        faults.push("a recipe has an empty id".to_string());
    }
    if recipe.inputs.is_empty() {
        faults.push(format!("the recipe '{id}' has no inputs"));
    }
    let (output, output_count) = (&recipe.output.0, recipe.output.1);
    let mut seen = Vec::new();
    for (input, count) in &recipe.inputs {
        if *count == 0 {
            faults.push(format!("the recipe '{id}' takes 0 of '{input}'"));
        }
        if seen.contains(&input) {
            faults.push(format!("the recipe '{id}' lists the input '{input}' twice"));
        }
        seen.push(input);
        if input == output {
            faults.push(format!("the recipe '{id}' takes its own output '{input}'"));
        }
        if items.get(input).is_none() {
            faults.push(format!(
                "the recipe '{id}' takes '{input}', which is no item"
            ));
        }
    }
    if output_count == 0 {
        faults.push(format!("the recipe '{id}' makes 0 of '{output}'"));
    }
    if items.get(output).is_none() {
        faults.push(format!(
            "the recipe '{id}' makes '{output}', which is no item"
        ));
    }
    if !recipe.duration_secs.is_finite() || recipe.duration_secs <= 0.0 {
        faults.push(format!(
            "the recipe '{id}' runs {} s a batch; it must be finite and above 0",
            recipe.duration_secs
        ));
    }
    faults
}

/// The faults of `recipe` that the prices and masses of its items cause:
/// a batch whose inputs at ask plus the fee are worth less than its output
/// at bid (lot-exact), and an output heavier than the inputs. Empty when an
/// input or the output is no item in `items`. A pack that last set one of
/// those items can cause them as well as the pack that defines the recipe.
pub fn recipe_value_faults(recipe: &RecipeDesign, items: &GameItems) -> Vec<String> {
    let id = &recipe.id;
    let (output, output_count) = (&recipe.output.0, recipe.output.1);
    let Some(output_design) = items.get(output) else {
        return Vec::new();
    };
    let Some(inputs) = recipe
        .inputs
        .iter()
        .map(|(input, count)| items.get(input).map(|design| (design, *count)))
        .collect::<Option<Vec<_>>>()
    else {
        return Vec::new();
    };
    let mut faults = Vec::new();
    if let Some(message) = price_fault(
        id,
        output_design,
        output_count,
        recipe.site.fee_cr(),
        &inputs,
    ) {
        faults.push(message);
    }
    let mass_in: u64 = inputs
        .iter()
        .map(|(design, count)| design.stack_mass_g(*count))
        .sum();
    let mass_out = output_design.stack_mass_g(output_count);
    if mass_out > mass_in {
        faults.push(format!(
            "the recipe '{id}' makes {mass_out} g from {mass_in} g of inputs; an output must not \
             weigh more than its inputs"
        ));
    }
    faults
}

/// The fault of a recipe that pays: its `inputs` (each with its count) at
/// their reference ask plus `fee_cr` are worth less than `output_count` of
/// `output` at its reference bid, so buying the inputs, crafting and
/// selling the output would gain credits. `None` when a batch costs at
/// least what it sells for.
///
/// Compared exactly over the price lots, since one item of a lot of 100 is
/// worth a fraction of a credit; the message rounds the two figures.
fn price_fault(
    id: &RecipeDesignId,
    output: &ItemDesign,
    output_count: u32,
    fee_cr: u32,
    inputs: &[(&ItemDesign, u32)],
) -> Option<String> {
    fn gcd(a: u128, b: u128) -> u128 {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }
    // A price_lot of 0 is the item's own fault; the rule cannot price it.
    if output.price_lot == 0 || inputs.iter().any(|(input, _)| input.price_lot == 0) {
        return None;
    }
    // Every u32 lot divides `scale`, so each side below is a whole number
    // of credits times `scale`.
    let scale = inputs
        .iter()
        .map(|(input, _)| u128::from(input.price_lot))
        .fold(u128::from(output.price_lot), |scale, lot| {
            scale / gcd(scale, lot) * lot
        });
    let cost = inputs
        .iter()
        .map(|(input, count)| {
            u128::from(input.ask_cr) * u128::from(*count) * (scale / u128::from(input.price_lot))
        })
        .sum::<u128>()
        + u128::from(fee_cr) * scale;
    let earns = u128::from(output.bid_cr)
        * u128::from(output_count)
        * (scale / u128::from(output.price_lot));
    (cost < earns).then(|| {
        format!(
            "the recipe '{id}' costs {} cr (its inputs at ask plus the {fee_cr} cr fee) and its \
             output sells for {} cr at bid; a recipe must not cost less than its output sells for",
            cost as f64 / scale as f64,
            earns as f64 / scale as f64,
        )
    })
}

/// Why a craft does not start, or why a running one stops, in check order.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CraftRefusalType {
    /// The ship already runs a craft job.
    Running,
    /// A batch count of zero, or above [`CRAFT_BATCHES_MAX`].
    BadBatches,
    /// A station recipe, and the ship is not docked at a living station that
    /// offers Craft.
    NotAtCraftStation,
    /// The hold carries fewer of an input than the batches take.
    ShortInput {
        /// The first input, in authored order, the hold is short of.
        item: ItemDesignId,
        /// How many of it the batches take.
        need: u64,
        /// What the hold carries.
        have: u32,
    },
    /// The ship holds fewer credits than the fee of the batches.
    ShortCredits {
        /// The fee of the batches.
        need: u64,
        /// What the ship holds.
        have: u32,
    },
    /// The station's balance after the fee would not fit in a `u32`.
    CreditOverflow,
}

/// The first input of `recipe` the hold `hold` is short of for `batches`
/// batches, in authored order.
fn short_input(
    recipe: &RecipeDesign,
    batches: u32,
    hold: &ShipInventory,
) -> Option<CraftRefusalType> {
    recipe.inputs.iter().find_map(|(item, count)| {
        let need = u64::from(*count) * u64::from(batches);
        let have = hold.count(item);
        (need > u64::from(have)).then(|| CraftRefusalType::ShortInput {
            item: item.clone(),
            need,
            have,
        })
    })
}

/// Check a start of `batches` batches of `recipe`. `running` is whether the
/// ship already runs a craft job, `at_craft_station` whether it is docked at
/// a living station that offers Craft, `hold` the hold less what running
/// repair jobs hold back and `own_cr` its balance. Checks run in
/// [`CraftRefusalType`] order; a start takes nothing.
pub fn plan_craft_start(
    recipe: &RecipeDesign,
    batches: u32,
    running: bool,
    at_craft_station: bool,
    hold: &ShipInventory,
    own_cr: u32,
) -> Result<(), CraftRefusalType> {
    if running {
        return Err(CraftRefusalType::Running);
    }
    if batches == 0 || batches > CRAFT_BATCHES_MAX {
        return Err(CraftRefusalType::BadBatches);
    }
    if matches!(recipe.site, CraftSiteType::Station { .. }) && !at_craft_station {
        return Err(CraftRefusalType::NotAtCraftStation);
    }
    if let Some(short) = short_input(recipe, batches, hold) {
        return Err(short);
    }
    let fee = u64::from(recipe.site.fee_cr()) * u64::from(batches);
    if fee > u64::from(own_cr) {
        return Err(CraftRefusalType::ShortCredits {
            need: fee,
            have: own_cr,
        });
    }
    Ok(())
}

/// The whole batches of `recipe` the hold `hold` and the balance `own_cr`
/// cover, at most [`CRAFT_BATCHES_MAX`]. Zero when one input or the fee is
/// short.
pub fn craft_batches_max(recipe: &RecipeDesign, hold: &ShipInventory, own_cr: u32) -> u32 {
    let by_inputs = recipe
        .inputs
        .iter()
        .map(|(item, count)| hold.count(item) / (*count).max(1))
        .min()
        .unwrap_or(0);
    let by_fee = match recipe.site.fee_cr() {
        0 => u32::MAX,
        fee => own_cr / fee,
    };
    by_inputs.min(by_fee).min(CRAFT_BATCHES_MAX)
}

/// One finished batch: what leaves the hold, what enters it and where the
/// fee goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CraftBatch {
    /// Inputs to remove from the hold, by item.
    pub inputs: BTreeMap<ItemDesignId, u32>,
    /// The item to add.
    pub output: ItemDesignId,
    /// Units of it to add, above zero.
    pub count: u32,
    /// Credits to move from the ship to the station; zero on the ship.
    pub fee_cr: u32,
    /// The ship's balance after the fee.
    pub own_cr: u32,
    /// The station's balance after the fee; `None` for a ship recipe.
    pub station_cr: Option<u32>,
}

/// Plan the end of one batch of `recipe`. `station_cr` is the balance of
/// the living Craft station the ship is docked at, `None` when it is not
/// docked at one. Re-checks the site, the inputs in `hold` and the fee
/// against `own_cr`, in [`CraftRefusalType`] order. A refusal stops the job
/// and takes nothing.
pub fn plan_craft_batch(
    recipe: &RecipeDesign,
    station_cr: Option<u32>,
    hold: &ShipInventory,
    own_cr: u32,
) -> Result<CraftBatch, CraftRefusalType> {
    let station = match recipe.site {
        CraftSiteType::Ship => None,
        CraftSiteType::Station { .. } => {
            Some(station_cr.ok_or(CraftRefusalType::NotAtCraftStation)?)
        }
    };
    if let Some(short) = short_input(recipe, 1, hold) {
        return Err(short);
    }
    let fee_cr = recipe.site.fee_cr();
    if fee_cr > own_cr {
        return Err(CraftRefusalType::ShortCredits {
            need: u64::from(fee_cr),
            have: own_cr,
        });
    }
    let station_cr = station
        .map(|balance| {
            balance
                .checked_add(fee_cr)
                .ok_or(CraftRefusalType::CreditOverflow)
        })
        .transpose()?;
    let mut inputs = BTreeMap::new();
    for (item, count) in &recipe.inputs {
        *inputs.entry(item.clone()).or_insert(0) += count;
    }
    Ok(CraftBatch {
        inputs,
        output: recipe.output.0.clone(),
        count: recipe.output.1,
        fee_cr,
        own_cr: own_cr - fee_cr,
        station_cr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inventory::prelude::ItemRoleType, test_support::test_items};

    fn recipe(
        id: &str,
        inputs: &[(&str, u32)],
        output: (&str, u32),
        site: CraftSiteType,
    ) -> RecipeDesign {
        RecipeDesign {
            id: RecipeDesignId::from(id),
            inputs: inputs
                .iter()
                .map(|(item, count)| (ItemDesignId::from(*item), *count))
                .collect(),
            output: (ItemDesignId::from(output.0), output.1),
            duration_secs: 8.0,
            site,
        }
    }

    /// The base PDC recipe: one iron and one stone ore to 100 rounds.
    fn pdc() -> RecipeDesign {
        recipe(
            "base/pdc_round",
            &[("IronOre", 1), ("StoneOre", 1)],
            ("PdcRound", 100),
            CraftSiteType::Ship,
        )
    }

    /// The base alloy plate: two iron and one carbon ore and 30 cr.
    fn alloy() -> RecipeDesign {
        recipe(
            "base/alloy_plate",
            &[("IronOre", 2), ("CarbonOre", 1)],
            ("AlloyPlate", 1),
            CraftSiteType::Station { fee_cr: 30 },
        )
    }

    fn hold(stacks: &[(&str, u32)]) -> ShipInventory {
        ShipInventory::new(
            &test_items(),
            1_000_000,
            stacks
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|(item, count)| (ItemDesignId::from(*item), *count)),
        )
    }

    #[test]
    fn the_base_recipes_pass_the_rules() {
        let items = test_items();
        assert!(recipe_faults(&pdc(), &items).is_empty());
        assert!(recipe_faults(&alloy(), &items).is_empty());
    }

    #[test]
    fn recipe_faults_name_every_broken_rule() {
        let items = test_items();
        let mut bad = recipe(
            "",
            &[
                ("IronOre", 0),
                ("IronOre", 1),
                ("Unobtainium", 1),
                ("Rations", 1),
            ],
            ("Rations", 0),
            CraftSiteType::Ship,
        );
        bad.duration_secs = f32::NAN;
        assert_eq!(
            recipe_faults(&bad, &items),
            [
                "a recipe has an empty id",
                "the recipe '' takes 0 of 'IronOre'",
                "the recipe '' lists the input 'IronOre' twice",
                "the recipe '' takes 'Unobtainium', which is no item",
                "the recipe '' takes its own output 'Rations'",
                "the recipe '' makes 0 of 'Rations'",
                "the recipe '' runs NaN s a batch; it must be finite and above 0",
            ]
        );
        let none = recipe("x", &[], ("Nothing", 1), CraftSiteType::Ship);
        assert_eq!(
            recipe_faults(
                &RecipeDesign {
                    duration_secs: 0.0,
                    ..none
                },
                &items
            ),
            [
                "the recipe 'x' has no inputs",
                "the recipe 'x' makes 'Nothing', which is no item",
                "the recipe 'x' runs 0 s a batch; it must be finite and above 0",
            ]
        );
    }

    /// Inputs at ask plus the fee must be worth at least the output at bid:
    /// the alloy plate costs 70 cr and sells for 68 at 30 cr, and a 27 cr
    /// fee would pay.
    #[test]
    fn a_recipe_that_costs_less_than_its_output_sells_for_is_refused() {
        let items = test_items();
        let cheap = RecipeDesign {
            site: CraftSiteType::Station { fee_cr: 27 },
            ..alloy()
        };
        assert_eq!(
            recipe_faults(&cheap, &items),
            [
                "the recipe 'base/alloy_plate' costs 67 cr (its inputs at ask plus the 27 cr fee) \
              and its output sells for 68 cr at bid; a recipe must not cost less than its output \
              sells for"
            ]
        );
        let even = RecipeDesign {
            site: CraftSiteType::Station { fee_cr: 28 },
            ..alloy()
        };
        assert!(recipe_faults(&even, &items).is_empty());
    }

    /// One PDC round is a share of a lot of 100: 300 rounds at 7 cr a
    /// hundred sell for 21 cr, more than 20 cr of ore, and 280 for 19.6.
    #[test]
    fn the_price_rule_counts_a_share_of_a_price_lot() {
        let items = test_items();
        let rounds = |count| RecipeDesign {
            output: (ItemRoleType::PdcRound.into(), count),
            ..pdc()
        };
        assert!(recipe_faults(&rounds(280), &items)
            .iter()
            .all(|fault| !fault.contains("sells for")));
        assert!(recipe_faults(&rounds(300), &items)
            .iter()
            .any(|fault| fault.contains("costs 20 cr") && fault.contains("sells for 21 cr")));
    }

    /// An output never weighs more than its inputs, so it always fits the
    /// hold they leave: 20 kg of ore makes at most 100 rounds of 0.2 kg.
    #[test]
    fn a_recipe_whose_output_outweighs_its_inputs_is_refused() {
        let items = test_items();
        let heavy = RecipeDesign {
            output: (ItemRoleType::PdcRound.into(), 101),
            ..pdc()
        };
        assert!(recipe_faults(&heavy, &items).contains(
            &"the recipe 'base/pdc_round' makes 20200 g from 20000 g of inputs; an output must \
              not weigh more than its inputs"
                .to_string()
        ));
    }

    #[test]
    fn a_craft_start_refuses_in_order_and_takes_nothing() {
        use CraftRefusalType::*;
        let ore = hold(&[("IronOre", 4), ("StoneOre", 2), ("CarbonOre", 1)]);
        assert_eq!(
            plan_craft_start(&pdc(), 1, true, true, &ore, 0),
            Err(Running)
        );
        assert_eq!(
            plan_craft_start(&pdc(), 0, false, true, &ore, 0),
            Err(BadBatches)
        );
        assert_eq!(
            plan_craft_start(&pdc(), CRAFT_BATCHES_MAX + 1, false, true, &ore, 0),
            Err(BadBatches)
        );
        assert_eq!(
            plan_craft_start(&alloy(), 1, false, false, &ore, 1_000),
            Err(NotAtCraftStation)
        );
        assert_eq!(
            plan_craft_start(&pdc(), 3, false, false, &ore, 0),
            Err(ShortInput {
                item: ItemDesignId::from("StoneOre"),
                need: 3,
                have: 2
            })
        );
        assert_eq!(plan_craft_start(&pdc(), 2, false, false, &ore, 0), Ok(()));
        assert_eq!(
            plan_craft_start(&alloy(), 2, false, true, &ore, 1_000),
            Err(ShortInput {
                item: ItemDesignId::from("CarbonOre"),
                need: 2,
                have: 1
            })
        );
        assert_eq!(
            plan_craft_start(&alloy(), 1, false, true, &ore, 29),
            Err(ShortCredits { need: 30, have: 29 })
        );
        assert_eq!(plan_craft_start(&alloy(), 1, false, true, &ore, 30), Ok(()));
    }

    #[test]
    fn the_max_batches_are_what_the_hold_and_the_balance_cover() {
        let ore = hold(&[("IronOre", 7), ("StoneOre", 5), ("CarbonOre", 9)]);
        assert_eq!(craft_batches_max(&pdc(), &ore, 0), 5);
        assert_eq!(craft_batches_max(&alloy(), &ore, 1_000), 3);
        assert_eq!(craft_batches_max(&alloy(), &ore, 59), 1);
        assert_eq!(craft_batches_max(&pdc(), &hold(&[("IronOre", 7)]), 0), 0);
    }

    /// A batch end takes one batch of inputs and the fee and makes the
    /// output; a station recipe needs the station, and its balance takes
    /// the fee.
    #[test]
    fn a_batch_takes_its_inputs_and_fee_and_refuses_in_order() {
        use CraftRefusalType::*;
        let ore = hold(&[("IronOre", 2), ("CarbonOre", 1), ("StoneOre", 1)]);
        assert_eq!(
            plan_craft_batch(&pdc(), None, &ore, 5),
            Ok(CraftBatch {
                inputs: BTreeMap::from([
                    (ItemDesignId::from("IronOre"), 1),
                    (ItemDesignId::from("StoneOre"), 1),
                ]),
                output: ItemRoleType::PdcRound.into(),
                count: 100,
                fee_cr: 0,
                own_cr: 5,
                station_cr: None,
            })
        );
        assert_eq!(
            plan_craft_batch(&alloy(), None, &ore, 100),
            Err(NotAtCraftStation)
        );
        assert_eq!(
            plan_craft_batch(&alloy(), Some(10), &hold(&[("IronOre", 2)]), 100),
            Err(ShortInput {
                item: ItemDesignId::from("CarbonOre"),
                need: 1,
                have: 0
            })
        );
        assert_eq!(
            plan_craft_batch(&alloy(), Some(10), &ore, 29),
            Err(ShortCredits { need: 30, have: 29 })
        );
        assert_eq!(
            plan_craft_batch(&alloy(), Some(u32::MAX), &ore, 30),
            Err(CreditOverflow)
        );
        let batch = plan_craft_batch(&alloy(), Some(10), &ore, 100).unwrap();
        assert_eq!(
            (batch.fee_cr, batch.own_cr, batch.station_cr),
            (30, 70, Some(40))
        );
    }
}
