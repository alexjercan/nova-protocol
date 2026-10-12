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
//! Every section config carries a [`BuildRecipe`]; a Ship pane rebuild at a
//! docked station spends it by the [`plan_section_rebuild`] rule.
//! A saved world keeps stock and credits across a Load; any other scenario
//! returns them to their authored values when it loads again. A stack exists
//! only while its count is above zero, and the mass of all stacks never passes
//! the capacity. Mass is counted in grams; [`kg_text`] shows it in kilograms.

use std::{collections::BTreeMap, sync::Arc};

use bevy::prelude::*;
use nova_events::prelude::{CreditChangeCauseType, ItemTradeType};

use crate::integrity::prelude::Health;

/// The whole module.
pub mod prelude {
    pub use nova_events::prelude::{CreditChangeCauseType, ItemTradeType};

    pub use super::{
        kg_text, plan_credit_take, plan_item_jettison, plan_item_trade, plan_item_transfer,
        plan_plate_repair, plan_section_rebuild, BuildRecipe, CargoCanister,
        CargoCanisterIdAllocator, CargoCanisterRuntimeId, CreditTakeRefusalType, GameItems,
        ItemCategoryType, ItemDesign, ItemDesignId, ItemJettison, ItemJettisonRefusalType,
        ItemRoleType, ItemTrade, ItemTradeRefusalType, ItemTransferRefusalType, ItemTransferType,
        LootableShipMarker, PlateRepair, PlateRepairRefusalType, RebuildPartner, SectionRebuild,
        SectionRebuildRefusalType, ShipCredits, ShipInventory, ShipInventoryReport,
        ShipInventoryReports, ShipInventoryStock, CARGO_CANISTER_MAX_MASS_G, HULL_PLATE_HEALTH,
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

/// Plan a trade of `quantity` of `item` between the player ship (`own`,
/// holding `own_cr`) and its docked `partner` (holding `partner_cr`).
///
/// `partner_trades` is true when the partner is neither neutralized nor
/// carries [`LootableShipMarker`]. A Buy moves items from the partner to the
/// player at [`ItemDesign::ask_cr`] each; a Sell moves them from the player to
/// the partner at [`ItemDesign::bid_cr`] each. Checks run in
/// [`ItemTradeRefusalType`] order, and a refusal changes nothing. `item`
/// resolves in `items`.
#[expect(
    clippy::too_many_arguments,
    reason = "both ships' stock and credits, read as one atomic plan"
)]
pub fn plan_item_trade(
    items: &GameItems,
    trade: ItemTradeType,
    partner_trades: bool,
    item: &ItemDesignId,
    quantity: Option<u32>,
    own: &ShipInventory,
    own_cr: u32,
    partner: &ShipInventory,
    partner_cr: u32,
) -> Result<ItemTrade, ItemTradeRefusalType> {
    if !partner_trades {
        return Err(ItemTradeRefusalType::NotTrader);
    }
    let quantity = quantity.ok_or(ItemTradeRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemTradeRefusalType::ZeroQuantity);
    }
    let design = items.design(item);
    let (seller, buyer, buyer_cr, seller_cr, unit_cr) = match trade {
        ItemTradeType::Buy => (partner, own, own_cr, partner_cr, design.ask_cr),
        ItemTradeType::Sell => (own, partner, partner_cr, own_cr, design.bid_cr),
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

/// Maximum mass of a drifting canister, in grams.
pub const CARGO_CANISTER_MAX_MASS_G: u32 = 200_000;

/// Item stacks drifting free in a canister. Every count is above zero and the
/// total mass is at most [`CARGO_CANISTER_MAX_MASS_G`].
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CargoCanister {
    stacks: BTreeMap<ItemDesignId, u32>,
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
    /// One item is heavier than a canister holds.
    Overweight,
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
    let per_canister = CARGO_CANISTER_MAX_MASS_G / mass_g;
    if per_canister == 0 {
        return Err(ItemJettisonRefusalType::Overweight);
    }
    let tail_room = tail.map_or(0, |tail| {
        (CARGO_CANISTER_MAX_MASS_G - tail.total_mass_g(items)) / mass_g
    });
    let merged = quantity.min(tail_room);
    let mut rest = quantity - merged;
    let mut canisters = Vec::new();
    while rest > 0 {
        let count = rest.min(per_canister);
        canisters.push(CargoCanister::new(items, item, count));
        rest -= count;
    }
    Ok(ItemJettison {
        count: quantity,
        merged,
        canisters,
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

    /// What a strip returns at bid: `floor(strip_yield * count)` of each part
    /// at [`ItemDesign::bid_cr`].
    ///
    /// # Panics
    ///
    /// When `items` lacks a part; content lint refuses that recipe first.
    pub fn strip_value_cr(&self, items: &GameItems) -> u64 {
        self.parts
            .iter()
            .map(|(item, count)| {
                let returned = (f64::from(self.strip_yield) * f64::from(*count)).floor() as u64;
                returned * u64::from(items.design(item).bid_cr)
            })
            .sum()
    }
}

/// The docked partner a rebuild reads: whether it offers the service, the
/// stock missing parts are bought from and its balance.
#[derive(Clone, Copy, Debug)]
pub struct RebuildPartner<'a> {
    /// The partner is a living station whose design offers Rebuild.
    pub offers_rebuild: bool,
    /// The partner's hold.
    pub stock: &'a ShipInventory,
    /// The partner's credits.
    pub credits_cr: u32,
}

/// A planned rebuild: the parts to take from each hold and the credits to
/// move from the ship to the station.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionRebuild {
    /// Parts to remove from the ship's hold.
    pub from_hold: BTreeMap<ItemDesignId, u32>,
    /// Parts to remove from the station's hold, paid at ask.
    pub bought: BTreeMap<ItemDesignId, u32>,
    /// Credits to remove from the ship and add to the station: the recipe's
    /// credits plus the bought parts at ask.
    pub price_cr: u32,
}

/// Why a rebuild changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectionRebuildRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Rebuild.
    NoRebuildService,
    /// The section is live on the ship.
    SectionNotLost,
    /// No live section mates with the section where the design places it.
    Unanchored,
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
    /// The station's balance after the rebuild would not fit in
    /// [`ShipCredits`].
    CreditOverflow,
}

