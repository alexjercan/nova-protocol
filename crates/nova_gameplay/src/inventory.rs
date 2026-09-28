//! What a ship carries: a closed set of item types, the category each belongs
//! to, and the per-ship stack counts.
//!
//! Every ship root requires a [`ShipInventory`], empty by default, so a reader
//! can fetch it from any ship without a fallback. A ship starts with authored
//! stock through `SpaceshipConfig::inventory`. A Ship pane repair spends
//! [`ItemType::HullPlate`] by the [`plan_plate_repair`] rule, and an Inventory
//! pane transfer moves items between two docked ships by the
//! [`plan_item_transfer`] rule. Stock is not saved: it returns to its authored
//! counts when the scenario loads again. A stack exists only while its count
//! is above zero.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::integrity::prelude::Health;

/// The whole module.
pub mod prelude {
    pub use super::{
        plan_item_transfer, plan_plate_repair, ItemCategoryType, ItemTransferRefusalType,
        ItemTransferType, ItemType, LootableShipMarker, PlateRepair, PlateRepairRefusalType,
        ShipInventory, HULL_PLATE_HEALTH,
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

/// The items one ship carries, as a count per item type. Required by every
/// [`SpaceshipRootMarker`](crate::markers::SpaceshipRootMarker).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct ShipInventory {
    stacks: BTreeMap<ItemType, u32>,
}

impl ShipInventory {
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
    /// On `count == 0` or a total above `u32::MAX`: the caller has a bug and
    /// must plan the add first, as [`plan_item_transfer`] does.
    pub fn add(&mut self, item: ItemType, count: u32) {
        assert!(count > 0, "ShipInventory adds 0 of {item:?}");
        let held = self.count(item);
        let total = held.checked_add(count).unwrap_or_else(|| {
            panic!("ShipInventory adds {count} of {item:?} to {held}, past u32::MAX")
        });
        self.stacks.insert(item, total);
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
    /// The target ship's count would pass `u32::MAX`.
    Overflow {
        /// What the target ship carries.
        held: u32,
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
    let held = target.count(item);
    if held.checked_add(quantity).is_none() {
        return Err(ItemTransferRefusalType::Overflow { held });
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

impl FromIterator<(ItemType, u32)> for ShipInventory {
    /// Collect one stack per item into an inventory.
    ///
    /// # Panics
    ///
    /// On a zero quantity or a repeated item: an empty stack is not a stack,
    /// and a caller that lists one item twice has a bug.
    fn from_iter<I: IntoIterator<Item = (ItemType, u32)>>(iter: I) -> Self {
        let mut stacks = BTreeMap::new();
        for (item, count) in iter {
            assert!(count > 0, "ShipInventory stack of {item:?} has quantity 0");
            let repeated = stacks.insert(item, count).is_some();
            assert!(!repeated, "ShipInventory lists {item:?} twice");
        }
        Self { stacks }
    }
}

// Hand-written rather than derived: a derived `Deserialize` would accept a
// zero-quantity or repeated-item stack and only the (panicking)
// `FromIterator` path would ever catch it, which is not how content lint
// reports an authoring error. This mirrors that same message so both paths
// name the fault the same way.
#[cfg(feature = "serde")]
impl serde::Serialize for ShipInventory {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.stacks.len()))?;
        for (item, count) in &self.stacks {
            map.serialize_entry(item, count)?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ShipInventory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ShipInventoryVisitor;

        impl<'de> serde::de::Visitor<'de> for ShipInventoryVisitor {
            type Value = ShipInventory;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map of item type to stack count")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut stacks = BTreeMap::new();
                while let Some((item, count)) = map.next_entry::<ItemType, u32>()? {
                    if count == 0 {
                        return Err(serde::de::Error::custom(format!(
                            "ShipInventory stack of {item:?} has quantity 0"
                        )));
                    }
                    if stacks.insert(item, count).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "ShipInventory lists {item:?} twice"
                        )));
                    }
                }
                Ok(ShipInventory { stacks })
            }
        }

        deserializer.deserialize_map(ShipInventoryVisitor)
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
    fn item_transfer_plans_refuse_in_order_and_never_overflow() {
        use ItemTransferRefusalType::*;
        use ItemTransferType::*;
        let plates =
            |count: u32| -> ShipInventory { [(ItemType::HullPlate, count)].into_iter().collect() };
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
        let full = plates(u32::MAX);
        assert_eq!(
            plan(Take, true, Some(1), &full, &partner),
            Err(Overflow { held: u32::MAX })
        );

        // A planned move conserves the total and empties a drained stack.
        let (mut own, mut partner) = (own, partner);
        let moved = plan(Take, true, Some(8), &own, &partner).expect("planned");
        partner.remove(ItemType::HullPlate, moved);
        own.add(ItemType::HullPlate, moved);
        assert_eq!(own.count(ItemType::HullPlate), 20);
        assert!(partner.is_empty());
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn authored_inventory_rejects_a_zero_stack_and_a_repeated_item() {
        let ok: ShipInventory = ron::from_str("{HullPlate: 12}").expect("valid stack parses");
        assert_eq!(ok.count(ItemType::HullPlate), 12);

        let zero = ron::from_str::<ShipInventory>("{HullPlate: 0}")
            .expect_err("a zero-quantity stack must fail");
        assert!(zero.to_string().contains("quantity 0"), "{zero}");

        let dup = ron::from_str::<ShipInventory>("{HullPlate: 1, HullPlate: 2}")
            .expect_err("a repeated item must fail");
        assert!(dup.to_string().contains("twice"), "{dup}");
    }
}
