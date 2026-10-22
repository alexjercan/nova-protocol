//! What a ship carries: the authored item catalog, the category each item
//! belongs to, the per-ship stack counts and the mass the ship has room for.
//!
//! Items are content. Each [`ItemDesign`] is authored in RON and keyed by an
//! [`ItemDesignId`]; the mod merge publishes the effective catalog as
//! [`GameItems`], and every mass, price and name comes from it. Engine code
//! names only the role items of [`ItemRoleType`], which content lint and the
//! merge require with their categories.
//!
//! Every ship root requires a [`ShipInventory`]. Its `Default` has no room and
//! no stock: a code-built ship that never states an inventory carries nothing.
//! An authored ship states its stock as a [`ShipInventoryStock`] through
//! `SpaceshipConfig::inventory`, which RON requires; the spawn consumes that
//! stock into a `ShipInventory` whose capacity its design derives. A Ship pane
//! repair spends [`ItemRoleType::HullPlate`] by the [`plan_plate_repair`]
//! rule, an Inventory pane transfer moves items between two docked ships by
//! the [`plan_item_transfer`] rule, and a jettison queues [`CargoCanister`]s
//! by the [`plan_item_jettison`] rule, a weapon's idle reload moves its
//! ammunition item into the magazine, and a Buy or Sell with a docked trader
//! moves items and [`ShipCredits`] together by the [`plan_item_trade`] rule.
//! Every section config carries a [`BuildRecipe`]; a Ship pane rebuild or
//! swap at a docked station pays it by the [`plan_recipe_payment`] rule. A living
//! station that offers Storage keeps a capped [`StationStorage`] beside its
//! hold, and a Store or Withdraw moves items between it and the docked ship
//! by the [`plan_item_storage`] rule, with no credits.
//! A saved world keeps stock and credits across a Load; any other scenario
//! returns them to their authored values when it loads again. A stack exists
//! only while its count is above zero, and the mass of all stacks never passes
//! the capacity. Mass is counted in grams; [`kg_text`] shows it in kilograms.

use std::{collections::BTreeMap, sync::Arc};

use bevy::prelude::*;
use nova_events::prelude::{CreditChangeCauseType, ItemTradeType};

use crate::{integrity::prelude::Health, market::ItemPrice};

/// The whole module.
pub mod prelude {
    pub use nova_events::prelude::{CreditChangeCauseType, ItemTradeType};

    pub use super::{
        cr_digits, cr_text, kg_text, plan_credit_take, plan_hull_strip, plan_item_jettison,
        plan_item_storage, plan_item_trade, plan_item_transfer, plan_loot_take, plan_plate_repair,
        plan_recipe_payment, plan_section_rebuild, plan_section_yield, strip_duration_secs,
        BuildRecipe, CargoCanister, CargoCanisterIdAllocator, CargoCanisterRuntimeId,
        CreditTakeRefusalType, GameItems, HullStrip, HullStripPartner, HullStripPauseType,
        HullStripRefusalType, ItemCategoryType, ItemDesign, ItemDesignId, ItemJettison,
        ItemJettisonRefusalType, ItemRoleType, ItemStorageRefusalType, ItemStorageType, ItemTrade,
        ItemTradeRefusalType, ItemTransferRefusalType, ItemTransferType, LootTake,
        LootableShipMarker, PlateRepair, PlateRepairRefusalType, RecipePayment,
        RecipePaymentRefusalType, SectionRebuildRefusalType, ShipCredits, ShipInventory,
        ShipInventoryReport, ShipInventoryReports, ShipInventoryStock, StationPartner,
        StationStorage, StripTarget, TradePriceType, CARGO_CANISTER_MAX_MASS_G, HULL_PLATE_HEALTH,
        STRIP_MIN_SECS, STRIP_SECS_PER_PART,
    };
}

/// The id an item is referenced by from stock, cargo, a save or a command.
///
/// An authored catalog id, resolved against [`GameItems`]. Serialized as the
/// bare id string, so RON writes `{"PdcRound": 2000}`. An id no loaded item
/// carries is refused at lint and at load; nothing resolves it to another
/// item. Shared rather than owned, so the many copies a hold, a reload and a
/// canister keep cost no allocation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ItemDesignId(Arc<str>);

impl ItemDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ItemDesignId {
    fn from(id: &str) -> Self {
        Self(id.into())
    }
}

/// The derived order and hash are the id string's, so a catalog keyed by id
/// answers a `&str` lookup without allocating one.
impl std::borrow::Borrow<str> for ItemDesignId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ItemDesignId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Every item engine code names, with the category it must keep. Content lint
/// and the merge refuse a catalog without one of these ids and a definition
/// that files one under another category: the repair, the weapon reloads,
/// the mining yield and the sector hold draws would otherwise spend or make
/// something else. A closed enum, so a match over the roles is exhaustive
/// where a match over id strings is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemRoleType {
    /// Hull plating stock, counted in plates. The repair item.
    HullPlate,
    /// One point-defense round. Kinetic and Pierce mounts load the same round;
    /// the mount decides its damage type.
    PdcRound,
    /// One railgun slug.
    RailSlug,
    /// One torpedo. Every bay type loads the same torpedo; the bay decides its
    /// flight.
    Torpedo,
    /// Ore mined from a rock-kind asteroid.
    StoneOre,
    /// Ore mined from a metal-kind asteroid.
    IronOre,
    /// Ice mined from an ice-kind asteroid.
    WaterIce,
    /// Ore mined from a carbon-kind asteroid.
    CarbonOre,
    /// Packed crew food. Generated sector ship holds draw it.
    Rations,
    /// Reusable ship components. Generated sector ship holds draw them.
    SalvagedParts,
}

impl ItemRoleType {
    /// Every role, in base catalog order.
    pub const ALL: [Self; 10] = [
        Self::HullPlate,
        Self::PdcRound,
        Self::RailSlug,
        Self::Torpedo,
        Self::StoneOre,
        Self::IronOre,
        Self::WaterIce,
        Self::CarbonOre,
        Self::Rations,
        Self::SalvagedParts,
    ];

    /// The authored catalog id the base game defines this role under.
    pub const fn id(self) -> &'static str {
        match self {
            Self::HullPlate => "HullPlate",
            Self::PdcRound => "PdcRound",
            Self::RailSlug => "RailSlug",
            Self::Torpedo => "Torpedo",
            Self::StoneOre => "StoneOre",
            Self::IronOre => "IronOre",
            Self::WaterIce => "WaterIce",
            Self::CarbonOre => "CarbonOre",
            Self::Rations => "Rations",
            Self::SalvagedParts => "SalvagedParts",
        }
    }

    /// The category the item under [`id`](Self::id) must keep.
    pub const fn category(self) -> ItemCategoryType {
        match self {
            Self::HullPlate => ItemCategoryType::Repair,
            Self::PdcRound | Self::RailSlug | Self::Torpedo => ItemCategoryType::Ammo,
            Self::StoneOre | Self::IronOre | Self::WaterIce | Self::CarbonOre => {
                ItemCategoryType::Raw
            }
            Self::Rations => ItemCategoryType::Food,
            Self::SalvagedParts => ItemCategoryType::Parts,
        }
    }
}

impl From<ItemRoleType> for ItemDesignId {
    /// A fresh shared id. A path that runs per pulse or per spawn clones the
    /// id of [`GameItems::role`] instead of allocating one.
    fn from(role: ItemRoleType) -> Self {
        Self::from(role.id())
    }
}

/// One authored item: what it is called, what it is for, what it weighs and
/// what a trader pays for it. Every field is required.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ItemDesign {
    /// The catalog id stock, cargo and saves name it by.
    pub id: ItemDesignId,
    /// The display name, as the Inventory pane and a canister's tag show it.
    pub name: String,
    /// One or two sentences the Inventory pane shows for the selected item.
    pub about: String,
    /// The category the item is filed under; picks its icon and color.
    pub category: ItemCategoryType,
    /// Fixed mass of one item in grams, above zero. Grams keep capacity and
    /// conservation arithmetic exact for items lighter than a kilogram.
    pub mass_g: u32,
    /// Credits a trader asks for one item: what a Buy pays.
    pub ask_cr: u32,
    /// Credits a trader bids for one item: what a Sell earns. At most
    /// [`ask_cr`](Self::ask_cr), so buying and selling back never gains
    /// credits.
    pub bid_cr: u32,
}

impl ItemDesign {
    /// Mass of `count` of this item in grams. Wide so a count read from
    /// content or typed at the Command shell cannot overflow before a
    /// capacity check refuses it.
    pub fn stack_mass_g(&self, count: u32) -> u64 {
        u64::from(count) * u64::from(self.mass_g)
    }
}

/// The effective item catalog: every [`ItemDesign`] the enabled mods
/// registered, after the merge applied its overlay rule. Inserted by the mod
/// merge (`nova_assets`'s `register_bundles`) and read by every item flow.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GameItems(BTreeMap<ItemDesignId, ItemDesign>);

impl GameItems {
    /// The catalog of `designs`; a later design with an id replaces an earlier
    /// one, as a dependent mod's overlay does.
    pub fn new(designs: impl IntoIterator<Item = ItemDesign>) -> Self {
        Self(
            designs
                .into_iter()
                .map(|design| (design.id.clone(), design))
                .collect(),
        )
    }

    /// The item with this id, or `None` if nothing authored it.
    pub fn get(&self, id: &ItemDesignId) -> Option<&ItemDesign> {
        self.0.get(id)
    }

    /// The item with this id, for an id content lint or a save check already
    /// resolved.
    ///
    /// # Panics
    ///
    /// When no item has the id: the caller let an unchecked id into a hold,
    /// a canister or a reload.
    pub fn design(&self, id: &ItemDesignId) -> &ItemDesign {
        self.get(id)
            .unwrap_or_else(|| panic!("item '{id}' is not in the loaded item catalog"))
    }

    /// The item filling `role`, for a catalog content lint or the merge
    /// already checked.
    ///
    /// # Panics
    ///
    /// When the catalog lacks the role's id: the catalog skipped
    /// `item_pack_faults`, which refuses a base pack missing a role.
    pub fn role(&self, role: ItemRoleType) -> &ItemDesign {
        self.0.get(role.id()).unwrap_or_else(|| {
            panic!(
                "role item {role:?} ('{}') is not in the loaded item catalog",
                role.id()
            )
        })
    }

    /// Every item, in id order.
    pub fn iter(&self) -> impl Iterator<Item = &ItemDesign> {
        self.0.values()
    }
}

/// `grams` as display kilograms: whole kilograms print bare (`10 kg`), a
/// fraction prints without trailing zeros (`0.2 kg`, `3520.05 kg`).
pub fn kg_text(grams: u64) -> String {
    let (kg, rest) = (grams / 1000, grams % 1000);
    if rest == 0 {
        format!("{kg} kg")
    } else {
        let fraction = format!("{rest:03}");
        format!("{kg}.{} kg", fraction.trim_end_matches('0'))
    }
}

/// Credits grouped by thousands, no unit: `22,980`.
pub fn cr_digits(credits: u64) -> String {
    let digits = credits.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// Credits as sentences and notes write them: `22,980 cr`.
pub fn cr_text(credits: impl Into<u64>) -> String {
    format!("{} cr", cr_digits(credits.into()))
}

/// What an item is for. There is no fuel category: thrust never consumes
/// stock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemCategoryType {
    /// Mined or salvaged bulk material.
    Raw,
    /// Material a repair consumes.
    Repair,
    /// Weapon magazine supply.
    Ammo,
    /// Provisions carried for trade.
    Food,
    /// Scavenged objects carried for a story, objective or trade.
    Parts,
}

/// The items an authored ship carries at spawn, as a count per item id.
/// RON writes a map with quoted keys: `{"HullPlate": 12}`, or `{}` for none.
///
/// Holds no capacity: the spawn reads that from the resolved design and
/// consumes this component into the ship's [`ShipInventory`]. Content lint
/// refuses an unknown item and stock heavier than the design's hold before
/// the spawn does.
///
/// The `Default` holds nothing, as `SpaceshipConfig`'s `Default` needs; RON
/// still requires the field.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipInventoryStock {
    stacks: BTreeMap<ItemDesignId, u32>,
}

impl ShipInventoryStock {
    /// Stock holding `stacks`.
    ///
    /// # Panics
    ///
    /// On a zero quantity or a repeated item: the same faults the authored RON
    /// refuses, so a builder that writes one has a bug.
    pub fn new(stacks: impl IntoIterator<Item = (ItemDesignId, u32)>) -> Self {
        match Self::checked(stacks) {
            Ok(stock) => stock,
            Err(fault) => panic!("{fault}"),
        }
    }

    /// The one validation both [`new`](Self::new) and the RON parse run.
    fn checked(stacks: impl IntoIterator<Item = (ItemDesignId, u32)>) -> Result<Self, String> {
        let mut held = BTreeMap::new();
        for (item, count) in stacks {
            if count == 0 {
                return Err(format!(
                    "ShipInventoryStock stack of '{item}' has quantity 0"
                ));
            }
            if held.contains_key(&item) {
                return Err(format!("ShipInventoryStock lists '{item}' twice"));
            }
            held.insert(item, count);
        }
        Ok(Self { stacks: held })
    }