/// Plan rebuilding one lost section from its `recipe` while docked with
/// `partner`.
///
/// `partner` is `None` while the ship is not docked. `lost` is true when the
/// design section is not live on the ship, and `anchored` when a live section
/// mates with it where the design places it. Parts come from the ship's `own`
/// hold first; the rest are bought from the partner's stock at
/// [`ItemDesign::ask_cr`]. The price is the recipe's credits plus the bought
/// parts. Checks run in [`SectionRebuildRefusalType`] order, and a refusal
/// changes nothing. Every part resolves in `items`.
pub fn plan_section_rebuild(
    items: &GameItems,
    recipe: &BuildRecipe,
    partner: Option<RebuildPartner<'_>>,
    lost: bool,
    anchored: bool,
    own: &ShipInventory,
    own_cr: u32,
) -> Result<SectionRebuild, SectionRebuildRefusalType> {
    let partner = partner.ok_or(SectionRebuildRefusalType::NotDocked)?;
    if !partner.offers_rebuild {
        return Err(SectionRebuildRefusalType::NoRebuildService);
    }
    if !lost {
        return Err(SectionRebuildRefusalType::SectionNotLost);
    }
    if !anchored {
        return Err(SectionRebuildRefusalType::Unanchored);
    }
    let mut from_hold = BTreeMap::new();
    let mut bought = BTreeMap::new();
    let mut price = u64::from(recipe.credits);
    for (item, &count) in &recipe.parts {
        let held = own.count(item).min(count);
        let missing = count - held;
        let stocked = partner.stock.count(item);
        if missing > stocked {
            return Err(SectionRebuildRefusalType::MissingParts {
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
        return Err(SectionRebuildRefusalType::InsufficientCredits {
            need: price,
            have: own_cr,
        });
    }
    // The price fits: it is at most the ship's u32 balance.
    let price_cr = price as u32;
    if partner.credits_cr.checked_add(price_cr).is_none() {
        return Err(SectionRebuildRefusalType::CreditOverflow);
    }
    Ok(SectionRebuild {
        from_hold,
        bought,
        price_cr,
    })
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
        let ore = |count: u32| {
            ShipInventory::new(&items, 400_000, [(ItemRoleType::StoneOre.into(), count)])
        };
        let plan = |trade,
                    trades,
                    quantity,
                    own: &ShipInventory,
                    own_cr,
                    partner: &ShipInventory,
                    partner_cr| {
            plan_item_trade(
                &items,
                trade,
                trades,
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
        let partner = RebuildPartner {
            offers_rebuild: true,
            stock: &station,
            credits_cr: 1_000,
        };

        let rebuild =
            plan_section_rebuild(&items, &pdc_recipe(), Some(partner), true, true, &own, 500)
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
        let station = |offers_rebuild, stock, credits_cr| {
            Some(RebuildPartner {
                offers_rebuild,
                stock,
                credits_cr,
            })
        };
        let plan = |partner, lost, anchored, own_cr| {
            plan_section_rebuild(&items, &recipe, partner, lost, anchored, &empty, own_cr)
        };

        assert_eq!(plan(None, false, false, 0), Err(NotDocked));
        assert_eq!(
            plan(station(false, &stocked, 0), false, false, 0),
            Err(NoRebuildService)
        );
        assert_eq!(
            plan(station(true, &stocked, 0), false, false, 0),
            Err(SectionNotLost)
        );
        assert_eq!(
            plan(station(true, &stocked, 0), true, false, 0),
            Err(Unanchored)
        );
        assert_eq!(
            plan(station(true, &empty, 0), true, true, 0),
            Err(MissingParts {
                item: "ActuatorPack".into(),
                short: 1,
            })
        );
        let price = 100 + 200 + 60 + 80 + 2 * 20;
        assert_eq!(
            plan(station(true, &stocked, 0), true, true, price - 1),
            Err(InsufficientCredits {
                need: u64::from(price),
                have: price - 1,
            })
        );
        assert_eq!(
            plan(station(true, &stocked, u32::MAX), true, true, price),
            Err(CreditOverflow)
        );
        assert_eq!(
            plan(station(true, &stocked, 0), true, true, price).map(|plan| plan.price_cr),
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
