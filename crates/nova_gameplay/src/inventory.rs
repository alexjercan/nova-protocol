//! What a ship carries: a closed set of item types, the category each belongs
//! to, the per-ship stack counts and the count the ship has room for.
//!
//! Every ship root requires a [`ShipInventory`]. Its `Default` has no room and
//! no stock: a code-built ship that never states an inventory carries nothing.
//! An authored ship states its capacity and stock through
//! `SpaceshipConfig::inventory`, which RON requires. A Ship pane repair spends
//! [`ItemType::HullPlate`] by the [`plan_plate_repair`] rule, an Inventory pane
//! transfer moves items between two docked ships by the [`plan_item_transfer`]
//! rule, and a jettison drops a [`CargoCanister`] by the [`plan_item_jettison`]
//! rule. Stock is not saved: it returns to its authored counts when the
//! scenario loads again. A stack exists only while its count is above zero, and
//! the counts of all stacks never pass the capacity.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::integrity::prelude::Health;

/// The whole module.
pub mod prelude {
    pub use super::{
        plan_item_jettison, plan_item_transfer, plan_plate_repair, CargoCanister, ItemCategoryType,
        ItemJettisonRefusalType, ItemTransferRefusalType, ItemTransferType, ItemType,
        LootableShipMarker, PlateRepair, PlateRepairRefusalType, ShipInventory, HULL_PLATE_HEALTH,
        SHIP_CARGO_CAPACITY,
    };
}

/// An item a ship can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemType {
    /// Hull plating stock, counted in plates.
    HullPlate,
}

impl ItemType {
    /// The category the item is filed under.
    pub fn category(self) -> ItemCategoryType {
        match self {
            Self::HullPlate => ItemCategoryType::Repair,
        }
    }

    /// The item's display name, as the Inventory pane and a canister's tag
    /// show it.
    pub fn label(self) -> &'static str {
        match self {
            Self::HullPlate => "Hull plate",
        }
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

/// The capacity every base-game ship states: the items it has room for across
/// all stacks. Named at each authored ship rather than taken as a default, so
/// a ship with a different hold states a different number.
pub const SHIP_CARGO_CAPACITY: u32 = 40;

/// The items one ship carries, as a count per item type, and the count it has
/// room for across all stacks. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker).
///
/// The `Default` has a capacity of zero: it is the required-component value
/// for a ship that no config states, and such a ship can take nothing.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipInventory {
    capacity: u32,
    stacks: BTreeMap<ItemType, u32>,
}

impl ShipInventory {
    /// An inventory with room for `capacity` items, holding `stacks`.
    ///
    /// # Panics
    ///
    /// On a zero quantity, a repeated item, or stacks that total more than
    /// `capacity`: the same faults the authored RON refuses, so a builder that
    /// writes one has a bug.
    pub fn new(capacity: u32, stacks: impl IntoIterator<Item = (ItemType, u32)>) -> Self {
        match Self::checked(capacity, stacks) {
            Ok(inventory) => inventory,
            Err(fault) => panic!("{fault}"),
        }
    }

    /// The one validation both [`new`](Self::new) and the RON parse run.
    fn checked(
        capacity: u32,
        stacks: impl IntoIterator<Item = (ItemType, u32)>,
    ) -> Result<Self, String> {
        let mut held = BTreeMap::new();
        let mut total: u64 = 0;
        for (item, count) in stacks {
            if count == 0 {
                return Err(format!("ShipInventory stack of {item:?} has quantity 0"));
            }
            if held.insert(item, count).is_some() {
                return Err(format!("ShipInventory lists {item:?} twice"));
            }
            total += u64::from(count);
        }
        if total > u64::from(capacity) {
            return Err(format!(
                "ShipInventory holds {total} items but has capacity {capacity}"
            ));
        }
        Ok(Self {
            capacity,
            stacks: held,
        })
    }

    /// How many items the ship has room for across all stacks.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// How many items the ship carries across all stacks.
    pub fn total(&self) -> u32 {
        // The capacity bounds the sum, so it fits.
        self.stacks.values().sum()
    }