    /// Every stack, in id order. Each count is above zero.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.stacks.iter().map(|(item, count)| (item, *count))
    }

    /// Mass of every stack in grams. Wide because authored stock has no
    /// capacity bound until the spawn applies one.
    ///
    /// # Panics
    ///
    /// When `items` lacks a stacked id; content lint refuses that stock first.
    pub fn mass_g(&self, items: &GameItems) -> u64 {
        self.stacks()
            .map(|(item, count)| items.design(item).stack_mass_g(count))
            .sum()
    }
}

/// The items one ship carries, as a count per item id, and the mass it has
/// room for across all stacks. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker).
///
/// The `Default` has a capacity of zero: it is the required-component value
/// for a ship that no config states, and such a ship can take nothing.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipInventory {
    capacity_g: u32,
    stacks: BTreeMap<ItemDesignId, u32>,
}

impl ShipInventory {
    /// An inventory with room for `capacity_g` grams, holding `stacks`.
    ///
    /// # Panics
    ///
    /// On a zero quantity, a repeated item, an item `items` lacks, or stacks
    /// heavier than `capacity_g`. Content lint refuses authored stock past a
    /// design's hold, so a spawn that reaches this panic loaded unlinted
    /// content.
    pub fn new(
        items: &GameItems,
        capacity_g: u32,
        stacks: impl IntoIterator<Item = (ItemDesignId, u32)>,
    ) -> Self {
        let stock = ShipInventoryStock::new(stacks);
        let mass_g = stock.mass_g(items);
        assert!(
            mass_g <= u64::from(capacity_g),
            "ShipInventory holds {} but has capacity {}",
            kg_text(mass_g),
            kg_text(u64::from(capacity_g)),
        );
        Self {
            capacity_g,
            stacks: stock.stacks,
        }
    }

    /// How many grams the ship has room for across all stacks.
    pub fn capacity_g(&self) -> u32 {
        self.capacity_g
    }

    /// How many grams the ship carries across all stacks.
    pub fn used_g(&self, items: &GameItems) -> u32 {
        // The capacity bounds the sum, so each product and the sum fit.
        self.stacks()
            .map(|(item, count)| items.design(item).mass_g * count)
            .sum()
    }

    /// How many more grams the ship has room for.
    pub fn free_g(&self, items: &GameItems) -> u32 {
        self.capacity_g - self.used_g(items)
    }

    /// How many of `item` the ship carries; zero when it has no stack.
    pub fn count(&self, item: &ItemDesignId) -> u32 {
        self.stacks.get(item).copied().unwrap_or(0)
    }

    /// Every stack the ship carries, in id order. Each count is above zero.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.stacks.iter().map(|(item, count)| (item, *count))
    }

    /// True when the ship carries no stack.
    pub fn is_empty(&self) -> bool {
        self.stacks.is_empty()
    }

    /// Add `count` of `item`; create the stack when the ship has none.
    ///
    /// # Panics
    ///
    /// On `count == 0` or a mass past [`free_g`](Self::free_g): the caller
    /// has a bug and must plan the add first, as [`plan_item_transfer`] does.
    pub fn add(&mut self, items: &GameItems, item: &ItemDesignId, count: u32) {
        assert!(count > 0, "ShipInventory adds 0 of '{item}'");
        let free_g = self.free_g(items);
        assert!(
            items.design(item).stack_mass_g(count) <= u64::from(free_g),
            "ShipInventory adds {count} of '{item}' but has room for {}",
            kg_text(u64::from(free_g)),
        );
        let held = self.count(item);
        self.stacks.insert(item.clone(), held + count);
    }

    /// Remove `count` of `item`; delete the stack when it empties.
    ///
    /// # Panics
    ///
    /// On `count == 0` or more than the ship carries: the caller has a bug and
    /// must check [`count`](Self::count) first.
    pub fn remove(&mut self, item: &ItemDesignId, count: u32) {
        assert!(count > 0, "ShipInventory removes 0 of '{item}'");
        let held = self.count(item);
        assert!(
            count <= held,
            "ShipInventory removes {count} of '{item}' but carries {held}"
        );
        if count == held {
            self.stacks.remove(item);
        } else {
            self.stacks.insert(item.clone(), held - count);
        }
    }
}

/// What a station keeps for the ships that dock with it: a count per item id
/// beside the hull's own hold, and the mass it takes across all stacks.
/// Inserted empty by the station spawn with its design's `storage_cap_g`,
/// frozen and thawed with the station, and saved in its record.
///
/// One store per station, shared by every ship that docks there; the player
/// is the only depositor. Any item may be stored: sections are never items,
/// so nothing is refused by category. A stack exists only while its count is
/// above zero, and the mass of all stacks never passes the cap.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationStorage {
    cap_g: u32,
    items: BTreeMap<ItemDesignId, u32>,
}

impl StationStorage {
    /// An empty store that takes `cap_g` grams.
    pub fn new(cap_g: u32) -> Self {
        Self {
            cap_g,
            items: BTreeMap::new(),
        }
    }

    /// How many grams the store takes.
    pub fn cap_g(&self) -> u32 {
        self.cap_g
    }

    /// How many grams the store holds across all stacks.
    pub fn used_g(&self, items: &GameItems) -> u32 {
        // The cap bounds the sum, so each product and the sum fit.
        self.stacks()
            .map(|(item, count)| items.design(item).mass_g * count)
            .sum()
    }

    /// How many more grams the store takes.
    pub fn free_g(&self, items: &GameItems) -> u32 {
        self.cap_g - self.used_g(items)
    }

    /// How many of `item` the store holds; zero when it has no stack.
    pub fn count(&self, item: &ItemDesignId) -> u32 {
        self.items.get(item).copied().unwrap_or(0)
    }

    /// Every stack held, in id order. Each count is above zero.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.items.iter().map(|(item, count)| (item, *count))
    }

    /// Every item id held.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        self.items.keys()
    }

    /// True when the store holds no stack.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add `count` of `item`; create the stack when the store has none.
    ///
    /// # Panics
    ///
    /// On `count == 0` or a mass past [`free_g`](Self::free_g): the caller
    /// has a bug and must plan the add first with [`plan_item_storage`].
    pub fn add(&mut self, items: &GameItems, item: &ItemDesignId, count: u32) {
        assert!(count > 0, "StationStorage adds 0 of '{item}'");
        let free_g = self.free_g(items);
        assert!(
            items.design(item).stack_mass_g(count) <= u64::from(free_g),
            "StationStorage adds {count} of '{item}' but has room for {}",
            kg_text(u64::from(free_g)),
        );
        let held = self.count(item);
        self.items.insert(item.clone(), held + count);
    }

    /// Remove `count` of `item`; delete the stack when it empties.
    ///
    /// # Panics
    ///
    /// On `count == 0` or more than the store holds: the caller has a bug
    /// and must plan the removal first with [`plan_item_storage`].
    pub fn remove(&mut self, item: &ItemDesignId, count: u32) {
        assert!(count > 0, "StationStorage removes 0 of '{item}'");
        let held = self.count(item);
        assert!(
            count <= held,
            "StationStorage removes {count} of '{item}' but holds {held}"
        );
        if count == held {
            self.items.remove(item);
        } else {
            self.items.insert(item.clone(), held - count);
        }
    }
}

/// The credits one ship holds. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker); the `Default`
/// of zero is the value for a ship that no config states. An authored ship
/// states its balance through `SpaceshipConfig::credits`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipCredits(pub u32);

/// One hold or balance change a ship's owning system made, queued for the
/// scenario layer to translate into a scenario event.
///
/// The ship and gameplay crates sit below `nova_scenario`, so they cannot
/// fire a scenario event themselves; they push here instead, the way a helm
/// order pushes its `ShipOrderReport`, and a scenario tracker drains the
/// queue once per fixed tick. Every variant names what moved in typed ids so
/// the tracker only resolves scenario ids.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
pub enum ShipInventoryReport {
    /// An intake took one stack of a canister into the hold.
    PickedUp {
        /// Runtime id of the taken canister.
        canister: CargoCanisterRuntimeId,
        /// The stack's item.
        item: ItemDesignId,
        /// Units in the stack.
        count: u32,
    },
    /// An intake dropped one stack of a canister out of the hold.
    Jettisoned {
        /// Runtime id of the dropped canister.
        canister: CargoCanisterRuntimeId,
        /// The stack's item.
        item: ItemDesignId,
        /// Units in the stack.
        count: u32,
    },
    /// A Buy or Sell with a docked trader went through; queued on the ship
    /// that traded, naming the trader.
    Traded {
        /// Root of the trader.
        partner: Entity,
        /// Which way the items moved, seen from this ship.
        trade: ItemTradeType,
        /// The item that moved.
        item: ItemDesignId,
        /// Units that moved.
        count: u32,
        /// Credits the buyer paid the seller.
        price_cr: u32,
    },
    /// This ship's credits balance moved.
    CreditsChanged {
        /// Balance before the change.
        previous_cr: u32,
        /// Balance after the change.
        current_cr: u32,
        /// Why it moved.
        cause: CreditChangeCauseType,
    },
    /// A Store at a docked station moved items from this ship's hold into
    /// the station's storage; queued on the ship that stored.
    Stored {
        /// Root of the station.
        station: Entity,
        /// The item that moved.
        item: ItemDesignId,
        /// Units that moved.
        count: u32,
    },
    /// A Withdraw at a docked station moved items from the station's storage
    /// into this ship's hold; queued on the ship that withdrew.
    Withdrawn {
        /// Root of the station.
        station: Entity,
        /// The item that moved.
        item: ItemDesignId,
        /// Units that moved.
        count: u32,
    },
    /// A Ship pane repair spent hull plates on one section.
    Repaired {
        /// Pane code of the repaired section (`HULL-3`).
        section: String,
        /// The plate item spent.
        item: ItemDesignId,
        /// Plates spent.
        count: u32,
    },
    /// A Ship pane rebuild at a docked station spent one section's
    /// [`BuildRecipe`] and put the section back.
    Rebuilt {
        /// Pane code of the rebuilt section (`PDC-1`).
        section: String,
        /// Every part the recipe spent, from the hold and bought together.
        parts: BTreeMap<ItemDesignId, u32>,
        /// Credits the ship paid the station.
        price_cr: u32,
    },
    /// A Ship pane swap at a docked station spent the offered part's
    /// [`BuildRecipe`] and replaced a live section with it.
    Swapped {
        /// Pane code of the swapped section (`PDC-1`); a swap keeps the
        /// section's family, so the code holds.
        section: String,
        /// The section prototype id the section was.
        from: String,
        /// The section prototype id it is now.
        to: String,
        /// Every part the recipe spent, from the hold and bought together.
        parts: BTreeMap<ItemDesignId, u32>,
        /// Credits the ship paid the station.
        price_cr: u32,
    },
    /// A Ship pane add at a docked station spent the offered part's
    /// [`BuildRecipe`] and mounted it at a free socket of a live section.
    Added {
        /// Pane code the new section takes (`PDC-3`).
        section: String,
        /// The section prototype id added.
        part: String,
        /// Pane code of the section it was mounted on (`HULL-2`).
        at: String,
        /// The socket id of that section it mates.
        point: String,
        /// Every part the recipe spent, from the hold and bought together.
        parts: BTreeMap<ItemDesignId, u32>,
        /// Credits the ship paid the station.
        price_cr: u32,
    },
    /// An Inventory pane strip took one section off the docked partner, or a
    /// Ship pane delete took one off this ship, and put its parts in this
    /// ship's hold.
    Stripped {
        /// The stripped hull's scenario id: this ship's own for a delete.
        hull: String,
        /// Pane code of the stripped section on that hull (`PDC-1`).
        section: String,
        /// Every part the strip put in the hold.
        parts: BTreeMap<ItemDesignId, u32>,
    },
    /// A Ship pane buy at a docked station replaced the ship's hull with one
    /// from the station's catalogue. The old hold stays in the old ship,
    /// parked as the player's own.
    ShipBought {
        /// The catalogue name of the bought hull.
        hull: String,
        /// The new hull's price: every section's recipe at ask.
        price_cr: u32,
    },
    /// A Ship pane hire at a docked station added an escort to the player's
    /// fleet.
    EscortHired {
        /// The escort's scenario id.
        escort: String,
        /// The catalogue name of the escort's hull.
        hull: String,
        /// The hire price.
        price_cr: u32,
    },
    /// A Ship pane buy at a docked station made the station the player's.
    StationBought {
        /// The station's scenario id.
        station: String,
        /// The price.
        price_cr: u32,
    },
}

/// The [`ShipInventoryReport`]s a ship's owning systems queued since the
/// scenario layer last drained them. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker), so a
/// pushing system never has to check for it.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ShipInventoryReports(pub Vec<ShipInventoryReport>);

impl ShipInventoryReports {
    /// Set a ship's balance and queue the change: the one writer every
    /// balance move after spawn goes through, so `OnCreditsChanged` fires
    /// exactly once per change and never for a write that changed nothing.
    pub fn change_credits(
        &mut self,
        credits: &mut ShipCredits,
        current_cr: u32,
        cause: CreditChangeCauseType,
    ) {
        let previous_cr = credits.0;
        if previous_cr == current_cr {
            return;
        }
        credits.0 = current_cr;
        self.0.push(ShipInventoryReport::CreditsChanged {
            previous_cr,
            current_cr,
            cause,
        });
    }
}

