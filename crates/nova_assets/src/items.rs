//! The item catalog rule: which packs may define which item, and what a valid
//! item is.
//!
//! An item id may be defined by several packs only along a dependency chain:
//! a pack that depends on another, directly or through others, may replace
//! that pack's whole item, and the merge order puts it last. Two packs that
//! define one id with no such chain between them, or with a chain both ways
//! (a dependency cycle), have no order the rule can trust, so BOTH are refused,
//! whatever order they load in. The merge then refuses every pack that
//! depends on a refused one. The base game is a dependency of every other pack.
//!
//! The runtime merge (`register_bundles`) and the `content lint` walk both
//! call [`item_pack_faults`], so a pack lint passes is a pack the game loads.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use nova_gameplay::prelude::{ItemDesign, ItemRoleType, CARGO_CANISTER_MAX_MASS_G};
use nova_mod_format::deps::{transitive_deps, DepGraph};
use nova_modding::prelude::BASE_MOD_ID;

/// Glob-import surface: `use nova_assets::items::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{item_pack_faults, ItemPack};
}

/// One pack's item definitions, as the rule reads them.
#[derive(Clone, Debug)]
pub struct ItemPack<'a> {
    /// The mod id.
    pub id: &'a str,
    /// The mod ids it declares as dependencies. The base game is implicit.
    pub dependencies: &'a [String],
    /// Every item it defines, in authored order, repeats included.
    pub items: Vec<&'a ItemDesign>,
}

