//! What a ship carries: a closed set of item types, the category each belongs
//! to, the per-ship stack counts and the mass the ship has room for.
//!
//! Every ship root requires a [`ShipInventory`]. Its `Default` has no room and
//! no stock: a code-built ship that never states an inventory carries nothing.
//! An authored ship states its stock as a [`ShipInventoryStock`] through
//! `SpaceshipConfig::inventory`, which RON requires; the spawn consumes that
//! stock into a `ShipInventory` whose capacity its design derives. A Ship pane
//! repair spends [`ItemType::HullPlate`] by the [`plan_plate_repair`] rule, an
//! Inventory pane transfer moves items between two docked ships by the
//! [`plan_item_transfer`] rule, and a jettison queues [`CargoCanister`]s by the
//! [`plan_item_jettison`] rule, a weapon's idle reload moves its ammunition
//! item into the magazine, and a Buy or Sell with a docked trader moves items
//! and [`ShipCredits`] together by the [`plan_item_trade`] rule. Stock and
//! credits are not saved: they return to their authored values when the
//! scenario loads again. A stack exists only while its count is
//! above zero, and the mass of all stacks never passes the capacity. Mass is
//! counted in grams; [`kg_text`] shows it in kilograms.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::integrity::prelude::Health;

/// The whole module.
pub mod prelude {
    pub use super::{
        kg_text, plan_credit_take, plan_item_jettison, plan_item_trade, plan_item_transfer,
        plan_plate_repair, CargoCanister, CargoCanisterIdAllocator, CargoCanisterRuntimeId,
        CreditTakeRefusalType, ItemCategoryType, ItemJettison, ItemJettisonRefusalType, ItemTrade,
        ItemTradeRefusalType, ItemTradeType, ItemTransferRefusalType, ItemTransferType, ItemType,
        LootableShipMarker, PlateRepair, PlateRepairRefusalType, ShipCredits, ShipInventory,
        ShipInventoryStock, CARGO_CANISTER_MAX_MASS_G, HULL_PLATE_HEALTH,
    };
}

/// An item a ship can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemType {
    /// Hull plating stock, counted in plates.
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
    /// Packed provisions for trade. Nothing eats them.
    Rations,
    /// Scavenged machine parts for trade.
    SalvagedParts,
}

impl ItemType {
    /// Fixed mass of one item in grams. Grams keep capacity and conservation
    /// arithmetic exact for items lighter than a kilogram.
    pub fn mass_g(self) -> u32 {
        match self {
            Self::HullPlate => 10_000,
            Self::PdcRound => 200,
            Self::RailSlug => 20_000,
            Self::Torpedo => 150_000,
            Self::StoneOre | Self::IronOre | Self::WaterIce | Self::CarbonOre => 10_000,
            Self::Rations => 2_000,
            Self::SalvagedParts => 25_000,
        }
    }

    /// Credits a trader asks for one item: what a Buy pays. Provisional
    /// values, not a balance decision.
    pub fn ask_cr(self) -> u32 {
        match self {
            Self::HullPlate => 40,
            Self::PdcRound => 4,
            Self::RailSlug => 40,
            Self::Torpedo => 400,
            Self::StoneOre => 4,
            Self::IronOre => 16,
            Self::WaterIce => 12,
            Self::CarbonOre => 8,
            Self::Rations => 8,
            Self::SalvagedParts => 120,
        }
    }

    /// Credits a trader bids for one item: what a Sell earns. Below
    /// [`ask_cr`](Self::ask_cr), so buying and selling back loses credits.
    pub fn bid_cr(self) -> u32 {
        match self {
            Self::HullPlate => 30,
            Self::PdcRound => 3,
            Self::RailSlug => 30,
            Self::Torpedo => 300,
            Self::StoneOre => 3,
            Self::IronOre => 12,
            Self::WaterIce => 9,
            Self::CarbonOre => 6,
            Self::Rations => 6,
            Self::SalvagedParts => 90,
        }
    }

    /// Mass of `count` of this item in grams. Wide so a count read from
    /// content or typed at the Command shell cannot overflow before a
    /// capacity check refuses it.
    pub fn stack_mass_g(self, count: u32) -> u64 {
        u64::from(count) * u64::from(self.mass_g())
    }