/// Marks a ship root that a docked ship may Take from although it was never
/// neutralized: a derelict. Authored through `SpaceshipConfig::lootable`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct LootableShipMarker;

/// Which way a transfer between two docked ships moves items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum ItemTransferType {
    /// From the docked partner into the player ship.
    Take,
    /// From the player ship into the docked partner.
    Give,
}

/// Why a transfer moves nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemTransferRefusalType {
    /// A Take from a partner that is neither neutralized nor lootable.
    NotLootable,
    /// The quantity text is not a whole number.
    NoQuantity,
    /// A quantity of zero.
    ZeroQuantity,
    /// The source ship carries fewer than the quantity.
    Short {
        /// What the source ship carries.
        held: u32,
    },
    /// The target ship has room for less than the quantity's mass.
    NoRoom {
        /// How many more grams the target ship has room for.
        free_g: u32,
    },
}

/// Plan a transfer of `quantity` of `item` between the player ship's `own`
/// inventory and its docked `partner`'s.
///
/// Returns the count to remove from the source and add to the target. A Take
/// needs `partner_lootable`: the partner is neutralized or carries
/// [`LootableShipMarker`]; a Give does not read it. Checks run in
/// [`ItemTransferRefusalType`] order. `quantity` is `None` when the typed
/// text is not a whole number. `item` resolves in `items`.
pub fn plan_item_transfer(
    items: &GameItems,
    transfer: ItemTransferType,
    partner_lootable: bool,
    item: &ItemDesignId,
    quantity: Option<u32>,
    own: &ShipInventory,
    partner: &ShipInventory,
) -> Result<u32, ItemTransferRefusalType> {
    let (source, target) = match transfer {
        ItemTransferType::Take => {
            if !partner_lootable {
                return Err(ItemTransferRefusalType::NotLootable);
            }
            (partner, own)
        }
        ItemTransferType::Give => (own, partner),
    };
    let quantity = quantity.ok_or(ItemTransferRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemTransferRefusalType::ZeroQuantity);
    }
    let held = source.count(item);
    if quantity > held {
        return Err(ItemTransferRefusalType::Short { held });
    }
    let free_g = target.free_g(items);
    if items.design(item).stack_mass_g(quantity) > u64::from(free_g) {
        return Err(ItemTransferRefusalType::NoRoom { free_g });
    }
    Ok(quantity)
}

/// A planned trade: the count to move from seller to buyer and the credits to
/// move from buyer to seller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemTrade {
    /// Items to remove from the seller and add to the buyer; above zero.
    pub count: u32,
    /// Credits to remove from the buyer and add to the seller.
    pub price_cr: u32,
}

/// Why a trade moves nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemTradeRefusalType {
    /// The docked partner is neutralized or lootable: a derelict does not
    /// trade.
    NotTrader,
    /// The quantity text is not a whole number.
    NoQuantity,
    /// A quantity of zero.
    ZeroQuantity,
    /// The seller carries fewer than the quantity.
    Short {
        /// What the seller carries.
        held: u32,
    },
    /// The buyer has room for less than the quantity's mass.
    NoRoom {
        /// How many more grams the buyer has room for.
        free_g: u32,
    },
    /// The buyer holds fewer credits than the price.
    NoCredits {
        /// What the buyer holds.
        credits: u32,
    },
    /// The seller's balance after the trade would not fit in [`ShipCredits`].
    CreditOverflow,
}

/// The prices a trading partner deals at for one item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TradePriceType {
    /// A partner the player does not own: a Buy pays the ask, a Sell earns
    /// the bid. The reference price ([`ItemDesign::reference_price`]), or a
    /// generated trader's computed one ([`price`](crate::market::price)).
    Market(ItemPrice),
    /// A station the player owns: its stock is the player's, so a Buy pays
    /// the bid as a Sell earns it, and no spread is taken from the owner.
    Owned(ItemPrice),
}

/// Plan a trade of `quantity` of `item` between the player ship (`own`,
/// holding `own_cr`) and its docked `partner` (holding `partner_cr`).
///
/// `prices` is `None` when the partner does not trade: it is neutralized or
/// carries [`LootableShipMarker`], or it is a station that offers no Trade.
/// A Buy moves items from the partner to the player at the ask each, or at
/// the bid from a station the player owns ([`TradePriceType::Owned`]); a
/// Sell moves them from the player to the partner at the bid each. The whole
/// quantity trades at the one unit price. Checks run in
/// [`ItemTradeRefusalType`] order, and a refusal changes nothing. `item`
/// resolves in `items`.
#[expect(
    clippy::too_many_arguments,
    reason = "both ships' stock and credits, read as one atomic plan"
)]
pub fn plan_item_trade(
    items: &GameItems,
    trade: ItemTradeType,
    prices: Option<TradePriceType>,
    item: &ItemDesignId,
    quantity: Option<u32>,
    own: &ShipInventory,
    own_cr: u32,
    partner: &ShipInventory,
    partner_cr: u32,
) -> Result<ItemTrade, ItemTradeRefusalType> {
    let prices = prices.ok_or(ItemTradeRefusalType::NotTrader)?;
    let quantity = quantity.ok_or(ItemTradeRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemTradeRefusalType::ZeroQuantity);
    }
    let design = items.design(item);
    let (ask_cr, bid_cr) = match prices {
        TradePriceType::Market(price) => (price.ask_cr, price.bid_cr),
        TradePriceType::Owned(price) => (price.bid_cr, price.bid_cr),
    };
    let (seller, buyer, buyer_cr, seller_cr, unit_cr) = match trade {
        ItemTradeType::Buy => (partner, own, own_cr, partner_cr, ask_cr),
        ItemTradeType::Sell => (own, partner, partner_cr, own_cr, bid_cr),
    };
    let held = seller.count(item);
    if quantity > held {
        return Err(ItemTradeRefusalType::Short { held });
    }
    let free_g = buyer.free_g(items);
    if design.stack_mass_g(quantity) > u64::from(free_g) {
        return Err(ItemTradeRefusalType::NoRoom { free_g });
    }
    let price = u64::from(quantity) * u64::from(unit_cr);
    if price > u64::from(buyer_cr) {
        return Err(ItemTradeRefusalType::NoCredits { credits: buyer_cr });
    }
    // The price fits: it is at most the buyer's u32 balance.
    let price_cr = price as u32;
    if seller_cr.checked_add(price_cr).is_none() {
        return Err(ItemTradeRefusalType::CreditOverflow);
    }
    Ok(ItemTrade {
        count: quantity,
        price_cr,
    })
}

/// Why a credit take moves nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditTakeRefusalType {
    /// The partner is neither neutralized nor lootable: an intact trading
    /// ship is never robbed.
    NotEligible,
    /// The partner holds no credits to take.
    ZeroBalance,
    /// The player's balance after the take would not fit in [`ShipCredits`].
    Overflow,
}

/// Plan taking the docked partner's whole credit balance into the player
/// ship's `own_cr`.
///
/// `eligible` is true when the partner is neutralized or carries
/// [`LootableShipMarker`]; an intact trading partner is never robbed. On
/// success, returns the whole `partner_cr` to add to `own_cr`, leaving the
/// partner at zero; a refusal changes nothing. Checks run in
/// [`CreditTakeRefusalType`] order.
pub fn plan_credit_take(
    eligible: bool,
    own_cr: u32,
    partner_cr: u32,
) -> Result<u32, CreditTakeRefusalType> {
    if !eligible {
        return Err(CreditTakeRefusalType::NotEligible);
    }
    if partner_cr == 0 {
        return Err(CreditTakeRefusalType::ZeroBalance);
    }
    own_cr
        .checked_add(partner_cr)
        .ok_or(CreditTakeRefusalType::Overflow)?;
    Ok(partner_cr)
}

/// A planned Take all from a looted partner: what moves into the ship's
/// hold and balance, and what stays behind for want of room.
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct LootTake {
    /// The count to move of each stack, the partner's hold and a dead
    /// station's storage summed, in id order; each above zero.
    pub taken: BTreeMap<ItemDesignId, u32>,
    /// The part of each `taken` count that comes out of a dead station's
    /// storage; each above zero and at most its `taken` count.
    pub taken_stored: BTreeMap<ItemDesignId, u32>,
    /// The count left behind of each stack the hold has no room for, the
    /// partner's hold and a dead station's storage summed, in id order; each
    /// above zero.
    pub stayed: BTreeMap<ItemDesignId, u32>,
    /// Credits to move: the partner's whole balance, or zero when it holds
    /// none or the ship's balance would overflow.
    pub taken_cr: u32,
    /// Credits left in the partner because the ship's balance would
    /// overflow.
    pub stayed_cr: u32,
}

impl LootTake {
    /// True when the take moves nothing.
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.taken_cr == 0
    }

    /// True when the take leaves nothing behind.
    pub fn is_whole(&self) -> bool {
        self.stayed.is_empty() && self.stayed_cr == 0
    }
}

/// Plan a Take all of `partner`'s hold, a dead station's `storage`, and
/// `partner_cr` into the ship's `own` hold and `own_cr` balance.
///
/// Each hold stack, in id order, moves as many units as the hold still has
/// room for; then each storage stack the same way against the room left;
/// the rest stays. The credits move whole or stay whole: a balance that
/// would overflow the ship's stays. `storage` is `None` for a ship partner
/// and for a living station, whose storage is not loot. Whether the partner
/// may be looted is not read here: [`plan_item_transfer`] and
/// [`plan_credit_take`] check it for each move.
pub fn plan_loot_take(
    items: &GameItems,
    own: &ShipInventory,
    own_cr: u32,
    partner: &ShipInventory,
    partner_cr: u32,
    storage: Option<&StationStorage>,
) -> LootTake {
    let mut take = LootTake::default();
    let mut free_g = u64::from(own.free_g(items));
    let mut fit = |item: &ItemDesignId, held: u32, stored: bool| {
        let mass_g = u64::from(items.design(item).mass_g);
        let fits = if mass_g == 0 {
            held
        } else {
            // At most `held`, so the cast back to u32 is exact.
            u64::from(held).min(free_g / mass_g) as u32
        };
        if fits > 0 {
            free_g -= u64::from(fits) * mass_g;
            *take.taken.entry(item.clone()).or_insert(0) += fits;
            if stored {
                take.taken_stored.insert(item.clone(), fits);
            }
        }
        if fits < held {
            *take.stayed.entry(item.clone()).or_insert(0) += held - fits;
        }
    };
    for (item, held) in partner.stacks() {
        fit(item, held, false);
    }
    for (item, held) in storage.into_iter().flat_map(StationStorage::stacks) {
        fit(item, held, true);
    }
    match plan_credit_take(true, own_cr, partner_cr) {
        Ok(amount) => take.taken_cr = amount,
        Err(CreditTakeRefusalType::Overflow) => take.stayed_cr = partner_cr,
        Err(CreditTakeRefusalType::ZeroBalance | CreditTakeRefusalType::NotEligible) => {}
    }
    take
}

/// Which way a Store or Withdraw at a docked station moves items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum ItemStorageType {
    /// From the docked ship's hold into the station's storage.
    Store,
    /// From the station's storage into the docked ship's hold.
    Withdraw,
}

/// Why a Store or Withdraw moves nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemStorageRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Storage.
    NoStorageService,
    /// No loaded item carries the id.
    UnknownItem,
    /// The quantity text is not a whole number.
    NoQuantity,
    /// A quantity of zero.
    ZeroQuantity,
    /// The source holds fewer than the quantity: the hold for a Store, the
    /// storage for a Withdraw.
    NotEnoughItems {
        /// What the source holds.
        have: u32,
    },
    /// A Store heavier than the room left in the storage.
    StorageFull {
        /// How many more grams the storage takes.
        free_g: u32,
    },
    /// A Withdraw heavier than the room left in the hold.
    HoldFull {
        /// How many more grams the hold takes.
        free_g: u32,
    },
}

/// Plan a Store or Withdraw of `quantity` of `item` between the player ship's
/// `own` hold and its docked station's storage.
///
/// `docked` is true while the ship has a docked partner. `storage` is that
/// partner's [`StationStorage`] when the partner is a living station that
/// offers Storage, and `None` otherwise. Returns the count to remove from
/// the source and add to the target; no credits move. Checks run in
/// [`ItemStorageRefusalType`] order, and a refusal changes nothing.
/// `quantity` is `None` when the typed text is not a whole number.
pub fn plan_item_storage(
    items: &GameItems,
    direction: ItemStorageType,
    docked: bool,
    storage: Option<&StationStorage>,
    item: &ItemDesignId,
    quantity: Option<u32>,
    own: &ShipInventory,
) -> Result<u32, ItemStorageRefusalType> {
    if !docked {
        return Err(ItemStorageRefusalType::NotDocked);
    }
    let storage = storage.ok_or(ItemStorageRefusalType::NoStorageService)?;
    let design = items.get(item).ok_or(ItemStorageRefusalType::UnknownItem)?;
    let quantity = quantity.ok_or(ItemStorageRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemStorageRefusalType::ZeroQuantity);
    }
    let have = match direction {
        ItemStorageType::Store => own.count(item),
        ItemStorageType::Withdraw => storage.count(item),
    };
    if quantity > have {
        return Err(ItemStorageRefusalType::NotEnoughItems { have });
    }
    let mass_g = design.stack_mass_g(quantity);
    match direction {
        ItemStorageType::Store => {
            let free_g = storage.free_g(items);
            if mass_g > u64::from(free_g) {
                return Err(ItemStorageRefusalType::StorageFull { free_g });
            }
        }
        ItemStorageType::Withdraw => {
            let free_g = own.free_g(items);
            if mass_g > u64::from(free_g) {
                return Err(ItemStorageRefusalType::HoldFull { free_g });
            }
        }
    }
    Ok(quantity)
}