/// Every fault the item rule finds, by the pack it refuses, in pack id order.
/// Empty when every pack may load. Independent of the order of `packs`.
///
/// A pack is refused for an invalid item it defines, an item id it defines
/// twice, an item id it shares with a pack it has no one-way dependency
/// chain with, and a dependency chain too deep to walk. A refine recipe is
/// judged against the catalog the merge keeps: its pack is refused for an
/// input no pack defines, and the pack that last set a price of the recipe
/// is refused when the inputs at ask plus the fee are worth less than the
/// output at bid. The base game is also refused when it lacks one of
/// [`ItemRoleType::ALL`].
pub fn item_pack_faults(packs: &[ItemPack<'_>]) -> BTreeMap<String, Vec<String>> {
    let mut faults: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut fault = |pack: &str, message: String| {
        faults.entry(pack.to_string()).or_default().push(message);
    };

    let graph: DepGraph = packs
        .iter()
        .map(|pack| (pack.id.to_string(), pack.dependencies.to_vec()))
        .collect();
    let mut reaches: HashMap<&str, HashSet<String>> = HashMap::new();
    for pack in packs {
        match transitive_deps(&graph, pack.id) {
            Ok(deps) => {
                let mut deps: HashSet<String> = deps.into_iter().collect();
                if pack.id != BASE_MOD_ID {
                    deps.insert(BASE_MOD_ID.to_string());
                }
                reaches.insert(pack.id, deps);
            }
            Err(error) => fault(pack.id, error.to_string()),
        }
    }

    let mut definers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for pack in packs {
        let mut seen = HashSet::new();
        for item in &pack.items {
            for message in item_faults(item) {
                fault(pack.id, message);
            }
            let id = item.id.as_str();
            if !seen.insert(id) {
                fault(pack.id, format!("item '{id}' is defined more than once"));
            }
            definers.entry(id).or_default().insert(pack.id);
        }
    }

    // Only a walked pack can be judged; a pack whose walk failed is refused
    // above already.
    let depends = |a: &str, b: &str| reaches.get(a).is_some_and(|deps| deps.contains(b));
    for (id, owners) in &definers {
        let owners: Vec<&str> = owners.iter().copied().collect();
        for (at, a) in owners.iter().enumerate() {
            for b in &owners[at + 1..] {
                let reason = match (depends(a, b), depends(b, a)) {
                    (true, false) | (false, true) => continue,
                    (true, true) => "they depend on each other",
                    (false, false) => "neither depends on the other",
                };
                let message = format!("item '{id}' is defined by '{a}' and '{b}', and {reason}");
                fault(a, message.clone());
                fault(b, message);
            }
        }
    }

    // The catalog the merge keeps: the one definition of each id, or the
    // one whose pack depends on every other pack that defines it. An id two
    // packs fight over has no definition here and is refused above.
    let effective: BTreeMap<&str, (&str, &ItemDesign)> = definers
        .iter()
        .filter_map(|(id, owners)| {
            let owner = owners.iter().copied().find(|owner| {
                owners.iter().all(|other| {
                    other == owner || (depends(owner, other) && !depends(other, owner))
                })
            })?;
            let pack = packs.iter().find(|pack| pack.id == owner)?;
            let item = pack
                .items
                .iter()
                .rev()
                .find(|item| item.id.as_str() == *id)?;
            Some((*id, (owner, *item)))
        })
        .collect();
    for (id, (owner, item)) in &effective {
        let Some(recipe) = &item.refine else {
            continue;
        };
        // The packs whose definitions set the two sides of the price rule.
        let mut priced_by = BTreeSet::from([*owner]);
        let mut inputs = Vec::new();
        for (input, count) in &recipe.inputs {
            match effective.get(input.as_str()) {
                Some((input_owner, design)) => {
                    priced_by.insert(*input_owner);
                    inputs.push((*design, *count));
                }
                None if definers.contains_key(input.as_str()) => {}
                None => fault(
                    owner,
                    format!("item '{id}' refines from unknown item '{input}'"),
                ),
            }
        }
        if inputs.len() < recipe.inputs.len() {
            continue;
        }
        if let Some(message) = refine_price_fault(item, recipe.fee_cr, &inputs) {
            // A pack that replaced an input or the output last is the one
            // that broke the rule; a pack it depends on stays loadable.
            for pack in &priced_by {
                if !priced_by.iter().any(|later| depends(later, pack)) {
                    fault(pack, message.clone());
                }
            }
        }
    }

    if let Some(base) = packs.iter().find(|pack| pack.id == BASE_MOD_ID) {
        for role in ItemRoleType::ALL {
            if !base.items.iter().any(|item| item.id.as_str() == role.id()) {
                fault(BASE_MOD_ID, format!("item '{}' is not defined", role.id()));
            }
        }
    }
    faults
}

/// The fault of a refine recipe that pays: `inputs` (each with its count)
/// at their reference ask plus `fee_cr` are worth less than one `output` at
/// its reference bid, so buying the inputs from a station, refining them and
/// selling the output would gain credits out of the station's stock. `None`
/// when a batch costs at least what it sells for.
///
/// Compared exactly over the price lots, since one item of a lot of 100 is
/// worth a fraction of a credit; the message rounds the two figures.
fn refine_price_fault(
    output: &ItemDesign,
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
    // A price_lot of 0 is its own fault; the rule cannot price the item.
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
    let earns = u128::from(output.bid_cr) * (scale / u128::from(output.price_lot));
    (cost < earns).then(|| {
        format!(
            "item '{}' refines for {} cr (its inputs at ask plus the {fee_cr} cr fee) and sells \
             for {} cr at bid; a refine must not cost less than its output sells for",
            output.id,
            cost as f64 / scale as f64,
            earns as f64 / scale as f64,
        )
    })
}

/// What is wrong with one item definition on its own.
fn item_faults(item: &ItemDesign) -> Vec<String> {
    let id = item.id.as_str();
    let mut faults = Vec::new();
    if id.trim().is_empty() {
        faults.push("an item has an empty id".to_string());
    }
    if item.name.trim().is_empty() {
        faults.push(format!("item '{id}' has an empty name"));
    }
    if item.about.trim().is_empty() {
        faults.push(format!("item '{id}' has an empty about"));
    }
    if item.mass_g == 0 {
        faults.push(format!("item '{id}' has a mass_g of 0"));
    }
    if item.price_lot == 0 {
        faults.push(format!("item '{id}' has a price_lot of 0"));
    }
    if item.bid_cr > item.ask_cr {
        faults.push(format!(
            "item '{id}' has a bid_cr of {} above its ask_cr of {}",
            item.bid_cr, item.ask_cr
        ));
    }
    if let Some(role) = ItemRoleType::ALL.iter().find(|role| role.id() == id) {
        if item.category != role.category() {
            faults.push(format!(
                "item '{id}' is filed under {:?}; the game uses it as {:?}",
                item.category,
                role.category()
            ));
        }
    }
    if let Some(recipe) = &item.refine {
        if recipe.inputs.is_empty() {
            faults.push(format!("item '{id}' has a refine recipe with no inputs"));
        }
        let mut seen = HashSet::new();
        for (input, count) in &recipe.inputs {
            if input.as_str() == id {
                faults.push(format!("item '{id}' refines from itself"));
            }
            if *count == 0 {
                faults.push(format!("item '{id}' refines from 0 of '{input}'"));
            }
            if !seen.insert(input.as_str()) {
                faults.push(format!(
                    "item '{id}' lists refine input '{input}' more than once"
                ));
            }
        }
    }
    // Mining, a jettison and a destroyed hold all pack items into canisters.
    if item.mass_g > CARGO_CANISTER_MAX_MASS_G {
        faults.push(format!(
            "item '{id}' has a mass_g of {}; an item must fit one {CARGO_CANISTER_MAX_MASS_G} g \
             canister",
            item.mass_g
        ));
    }
    faults
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{ItemCategoryType, ItemDesignId, ItemRefineRecipe};

    use super::*;

    fn item(id: &str, mass_g: u32) -> ItemDesign {
        let category = ItemRoleType::ALL
            .iter()
            .find(|role| role.id() == id)
            .map_or(ItemCategoryType::Parts, |role| role.category());
        ItemDesign {
            id: ItemDesignId::from(id),
            name: id.to_string(),
            about: format!("{id} for tests."),
            category,
            mass_g,
            price_lot: 1,
            ask_cr: 10,
            bid_cr: 5,
            refine: None,
        }
    }

    /// An item sold at `bid_cr` that a station refines from `inputs` for
    /// `fee_cr`.
    fn refined(id: &str, bid_cr: u32, inputs: &[(&str, u32)], fee_cr: u32) -> ItemDesign {
        ItemDesign {
            ask_cr: bid_cr,
            bid_cr,
            refine: Some(ItemRefineRecipe {
                inputs: inputs
                    .iter()
                    .map(|(input, count)| (ItemDesignId::from(*input), *count))
                    .collect(),
                fee_cr,
            }),
            ..item(id, 1_000)
        }
    }

    /// The faults of the base game with `extra` added to its role items.
    fn base_faults_with(extra: ItemDesign) -> Vec<String> {
        let mut base = base_items();
        base.push(extra);
        let none: [String; 0] = [];
        item_pack_faults(&[pack(BASE_MOD_ID, &none, &base)])
            .remove(BASE_MOD_ID)
            .unwrap_or_default()
    }

    fn base_items() -> Vec<ItemDesign> {
        ItemRoleType::ALL
            .iter()
            .map(|role| item(role.id(), 1_000))
            .collect()
    }

    fn pack<'a>(id: &'a str, deps: &'a [String], items: &'a [ItemDesign]) -> ItemPack<'a> {
        ItemPack {
            id,
            dependencies: deps,
            items: items.iter().collect(),
        }
    }

    /// Two unrelated packs that define one id are both refused, and the
    /// verdict is the same in either load order.
    #[test]
    fn unrelated_packs_defining_one_item_are_both_refused_in_any_order() {
        let base = base_items();
        let core = [item("deep-salvage/reactor_core", 80_000)];
        let rival = [item("deep-salvage/reactor_core", 1_000)];
        let none: [String; 0] = [];
        let forward = item_pack_faults(&[
            pack(BASE_MOD_ID, &none, &base),
            pack("deep-salvage", &none, &core),
            pack("ore-rush", &none, &rival),
        ]);
        let backward = item_pack_faults(&[
            pack("ore-rush", &none, &rival),
            pack("deep-salvage", &none, &core),
            pack(BASE_MOD_ID, &none, &base),
        ]);
        assert_eq!(forward, backward);
        let message = "item 'deep-salvage/reactor_core' is defined by 'deep-salvage' and \
                       'ore-rush', and neither depends on the other";
        assert_eq!(
            forward,
            BTreeMap::from([
                ("deep-salvage".to_string(), vec![message.to_string()]),
                ("ore-rush".to_string(), vec![message.to_string()]),
            ])
        );
    }

    /// A pack may replace an item of a pack it depends on, the base game
    /// included, with its own mass and prices but a role item's category; a
    /// pack the item's owner depends on back may not.
    #[test]
    fn a_dependent_pack_may_replace_an_item_and_a_cycle_may_not() {
        let base = base_items();
        let mut rations = item(ItemRoleType::Rations.id(), 3_000);
        rations.ask_cr = 20;
        rations.bid_cr = 15;
        let plate = [
            item(ItemRoleType::HullPlate.id(), 12_000),
            item(ItemRoleType::SalvagedParts.id(), 30_000),
            rations.clone(),
        ];
        let core = [item("deep-salvage/reactor_core", 80_000)];
        let none: [String; 0] = [];
        let on_salvage = ["deep-salvage".to_string()];
        assert!(item_pack_faults(&[
            pack(BASE_MOD_ID, &none, &base),
            pack("heavy-plates", &none, &plate),
            pack("deep-salvage", &none, &core),
            pack("salvage-plus", &on_salvage, &core),
        ])
        .is_empty());

        rations.category = ItemCategoryType::Parts;
        let refiled = [rations];
        assert_eq!(
            item_pack_faults(&[
                pack(BASE_MOD_ID, &none, &base),
                pack("cheap-rations", &none, &refiled),
            ]),
            BTreeMap::from([(
                "cheap-rations".to_string(),
                vec!["item 'Rations' is filed under Parts; the game uses it as Food".to_string()],
            )])
        );

        let on_plus = ["salvage-plus".to_string()];
        let faults = item_pack_faults(&[
            pack(BASE_MOD_ID, &none, &base),
            pack("deep-salvage", &on_plus, &core),
            pack("salvage-plus", &on_salvage, &core),
        ]);
        assert_eq!(
            faults.keys().collect::<Vec<_>>(),
            ["deep-salvage", "salvage-plus"]
        );
        assert!(faults["deep-salvage"][0].ends_with("they depend on each other"));
    }

    /// The base game must define every role item under its category, an id
    /// may appear once per pack, a mass of zero is refused, and an item must
    /// fit one canister.
    #[test]
    fn the_base_game_keeps_every_role_and_no_pack_repeats_or_weightless_items() {
        let mut base = base_items();
        base.retain(|item| {
            ![ItemRoleType::Torpedo, ItemRoleType::SalvagedParts]
                .iter()
                .any(|role| role.id() == item.id.as_str())
        });
        base[0].category = ItemCategoryType::Food;
        for item in &mut base {
            if item.id.as_str() == ItemRoleType::PdcRound.id() {
                item.mass_g = CARGO_CANISTER_MAX_MASS_G + 1;
            } else if item.id.as_str() == ItemRoleType::Rations.id() {
                item.category = ItemCategoryType::Parts;
            }
        }
        base.push(item("Scrap", 0));
        base.push(item("Scrap", 2_000));
        let none: [String; 0] = [];
        let faults = item_pack_faults(&[pack(BASE_MOD_ID, &none, &base)]);
        assert_eq!(
            faults[BASE_MOD_ID],
            [
                "item 'HullPlate' is filed under Food; the game uses it as Repair",
                "item 'PdcRound' has a mass_g of 1000001; an item must fit one 1000000 g \
                 canister",
                "item 'Rations' is filed under Parts; the game uses it as Food",
                "item 'Scrap' has a mass_g of 0",
                "item 'Scrap' is defined more than once",
                "item 'Torpedo' is not defined",
                "item 'SalvagedParts' is not defined",
            ]
        );
    }

    /// Iron ore asks 10 here: two of it and a 60 cr fee cost the 80 cr the
    /// plate sells for, which is the least a recipe may cost.
    #[test]
    fn a_refine_recipe_that_costs_what_its_output_sells_for_is_accepted() {
        assert!(base_faults_with(refined("Alloy", 80, &[("IronOre", 2)], 60)).is_empty());
    }

    #[test]
    fn a_refine_recipe_that_costs_less_than_its_output_sells_for_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 80, &[("IronOre", 2)], 59)),
            [
                "item 'Alloy' refines for 79 cr (its inputs at ask plus the 59 cr fee) and sells \
                 for 80 cr at bid; a refine must not cost less than its output sells for"
            ]
        );
    }

    /// A PDC round prices by the lot of 100, so 49 rounds at 10 cr a lot
    /// are worth 4.9 cr: under the 5 cr bid, where 50 rounds are not.
    #[test]
    fn the_refine_price_rule_counts_a_share_of_a_price_lot() {
        let rounds = |count: u32| {
            let mut base = base_items();
            for item in &mut base {
                if item.id.as_str() == ItemRoleType::PdcRound.id() {
                    item.price_lot = 100;
                }
            }
            base.push(refined("Alloy", 5, &[("PdcRound", count)], 0));
            let none: [String; 0] = [];
            item_pack_faults(&[pack(BASE_MOD_ID, &none, &base)])
        };
        assert!(rounds(50).is_empty());
        assert_eq!(
            rounds(49)[BASE_MOD_ID],
            [
                "item 'Alloy' refines for 4.9 cr (its inputs at ask plus the 0 cr fee) and sells \
                 for 5 cr at bid; a refine must not cost less than its output sells for"
            ]
        );
    }

    #[test]
    fn a_refine_recipe_with_no_inputs_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 5, &[], 5)),
            ["item 'Alloy' has a refine recipe with no inputs"]
        );
    }

    #[test]
    fn a_refine_input_count_of_zero_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 5, &[("IronOre", 0)], 5)),
            ["item 'Alloy' refines from 0 of 'IronOre'"]
        );
    }

    #[test]
    fn a_refine_recipe_that_consumes_its_own_output_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 5, &[("Alloy", 1)], 5)),
            ["item 'Alloy' refines from itself"]
        );
    }

    #[test]
    fn a_refine_input_listed_twice_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 5, &[("IronOre", 1), ("IronOre", 1)], 0)),
            ["item 'Alloy' lists refine input 'IronOre' more than once"]
        );
    }

    #[test]
    fn a_refine_input_no_pack_defines_is_refused() {
        assert_eq!(
            base_faults_with(refined("Alloy", 5, &[("Unobtainium", 1)], 5)),
            ["item 'Alloy' refines from unknown item 'Unobtainium'"]
        );
    }

    /// The recipe is priced by the catalog the merge keeps. A dependent
    /// pack that cheapens an input until the base recipe pays is the pack
    /// refused; the base game it replaced an item of still loads.
    #[test]
    fn a_pack_that_cheapens_a_refine_input_is_the_one_refused() {
        let mut base = base_items();
        base.push(refined("Alloy", 80, &[("IronOre", 2)], 60));
        let mut ore = item(ItemRoleType::IronOre.id(), 1_000);
        ore.ask_cr = 1;
        ore.bid_cr = 1;
        let cheap = [ore];
        let none: [String; 0] = [];
        let faults = item_pack_faults(&[
            pack(BASE_MOD_ID, &none, &base),
            pack("cheap-ore", &none, &cheap),
        ]);
        assert_eq!(faults.keys().collect::<Vec<_>>(), ["cheap-ore"]);
        assert!(faults["cheap-ore"][0].starts_with("item 'Alloy' refines for 62 cr"));
    }
}
