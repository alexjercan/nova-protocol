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
//! Recipes and consumables are judged here too, against the item catalog
//! the merge keeps, since a pack that replaces an item can break a recipe or
//! a consumable another pack owns.
//!
//! The runtime merge (`register_bundles`) and the `content lint` walk both
//! call [`item_pack_faults`], so a pack lint passes is a pack the game loads.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use nova_events::prelude::Meters;
use nova_gameplay::prelude::{
    consumable_faults, recipe_shape_faults, recipe_value_faults, ConsumableDesign, GameItems,
    ItemDesign, ItemRoleType, RecipeDesign, CARGO_CANISTER_MAX_MASS_G,
};
use nova_mod_format::deps::{transitive_deps, DepGraph};
use nova_modding::prelude::{Content, BASE_MOD_ID};
use nova_scenario::prelude::{
    EventActionConfig, ScenarioObjectKind, SectionSource, ShipDesign, ShipDesignSource,
    StationDesignSource,
};
use nova_ship::prelude::{SectionConfig, SectionConfigPatch, SectionKind, SectionKindPatch};

/// Glob-import surface: `use nova_assets::items::prelude::*` re-exports the
/// public API of this module.
pub mod prelude {
    pub use super::{item_pack_faults, largest_torpedo_blast, ItemPack};
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
    /// Every recipe it defines, in authored order.
    pub recipes: Vec<&'a RecipeDesign>,
    /// Every consumable it defines, in authored order.
    pub consumables: Vec<&'a ConsumableDesign>,
    /// The largest torpedo blast radius among the sections it defines; zero
    /// when it defines no torpedo bay. A flare lure must stand outside the
    /// largest of every pack.
    pub torpedo_blast: Meters,
}