/// Maximum mass of a drifting canister, in grams.
pub const CARGO_CANISTER_MAX_MASS_G: u32 = 200_000;

/// Item stacks drifting free in a canister. Every count is above zero and the
/// total mass is at most [`CARGO_CANISTER_MAX_MASS_G`].
///
/// A canister entity is spawned with its [`CargoCanisterRuntimeId`] in the
/// same bundle; adding this component to an entity without one panics. The
/// intake pass, the save and the probe all key a canister by that id, and a
/// canister without it would drift past every intake unseen.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
#[component(on_add = require_canister_runtime_id)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CargoCanister {
    stacks: BTreeMap<ItemDesignId, u32>,
}

/// Refuse a canister entity spawned without its runtime id.
///
/// # Panics
///
/// When the entity has no [`CargoCanisterRuntimeId`] once the bundle that
/// added the canister is in.
fn require_canister_runtime_id(
    world: bevy::ecs::world::DeferredWorld,
    context: bevy::ecs::lifecycle::HookContext,
) {
    assert!(
        world
            .get::<CargoCanisterRuntimeId>(context.entity)
            .is_some(),
        "cargo canister {:?} has no CargoCanisterRuntimeId: spawn it with a minted id in the \
         same bundle",
        context.entity
    );
}

impl CargoCanister {
    /// Start a canister with one valid stack of an item `items` holds.
    pub fn new(items: &GameItems, item: &ItemDesignId, count: u32) -> Self {
        assert!(
            count > 0
                && items.design(item).stack_mass_g(count) <= u64::from(CARGO_CANISTER_MAX_MASS_G),
            "invalid canister stack"
        );
        Self {
            stacks: [(item.clone(), count)].into(),
        }
    }

    /// `count` of `item` packed into new canisters, each filled to the last
    /// whole item under [`CARGO_CANISTER_MAX_MASS_G`] except the last. Empty
    /// for a count of zero.
    ///
    /// # Panics
    ///
    /// When one `item` is heavier than a canister holds; the item lint
    /// refuses that item first.
    pub fn split(items: &GameItems, item: &ItemDesignId, count: u32) -> Vec<Self> {
        let per_canister = CARGO_CANISTER_MAX_MASS_G / items.design(item).mass_g;
        assert!(per_canister > 0, "item '{item}' is heavier than a canister");
        let mut rest = count;
        let mut canisters = Vec::new();
        while rest > 0 {
            let packed = rest.min(per_canister);
            canisters.push(Self::new(items, item, packed));
            rest -= packed;
        }
        canisters
    }

    /// Every held stack in id order.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.stacks.iter().map(|(item, &count)| (item, count))
    }

    /// Total mass in grams.
    pub fn total_mass_g(&self, items: &GameItems) -> u32 {
        self.stacks()
            .map(|(item, count)| items.design(item).mass_g * count)
            .sum()
    }

    /// Add a stack after the jettison rule has checked the resulting mass.
    pub fn add(&mut self, items: &GameItems, item: &ItemDesignId, count: u32) {
        assert!(
            count > 0
                && u64::from(self.total_mass_g(items)) + items.design(item).stack_mass_g(count)
                    <= u64::from(CARGO_CANISTER_MAX_MASS_G),
            "canister mass exceeded"
        );
        *self.stacks.entry(item.clone()).or_default() += count;
    }
}

/// Runtime identity for one cargo canister entity, minted the moment it is
/// spawned. A canister has no `EntityId`, so this is its durable key: a saved
/// torpedo that tracks a canister keeps it as [`SavedTargetRef::Canister`],
/// and the Load finds the canister again by this value. An external reader
/// (bench, probe, logs) also uses it to tell two canisters apart without
/// `Entity` (recycled) or `Name` (every canister shares `"Cargo Canister"`).
///
/// [`SavedTargetRef::Canister`]: crate::saved_refs::SavedTargetRef::Canister
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CargoCanisterRuntimeId(pub u64);

/// Mints unique [`CargoCanisterRuntimeId`]s. Owned and initialized only by
/// `NovaGameplayPlugin`: the counter is never reset by a scenario load, retry,
/// or New Game for as long as the app process runs. No ID can be reused even
/// if a canister survives a world transition. A saved world stores
/// [`Self::next_unminted`], and loading it calls [`Self::resume_after`], so a
/// canister it froze keeps an id no later mint repeats.
#[derive(Resource, Default, Debug)]
pub struct CargoCanisterIdAllocator(u64);

impl CargoCanisterIdAllocator {
    /// Mints an id unused by any earlier call.
    ///
    /// # Panics
    ///
    /// On exhausting `u64`: an id is never reused, so a counter that wrapped
    /// would silently collide with a still-live canister instead.
    pub fn mint(&mut self) -> CargoCanisterRuntimeId {
        let id = self.0;
        self.0 = self
            .0
            .checked_add(1)
            .expect("CargoCanisterIdAllocator exhausted u64 ids");
        CargoCanisterRuntimeId(id)
    }

    /// The id the next [`Self::mint`] returns.
    pub fn next_unminted(&self) -> CargoCanisterRuntimeId {
        CargoCanisterRuntimeId(self.0)
    }

    /// Never mint below `saved_next`, the [`Self::next_unminted`] a saved
    /// world stored. Keeps the higher counter: ids this process minted
    /// before the load are not reused either.
    pub fn resume_after(&mut self, saved_next: CargoCanisterRuntimeId) {
        self.0 = self.0.max(saved_next.0);
    }
}

/// Why a jettison drops nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemJettisonRefusalType {
    /// The ship is docked: a canister would leave into the docked pair.
    Docked,
    /// The ship has no live cargo intake to drop the canister through.
    NoIntake,
    /// The quantity text is not a whole number.
    NoQuantity,
    /// A quantity of zero.
    ZeroQuantity,
    /// The ship carries fewer than the quantity.
    Short {
        /// What the ship carries.
        held: u32,
    },
}

/// A planned jettison. The whole count leaves the ship: `merged` joins the
/// intake's last waiting canister, and `canisters` queue after it in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemJettison {
    /// The count to remove from the ship: the whole quantity.
    pub count: u32,
    /// The count to add to the last waiting canister; zero with no waiting
    /// canister or no room in it.
    pub merged: u32,
    /// New canisters, each filled to the last whole item under
    /// [`CARGO_CANISTER_MAX_MASS_G`] except the last.
    pub canisters: Vec<CargoCanister>,
}

/// Plan a jettison of `quantity` of `item` from the player ship's `own`
/// inventory.
///
/// `docked` is true while the ship is docked, `has_intake` while it has a live
/// cargo intake, and `tail` is the last canister waiting on that intake.
/// Checks run in [`ItemJettisonRefusalType`] order. `quantity` is `None` when
/// the typed text is not a whole number.
pub fn plan_item_jettison(
    items: &GameItems,
    docked: bool,
    has_intake: bool,
    tail: Option<&CargoCanister>,
    item: &ItemDesignId,
    quantity: Option<u32>,
    own: &ShipInventory,
) -> Result<ItemJettison, ItemJettisonRefusalType> {
    if docked {
        return Err(ItemJettisonRefusalType::Docked);
    }
    if !has_intake {
        return Err(ItemJettisonRefusalType::NoIntake);
    }
    let quantity = quantity.ok_or(ItemJettisonRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemJettisonRefusalType::ZeroQuantity);
    }
    let held = own.count(item);
    if quantity > held {
        return Err(ItemJettisonRefusalType::Short { held });
    }
    let mass_g = items.design(item).mass_g;
    let tail_room = tail.map_or(0, |tail| {
        (CARGO_CANISTER_MAX_MASS_G - tail.total_mass_g(items)) / mass_g
    });
    let merged = quantity.min(tail_room);
    Ok(ItemJettison {
        count: quantity,
        merged,
        canisters: CargoCanister::split(items, item, quantity - merged),
    })
}

/// Health one hull plate restores. Restore capacity a repair does not use is
/// lost, not banked.
pub const HULL_PLATE_HEALTH: f32 = 20.0;

/// The plates a repair spends and the Health the section ends at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateRepair {
    /// Hull plates to remove from the ship's inventory; above zero.
    pub plates: u32,
    /// The section's `Health::current` after the repair.
    pub current: f32,
}

/// Why a section takes no plate repair, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlateRepairRefusalType {
    /// No `Health`, or a max of zero or less.
    NoIntegrity,
    /// At zero Health, or disabled by the integrity core with Health left.
    Destroyed,
    /// Already at max Health.
    Full,
    /// The requested quantity is zero or live stock is empty.
    NoPlates,
    /// Live stock is below the selected quantity.
    InsufficientPlates,
    /// Live missing integrity needs fewer plates than selected.
    ExcessPlates,
}

/// Plan a plate repair for a selected whole-plate quantity against live stock.
///
/// Refuse an outdated selection rather than silently spending fewer plates.
/// `disabled` is true when the section carries `IntegrityDisabledMarker`.
pub fn plan_plate_repair(
    health: Option<&Health>,
    disabled: bool,
    requested_plates: u32,
    stock: u32,
) -> Result<PlateRepair, PlateRepairRefusalType> {
    let health = health
        .filter(|health| health.max > 0.0)
        .ok_or(PlateRepairRefusalType::NoIntegrity)?;
    if health.current <= 0.0 || disabled {
        return Err(PlateRepairRefusalType::Destroyed);
    }
    if health.current >= health.max {
        return Err(PlateRepairRefusalType::Full);
    }
    if requested_plates == 0 || stock == 0 {
        return Err(PlateRepairRefusalType::NoPlates);
    }
    if requested_plates > stock {
        return Err(PlateRepairRefusalType::InsufficientPlates);
    }
    let needed = ((health.max - health.current) / HULL_PLATE_HEALTH).ceil() as u32;
    if requested_plates > needed {
        return Err(PlateRepairRefusalType::ExcessPlates);
    }
    Ok(PlateRepair {
        plates: requested_plates,
        current: health
            .max
            .min(health.current + HULL_PLATE_HEALTH * requested_plates as f32),
    })
}

/// What a station charges to build one section, and what stripping the
/// section would give back. Authored as the required `build` field of every
/// section config.
///
/// The value rule keeps a build from paying: content lint refuses a recipe
/// whose strip value is not below its recipe value, see
/// [`value_cr`](Self::value_cr) and [`strip_value_cr`](Self::strip_value_cr).
/// The `Default` is the empty recipe that lint refuses; it exists for code
/// built test sections, never for authored content.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BuildRecipe {
    /// Credits the station takes for the work, above zero.
    pub credits: u32,
    /// Part items the build consumes, as a count per item id. Each item is in
    /// [`ItemCategoryType::Parts`] and each count is above zero.
    pub parts: BTreeMap<ItemDesignId, u32>,
    /// The share of each part count a strip returns, rounded down per part,
    /// in `(0, 1]`.
    pub strip_yield: f32,
}

impl BuildRecipe {
    /// The recipe at ask: its credits plus every part at
    /// [`ItemDesign::ask_cr`]. Wide so no authored count overflows it.
    ///
    /// # Panics
    ///
    /// When `items` lacks a part; content lint refuses that recipe first.
    pub fn value_cr(&self, items: &GameItems) -> u64 {
        u64::from(self.credits)
            + self
                .parts
                .iter()
                .map(|(item, count)| u64::from(*count) * u64::from(items.design(item).ask_cr))
                .sum::<u64>()
    }

    /// The parts a strip of a whole section returns:
    /// [`strip_parts_at`](Self::strip_parts_at) at full health.
    pub fn strip_parts(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.strip_parts_at(1.0)
    }

    /// The parts a strip of a section at health `share` (current over max)
    /// returns: `floor(count * strip_yield * share)` of each part, rounded
    /// down once, in id order. A share outside `[0, 1]` is clamped, so a
    /// strip never returns more than [`strip_parts`](Self::strip_parts). A
    /// part whose share rounds down to zero is left out.
    pub fn strip_parts_at(&self, share: f32) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        let share = f64::from(share.clamp(0.0, 1.0));
        self.parts.iter().filter_map(move |(item, &count)| {
            // `strip_yield` and `share` are in `[0, 1]`, so the product is at
            // most `count`.
            let returned = (f64::from(count) * f64::from(self.strip_yield) * share).floor() as u32;
            (returned > 0).then_some((item, returned))
        })
    }

    /// How many part units the recipe consumes across all parts.
    pub fn part_count(&self) -> u32 {
        self.parts.values().sum()
    }

    /// What a strip returns at bid: [`Self::strip_parts`] at
    /// [`ItemDesign::bid_cr`].
    ///
    /// # Panics
    ///
    /// When `items` lacks a part; content lint refuses that recipe first.
    pub fn strip_value_cr(&self, items: &GameItems) -> u64 {
        self.strip_parts()
            .map(|(item, count)| u64::from(count) * u64::from(items.design(item).bid_cr))
            .sum()
    }
}