    /// The category the item is filed under.
    pub fn category(self) -> ItemCategoryType {
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

    /// The item's display name, as the Inventory pane and a canister's tag
    /// show it.
    pub fn label(self) -> &'static str {
        match self {
            Self::HullPlate => "Hull plate",
            Self::PdcRound => "PDC round",
            Self::RailSlug => "Rail slug",
            Self::Torpedo => "Torpedo",
            Self::StoneOre => "Stone ore",
            Self::IronOre => "Iron ore",
            Self::WaterIce => "Water ice",
            Self::CarbonOre => "Carbon ore",
            Self::Rations => "Rations",
            Self::SalvagedParts => "Salvaged parts",
        }
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

/// The items an authored ship carries at spawn, as a count per item type.
/// RON writes a bare map: `{HullPlate: 12}`, or `{}` for none.
///
/// Holds no capacity: the spawn reads that from the resolved design and
/// consumes this component into the ship's [`ShipInventory`]. Content lint
/// refuses stock heavier than the design's hold before the spawn does.
///
/// The `Default` holds nothing, as `SpaceshipConfig`'s `Default` needs; RON
/// still requires the field.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipInventoryStock {
    stacks: BTreeMap<ItemType, u32>,
}

impl ShipInventoryStock {
    /// Stock holding `stacks`.
    ///
    /// # Panics
    ///
    /// On a zero quantity or a repeated item: the same faults the authored RON
    /// refuses, so a builder that writes one has a bug.
    pub fn new(stacks: impl IntoIterator<Item = (ItemType, u32)>) -> Self {
        match Self::checked(stacks) {
            Ok(stock) => stock,
            Err(fault) => panic!("{fault}"),
        }
    }

    /// The one validation both [`new`](Self::new) and the RON parse run.
    fn checked(stacks: impl IntoIterator<Item = (ItemType, u32)>) -> Result<Self, String> {
        let mut held = BTreeMap::new();
        for (item, count) in stacks {
            if count == 0 {
                return Err(format!(
                    "ShipInventoryStock stack of {item:?} has quantity 0"
                ));
            }
            if held.insert(item, count).is_some() {
                return Err(format!("ShipInventoryStock lists {item:?} twice"));
            }
        }
        Ok(Self { stacks: held })
    }

    /// Every stack, in [`ItemType`] order. Each count is above zero.
    pub fn stacks(&self) -> impl Iterator<Item = (ItemType, u32)> + '_ {
        self.stacks.iter().map(|(item, count)| (*item, *count))
    }

    /// Mass of every stack in grams. Wide because authored stock has no
    /// capacity bound until the spawn applies one.
    pub fn mass_g(&self) -> u64 {
        self.stacks()
            .map(|(item, count)| item.stack_mass_g(count))
            .sum()
    }
}

/// The items one ship carries, as a count per item type, and the mass it has
/// room for across all stacks. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker).
///
/// The `Default` has a capacity of zero: it is the required-component value
/// for a ship that no config states, and such a ship can take nothing.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipInventory {
    capacity_g: u32,
    stacks: BTreeMap<ItemType, u32>,
}