/// Every fault the item rule finds, by the pack it refuses, in pack id order.
/// Empty when every pack may load. Independent of the order of `packs`.
///
/// A pack is refused for an invalid item it defines, an item id it defines
/// twice, an item id it shares with a pack it has no one-way dependency
/// chain with, and a dependency chain too deep to walk. The base game is
/// also refused when it lacks one of [`ItemRoleType::ALL`].
///
/// A recipe is judged against the catalog the merge keeps: its pack is
/// refused for each [`recipe_shape_faults`] finding, such as an unknown
/// item or a bad duration. For each [`recipe_value_faults`] finding (a
/// batch that pays, an output heavier than its inputs), the last pack that
/// set the recipe or the price or mass of one of its items is refused: a
/// pack that cheapens a base input is refused, and the base game stays.
///
/// A consumable is judged against the same catalog: its pack is refused for
/// each [`consumable_faults`] finding, for a second consumable of one item,
/// and, with the packs that set the other side, for a flare lure inside the
/// largest torpedo blast of any pack and for two consumables of one effect
/// type. Of those packs, the ones no other of them depends on are refused.
pub fn item_pack_faults<'a>(packs: &[ItemPack<'a>]) -> BTreeMap<String, Vec<String>> {
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
    // packs fight over is refused above; its first definition stands in, so
    // a recipe that uses it is not also refused for an unknown item.
    let effective: BTreeMap<&str, (&str, &ItemDesign)> = definers
        .iter()
        .filter_map(|(id, owners)| {
            let owner = owners
                .iter()
                .copied()
                .find(|owner| {
                    owners.iter().all(|other| {
                        other == owner || (depends(owner, other) && !depends(other, owner))
                    })
                })
                .or_else(|| owners.first().copied())?;
            let pack = packs.iter().find(|pack| pack.id == owner)?;
            let item = pack
                .items
                .iter()
                .rev()
                .find(|item| item.id.as_str() == *id)?;
            Some((*id, (owner, *item)))
        })
        .collect();
    let catalog = GameItems::new(effective.values().map(|(_, item)| (*item).clone()));
    for pack in packs {
        for recipe in &pack.recipes {
            for message in recipe_shape_faults(recipe, &catalog) {
                fault(pack.id, message);
            }
            let value = recipe_value_faults(recipe, &catalog);
            if value.is_empty() {
                continue;
            }
            // The packs whose definitions set the two sides of the rules.
            let priced_by: BTreeSet<&str> = recipe
                .inputs
                .iter()
                .map(|(input, _)| input)
                .chain(std::iter::once(&recipe.output.0))
                .filter_map(|item| effective.get(item.as_str()).map(|(owner, _)| *owner))
                .chain(std::iter::once(pack.id))
                .collect();
            // A pack that replaced an item or the recipe last is the one
            // that broke the rule; a pack it depends on stays loadable.
            for owner in &priced_by {
                if !priced_by.iter().any(|later| depends(later, owner)) {
                    for message in &value {
                        fault(owner, message.clone());
                    }
                }
            }
        }
    }

    // The packs among `owners` that no other of them depends on: the last
    // to set a side of a rule, so the ones that broke it.
    let last = |owners: &BTreeSet<&'a str>| -> Vec<&'a str> {
        owners
            .iter()
            .copied()
            .filter(|owner| {
                !owners
                    .iter()
                    .any(|later| later != owner && depends(later, owner) && !depends(owner, later))
            })
            .collect()
    };
    let max_blast = packs
        .iter()
        .map(|pack| pack.torpedo_blast)
        .fold(Meters(0.0), |a, b| if b > a { b } else { a });
    // The consumables the merge keeps: the last definition of each item in
    // load order, as a recipe overlay.
    let mut kept: BTreeMap<&str, (&str, &ConsumableDesign)> = BTreeMap::new();
    for pack in packs {
        let mut seen = HashSet::new();
        for consumable in &pack.consumables {
            let item = consumable.item.as_str();
            if !seen.insert(item) {
                fault(
                    pack.id,
                    format!("the consumable '{item}' is defined more than once"),
                );
            }
            kept.insert(item, (pack.id, consumable));
        }
    }
    for (owner, consumable) in kept.values() {
        let found = consumable_faults(consumable, &catalog, max_blast);
        let (lure, own): (Vec<String>, Vec<String>) = found
            .into_iter()
            .partition(|message| message.contains("largest torpedo blast"));
        for message in own {
            fault(owner, message);
        }
        if lure.is_empty() {
            continue;
        }
        let mut owners: BTreeSet<&str> = BTreeSet::from([*owner]);
        if let nova_gameplay::prelude::ConsumableType::Flare { lure_m, .. } = consumable.effect {
            owners.extend(
                packs
                    .iter()
                    .filter(|pack| {
                        pack.torpedo_blast.0 * nova_gameplay::prelude::FLARE_LURE_BLAST_FACTOR
                            >= lure_m.0
                    })
                    .map(|pack| pack.id),
            );
        }
        for culprit in last(&owners) {
            for message in &lure {
                fault(culprit, message.clone());
            }
        }
    }
    let kept: Vec<(&str, &ConsumableDesign)> = kept.into_values().collect();
    for (at, (a_owner, a)) in kept.iter().enumerate() {
        for (b_owner, b) in &kept[at + 1..] {
            if a.effect.slot() != b.effect.slot() {
                continue;
            }
            let message = format!(
                "the consumables '{}' and '{}' both have a {:?} effect; each effect type has one \
                 flight key, so one consumable",
                a.item,
                b.item,
                a.effect.slot()
            );
            for culprit in last(&BTreeSet::from([*a_owner, *b_owner])) {
                fault(culprit, message.clone());
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

/// The largest torpedo blast radius `content` authors anywhere; zero when it
/// authors none. The merge and content lint fill [`ItemPack::torpedo_blast`]
/// with it.
///
/// It reads every place a blast radius is written: a section prototype, a
/// bay inline in a ship, station or scenario design, and a patch on a
/// prototype bay in a design or a spawn. A pack that redefines a section by
/// id counts both definitions, so the largest of them is never less than the
/// effective one.
pub fn largest_torpedo_blast<'a>(content: impl IntoIterator<Item = &'a Content>) -> Meters {
    let mut largest = Meters(0.0);
    let mut note = |blast: Meters| {
        if blast > largest {
            largest = blast;
        }
    };
    for item in content {
        match item {
            Content::Section(section) => section_blast(section).into_iter().for_each(&mut note),
            Content::Ship(ship) => design_blasts(&ship.design, &mut note),
            Content::Station(station) => design_blasts(&station.design.hull, &mut note),
            Content::Scenario(scenario) => {
                for event in &scenario.events {
                    for action in &event.actions {
                        action.walk(&mut |action| {
                            let object = match action {
                                EventActionConfig::SpawnScenarioObject(object) => object,
                                EventActionConfig::ScatterObjects(scatter) => &scatter.template,
                                _ => return,
                            };
                            match &object.kind {
                                ScenarioObjectKind::Spaceship(ship) => {
                                    ship_source_blasts(&ship.design, &mut note);
                                }
                                ScenarioObjectKind::Station(station) => {
                                    if let StationDesignSource::Inline(design) = &station.design {
                                        design_blasts(&design.hull, &mut note);
                                    }
                                }
                                _ => {}
                            }
                        });
                    }
                }
            }
            _ => {}
        }
    }
    largest
}

/// A torpedo bay's blast radius.
fn section_blast(section: &SectionConfig) -> Option<Meters> {
    match &section.kind {
        SectionKind::Torpedo(torpedo) => Some(torpedo.blast_radius),
        _ => None,
    }
}

/// The blast radius a section patch writes.
fn patch_blast(patch: &SectionConfigPatch) -> Option<Meters> {
    match &patch.kind {
        Some(SectionKindPatch::Torpedo(torpedo)) => torpedo.blast_radius,
        _ => None,
    }
}

/// Every blast radius a design's placed sections write.
fn design_blasts(design: &ShipDesign, note: &mut impl FnMut(Meters)) {
    for placed in &design.sections {
        let blast = match &placed.source {
            SectionSource::Inline(section) => section_blast(section),
            SectionSource::Prototype { patch, .. } => patch_blast(patch),
        };
        blast.into_iter().for_each(&mut *note);
    }
}

/// Every blast radius a spawned ship's design writes, inline or patched.
fn ship_source_blasts(source: &ShipDesignSource, note: &mut impl FnMut(Meters)) {
    match source {
        ShipDesignSource::Inline(design) => design_blasts(design, note),
        ShipDesignSource::Prototype {
            section_patches, ..
        } => {
            for patch in section_patches.values() {
                patch_blast(&patch.config).into_iter().for_each(&mut *note);
            }
        }
    }
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
    use nova_gameplay::prelude::{
        ConsumableType, CraftSiteType, ItemCategoryType, ItemDesignId, RecipeDesignId,
    };

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
        }
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
            recipes: Vec::new(),
            consumables: Vec::new(),
            torpedo_blast: Meters(0.0),
        }
    }

    /// A station recipe that makes one `output` from `inputs` for `fee_cr`.
    fn recipe(id: &str, inputs: &[(&str, u32)], output: &str, fee_cr: u32) -> RecipeDesign {
        RecipeDesign {
            id: RecipeDesignId::from(id),
            inputs: inputs
                .iter()
                .map(|(input, count)| (ItemDesignId::from(*input), *count))
                .collect(),
            output: (ItemDesignId::from(output), 1),
            duration_secs: 10.0,
            site: CraftSiteType::Station { fee_cr },
        }
    }

    /// The base game with an `Alloy` that sells for 80 cr at bid and the
    /// recipe that makes it from two iron ore (10 cr ask) for a 60 cr fee:
    /// a batch costs exactly what it sells for.
    fn base_with_alloy() -> (Vec<ItemDesign>, RecipeDesign) {
        let mut base = base_items();
        let mut alloy = item("Alloy", 1_000);
        alloy.ask_cr = 80;
        alloy.bid_cr = 80;
        base.push(alloy);
        let alloy_recipe = recipe(
            "base/alloy",
            &[(ItemRoleType::IronOre.id(), 2)],
            "Alloy",
            60,
        );
        (base, alloy_recipe)
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

    /// The recipe is priced by the catalog the merge keeps. A dependent
    /// pack that cheapens an input until the base recipe pays is the pack
    /// refused; the base game it replaced an item of still loads.
    #[test]
    fn a_pack_that_cheapens_a_recipe_input_is_the_one_refused() {
        let (base_items, alloy_recipe) = base_with_alloy();
        let mut ore = item(ItemRoleType::IronOre.id(), 1_000);
        ore.ask_cr = 1;
        ore.bid_cr = 1;
        let cheap = [ore];
        let none: [String; 0] = [];
        let mut base = pack(BASE_MOD_ID, &none, &base_items);
        base.recipes = vec![&alloy_recipe];
        assert!(item_pack_faults(std::slice::from_ref(&base)).is_empty());
        let faults = item_pack_faults(&[base, pack("cheap-ore", &none, &cheap)]);
        assert_eq!(faults.keys().collect::<Vec<_>>(), ["cheap-ore"]);
        assert!(
            faults["cheap-ore"][0].starts_with("the recipe 'base/alloy'"),
            "{faults:?}"
        );
    }

    /// A mod recipe that takes an item no pack defines refuses the mod, not
    /// the base game.
    #[test]
    fn a_mod_recipe_with_an_unknown_input_refuses_its_pack() {
        let (base_items, _) = base_with_alloy();
        let broken = recipe("odd/core", &[("Unobtainium", 1)], "Alloy", 0);
        let none: [String; 0] = [];
        let mut odd = pack("odd", &none, &[]);
        odd.recipes = vec![&broken];
        let faults = item_pack_faults(&[pack(BASE_MOD_ID, &none, &base_items), odd]);
        assert_eq!(
            faults,
            BTreeMap::from([(
                "odd".to_string(),
                vec!["the recipe 'odd/core' takes 'Unobtainium', which is no item".to_string()]
            )])
        );
    }

    fn flare(item: &str, lure_m: f32) -> ConsumableDesign {
        ConsumableDesign {
            item: ItemDesignId::from(item),
            effect: ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(lure_m),
            },
        }
    }

    /// Each effect type has one flight key: a mod that adds a second flare
    /// beside the base one is refused, and a mod that depends on the base
    /// game may replace the base flare by its item.
    #[test]
    fn a_second_consumable_of_one_effect_refuses_the_later_pack() {
        let mut base_items = base_items();
        base_items.push(item("Flare", 1_000));
        base_items.push(item("Glare", 1_000));
        let base_flare = flare("Flare", 800.0);
        let mod_flare = flare("Glare", 800.0);
        let overlay = flare("Flare", 900.0);
        let none: [String; 0] = [];
        let mut base = pack(BASE_MOD_ID, &none, &base_items);
        base.consumables = vec![&base_flare];
        base.torpedo_blast = Meters(300.0);
        assert!(item_pack_faults(std::slice::from_ref(&base)).is_empty());

        let mut glare = pack("glare", &none, &[]);
        glare.consumables = vec![&mod_flare];
        let faults = item_pack_faults(&[base.clone(), glare]);
        assert_eq!(faults.keys().collect::<Vec<_>>(), ["glare"]);
        assert!(
            faults["glare"][0].contains("both have a Flare effect"),
            "{faults:?}"
        );

        let mut replace = pack("bright", &none, &[]);
        replace.consumables = vec![&overlay];
        assert!(item_pack_faults(&[base, replace]).is_empty());
    }

    /// A mod torpedo bay whose blast reaches the base flare lure is the pack
    /// refused, not the base game.
    #[test]
    fn a_mod_blast_over_the_base_lure_refuses_the_mod() {
        let mut base_items = base_items();
        base_items.push(item("Flare", 1_000));
        let base_flare = flare("Flare", 800.0);
        let none: [String; 0] = [];
        let mut base = pack(BASE_MOD_ID, &none, &base_items);
        base.consumables = vec![&base_flare];
        base.torpedo_blast = Meters(300.0);
        let mut big = pack("big-bang", &none, &[]);
        big.torpedo_blast = Meters(900.0);
        let faults = item_pack_faults(&[base, big]);
        assert_eq!(faults.keys().collect::<Vec<_>>(), ["big-bang"]);
        assert!(
            faults["big-bang"][0].contains("largest torpedo blast"),
            "{faults:?}"
        );
    }

    fn bay(blast: f32) -> SectionConfig {
        use bevy::prelude::default;
        SectionConfig {
            base: default(),
            kind: SectionKind::Torpedo(nova_ship::prelude::TorpedoSectionConfig {
                blast_radius: Meters(blast),
                ..default()
            }),
        }
    }

    /// A blast written inline in a ship design or by a patch on a placed
    /// prototype bay counts, not only a section prototype's.
    #[test]
    fn the_largest_blast_reads_inline_bays_and_patches_in_designs() {
        use bevy::prelude::{default, Quat, Vec3};
        use nova_scenario::prelude::{ShipDesignPrototype, SpaceshipSectionConfig};
        let placed = |id: &str, source| SpaceshipSectionConfig {
            id: id.into(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            source,
        };
        let ship = |source| {
            Content::Ship(ShipDesignPrototype {
                id: "ship".into(),
                name: "Ship".into(),
                design: ShipDesign {
                    sections: vec![placed("bay", source)],
                    ..default()
                },
            })
        };
        let prototype = Content::Section(Box::new(bay(300.0)));
        assert_eq!(largest_torpedo_blast([&prototype]), Meters(300.0));

        let inline = ship(SectionSource::Inline(bay(900.0)));
        assert_eq!(largest_torpedo_blast([&prototype, &inline]), Meters(900.0));

        let patched = ship(SectionSource::Prototype {
            id: "bay".into(),
            patch: SectionConfigPatch {
                kind: Some(SectionKindPatch::Torpedo(
                    nova_ship::prelude::TorpedoSectionConfigPatch {
                        blast_radius: Some(Meters(1_200.0)),
                        ..default()
                    },
                )),
                ..default()
            },
        });
        assert_eq!(
            largest_torpedo_blast([&prototype, &patched]),
            Meters(1_200.0)
        );
    }
}