/// The shortest strip, in game seconds: a section of one or two parts still
/// takes a moment to unbolt.
pub const STRIP_MIN_SECS: f32 = 3.0;

/// Game seconds a strip takes per recipe part unit: the recipe is what is
/// taken apart, so a capital drive of forty parts takes forty seconds.
pub const STRIP_SECS_PER_PART: f32 = 1.0;

/// How long stripping a section built by `recipe` takes, in game seconds:
/// [`STRIP_SECS_PER_PART`] per part unit, at least [`STRIP_MIN_SECS`].
pub fn strip_duration_secs(recipe: &BuildRecipe) -> f32 {
    (recipe.part_count() as f32 * STRIP_SECS_PER_PART).max(STRIP_MIN_SECS)
}

/// One live section of the docked partner, as a strip reads it.
#[derive(Clone, Copy, Debug)]
pub struct StripTarget<'a> {
    /// The section's build recipe.
    pub recipe: &'a BuildRecipe,
    /// Its health, current over max.
    pub health_share: f32,
}

impl StripTarget<'_> {
    /// The parts a strip of this section returns:
    /// [`BuildRecipe::strip_parts_at`] its health share.
    pub fn parts(&self) -> BTreeMap<ItemDesignId, u32> {
        self.recipe
            .strip_parts_at(self.health_share)
            .map(|(item, count)| (item.clone(), count))
            .collect()
    }
}

/// The docked partner a whole-hull strip reads.
#[derive(Clone, Debug)]
pub struct HullStripPartner<'a> {
    /// The partner is a station that is not a derelict.
    pub living_station: bool,
    /// The partner carries [`OwnedStationMarker`](crate::markers::OwnedStationMarker):
    /// the player's own station, dead.
    pub owned_station: bool,
    /// The partner may be looted: it is neutralized or carries
    /// [`LootableShipMarker`], the rule a Take reads.
    pub lootable: bool,
    /// Hulls the player owns parked at the partner: each would despawn with
    /// it.
    pub owned_hulls: u32,
    /// Every live section of the partner, the berth port included.
    pub sections: Vec<StripTarget<'a>>,
    /// The Take all of the partner's hold, a dead station's storage and its
    /// balance a strip runs first, by [`plan_loot_take`].
    pub loot: LootTake,
}

impl HullStripPartner<'_> {
    /// The parts and time a strip of every live section gives, with no rule
    /// checked: what the strip block shows before a strip may start.
    pub fn plan(&self) -> HullStrip {
        let mut parts = BTreeMap::new();
        let mut duration_secs = 0.0;
        for section in &self.sections {
            for (item, count) in section.parts() {
                *parts.entry(item).or_insert(0) += count;
            }
            duration_secs += strip_duration_secs(section.recipe);
        }
        HullStrip {
            parts,
            duration_secs,
        }
    }
}

/// A planned whole-hull strip: the parts it is expected to put in the hold
/// and how long it takes, summed over every live section.
#[derive(Clone, Debug, PartialEq)]
pub struct HullStrip {
    /// Each live section's [`StripTarget::parts`], summed. A section that
    /// yields no whole part adds nothing but still comes off.
    pub parts: BTreeMap<ItemDesignId, u32>,
    /// Game seconds the strip takes: each section's
    /// [`strip_duration_secs`], summed.
    pub duration_secs: f32,
}

/// Why a whole-hull strip does not start, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HullStripRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The ship is already stripping a hull.
    Busy,
    /// The partner is a living station: it trades, it is not salvage.
    LivingStation,
    /// The partner is the player's own station: a strip would end the
    /// ownership with no record of it.
    OwnedStation,
    /// The partner is neither neutralized nor lootable.
    NotLootable,
    /// Hulls the player owns are parked at the partner: they would despawn
    /// with it.
    OwnedHullsParked {
        /// The parked hulls the player owns.
        count: u32,
    },
    /// The Take all a strip runs first would leave part of the partner's
    /// hold, a dead station's storage or its balance behind:
    /// [`HullStripPartner::loot`] is not whole. A strip never demolishes a
    /// hull with loot still in it.
    NoRoom,
}

/// Why a running strip holds at a finished section instead of taking it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HullStripPauseType {
    /// The section's parts weigh more than the ship's hold has room for.
    HoldFull {
        /// Grams the parts weigh.
        need_g: u64,
        /// Grams the hold has room for.
        free_g: u32,
    },
}

/// Plan stripping the whole docked partner.
///
/// `partner` is `None` while the ship is not docked. `busy` is whether the
/// ship is already stripping. Checks run in [`HullStripRefusalType`] order,
/// and a refusal changes nothing. A strip starts by taking the partner's
/// whole hold and balance ([`HullStripPartner::loot`]), so it refuses when
/// that take would leave anything behind. The plan's parts are what the
/// strip gives if no section is damaged meanwhile; the hold is not checked
/// for them here, because a strip that fills the hold pauses
/// ([`plan_section_yield`]) rather than refusing.
pub fn plan_hull_strip(
    partner: Option<&HullStripPartner<'_>>,
    busy: bool,
) -> Result<HullStrip, HullStripRefusalType> {
    let partner = partner.ok_or(HullStripRefusalType::NotDocked)?;
    if busy {
        return Err(HullStripRefusalType::Busy);
    }
    if partner.living_station {
        return Err(HullStripRefusalType::LivingStation);
    }
    if partner.owned_station {
        return Err(HullStripRefusalType::OwnedStation);
    }
    if !partner.lootable {
        return Err(HullStripRefusalType::NotLootable);
    }
    if partner.owned_hulls > 0 {
        return Err(HullStripRefusalType::OwnedHullsParked {
            count: partner.owned_hulls,
        });
    }
    if !partner.loot.is_whole() {
        return Err(HullStripRefusalType::NoRoom);
    }
    Ok(partner.plan())
}

/// The parts one finished section puts in the ship's `own` hold, or why the
/// strip holds there: the parts do not fit. An empty map is a section that
/// yields no whole part; it comes off all the same.
pub fn plan_section_yield(
    items: &GameItems,
    target: StripTarget<'_>,
    own: &ShipInventory,
) -> Result<BTreeMap<ItemDesignId, u32>, HullStripPauseType> {
    let parts = target.parts();
    let need_g: u64 = parts
        .iter()
        .map(|(item, &count)| items.design(item).stack_mass_g(count))
        .sum();
    let free_g = own.free_g(items);
    if need_g > u64::from(free_g) {
        return Err(HullStripPauseType::HoldFull { need_g, free_g });
    }
    Ok(parts)
}

/// The docked partner a station service reads: whether it offers the
/// service, which section prototypes it builds, the stock missing parts
/// are bought from and its balance.
#[derive(Clone, Copy, Debug)]
pub struct StationPartner<'a> {
    /// The partner is a living station whose design offers the service.
    pub offers: bool,
    /// The section prototype ids the partner builds (its `StationBuilds`);
    /// empty for a partner that is no station, which `offers` nothing.
    pub builds: &'a [String],
    /// The partner's hold.
    pub stock: &'a ShipInventory,
    /// The partner's credits.
    pub credits_cr: u32,
}

/// A planned [`BuildRecipe`] payment: the parts to take from each hold and
/// the credits to move from the ship to the station.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipePayment {
    /// Parts to remove from the ship's hold.
    pub from_hold: BTreeMap<ItemDesignId, u32>,
    /// Parts to remove from the station's hold, paid at ask.
    pub bought: BTreeMap<ItemDesignId, u32>,
    /// Credits to remove from the ship and add to the station: the recipe's
    /// credits plus the bought parts at ask.
    pub price_cr: u32,
}

/// Why a recipe cannot be paid, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipePaymentRefusalType {
    /// Neither hold has enough of a part.
    MissingParts {
        /// The first part, in id order, the holds cannot cover.
        item: ItemDesignId,
        /// How many more of it the build needs.
        short: u32,
    },
    /// The ship holds fewer credits than the price.
    InsufficientCredits {
        /// The price: the recipe's credits plus the bought parts at ask.
        need: u64,
        /// What the ship holds.
        have: u32,
    },
    /// The station's balance after the payment would not fit in
    /// [`ShipCredits`].
    CreditOverflow,
}

/// Plan paying `recipe` at a station whose hold is `stock` and whose balance
/// is `stock_cr`, from the ship's `own` hold and `own_cr` balance.
///
/// Parts come from `own` first; the rest are bought from `stock` at
/// [`ItemDesign::ask_cr`]. The price is the recipe's credits plus the bought
/// parts. Checks run in [`RecipePaymentRefusalType`] order. Every part
/// resolves in `items`. The one payment rule a rebuild and a swap share.
pub fn plan_recipe_payment(
    items: &GameItems,
    recipe: &BuildRecipe,
    stock: &ShipInventory,
    stock_cr: u32,
    own: &ShipInventory,
    own_cr: u32,
) -> Result<RecipePayment, RecipePaymentRefusalType> {
    let mut from_hold = BTreeMap::new();
    let mut bought = BTreeMap::new();
    let mut price = u64::from(recipe.credits);
    for (item, &count) in &recipe.parts {
        let held = own.count(item).min(count);
        let missing = count - held;
        let stocked = stock.count(item);
        if missing > stocked {
            return Err(RecipePaymentRefusalType::MissingParts {
                item: item.clone(),
                short: missing - stocked,
            });
        }
        if held > 0 {
            from_hold.insert(item.clone(), held);
        }
        if missing > 0 {
            bought.insert(item.clone(), missing);
            price += u64::from(missing) * u64::from(items.design(item).ask_cr);
        }
    }
    if price > u64::from(own_cr) {
        return Err(RecipePaymentRefusalType::InsufficientCredits {
            need: price,
            have: own_cr,
        });
    }
    // The price fits: it is at most the ship's u32 balance.
    let price_cr = price as u32;
    if stock_cr.checked_add(price_cr).is_none() {
        return Err(RecipePaymentRefusalType::CreditOverflow);
    }
    Ok(RecipePayment {
        from_hold,
        bought,
        price_cr,
    })
}

/// Why a rebuild changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectionRebuildRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Rebuild.
    NoRebuildService,
    /// The station does not build the section's prototype, whatever parts
    /// the holds carry.
    NotBuilt,
    /// The section is live on the ship.
    SectionNotLost,
    /// No live section mates with the section where the design places it.
    Unanchored,
    /// The recipe cannot be paid, see [`plan_recipe_payment`].
    Payment(RecipePaymentRefusalType),
}

/// Plan rebuilding one lost section, built from the prototype `prototype`
/// by its `recipe`, while docked with `partner`.
///
/// `partner` is `None` while the ship is not docked; its `offers` is whether
/// it is a living station that offers Rebuild, and its `builds` must name
/// `prototype`: a lost section is crafted again, and a station crafts only
/// what it builds. `lost` is true when the design section is not live on the
/// ship, and `anchored` when a live section mates with it where the design
/// places it. The recipe is paid by [`plan_recipe_payment`]. Checks run in
/// [`SectionRebuildRefusalType`] order, and a refusal changes nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "the recipe, its prototype, the partner, the section's state and the hold"
)]
pub fn plan_section_rebuild(
    items: &GameItems,
    prototype: &str,
    recipe: &BuildRecipe,
    partner: Option<StationPartner<'_>>,
    lost: bool,
    anchored: bool,
    own: &ShipInventory,
    own_cr: u32,
) -> Result<RecipePayment, SectionRebuildRefusalType> {
    let partner = partner.ok_or(SectionRebuildRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(SectionRebuildRefusalType::NoRebuildService);
    }
    if !partner.builds.iter().any(|known| known == prototype) {
        return Err(SectionRebuildRefusalType::NotBuilt);
    }
    if !lost {
        return Err(SectionRebuildRefusalType::SectionNotLost);
    }
    if !anchored {
        return Err(SectionRebuildRefusalType::Unanchored);
    }
    plan_recipe_payment(
        items,
        recipe,
        partner.stock,
        partner.credits_cr,
        own,
        own_cr,
    )
    .map_err(SectionRebuildRefusalType::Payment)
}

// Hand-written rather than derived: a derived `Deserialize` would accept a
// zero-quantity stack, and a `BTreeMap` would keep the last of a repeated key,
// so only the panicking `ShipInventoryStock::new` would ever catch either,
// which is not how content lint reports an authoring error. Both run
// `ShipInventoryStock::checked`, so both name the fault the same way.
#[cfg(feature = "serde")]
impl serde::Serialize for ShipInventoryStock {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.stacks.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShipInventoryStock {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Reads the map as pairs so a repeated key reaches the check.
        struct StockVisitor;

