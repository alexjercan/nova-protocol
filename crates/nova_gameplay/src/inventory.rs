//! What a ship carries: a closed set of item types, the category each belongs
//! to, and the per-ship stack counts.
//!
//! Every ship root requires a [`ShipInventory`], empty by default, so a reader
//! can fetch it from any ship without a fallback. The open-world player ship
//! starts with authored stock through `SpaceshipConfig::inventory`; nothing
//! else seeds it, and nothing spends or adds items at runtime. A stack exists
//! only while its count is above zero.

use std::collections::BTreeMap;

use bevy::prelude::*;

/// The whole module.
pub mod prelude {
    pub use super::{ItemCategoryType, ItemType, ShipInventory};
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
