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

use nova_gameplay::prelude::{ItemDesign, CARGO_CANISTER_MAX_MASS_G, ITEM_ROLES};
use nova_mod_format::deps::{transitive_deps, DepGraph};
use nova_modding::prelude::BASE_MOD_ID;
use nova_scenario::prelude::{ore_for_asteroid_kind, AsteroidKindId, ASTEROID_KINDS};

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
/// chain with, and a dependency chain too deep to walk. The base game is
/// also refused when it lacks one of [`ITEM_ROLES`].
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

    if let Some(base) = packs.iter().find(|pack| pack.id == BASE_MOD_ID) {
        for (role, _) in ITEM_ROLES {
            if !base.items.iter().any(|item| item.id.as_str() == role) {
                fault(BASE_MOD_ID, format!("item '{role}' is not defined"));
            }
        }
    }
    faults
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
    if item.bid_cr > item.ask_cr {
        faults.push(format!(
            "item '{id}' has a bid_cr of {} above its ask_cr of {}",
            item.bid_cr, item.ask_cr
        ));
    }
    if let Some((_, category)) = ITEM_ROLES.iter().find(|(role, _)| *role == id) {
        if item.category != *category {
            faults.push(format!(
                "item '{id}' is filed under {:?}; the game uses it as {category:?}",
                item.category
            ));
        }
    }
    // Mining packs each ore it yields into one canister.
    let mined = ASTEROID_KINDS
        .iter()
        .filter_map(|kind| ore_for_asteroid_kind(&AsteroidKindId::from(*kind)))
        .any(|ore| ore.as_str() == id);
    if mined && item.mass_g > CARGO_CANISTER_MAX_MASS_G {
        faults.push(format!(
            "item '{id}' has a mass_g of {}; mined ore must fit one {CARGO_CANISTER_MAX_MASS_G} g \
             canister",
            item.mass_g
        ));
    }
    faults
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{ItemCategoryType, ItemDesignId};

    use super::*;

    fn item(id: &str, mass_g: u32) -> ItemDesign {
        let category = ITEM_ROLES
            .iter()
            .find(|(role, _)| *role == id)
            .map_or(ItemCategoryType::Parts, |(_, category)| *category);
        ItemDesign {
            id: ItemDesignId::from(id),
            name: id.to_string(),
            about: format!("{id} for tests."),
            category,
            mass_g,
            ask_cr: 10,
            bid_cr: 5,
        }
    }

    fn base_items() -> Vec<ItemDesign> {
        ITEM_ROLES
            .iter()
            .map(|(role, _)| item(role, 1_000))
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
        let mut rations = item(nova_gameplay::prelude::ITEM_RATIONS, 3_000);
        rations.ask_cr = 20;
        rations.bid_cr = 15;
        let plate = [
            item(nova_gameplay::prelude::ITEM_HULL_PLATE, 12_000),
            item(nova_gameplay::prelude::ITEM_SALVAGED_PARTS, 30_000),
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
    /// may appear once per pack, a mass of zero is refused, and an ore mining
    /// yields must fit one canister.
    #[test]
    fn the_base_game_keeps_every_role_and_no_pack_repeats_or_weightless_items() {
        let mut base = base_items();
        base.retain(|item| {
            ![
                nova_gameplay::prelude::ITEM_TORPEDO,
                nova_gameplay::prelude::ITEM_SALVAGED_PARTS,
            ]
            .contains(&item.id.as_str())
        });
        base[0].category = ItemCategoryType::Food;
        for item in &mut base {
            match item.id.as_str() {
                nova_gameplay::prelude::ITEM_STONE_ORE => {
                    item.mass_g = CARGO_CANISTER_MAX_MASS_G + 1;
                }
                nova_gameplay::prelude::ITEM_RATIONS => item.category = ItemCategoryType::Parts,
                _ => {}
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
                "item 'StoneOre' has a mass_g of 200001; mined ore must fit one 200000 g \
                 canister",
                "item 'Rations' is filed under Parts; the game uses it as Food",
                "item 'Scrap' has a mass_g of 0",
                "item 'Scrap' is defined more than once",
                "item 'Torpedo' is not defined",
                "item 'SalvagedParts' is not defined",
            ]
        );
    }
}