        impl<'de> serde::de::Visitor<'de> for StockVisitor {
            type Value = ShipInventoryStock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map of item id to stack count")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut stacks = Vec::new();
                while let Some(entry) = map.next_entry::<ItemDesignId, u32>()? {
                    stacks.push(entry);
                }
                ShipInventoryStock::checked(stacks).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_map(StockVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The credits choke point reports every real move once and a no-op move
    /// never: a trade at price zero or a Take from an empty strongbox must
    /// not wake an `OnCreditsChanged` handler.
    #[test]
    fn change_credits_reports_each_real_move_once_and_skips_a_no_op() {
        let mut reports = ShipInventoryReports::default();
        let mut credits = ShipCredits(50);

        reports.change_credits(&mut credits, 50, CreditChangeCauseType::Trade);
        assert_eq!(credits, ShipCredits(50));
        assert!(reports.0.is_empty(), "an unchanged balance reports nothing");

        reports.change_credits(&mut credits, 80, CreditChangeCauseType::Trade);
        reports.change_credits(&mut credits, 0, CreditChangeCauseType::Take);
        assert_eq!(credits, ShipCredits(0));
        assert_eq!(
            reports.0,
            vec![
                ShipInventoryReport::CreditsChanged {
                    previous_cr: 50,
                    current_cr: 80,
                    cause: CreditChangeCauseType::Trade,
                },
                ShipInventoryReport::CreditsChanged {
                    previous_cr: 80,
                    current_cr: 0,
                    cause: CreditChangeCauseType::Take,
                },
            ]
        );
    }

    /// A Store moves hold items into the storage and a Withdraw moves them
    /// back, the count planned and no credit in either; each refusal comes
    /// in check order and names what it read.
    #[test]
    fn storage_plans_store_and_withdraw_and_refuses_in_order() {
        use ItemStorageRefusalType::*;
        use ItemStorageType::{Store, Withdraw};

        let items = crate::test_support::test_items();
        let panel = ItemDesignId::from("SectionPanel");
        // 30 kg hold carrying three 10 kg panels; a 25 kg store.
        let mut own = ShipInventory::new(&items, 30_000, [(panel.clone(), 3)]);
        let mut storage = StationStorage::new(25_000);
        let plan = |direction,
                    docked,
                    storage: Option<&StationStorage>,
                    item: &ItemDesignId,
                    quantity,
                    own: &ShipInventory| {
            plan_item_storage(&items, direction, docked, storage, item, quantity, own)
        };

        assert_eq!(
            plan(Store, false, Some(&storage), &panel, Some(1), &own),
            Err(NotDocked)
        );
        assert_eq!(
            plan(Store, true, None, &panel, Some(1), &own),
            Err(NoStorageService)
        );
        assert_eq!(
            plan(
                Store,
                true,
                Some(&storage),
                &"NoSuchItem".into(),
                Some(1),
                &own
            ),
            Err(UnknownItem)
        );
        assert_eq!(
            plan(Store, true, Some(&storage), &panel, None, &own),
            Err(NoQuantity)
        );
        assert_eq!(
            plan(Store, true, Some(&storage), &panel, Some(0), &own),
            Err(ZeroQuantity)
        );
        assert_eq!(
            plan(Store, true, Some(&storage), &panel, Some(4), &own),
            Err(NotEnoughItems { have: 3 })
        );
        assert_eq!(
            plan(Store, true, Some(&storage), &panel, Some(3), &own),
            Err(StorageFull { free_g: 25_000 })
        );

        let stored = plan(Store, true, Some(&storage), &panel, Some(2), &own).expect("two fit");
        assert_eq!(stored, 2);
        own.remove(&panel, stored);
        storage.add(&items, &panel, stored);
        assert_eq!((own.count(&panel), storage.count(&panel)), (1, 2));
        assert_eq!(storage.free_g(&items), 5_000);
        assert_eq!(
            plan(Store, true, Some(&storage), &panel, Some(1), &own),
            Err(StorageFull { free_g: 5_000 })
        );

        assert_eq!(
            plan(Withdraw, true, Some(&storage), &panel, Some(3), &own),
            Err(NotEnoughItems { have: 2 })
        );
        // Fill the hold to one free panel's room: the second withdrawn panel
        // has nowhere to go.
        let full = ShipInventory::new(&items, 30_000, [(panel.clone(), 2)]);
        assert_eq!(
            plan(Withdraw, true, Some(&storage), &panel, Some(2), &full),
            Err(HoldFull { free_g: 10_000 })
        );
        let withdrawn = plan(Withdraw, true, Some(&storage), &panel, Some(2), &own)
            .expect("the hold takes both back");
        storage.remove(&panel, withdrawn);
        own.add(&items, &panel, withdrawn);
        assert_eq!(own.count(&panel), 3);
        assert!(storage.is_empty(), "an emptied stack is deleted");
    }

    #[test]
    fn kg_text_shows_grams_as_kilograms_without_trailing_zeros() {
        assert_eq!(kg_text(0), "0 kg");
        let items = crate::test_support::test_items();
        let mass_g = |role: ItemRoleType| u64::from(items.role(role).mass_g);
        assert_eq!(kg_text(mass_g(ItemRoleType::PdcRound)), "0.2 kg");
        assert_eq!(kg_text(mass_g(ItemRoleType::HullPlate)), "10 kg");
        assert_eq!(kg_text(1_050), "1.05 kg");
        assert_eq!(kg_text(3_520_001), "3520.001 kg");
        // 6000 PDC rounds weigh exactly 1200 kg: no float drift in the sum.
        let rounds = ShipInventory::new(&items, 1_200_000, [(ItemRoleType::PdcRound.into(), 6000)]);
        assert_eq!(rounds.used_g(&items), 1_200_000);
        assert_eq!(rounds.free_g(&items), 0);
    }

    #[test]
    fn cargo_canister_id_allocator_mints_unique_increasing_ids() {
        let mut allocator = CargoCanisterIdAllocator::default();
        assert_eq!(allocator.mint(), CargoCanisterRuntimeId(0));
        assert_eq!(allocator.mint(), CargoCanisterRuntimeId(1));
        assert_eq!(allocator.mint(), CargoCanisterRuntimeId(2));
    }

    /// A loaded world's canisters keep their ids: minting resumes past the
    /// saved counter, and past this process's own when that is higher.
    #[test]
    fn cargo_canister_id_allocator_resumes_past_a_saved_world() {
        let mut saved = CargoCanisterIdAllocator::default();
        let frozen: Vec<_> = (0..5).map(|_| saved.mint()).collect();

        let mut fresh = CargoCanisterIdAllocator::default();
        fresh.resume_after(saved.next_unminted());
        assert!(!frozen.contains(&fresh.mint()));
        assert_eq!(fresh.next_unminted(), CargoCanisterRuntimeId(6));

        let mut busy = CargoCanisterIdAllocator(40);
        busy.resume_after(saved.next_unminted());
        assert_eq!(busy.mint(), CargoCanisterRuntimeId(40));
    }

    #[test]
    #[should_panic(expected = "exhausted u64 ids")]
    fn cargo_canister_id_allocator_panics_rather_than_wrap_and_reuse_an_id() {
        let mut allocator = CargoCanisterIdAllocator(u64::MAX);
        allocator.mint();
    }

    /// A canister with no runtime id would drift past every intake unseen, so
    /// the spawn that made it fails; one spawned with its id stands.
    #[test]
    fn a_canister_spawned_without_a_runtime_id_panics_at_the_spawn() {
        let items = crate::test_support::test_items();
        let canister = CargoCanister::new(&items, &ItemDesignId::from(ItemRoleType::HullPlate), 1);
        let mut world = World::new();
        let mut allocator = CargoCanisterIdAllocator::default();
        world.spawn((canister.clone(), allocator.mint()));

        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            world.spawn(canister);
        }))
        .expect_err("a canister without an id must not spawn");
        let message = refused
            .downcast::<String>()
            .expect("a formatted panic message");
        assert!(
            message.contains("has no CargoCanisterRuntimeId"),
            "{message}"
        );
    }

    #[test]
    fn plate_repair_spends_selected_whole_plates_or_refuses_stale_selection() {
        let plan = |current: f32, max: f32, disabled: bool, selected: u32, stock: u32| {
            plan_plate_repair(Some(&Health { current, max }), disabled, selected, stock)
        };
        let repaired = |plates, current| Ok(PlateRepair { plates, current });

        assert_eq!(plan(40.0, 100.0, false, 1, 3), repaired(1, 60.0));
        assert_eq!(plan(40.0, 100.0, false, 2, 3), repaired(2, 80.0));
        assert_eq!(plan(40.0, 100.0, false, 3, 3), repaired(3, 100.0));
        assert_eq!(plan(99.5, 100.0, false, 1, 12), repaired(1, 100.0));
        assert_eq!(plan(55.0, 100.0, false, 3, 3), repaired(3, 100.0));

        use PlateRepairRefusalType::*;
        assert_eq!(plan(100.0, 100.0, false, 1, 12), Err(Full));
        assert_eq!(plan(0.0, 100.0, false, 1, 12), Err(Destroyed));
        assert_eq!(plan(50.0, 100.0, true, 1, 12), Err(Destroyed));
        assert_eq!(plan(50.0, 100.0, false, 1, 0), Err(NoPlates));
        assert_eq!(plan(50.0, 100.0, false, 2, 1), Err(InsufficientPlates));
        assert_eq!(plan(90.0, 100.0, false, 2, 12), Err(ExcessPlates));
        assert_eq!(plan(0.0, 0.0, false, 1, 12), Err(NoIntegrity));
        assert_eq!(plan_plate_repair(None, false, 1, 12), Err(NoIntegrity));
        assert_eq!(plan(100.0, 100.0, false, 1, 0), Err(Full));
    }
}

#[cfg(test)]
mod transfer_tests {
    use super::*;

    #[test]
    fn item_transfer_plans_refuse_in_order_and_never_pass_capacity() {
        let items = crate::test_support::test_items();
        use ItemTransferRefusalType::*;
        use ItemTransferType::*;
        // 400 kg holds 40 plates of 10 kg.
        let plates = |count: u32| {
            ShipInventory::new(&items, 400_000, [(ItemRoleType::HullPlate.into(), count)])
        };
        let plan = |transfer, lootable, quantity, own: &ShipInventory, partner: &ShipInventory| {
            plan_item_transfer(
                &items,
                transfer,
                lootable,
                &ItemDesignId::from(ItemRoleType::HullPlate),
                quantity,
                own,
                partner,
            )
        };
        let (own, partner) = (plates(12), plates(8));

        assert_eq!(plan(Take, true, Some(3), &own, &partner), Ok(3));
        assert_eq!(plan(Take, true, Some(8), &own, &partner), Ok(8));
        assert_eq!(plan(Give, false, Some(12), &own, &partner), Ok(12));
        // Take needs a lootable partner; Give never reads it.
        assert_eq!(plan(Take, false, Some(1), &own, &partner), Err(NotLootable));
        // Authorization wins over the quantity.
        assert_eq!(plan(Take, false, None, &own, &partner), Err(NotLootable));
        assert_eq!(plan(Give, false, None, &own, &partner), Err(NoQuantity));
        assert_eq!(
            plan(Give, false, Some(0), &own, &partner),
            Err(ZeroQuantity)
        );
        assert_eq!(
            plan(Take, true, Some(9), &own, &partner),
            Err(Short { held: 8 })
        );
        assert_eq!(
            plan(Give, false, Some(1), &ShipInventory::default(), &partner),
            Err(Short { held: 0 })
        );
        // Room is checked after stock: 350 kg held leaves 50 kg, 5 of the 8.
        let nearly_full = plates(35);
        assert_eq!(
            plan(Take, true, Some(6), &nearly_full, &partner),
            Err(NoRoom { free_g: 50_000 })
        );
        assert_eq!(plan(Take, true, Some(5), &nearly_full, &partner), Ok(5));
        assert_eq!(
            plan(Take, true, Some(9), &nearly_full, &partner),
            Err(Short { held: 8 })
        );
        assert_eq!(
            plan(Give, false, Some(1), &own, &ShipInventory::default()),
            Err(NoRoom { free_g: 0 })
        );

        // A planned move conserves the total and empties a drained stack.
        let (mut own, mut partner) = (own, partner);
        let moved = plan(Take, true, Some(8), &own, &partner).expect("planned");
        partner.remove(&ItemDesignId::from(ItemRoleType::HullPlate), moved);
        own.add(&items, &ItemDesignId::from(ItemRoleType::HullPlate), moved);
        assert_eq!(own.count(&ItemDesignId::from(ItemRoleType::HullPlate)), 20);
        assert_eq!(own.used_g(&items), 200_000);
        assert_eq!(own.free_g(&items), 200_000);
        assert!(partner.is_empty());
    }