    /// How many more items the ship has room for.
    pub fn free(&self) -> u32 {
        self.capacity - self.total()
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
    /// On `count == 0` or more than [`free`](Self::free): the caller has a bug
    /// and must plan the add first, as [`plan_item_transfer`] does.
    pub fn add(&mut self, item: ItemType, count: u32) {
        assert!(count > 0, "ShipInventory adds 0 of {item:?}");
        let free = self.free();
        assert!(
            count <= free,
            "ShipInventory adds {count} of {item:?} but has room for {free}"
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
    /// The target ship has room for fewer than the quantity.
    NoRoom {
        /// How many more items the target ship has room for.
        free: u32,
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
    let free = target.free();
    if quantity > free {
        return Err(ItemTransferRefusalType::NoRoom { free });
    }
    Ok(quantity)
}

/// One stack of items drifting free in a cargo canister: what a jettison
/// drops and what a cargo intake takes in whole. The count is above zero.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct CargoCanister {
    /// The item the canister holds.
    pub item: ItemType,
    /// How many it holds.
    pub count: u32,
}

/// Why a jettison drops nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemJettisonRefusalType {
    /// The ship is docked: a canister would leave into the docked pair.
    Docked,
    /// The ship has no live cargo intake to drop the canister through.
    NoIntake,
    /// The intake still holds a canister it has not dropped.
    IntakeBusy,
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
/// `intake_busy` while that intake still holds an earlier jettison. Checks run
/// in [`ItemJettisonRefusalType`] order. `quantity` is `None` when the typed
/// text is not a whole number.
pub fn plan_item_jettison(
    docked: bool,
    has_intake: bool,
    intake_busy: bool,
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
    if intake_busy {
        return Err(ItemJettisonRefusalType::IntakeBusy);
    }
    let quantity = quantity.ok_or(ItemJettisonRefusalType::NoQuantity)?;
    if quantity == 0 {
        return Err(ItemJettisonRefusalType::ZeroQuantity);
    }
    let held = own.count(item);
    if quantity > held {
        return Err(ItemJettisonRefusalType::Short { held });
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
// zero-quantity or repeated-item stack, or stock past the capacity, and only
// the panicking `ShipInventory::new` would ever catch it, which is not how
// content lint reports an authoring error. Both run `ShipInventory::checked`,
// so both name the fault the same way. RON: `(capacity: 40, stacks:
// {HullPlate: 12})`, both fields required.
#[cfg(feature = "serde")]
impl serde::Serialize for ShipInventory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut inventory = serializer.serialize_struct("ShipInventory", 2)?;
        inventory.serialize_field("capacity", &self.capacity)?;
        inventory.serialize_field("stacks", &self.stacks)?;
        inventory.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShipInventory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// The authored shape. A `BTreeMap` keeps a repeated key, so the
        /// stacks are read as pairs to catch one.
        #[derive(serde::Deserialize)]
        #[serde(rename = "ShipInventory", deny_unknown_fields)]
        struct Authored {
            capacity: u32,
            stacks: Stacks,
        }

        struct Stacks(Vec<(ItemType, u32)>);

        impl<'de> serde::Deserialize<'de> for Stacks {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct StacksVisitor;

                impl<'de> serde::de::Visitor<'de> for StacksVisitor {
                    type Value = Stacks;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
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
                        Ok(Stacks(stacks))
                    }
                }

                deserializer.deserialize_map(StacksVisitor)
            }
        }

        let authored = Authored::deserialize(deserializer)?;
        ShipInventory::checked(authored.capacity, authored.stacks.0)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let plates =
            |count: u32| ShipInventory::new(SHIP_CARGO_CAPACITY, [(ItemType::HullPlate, count)]);
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
        // Room is checked after stock: 35 held leaves room for 5 of the 8.
        let nearly_full = plates(35);
        assert_eq!(
            plan(Take, true, Some(6), &nearly_full, &partner),
            Err(NoRoom { free: 5 })
        );
        assert_eq!(plan(Take, true, Some(5), &nearly_full, &partner), Ok(5));
        assert_eq!(
            plan(Take, true, Some(9), &nearly_full, &partner),
            Err(Short { held: 8 })
        );
        assert_eq!(
            plan(Give, false, Some(1), &own, &ShipInventory::default()),
            Err(NoRoom { free: 0 })
        );

        // A planned move conserves the total and empties a drained stack.
        let (mut own, mut partner) = (own, partner);
        let moved = plan(Take, true, Some(8), &own, &partner).expect("planned");
        partner.remove(ItemType::HullPlate, moved);
        own.add(ItemType::HullPlate, moved);
        assert_eq!(own.count(ItemType::HullPlate), 20);
        assert_eq!(own.free(), SHIP_CARGO_CAPACITY - 20);
        assert!(partner.is_empty());
    }

    #[test]
    fn item_jettison_plans_refuse_in_order() {
        use ItemJettisonRefusalType::*;
        let own = ShipInventory::new(SHIP_CARGO_CAPACITY, [(ItemType::HullPlate, 12)]);
        let plan = |docked, has_intake, busy, quantity| {
            plan_item_jettison(
                docked,
                has_intake,
                busy,
                ItemType::HullPlate,
                quantity,
                &own,
            )
        };

        assert_eq!(plan(false, true, false, Some(4)), Ok(4));
        assert_eq!(plan(false, true, false, Some(12)), Ok(12));
        // Each check wins over every later one.
        assert_eq!(plan(true, false, true, None), Err(Docked));
        assert_eq!(plan(false, false, true, None), Err(NoIntake));
        assert_eq!(plan(false, true, true, None), Err(IntakeBusy));
        assert_eq!(plan(false, true, false, None), Err(NoQuantity));
        assert_eq!(plan(false, true, false, Some(0)), Err(ZeroQuantity));
        assert_eq!(plan(false, true, false, Some(13)), Err(Short { held: 12 }));
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn authored_inventory_rejects_a_zero_stack_a_repeated_item_and_stock_past_capacity() {
        let ok: ShipInventory = ron::from_str("(capacity: 40, stacks: {HullPlate: 12})")
            .expect("valid inventory parses");
        assert_eq!(ok.count(ItemType::HullPlate), 12);
        assert_eq!(ok.free(), 28);
        let round_trip: ShipInventory =
            ron::from_str(&ron::to_string(&ok).expect("serializes")).expect("parses back");
        assert_eq!(round_trip, ok);

        let zero = ron::from_str::<ShipInventory>("(capacity: 40, stacks: {HullPlate: 0})")
            .expect_err("a zero-quantity stack must fail");
        assert!(zero.to_string().contains("quantity 0"), "{zero}");

        let dup =
            ron::from_str::<ShipInventory>("(capacity: 40, stacks: {HullPlate: 1, HullPlate: 2})")
                .expect_err("a repeated item must fail");
        assert!(dup.to_string().contains("twice"), "{dup}");

        let over = ron::from_str::<ShipInventory>("(capacity: 10, stacks: {HullPlate: 12})")
            .expect_err("stock past capacity must fail");
        assert!(over.to_string().contains("capacity 10"), "{over}");

        let missing = ron::from_str::<ShipInventory>("(stacks: {HullPlate: 12})")
            .expect_err("a missing capacity must fail");
        assert!(missing.to_string().contains("capacity"), "{missing}");
    }
}