impl ShipInventory {
    /// An inventory with room for `capacity_g` grams, holding `stacks`.
    ///
    /// # Panics
    ///
    /// On a zero quantity, a repeated item, or stacks heavier than
    /// `capacity_g`. Content lint refuses authored stock past a design's
    /// hold, so a spawn that reaches this panic loaded unlinted content.
    pub fn new(capacity_g: u32, stacks: impl IntoIterator<Item = (ItemType, u32)>) -> Self {
        let stock = ShipInventoryStock::new(stacks);
        let mass_g = stock.mass_g();
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
    pub fn used_g(&self) -> u32 {
        // The capacity bounds the sum, so each product and the sum fit.
        self.stacks()
            .map(|(item, count)| item.mass_g() * count)
            .sum()
    }

    /// How many more grams the ship has room for.
    pub fn free_g(&self) -> u32 {
        self.capacity_g - self.used_g()
    }

    /// How many of `item` the ship carries; zero when it has no stack.
    pub fn count(&self, item: ItemType) -> u32 {
        self.stacks.get(&item).copied().unwrap_or(0)
    }

    /// Every stack the ship carries, in [`ItemType`] order. Each count is above
    /// zero.
    pub fn stacks(&self) -> impl Iterator<Item = (ItemType, u32)> + '_ {
        self.stacks.iter().map(|(item, count)| (*item, *count))
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
    pub fn add(&mut self, item: ItemType, count: u32) {
        assert!(count > 0, "ShipInventory adds 0 of {item:?}");
        let free_g = self.free_g();
        assert!(
            item.stack_mass_g(count) <= u64::from(free_g),
            "ShipInventory adds {count} of {item:?} but has room for {}",
            kg_text(u64::from(free_g)),
        );
        self.stacks.insert(item, self.count(item) + count);
    }

    /// Remove `count` of `item`; delete the stack when it empties.
    ///
    /// # Panics
    ///
    /// On `count == 0` or more than the ship carries: the caller has a bug and
    /// must check [`count`](Self::count) first.
    pub fn remove(&mut self, item: ItemType, count: u32) {
        assert!(count > 0, "ShipInventory removes 0 of {item:?}");
        let held = self.count(item);
        assert!(
            count <= held,
            "ShipInventory removes {count} of {item:?} but carries {held}"
        );
        if count == held {
            self.stacks.remove(&item);
        } else {
            self.stacks.insert(item, held - count);
        }
    }
}

/// The credits one ship holds. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker); the `Default`
/// of zero is the value for a ship that no config states. An authored ship
/// states its balance through `SpaceshipConfig::credits`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipCredits(pub u32);

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
/// text is not a whole number.
pub fn plan_item_transfer(
    transfer: ItemTransferType,
    partner_lootable: bool,
    item: ItemType,
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
    let free_g = target.free_g();
    if item.stack_mass_g(quantity) > u64::from(free_g) {
        return Err(ItemTransferRefusalType::NoRoom { free_g });
    }
    Ok(quantity)
}

/// Which way a trade with a docked trader moves items and credits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub enum ItemTradeType {
    /// Items from the trader into the player ship, paid at the item's ask.
    Buy,
    /// Items from the player ship into the trader, paid at the item's bid.
    Sell,
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
/// player at [`ItemType::ask_cr`] each; a Sell moves them from the player to
/// the partner at [`ItemType::bid_cr`] each. Checks run in
/// [`ItemTradeRefusalType`] order, and a refusal changes nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "both ships' stock and credits, read as one atomic plan"
)]
pub fn plan_item_trade(
    trade: ItemTradeType,
    partner_trades: bool,
    item: ItemType,
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
    let (seller, buyer, buyer_cr, seller_cr, unit_cr) = match trade {
        ItemTradeType::Buy => (partner, own, own_cr, partner_cr, item.ask_cr()),
        ItemTradeType::Sell => (own, partner, partner_cr, own_cr, item.bid_cr()),
    };
    let held = seller.count(item);
    if quantity > held {
        return Err(ItemTradeRefusalType::Short { held });
    }
    let free_g = buyer.free_g();
    if item.stack_mass_g(quantity) > u64::from(free_g) {
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
pub struct CargoCanister {
    stacks: BTreeMap<ItemType, u32>,
}

impl CargoCanister {
    /// Start a canister with one valid stack.
    pub fn new(item: ItemType, count: u32) -> Self {
        assert!(
            count > 0
                && u64::from(count) * u64::from(item.mass_g())
                    <= u64::from(CARGO_CANISTER_MAX_MASS_G),
            "invalid canister stack"
        );
        Self {
            stacks: [(item, count)].into(),
        }
    }

    /// Every held stack in item order.
    pub fn stacks(&self) -> impl Iterator<Item = (ItemType, u32)> + '_ {
        self.stacks.iter().map(|(&item, &count)| (item, count))
    }

    /// Total mass in grams.
    pub fn total_mass_g(&self) -> u32 {
        self.stacks()
            .map(|(item, count)| item.mass_g() * count)
            .sum()
    }

    /// Add a stack after the jettison rule has checked the resulting mass.
    pub fn add(&mut self, item: ItemType, count: u32) {
        assert!(
            count > 0
                && u64::from(self.total_mass_g()) + u64::from(count) * u64::from(item.mass_g())
                    <= u64::from(CARGO_CANISTER_MAX_MASS_G),
            "canister mass exceeded"
        );
        *self.stacks.entry(item).or_default() += count;
    }
}

/// Runtime identity for one cargo canister entity, minted the moment it is
/// spawned. Not addressable: nothing looks a canister up by this id, so a
/// mining or jettison spawn mints one and no system ever reads it back by
/// value. It exists so an external reader (bench, probe, logs) can tell two
/// canister entities apart without reaching for `Entity` (recycled) or `Name`
/// (every canister shares `"Cargo Canister"`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CargoCanisterRuntimeId(pub u64);

/// Mints unique [`CargoCanisterRuntimeId`]s. Owned and initialized only by
/// `NovaGameplayPlugin`: the counter is never reset by a scenario load, retry,
/// or New Game for as long as the app process runs. No ID can be reused even
/// if a canister survives a world transition. Numeric IDs are not stable
/// across revisions.
#[derive(Resource, Default, Debug)]
pub struct CargoCanisterIdAllocator(u64);

impl CargoCanisterIdAllocator {
    /// The next id, unused by any earlier call.
    ///
    /// # Panics
    ///
    /// On exhausting `u64`: an id is never reused, so a counter that wrapped
    /// would silently collide with a still-live canister instead.
    pub fn next(&mut self) -> CargoCanisterRuntimeId {
        let id = self.0;
        self.0 = self
            .0
            .checked_add(1)
            .expect("CargoCanisterIdAllocator exhausted u64 ids");
        CargoCanisterRuntimeId(id)
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
    /// One item is heavier than a canister holds. No current item is.
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
    docked: bool,
    has_intake: bool,
    tail: Option<&CargoCanister>,
    item: ItemType,
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
    let per_canister = CARGO_CANISTER_MAX_MASS_G / item.mass_g();
    if per_canister == 0 {
        return Err(ItemJettisonRefusalType::Overweight);
    }
    let tail_room = tail.map_or(0, |tail| {
        (CARGO_CANISTER_MAX_MASS_G - tail.total_mass_g()) / item.mass_g()
    });
    let merged = quantity.min(tail_room);
    let mut rest = quantity - merged;
    let mut canisters = Vec::new();
    while rest > 0 {
        let count = rest.min(per_canister);
        canisters.push(CargoCanister::new(item, count));
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
    /// The ship carries no hull plates.
    NoPlates,
}

/// Plan a plate repair of one section from `plates` in stock.
///
/// Spends `min(plates, ceil((max - current) / HULL_PLATE_HEALTH))` and ends at
/// `min(max, current + HULL_PLATE_HEALTH * spent)`. The section's own state is
/// checked before the stock, in [`PlateRepairRefusalType`] order. `disabled`
/// is true when the section carries `IntegrityDisabledMarker`.
pub fn plan_plate_repair(
    health: Option<&Health>,
    disabled: bool,
    plates: u32,
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
    if plates == 0 {
        return Err(PlateRepairRefusalType::NoPlates);
    }
    let needed = ((health.max - health.current) / HULL_PLATE_HEALTH).ceil() as u32;
    let spent = plates.min(needed);
    Ok(PlateRepair {
        plates: spent,
        current: health
            .max
            .min(health.current + HULL_PLATE_HEALTH * spent as f32),
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
                formatter.write_str("a map of item type to stack count")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut stacks = Vec::new();
                while let Some(entry) = map.next_entry::<ItemType, u32>()? {
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

    #[test]
    fn kg_text_shows_grams_as_kilograms_without_trailing_zeros() {
        assert_eq!(kg_text(0), "0 kg");
        assert_eq!(kg_text(u64::from(ItemType::PdcRound.mass_g())), "0.2 kg");
        assert_eq!(kg_text(u64::from(ItemType::HullPlate.mass_g())), "10 kg");
        assert_eq!(kg_text(1_050), "1.05 kg");
        assert_eq!(kg_text(3_520_001), "3520.001 kg");
        // 6000 PDC rounds weigh exactly 1200 kg: no float drift in the sum.
        let rounds = ShipInventory::new(1_200_000, [(ItemType::PdcRound, 6000)]);
        assert_eq!(rounds.used_g(), 1_200_000);
        assert_eq!(rounds.free_g(), 0);
    }

    #[test]
    fn cargo_canister_id_allocator_mints_unique_increasing_ids() {
        let mut allocator = CargoCanisterIdAllocator::default();
        assert_eq!(allocator.next(), CargoCanisterRuntimeId(0));
        assert_eq!(allocator.next(), CargoCanisterRuntimeId(1));
        assert_eq!(allocator.next(), CargoCanisterRuntimeId(2));
    }

    #[test]
    #[should_panic(expected = "exhausted u64 ids")]
    fn cargo_canister_id_allocator_panics_rather_than_wrap_and_reuse_an_id() {
        let mut allocator = CargoCanisterIdAllocator(u64::MAX);
        allocator.next();
    }

    #[test]
    fn plate_repair_spends_one_plate_per_20_missing_health_within_stock() {
        let plan = |current: f32, max: f32, disabled: bool, plates: u32| {
            plan_plate_repair(Some(&Health { current, max }), disabled, plates)
        };
        let repaired = |plates, current| Ok(PlateRepair { plates, current });

        // Understock: 60 missing needs 3, 2 in stock.
        assert_eq!(plan(40.0, 100.0, false, 2), repaired(2, 80.0));
        // A scratch or a fraction of HP still costs one whole plate.
        assert_eq!(plan(99.0, 100.0, false, 12), repaired(1, 100.0));
        assert_eq!(plan(99.5, 100.0, false, 12), repaired(1, 100.0));
        // The last plate's leftover capacity is lost: 45 missing, 3 plates, 100.
        assert_eq!(plan(55.0, 100.0, false, 3), repaired(3, 100.0));

        use PlateRepairRefusalType::*;
        assert_eq!(plan(100.0, 100.0, false, 12), Err(Full));
        assert_eq!(plan(0.0, 100.0, false, 12), Err(Destroyed));
        assert_eq!(plan(50.0, 100.0, true, 12), Err(Destroyed));
        assert_eq!(plan(50.0, 100.0, false, 0), Err(NoPlates));
        assert_eq!(plan(0.0, 0.0, false, 12), Err(NoIntegrity));
        assert_eq!(plan_plate_repair(None, false, 12), Err(NoIntegrity));
        // The section's state wins over the stock.
        assert_eq!(plan(100.0, 100.0, false, 0), Err(Full));
    }
}

#[cfg(test)]
mod transfer_tests {
    use super::*;

    #[test]
    fn item_transfer_plans_refuse_in_order_and_never_pass_capacity() {
        use ItemTransferRefusalType::*;
        use ItemTransferType::*;
        // 400 kg holds 40 plates of 10 kg.
        let plates = |count: u32| ShipInventory::new(400_000, [(ItemType::HullPlate, count)]);
        let plan = |transfer, lootable, quantity, own: &ShipInventory, partner: &ShipInventory| {
            plan_item_transfer(
                transfer,
                lootable,
                ItemType::HullPlate,
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
        partner.remove(ItemType::HullPlate, moved);
        own.add(ItemType::HullPlate, moved);
        assert_eq!(own.count(ItemType::HullPlate), 20);
        assert_eq!(own.used_g(), 200_000);
        assert_eq!(own.free_g(), 200_000);
        assert!(partner.is_empty());
    }

    #[test]
    fn item_trade_plans_refuse_in_order_and_conserve_items_and_credits() {
        use ItemTradeRefusalType::*;
        use ItemTradeType::*;
        // 400 kg of room; ore is 10 kg, ask 4 and bid 3 for stone.
        let ore = |count: u32| ShipInventory::new(400_000, [(ItemType::StoneOre, count)]);
        let plan = |trade,
                    trades,
                    quantity,
                    own: &ShipInventory,
                    own_cr,
                    partner: &ShipInventory,
                    partner_cr| {
            plan_item_trade(
                trade,
                trades,
                ItemType::StoneOre,
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
        let mut exact = ShipInventory::new(50_000, []);
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
                &ShipInventory::new(49_999, []),
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
        seller.remove(ItemType::StoneOre, 5);
        exact.add(ItemType::StoneOre, 5);
        buyer_cr -= 20;
        seller_cr += 20;
        assert_eq!((buyer_cr, seller_cr, exact.free_g()), (0, 20, 0));
        assert_eq!(
            exact.count(ItemType::StoneOre) + seller.count(ItemType::StoneOre),
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
                &ShipInventory::new(400_000, []),
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
        partner.remove(ItemType::StoneOre, trade.count);
        own.add(ItemType::StoneOre, trade.count);
        own_cr -= trade.price_cr;
        partner_cr += trade.price_cr;
        assert_eq!(
            own.count(ItemType::StoneOre) + partner.count(ItemType::StoneOre),
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
        use ItemJettisonRefusalType::*;
        let own = ShipInventory::new(400_000, [(ItemType::HullPlate, 12)]);
        let tail = CargoCanister::new(ItemType::HullPlate, 9);
        let plan = |docked, has_intake, tail: Option<&CargoCanister>, quantity| {
            plan_item_jettison(
                docked,
                has_intake,
                tail,
                ItemType::HullPlate,
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
        let own = ShipInventory::new(
            10_000_000,
            [
                (ItemType::HullPlate, 12),
                (ItemType::PdcRound, 2_500),
                (ItemType::RailSlug, 25),
                (ItemType::Torpedo, 3),
            ],
        );
        let plan = |tail: Option<&CargoCanister>, item, quantity| {
            plan_item_jettison(false, true, tail, item, Some(quantity), &own)
                .expect("the ship holds the quantity")
        };
        let split = |jettison: &ItemJettison| {
            jettison
                .canisters
                .iter()
                .map(|canister| canister.stacks().collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };

        // One 150 kg torpedo per canister; 10 slugs and 1000 rounds fill one.
        let torpedoes = plan(None, ItemType::Torpedo, 3);
        assert_eq!((torpedoes.count, torpedoes.merged), (3, 0));
        assert_eq!(split(&torpedoes), vec![vec![(ItemType::Torpedo, 1)]; 3]);
        let slugs = plan(None, ItemType::RailSlug, 25);
        assert_eq!(
            split(&slugs),
            [
                vec![(ItemType::RailSlug, 10)],
                vec![(ItemType::RailSlug, 10)],
                vec![(ItemType::RailSlug, 5)],
            ]
        );
        let rounds = plan(None, ItemType::PdcRound, 2_500);
        assert_eq!(
            split(&rounds),
            [
                vec![(ItemType::PdcRound, 1_000)],
                vec![(ItemType::PdcRound, 1_000)],
                vec![(ItemType::PdcRound, 500)],
            ]
        );

        // A waiting 90 kg tail has 110 kg of room: 5 slugs merge ...
        let tail = CargoCanister::new(ItemType::HullPlate, 9);
        let into_tail = plan(Some(&tail), ItemType::RailSlug, 7);
        assert_eq!((into_tail.count, into_tail.merged), (7, 5));
        assert_eq!(split(&into_tail), [vec![(ItemType::RailSlug, 2)]]);
        // ... and no torpedo, which starts a new canister.
        let past_tail = plan(Some(&tail), ItemType::Torpedo, 1);
        assert_eq!(past_tail.merged, 0);
        assert_eq!(split(&past_tail), [vec![(ItemType::Torpedo, 1)]]);
        let fits = plan(Some(&tail), ItemType::HullPlate, 11);
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

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn authored_stock_rejects_a_zero_stack_and_a_repeated_item() {
        let ok: ShipInventoryStock = ron::from_str("{HullPlate: 12}").expect("valid stock parses");
        assert_eq!(ok.stacks().collect::<Vec<_>>(), [(ItemType::HullPlate, 12)]);
        assert_eq!(ok.mass_g(), 120_000);
        let round_trip: ShipInventoryStock =
            ron::from_str(&ron::to_string(&ok).expect("serializes")).expect("parses back");
        assert_eq!(round_trip, ok);
        let empty: ShipInventoryStock = ron::from_str("{}").expect("empty stock parses");
        assert_eq!(empty.mass_g(), 0);

        let zero = ron::from_str::<ShipInventoryStock>("{HullPlate: 0}")
            .expect_err("a zero-quantity stack must fail");
        assert!(zero.to_string().contains("quantity 0"), "{zero}");

        let dup = ron::from_str::<ShipInventoryStock>("{HullPlate: 1, HullPlate: 2}")
            .expect_err("a repeated item must fail");
        assert!(dup.to_string().contains("twice"), "{dup}");

        // The largest count still weighs without overflow, for lint to refuse.
        let huge: ShipInventoryStock =
            ron::from_str("{HullPlate: 4294967295}").expect("a huge count parses");
        assert_eq!(huge.mass_g(), 42_949_672_950_000);
    }
}