    #[test]
    fn item_trade_plans_refuse_in_order_and_conserve_items_and_credits() {
        let items = crate::test_support::test_items();
        use ItemTradeRefusalType::*;
        use ItemTradeType::*;
        // 400 kg of room; ore is 10 kg, ask 4 and bid 3 for stone.
        let stone_price = items.role(ItemRoleType::StoneOre).reference_price();
        let ore = |count: u32| {
            ShipInventory::new(&items, 400_000, [(ItemRoleType::StoneOre.into(), count)])
        };
        let plan = |trade,
                    trades: bool,
                    quantity,
                    own: &ShipInventory,
                    own_cr,
                    partner: &ShipInventory,
                    partner_cr| {
            plan_item_trade(
                &items,
                trade,
                trades.then_some(TradePriceType::Market(stone_price)),
                &ItemDesignId::from(ItemRoleType::StoneOre),
                quantity,
                own,
                own_cr,
                partner,
                partner_cr,
            )
        };
        let (own, partner) = (ore(10), ore(30));

        assert_eq!(
            plan(Buy, true, Some(5), &own, 100, &partner, 0),
            Ok(ItemTrade {
                count: 5,
                price_cr: 20
            })
        );
        assert_eq!(
            plan(Sell, true, Some(10), &own, 0, &partner, 30),
            Ok(ItemTrade {
                count: 10,
                price_cr: 30
            })
        );
        // A station the player owns sells its stock at bid, not ask.
        assert_eq!(
            plan_item_trade(
                &items,
                Buy,
                Some(TradePriceType::Owned(stone_price)),
                &ItemDesignId::from(ItemRoleType::StoneOre),
                Some(5),
                &own,
                100,
                &partner,
                0,
            ),
            Ok(ItemTrade {
                count: 5,
                price_cr: 15
            })
        );
        // Exact funds and exact room are valid; both balances may reach zero.
        let mut exact = ShipInventory::new(&items, 50_000, []);
        let mut seller = ore(5);
        let (mut buyer_cr, mut seller_cr) = (20, 0);
        assert_eq!(
            plan(Buy, true, Some(5), &exact, buyer_cr, &seller, seller_cr),
            Ok(ItemTrade {
                count: 5,
                price_cr: 20
            })
        );
        assert_eq!(
            plan(
                Buy,
                true,
                Some(5),
                &ShipInventory::new(&items, 49_999, []),
                20,
                &seller,
                0
            ),
            Err(NoRoom { free_g: 49_999 })
        );
        assert_eq!(
            plan(Buy, true, Some(5), &exact, 19, &seller, 0),
            Err(NoCredits { credits: 19 })
        );
        seller.remove(&ItemDesignId::from(ItemRoleType::StoneOre), 5);
        exact.add(&items, &ItemDesignId::from(ItemRoleType::StoneOre), 5);
        buyer_cr -= 20;
        seller_cr += 20;
        assert_eq!((buyer_cr, seller_cr, exact.free_g(&items)), (0, 20, 0));
        assert_eq!(
            exact.count(&ItemDesignId::from(ItemRoleType::StoneOre))
                + seller.count(&ItemDesignId::from(ItemRoleType::StoneOre)),
            5
        );
        assert_eq!(
            plan(Buy, true, Some(1), &exact, buyer_cr, &ore(1), seller_cr),
            Err(NoRoom { free_g: 0 })
        );
        assert_eq!(
            plan(
                Buy,
                true,
                Some(1),
                &ShipInventory::new(&items, 400_000, []),
                buyer_cr,
                &ore(1),
                seller_cr,
            ),
            Err(NoCredits { credits: 0 })
        );
        // A derelict trades nothing, whatever else is wrong.
        assert_eq!(plan(Buy, false, None, &own, 0, &partner, 0), Err(NotTrader));
        assert_eq!(
            plan(Sell, true, None, &own, 0, &partner, 0),
            Err(NoQuantity)
        );
        assert_eq!(
            plan(Sell, true, Some(0), &own, 0, &partner, 0),
            Err(ZeroQuantity)
        );
        // Stock, then room, then credits.
        assert_eq!(
            plan(Sell, true, Some(11), &own, 0, &partner, 0),
            Err(Short { held: 10 })
        );
        assert_eq!(
            plan(
                Buy,
                true,
                Some(31),
                &ShipInventory::default(),
                0,
                &partner,
                0
            ),
            Err(Short { held: 30 })
        );
        assert_eq!(
            plan(
                Buy,
                true,
                Some(1),
                &ShipInventory::default(),
                0,
                &partner,
                0
            ),
            Err(NoRoom { free_g: 0 })
        );
        assert_eq!(
            plan(Buy, true, Some(5), &own, 19, &partner, 0),
            Err(NoCredits { credits: 19 })
        );
        assert_eq!(
            plan(Sell, true, Some(10), &own, 0, &partner, 29),
            Err(NoCredits { credits: 29 })
        );
        assert_eq!(
            plan(Sell, true, Some(1), &own, u32::MAX - 2, &partner, 3),
            Err(CreditOverflow)
        );

        // A planned Buy conserves items and credits across both ships.
        let (mut own, mut partner) = (own, partner);
        let (mut own_cr, mut partner_cr) = (100u32, 7u32);
        let trade = plan(Buy, true, Some(5), &own, own_cr, &partner, partner_cr).expect("planned");
        partner.remove(&ItemDesignId::from(ItemRoleType::StoneOre), trade.count);
        own.add(
            &items,
            &ItemDesignId::from(ItemRoleType::StoneOre),
            trade.count,
        );
        own_cr -= trade.price_cr;
        partner_cr += trade.price_cr;
        assert_eq!(
            own.count(&ItemDesignId::from(ItemRoleType::StoneOre))
                + partner.count(&ItemDesignId::from(ItemRoleType::StoneOre)),
            40
        );
        assert_eq!((own_cr, partner_cr), (80, 27));
    }

    #[test]
    fn credit_take_moves_the_whole_balance_and_refuses_without_mutation() {
        use CreditTakeRefusalType::*;

        // An intact, non-neutralized partner is never robbed, whatever its
        // balance.
        assert_eq!(plan_credit_take(false, 0, 500), Err(NotEligible));
        // A zero balance has nothing to take, even when eligible.
        assert_eq!(plan_credit_take(true, 0, 0), Err(ZeroBalance));
        // Eligibility wins over an empty balance.
        assert_eq!(plan_credit_take(false, 0, 0), Err(NotEligible));
        // A normal take moves the partner's whole balance.
        assert_eq!(plan_credit_take(true, 100, 500), Ok(500));
        // Repeating the action against the now-zero balance refuses again.
        assert_eq!(plan_credit_take(true, 600, 0), Err(ZeroBalance));
        // A take that would overflow the player's balance is refused, taking
        // nothing.
        assert_eq!(plan_credit_take(true, u32::MAX - 2, 3), Err(Overflow));
        assert_eq!(plan_credit_take(true, u32::MAX - 3, 3), Ok(3));
    }

    #[test]
    fn item_jettison_plans_refuse_in_order() {
        let items = crate::test_support::test_items();
        use ItemJettisonRefusalType::*;
        let own = ShipInventory::new(&items, 400_000, [(ItemRoleType::HullPlate.into(), 12)]);
        let tail = CargoCanister::new(&items, &ItemDesignId::from(ItemRoleType::HullPlate), 9);
        let plan = |docked, has_intake, tail: Option<&CargoCanister>, quantity| {
            plan_item_jettison(
                &items,
                docked,
                has_intake,
                tail,
                &ItemDesignId::from(ItemRoleType::HullPlate),
                quantity,
                &own,
            )
        };

        assert!(plan(false, true, None, Some(12)).is_ok());
        // Each check wins over every later one.
        assert_eq!(plan(true, false, Some(&tail), None), Err(Docked));
        assert_eq!(plan(false, false, Some(&tail), None), Err(NoIntake));
        assert_eq!(plan(false, true, Some(&tail), None), Err(NoQuantity));
        assert_eq!(plan(false, true, None, Some(0)), Err(ZeroQuantity));
        assert_eq!(plan(false, true, None, Some(13)), Err(Short { held: 12 }));
    }

