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
//! [`plan_item_transfer`] rule, and a jettison drops a [`CargoCanister`] by the
//! [`plan_item_jettison`] rule, and a weapon's idle reload moves its ammunition
//! item into the magazine. Stock is not saved: it returns to its authored
//! counts when the scenario loads again. A stack exists only while its count is
//! above zero, and the mass of all stacks never passes the capacity. Mass is
//! counted in grams; [`kg_text`] shows it in kilograms.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::integrity::prelude::Health;

/// The whole module.
pub mod prelude {
    pub use super::{
        kg_text, plan_item_jettison, plan_item_transfer, plan_plate_repair, CargoCanister,
        ItemCategoryType, ItemJettisonRefusalType, ItemTransferRefusalType, ItemTransferType,
        ItemType, LootableShipMarker, PlateRepair, PlateRepairRefusalType, ShipInventory,
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
        }
    }

    /// Mass of `count` of this item in grams. Wide so a count read from
    /// content cannot overflow before a capacity check refuses it.
    fn stack_mass_g(self, count: u32) -> u64 {
        u64::from(count) * u64::from(self.mass_g())
    }

    /// The category the item is filed under.
    pub fn category(self) -> ItemCategoryType {
        match self {
            Self::HullPlate => ItemCategoryType::Repair,
            Self::PdcRound | Self::RailSlug | Self::Torpedo => ItemCategoryType::Ammo,
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

/// Why a jettison drops nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemJettisonRefusalType {
    /// The ship is docked: a canister would leave into the docked pair.
    Docked,
    /// The ship has no live cargo intake to drop the canister through.
    NoIntake,
    /// The requested stack would exceed the canister mass limit.
    Overweight,
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

/// Plan a jettison of `quantity` of `item` from the player ship's `own`
/// inventory as one canister.
///
/// Returns the count to remove and put in the canister. `docked` is true while
/// the ship is docked, `has_intake` while it has a live cargo intake, and
/// `pending` when that intake holds an earlier jettison. Checks run
/// in [`ItemJettisonRefusalType`] order. `quantity` is `None` when the typed
/// text is not a whole number.
pub fn plan_item_jettison(
    docked: bool,
    has_intake: bool,
    pending: Option<&CargoCanister>,
    item: ItemType,
    quantity: Option<u32>,
    own: &ShipInventory,
) -> Result<u32, ItemJettisonRefusalType> {
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
    let mass = pending.map_or(0, CargoCanister::total_mass_g);
    if u64::from(mass) + u64::from(quantity) * u64::from(item.mass_g())
        > u64::from(CARGO_CANISTER_MAX_MASS_G)
    {
        return Err(ItemJettisonRefusalType::Overweight);
    }
    Ok(quantity)
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
    fn item_jettison_plans_refuse_in_order() {
        use ItemJettisonRefusalType::*;
        let own = ShipInventory::new(400_000, [(ItemType::HullPlate, 12)]);
        let pending = CargoCanister::new(ItemType::HullPlate, 9);
        let plan = |docked, has_intake, pending: Option<&CargoCanister>, quantity| {
            plan_item_jettison(
                docked,
                has_intake,
                pending,
                ItemType::HullPlate,
                quantity,
                &own,
            )
        };

        assert_eq!(plan(false, true, None, Some(4)), Ok(4));
        assert_eq!(plan(false, true, None, Some(12)), Ok(12));
        assert_eq!(plan(false, true, Some(&pending), Some(11)), Ok(11));
        // Each check wins over every later one.
        assert_eq!(plan(true, false, Some(&pending), None), Err(Docked));
        assert_eq!(plan(false, false, Some(&pending), None), Err(NoIntake));
        assert_eq!(plan(false, true, Some(&pending), None), Err(NoQuantity));
        assert_eq!(plan(false, true, None, Some(0)), Err(ZeroQuantity));
        assert_eq!(plan(false, true, None, Some(13)), Err(Short { held: 12 }));
        assert_eq!(plan(false, true, Some(&pending), Some(12)), Err(Overweight));
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