    #[test]
    fn item_jettison_fills_the_waiting_tail_then_splits_the_rest_by_whole_items() {
        let items = crate::test_support::test_items();
        let own = ShipInventory::new(
            &items,
            10_000_000,
            [
                (ItemRoleType::HullPlate.into(), 12),
                (ItemRoleType::PdcRound.into(), 2_500),
                (ItemRoleType::RailSlug.into(), 25),
                (ItemRoleType::Torpedo.into(), 3),
            ],
        );
        let plan = |tail: Option<&CargoCanister>, item, quantity| {
            plan_item_jettison(
                &items,
                false,
                true,
                tail,
                &ItemDesignId::from(item),
                Some(quantity),
                &own,
            )
            .expect("the ship holds the quantity")
        };
        let split = |jettison: &ItemJettison| {
            jettison
                .canisters
                .iter()
                .map(|canister| {
                    canister
                        .stacks()
                        .map(|(item, count)| (item.as_str().to_string(), count))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };

        // One 150 kg torpedo per canister; 10 slugs and 1000 rounds fill one.
        let torpedoes = plan(None, ItemRoleType::Torpedo, 3);
        assert_eq!((torpedoes.count, torpedoes.merged), (3, 0));
        assert_eq!(
            split(&torpedoes),
            vec![vec![(ItemRoleType::Torpedo.id().to_string(), 1)]; 3]
        );
        let slugs = plan(None, ItemRoleType::RailSlug, 25);
        assert_eq!(
            split(&slugs),
            [
                vec![(ItemRoleType::RailSlug.id().to_string(), 10)],
                vec![(ItemRoleType::RailSlug.id().to_string(), 10)],
                vec![(ItemRoleType::RailSlug.id().to_string(), 5)],
            ]
        );
        let rounds = plan(None, ItemRoleType::PdcRound, 2_500);
        assert_eq!(
            split(&rounds),
            [
                vec![(ItemRoleType::PdcRound.id().to_string(), 1_000)],
                vec![(ItemRoleType::PdcRound.id().to_string(), 1_000)],
                vec![(ItemRoleType::PdcRound.id().to_string(), 500)],
            ]
        );

        // A waiting 90 kg tail has 110 kg of room: 5 slugs merge ...
        let tail = CargoCanister::new(&items, &ItemDesignId::from(ItemRoleType::HullPlate), 9);
        let into_tail = plan(Some(&tail), ItemRoleType::RailSlug, 7);
        assert_eq!((into_tail.count, into_tail.merged), (7, 5));
        assert_eq!(
            split(&into_tail),
            [vec![(ItemRoleType::RailSlug.id().to_string(), 2)]]
        );
        // ... and no torpedo, which starts a new canister.
        let past_tail = plan(Some(&tail), ItemRoleType::Torpedo, 1);
        assert_eq!(past_tail.merged, 0);
        assert_eq!(
            split(&past_tail),
            [vec![(ItemRoleType::Torpedo.id().to_string(), 1)]]
        );
        let fits = plan(Some(&tail), ItemRoleType::HullPlate, 11);
        assert_eq!((fits.merged, fits.canisters.len()), (11, 0));

        // Every planned count is conserved across the tail and the canisters.
        for jettison in [&torpedoes, &slugs, &rounds, &into_tail, &past_tail, &fits] {
            let queued: u32 = jettison
                .canisters
                .iter()
                .flat_map(CargoCanister::stacks)
                .map(|(_, count)| count)
                .sum();
            assert_eq!(jettison.merged + queued, jettison.count);
        }
    }
}

#[cfg(test)]
mod rebuild_tests {
    use super::*;

    /// The approved PDC recipe: a turret body, a barrel, an actuator pack and
    /// two section panels, plus 100 cr of work.
    fn pdc_recipe() -> BuildRecipe {
        BuildRecipe {
            credits: 100,
            parts: [
                ("TurretBody".into(), 1),
                ("TurretBarrel".into(), 1),
                ("ActuatorPack".into(), 1),
                ("SectionPanel".into(), 2),
            ]
            .into(),
            strip_yield: 0.5,
        }
    }

    fn hold(items: &GameItems, stacks: &[(&str, u32)]) -> ShipInventory {
        ShipInventory::new(
            items,
            10_000_000,
            stacks
                .iter()
                .map(|(item, count)| (ItemDesignId::from(*item), *count)),
        )
    }

    /// Parts come from the hold first and only the rest is bought: two panels
    /// in the hold are spent, the body, barrel and actuator are bought at ask,
    /// and the price is the work plus those three asks.
    #[test]
    fn rebuild_spends_hold_parts_first_and_buys_the_rest_at_ask() {
        let items = crate::test_support::test_items();
        let own = hold(&items, &[("SectionPanel", 5)]);
        let station = hold(
            &items,
            &[("TurretBody", 1), ("TurretBarrel", 3), ("ActuatorPack", 2)],
        );
        let built = vec!["pdc".to_string()];
        let partner = StationPartner {
            offers: true,
            builds: &built,
            stock: &station,
            credits_cr: 1_000,
        };

        let rebuild = plan_section_rebuild(
            &items,
            "pdc",
            &pdc_recipe(),
            Some(partner),
            true,
            true,
            &own,
            500,
        )
        .expect("a docked, lost, anchored section with parts and credits rebuilds");

        assert_eq!(rebuild.from_hold, [("SectionPanel".into(), 2)].into());
        assert_eq!(
            rebuild.bought,
            [
                ("ActuatorPack".into(), 1),
                ("TurretBarrel".into(), 1),
                ("TurretBody".into(), 1),
            ]
            .into()
        );
        assert_eq!(rebuild.price_cr, 100 + 200 + 60 + 80);
    }

    /// Each refusal fires in its documented order: the first failing check
    /// names the refusal, whatever fails after it.
    #[test]
    fn rebuild_refuses_in_order_and_names_the_short_part_and_the_price() {
        use SectionRebuildRefusalType::*;
        let items = crate::test_support::test_items();
        let recipe = pdc_recipe();
        let empty = hold(&items, &[]);
        let stocked = hold(
            &items,
            &[
                ("TurretBody", 1),
                ("TurretBarrel", 1),
                ("ActuatorPack", 1),
                ("SectionPanel", 2),
            ],
        );
        let built = vec!["pdc".to_string()];
        let other: Vec<String> = vec!["hull".to_string()];
        let station = |offers, builds, stock, credits_cr| {
            Some(StationPartner {
                offers,
                builds,
                stock,
                credits_cr,
            })
        };
        let plan = |partner, lost, anchored, own_cr| {
            plan_section_rebuild(
                &items, "pdc", &recipe, partner, lost, anchored, &empty, own_cr,
            )
        };

        assert_eq!(plan(None, false, false, 0), Err(NotDocked));
        assert_eq!(
            plan(station(false, &built, &stocked, 0), false, false, 0),
            Err(NoRebuildService)
        );
        // Knowledge before materials: the stock covers the whole recipe and
        // the section is lost and anchored, and the station still refuses.
        assert_eq!(
            plan(
                station(true, &other, &stocked, u32::MAX),
                true,
                true,
                u32::MAX
            ),
            Err(NotBuilt)
        );
        assert_eq!(
            plan(station(true, &built, &stocked, 0), false, false, 0),
            Err(SectionNotLost)
        );
        assert_eq!(
            plan(station(true, &built, &stocked, 0), true, false, 0),
            Err(Unanchored)
        );
        assert_eq!(
            plan(station(true, &built, &empty, 0), true, true, 0),
            Err(Payment(RecipePaymentRefusalType::MissingParts {
                item: "ActuatorPack".into(),
                short: 1,
            }))
        );
        let price = 100 + 200 + 60 + 80 + 2 * 20;
        assert_eq!(
            plan(station(true, &built, &stocked, 0), true, true, price - 1),
            Err(Payment(RecipePaymentRefusalType::InsufficientCredits {
                need: u64::from(price),
                have: price - 1,
            }))
        );
        assert_eq!(
            plan(station(true, &built, &stocked, u32::MAX), true, true, price),
            Err(Payment(RecipePaymentRefusalType::CreditOverflow))
        );
        assert_eq!(
            plan(station(true, &built, &stocked, 0), true, true, price).map(|plan| plan.price_cr),
            Ok(price)
        );
    }

    /// The value rule's two sides: the recipe at ask counts the work, the
    /// strip at bid rounds each part's share down.
    #[test]
    fn recipe_value_counts_the_work_and_strip_value_rounds_each_part_down() {
        let items = crate::test_support::test_items();
        let recipe = pdc_recipe();
        assert_eq!(recipe.value_cr(&items), 100 + 200 + 60 + 80 + 2 * 20);
        // Half of one is none; half of two panels is one panel at 15 bid.
        assert_eq!(recipe.strip_value_cr(&items), 15);
    }
}

#[cfg(test)]
mod strip_tests {
    use super::*;

    /// Two panels and one spar at half yield, as a heavy hull strips.
    fn recipe() -> BuildRecipe {
        BuildRecipe {
            credits: 160,
            parts: [("SectionPanel".into(), 6), ("FrameSpar".into(), 2)].into(),
            strip_yield: 0.5,
        }
    }

    fn hold(items: &GameItems, capacity_g: u32) -> ShipInventory {
        ShipInventory::new(items, capacity_g, [])
    }

    fn target(recipe: &BuildRecipe, health_share: f32) -> StripTarget<'_> {
        StripTarget {
            recipe,
            health_share,
        }
    }

    /// A strip returns the recipe times the yield times the health share,
    /// rounded down once per part, and never more than a whole strip.
    #[test]
    fn a_strip_returns_the_recipe_times_yield_times_health_rounded_down() {
        let recipe = recipe();
        let at = |share: f32| target(&recipe, share).parts();
        assert_eq!(
            at(1.0),
            [("FrameSpar".into(), 1), ("SectionPanel".into(), 3)].into()
        );
        // 6 * 0.5 * 0.7 = 2.1 panels, 2 * 0.5 * 0.7 = 0.7 spars.
        assert_eq!(at(0.7), [("SectionPanel".into(), 2)].into());
        assert!(at(0.0).is_empty());
        for share in [0.0, 0.25, 0.5, 0.99, 1.0, 1.5] {
            for (item, count) in recipe.strip_parts_at(share) {
                assert!(count <= recipe.strip_parts().find(|(i, _)| *i == item).unwrap().1);
            }
        }
    }

    /// A hull strip refuses in its documented order, and a clean plan sums
    /// every section's parts and time; a section that yields nothing still
    /// takes its time.
    #[test]
    fn a_hull_strip_refuses_in_order_and_sums_every_section() {
        use HullStripRefusalType::*;
        let recipe = recipe();
        let partner = |living_station, lootable| HullStripPartner {
            living_station,
            owned_station: false,
            lootable,
            owned_hulls: 0,
            sections: vec![target(&recipe, 1.0), target(&recipe, 0.1)],
            loot: LootTake::default(),
        };
        assert_eq!(plan_hull_strip(None, true), Err(NotDocked));
        assert_eq!(plan_hull_strip(Some(&partner(true, true)), true), Err(Busy));
        assert_eq!(
            plan_hull_strip(Some(&partner(true, true)), false),
            Err(LivingStation)
        );
        let mut owned = partner(false, false);
        owned.owned_station = true;
        owned.owned_hulls = 2;
        assert_eq!(plan_hull_strip(Some(&owned), false), Err(OwnedStation));
        owned.owned_station = false;
        assert_eq!(plan_hull_strip(Some(&owned), false), Err(NotLootable));
        owned.lootable = true;
        owned.loot.stayed_cr = 1;
        assert_eq!(
            plan_hull_strip(Some(&owned), false),
            Err(OwnedHullsParked { count: 2 })
        );
        let mut stuffed = partner(false, true);
        stuffed.loot.stayed_cr = 1;
        assert_eq!(plan_hull_strip(Some(&stuffed), false), Err(NoRoom));
        let strip =
            plan_hull_strip(Some(&partner(false, true)), false).expect("a lootable wreck strips");
        assert_eq!(
            strip.parts,
            [("FrameSpar".into(), 1), ("SectionPanel".into(), 3)].into()
        );
        assert_eq!(strip.duration_secs, 16.0);
    }

    /// A Take all moves each stack in id order while the hold has room and
    /// names what stays; the strip refuses unless it leaves nothing behind.
    #[test]
    fn a_strip_refuses_while_a_take_all_would_leave_loot_behind() {
        let items = crate::test_support::test_items();
        let plate = ItemDesignId::from(ItemRoleType::HullPlate.id());
        let panel = ItemDesignId::from("SectionPanel");
        let spar = ItemDesignId::from("FrameSpar");
        // 40 kg of spars, 30 kg of panels, 60 kg of plates; 50 cr.
        let wreck = ShipInventory::new(
            &items,
            1_000_000,
            [(spar.clone(), 1), (panel.clone(), 3), (plate.clone(), 6)],
        );
        let recipe = recipe();
        let partner = |own: &ShipInventory, own_cr| HullStripPartner {
            living_station: false,
            owned_station: false,
            lootable: true,
            owned_hulls: 0,
            sections: vec![target(&recipe, 1.0)],
            loot: plan_loot_take(&items, own, own_cr, &wreck, 50, None),
        };

        // Room for everything: it all moves and the strip starts.
        let roomy = hold(&items, 130_000);
        let whole = plan_loot_take(&items, &roomy, 0, &wreck, 50, None);
        assert_eq!(
            whole,
            LootTake {
                taken: [(spar.clone(), 1), (panel.clone(), 3), (plate.clone(), 6)].into(),
                taken_stored: BTreeMap::new(),
                stayed: BTreeMap::new(),
                taken_cr: 50,
                stayed_cr: 0,
            }
        );
        assert!(plan_hull_strip(Some(&partner(&roomy, 0)), false).is_ok());

        // Ten kilograms short: the panels are last in id order, so two of
        // the three fit and one stays. The strip refuses.
        let short = hold(&items, 120_000);
        let take = plan_loot_take(&items, &short, 0, &wreck, 50, None);
        assert_eq!(take.taken.get(&panel), Some(&2));
        assert_eq!(take.stayed, [(panel.clone(), 1)].into());
        assert_eq!(take.taken_cr, 50);
        assert_eq!(
            plan_hull_strip(Some(&partner(&short, 0)), false),
            Err(HullStripRefusalType::NoRoom)
        );

        // A balance that would overflow stays whole and refuses the strip.
        let overflow = plan_loot_take(&items, &roomy, u32::MAX, &wreck, 50, None);
        assert_eq!((overflow.taken_cr, overflow.stayed_cr), (0, 50));
        assert_eq!(
            plan_hull_strip(Some(&partner(&roomy, u32::MAX)), false),
            Err(HullStripRefusalType::NoRoom)
        );

        // An empty wreck takes nothing and leaves nothing.
        let empty = plan_loot_take(&items, &short, 0, &hold(&items, 0), 0, None);
        assert!(empty.is_empty() && empty.is_whole());

        // A dead station's storage is loot after the hold: with 140 kg of
        // room the hold's 130 kg fits whole, one of two stored plates fills
        // the rest, and the other stays. The stored part is counted apart.
        let mut storage = StationStorage::new(100_000);
        storage.add(&items, &plate, 2);
        let stored = plan_loot_take(
            &items,
            &hold(&items, 140_000),
            0,
            &wreck,
            50,
            Some(&storage),
        );
        assert_eq!(
            stored,
            LootTake {
                taken: [(spar.clone(), 1), (panel.clone(), 3), (plate.clone(), 7)].into(),
                taken_stored: [(plate.clone(), 1)].into(),
                stayed: [(plate.clone(), 1)].into(),
                taken_cr: 50,
                stayed_cr: 0,
            }
        );
    }

    /// A finished section's parts go in while they fit; a hold short by one
    /// gram holds the strip with the mass needed and free.
    #[test]
    fn a_section_whose_parts_do_not_fit_pauses_the_strip() {
        let items = crate::test_support::test_items();
        let recipe = recipe();
        let need_g = 3 * items.design(&"SectionPanel".into()).mass_g as u64
            + items.design(&"FrameSpar".into()).mass_g as u64;
        assert_eq!(
            plan_section_yield(
                &items,
                target(&recipe, 1.0),
                &hold(&items, need_g as u32 - 1)
            ),
            Err(HullStripPauseType::HoldFull {
                need_g,
                free_g: need_g as u32 - 1
            })
        );
        assert_eq!(
            plan_section_yield(&items, target(&recipe, 1.0), &hold(&items, need_g as u32)),
            Ok([("FrameSpar".into(), 1), ("SectionPanel".into(), 3)].into())
        );
        // Nothing to carry fits a full hold: the section comes off empty.
        assert_eq!(
            plan_section_yield(&items, target(&recipe, 0.1), &hold(&items, 0)),
            Ok(BTreeMap::new())
        );
    }

    /// The time is one second per recipe part, with a floor for small parts.
    #[test]
    fn a_strip_takes_a_second_per_part_and_at_least_the_floor() {
        let light = BuildRecipe {
            credits: 80,
            parts: [("SectionPanel".into(), 2)].into(),
            strip_yield: 0.5,
        };
        assert_eq!(strip_duration_secs(&light), STRIP_MIN_SECS);
        assert_eq!(strip_duration_secs(&recipe()), 8.0 * STRIP_SECS_PER_PART);
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn authored_stock_rejects_a_zero_stack_and_a_repeated_item() {
        let items = crate::test_support::test_items();
        let ok: ShipInventoryStock =
            ron::from_str(r#"{"HullPlate": 12}"#).expect("valid stock parses");
        assert_eq!(
            ok.stacks().collect::<Vec<_>>(),
            [(&ItemDesignId::from(ItemRoleType::HullPlate), 12)]
        );
        assert_eq!(ok.mass_g(&items), 120_000);
        let round_trip: ShipInventoryStock =
            ron::from_str(&ron::to_string(&ok).expect("serializes")).expect("parses back");
        assert_eq!(round_trip, ok);
        let empty: ShipInventoryStock = ron::from_str(r#"{}"#).expect("empty stock parses");
        assert_eq!(empty.mass_g(&items), 0);

        let zero = ron::from_str::<ShipInventoryStock>(r#"{"HullPlate": 0}"#)
            .expect_err("a zero-quantity stack must fail");
        assert!(zero.to_string().contains("quantity 0"), "{zero}");

        let dup = ron::from_str::<ShipInventoryStock>(r#"{"HullPlate": 1, "HullPlate": 2}"#)
            .expect_err("a repeated item must fail");
        assert!(dup.to_string().contains("twice"), "{dup}");

        // Ids are strings: the unquoted spelling of the closed enum is gone.
        ron::from_str::<ShipInventoryStock>("{HullPlate: 1}")
            .expect_err("an unquoted item id must fail");

        // The largest count still weighs without overflow, for lint to refuse.
        let huge: ShipInventoryStock =
            ron::from_str(r#"{"HullPlate": 4294967295}"#).expect("a huge count parses");
        assert_eq!(huge.mass_g(&items), 42_949_672_950_000);
    }
}
