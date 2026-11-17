//! Consumables: hold items a pilot uses for one timed effect.
//!
//! A [`ConsumableDesign`] is authored content keyed by the item it makes
//! usable. It names one effect from the closed [`ConsumableType`]; today the
//! only effect is a repair over time. The mod merge publishes the effective
//! consumables as [`GameConsumables`]; [`consumable_faults`] is the rule the
//! merge and content lint share.
//!
//! A use takes one item at once and starts the effect; [`plan_consumable_use`]
//! checks it first and a refusal takes nothing. A ship runs one effect at a
//! time. `nova_ship` runs the effect on the game clock; the pure rule a tick
//! shares health by is [`plan_repair_split`].

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::inventory::prelude::{GameItems, ItemDesignId, ItemRoleType};

/// Glob-import surface: `use nova_gameplay::consumable::prelude::*`.
pub mod prelude {
    pub use super::{
        consumable_faults, plan_consumable_use, plan_repair_split, ConsumableDesign,
        ConsumableRefusalType, ConsumableType, ConsumableUseFacts, GameConsumables,
        CONSUMABLE_DURATION_MAX_SECS,
    };
}

/// The longest an effect may run, in game seconds.
pub const CONSUMABLE_DURATION_MAX_SECS: f32 = 60.0;

/// One consumable effect and its numbers. Closed: each variant is code. One
/// variant today; the authored form names the effect, so a new effect is a
/// new variant and no content changes.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConsumableType {
    /// Heal `health` over `duration_secs`, each tick to the live, enabled
    /// section with the lowest health share. Never revives a lost section.
    Repair {
        /// Health points the whole effect restores.
        health: f32,
        /// Game seconds it runs.
        duration_secs: f32,
    },
}

impl ConsumableType {
    /// The word the status chip reads while it runs: `KIT`.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Repair { .. } => "KIT",
        }
    }

    /// Game seconds the effect runs.
    pub fn duration_secs(&self) -> f32 {
        match *self {
            Self::Repair { duration_secs, .. } => duration_secs,
        }
    }
}

/// One authored consumable: the item it makes usable and its effect. Every
/// field is required.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ConsumableDesign {
    /// The item a use takes one of; also the consumable's id.
    pub item: ItemDesignId,
    /// What a use does.
    pub effect: ConsumableType,
}

/// The merged consumables, by item. Inserted by the mod merge.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GameConsumables(pub BTreeMap<ItemDesignId, ConsumableDesign>);

impl GameConsumables {
    /// The catalog of `consumables`; a later one for an item replaces an
    /// earlier one.
    pub fn new(consumables: impl IntoIterator<Item = ConsumableDesign>) -> Self {
        Self(
            consumables
                .into_iter()
                .map(|consumable| (consumable.item.clone(), consumable))
                .collect(),
        )
    }

    /// The consumable of `item`, if the item is usable.
    pub fn get(&self, item: &ItemDesignId) -> Option<&ConsumableDesign> {
        self.0.get(item)
    }

    /// The consumable of `item`, for an item a running effect or a save
    /// check already resolved.
    ///
    /// # Panics
    ///
    /// When no consumable names the item: a running effect named an item the
    /// loaded content does not make usable, which the save check refuses
    /// first.
    pub fn design(&self, item: &ItemDesignId) -> &ConsumableDesign {
        self.get(item)
            .unwrap_or_else(|| panic!("item '{item}' has no consumable in the loaded catalog"))
    }

    /// Every consumable, in item id order.
    pub fn iter(&self) -> impl Iterator<Item = &ConsumableDesign> {
        self.0.values()
    }
}

/// Every fault of `consumable` on its own against `items`, keyed in the
/// message by its item: an unknown item, an item engine code already spends
/// ([`ItemRoleType`]), and a number out of range.
pub fn consumable_faults(consumable: &ConsumableDesign, items: &GameItems) -> Vec<String> {
    let item = &consumable.item;
    let mut faults = Vec::new();
    if items.get(item).is_none() {
        faults.push(format!("the consumable '{item}' names no item"));
    }
    if ItemRoleType::ALL
        .iter()
        .any(|role| role.id() == item.as_str())
    {
        faults.push(format!(
            "the consumable '{item}' names an item the game already spends; use another item"
        ));
    }
    let duration_secs = consumable.effect.duration_secs();
    if !duration_secs.is_finite()
        || duration_secs <= 0.0
        || duration_secs > CONSUMABLE_DURATION_MAX_SECS
    {
        faults.push(format!(
            "the consumable '{item}' runs {duration_secs} s; it must be above 0 and at most \
             {CONSUMABLE_DURATION_MAX_SECS} s"
        ));
    }
    match consumable.effect {
        ConsumableType::Repair { health, .. } => {
            if !health.is_finite() || health <= 0.0 {
                faults.push(format!(
                    "the consumable '{item}' heals {health}; it must be finite and above 0"
                ));
            }
        }
    }
    faults
}

/// What a use reads off the ship, gathered by the caller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConsumableUseFacts {
    /// Units of the item in the hold, less what running repair jobs hold
    /// back.
    pub held: u32,
    /// Whether an effect runs on the ship.
    pub running: bool,
    /// Whether a live, enabled section is below its full health.
    pub damaged: bool,
}

/// Why a use takes nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum ConsumableRefusalType {
    /// The hold carries none of the item.
    NoneHeld,
    /// An effect runs on the ship.
    Running,
    /// A repair, and no live section is damaged.
    NothingToRepair,
}

/// Check a use of `consumable` against `facts`, in
/// [`ConsumableRefusalType`] order. A pass takes one item; a refusal takes
/// nothing.
pub fn plan_consumable_use(
    consumable: &ConsumableDesign,
    facts: &ConsumableUseFacts,
) -> Result<(), ConsumableRefusalType> {
    if facts.held == 0 {
        return Err(ConsumableRefusalType::NoneHeld);
    }
    if facts.running {
        return Err(ConsumableRefusalType::Running);
    }
    match consumable.effect {
        ConsumableType::Repair { .. } if !facts.damaged => {
            Err(ConsumableRefusalType::NothingToRepair)
        }
        ConsumableType::Repair { .. } => Ok(()),
    }
}

/// Share `amount` health over `sections`, each `(current, max)`: the section
/// with the lowest share `current / max` first, filled to its max, then the
/// next, until `amount` runs out. A section at 0 health (lost) or at its max
/// takes nothing. Ties go to the earlier section. Returns `(index, new
/// current)` for each section that took health, in the order they took it.
/// What no section takes is lost.
pub fn plan_repair_split(amount: f32, sections: &[(f32, f32)]) -> Vec<(usize, f32)> {
    let mut order: Vec<usize> = (0..sections.len())
        .filter(|&index| {
            let (current, max) = sections[index];
            current > 0.0 && max > 0.0 && current < max
        })
        .collect();
    let share = |index: usize| sections[index].0 / sections[index].1;
    order.sort_by(|a, b| share(*a).total_cmp(&share(*b)).then(a.cmp(b)));
    let mut left = amount.max(0.0);
    let mut healed = Vec::new();
    for index in order {
        if left <= 0.0 {
            break;
        }
        let (current, max) = sections[index];
        let given = left.min(max - current);
        left -= given;
        healed.push((index, current + given));
    }
    healed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_items;

    fn kit(health: f32, duration_secs: f32) -> ConsumableDesign {
        ConsumableDesign {
            item: ItemDesignId::from("RepairKit"),
            effect: ConsumableType::Repair {
                health,
                duration_secs,
            },
        }
    }

    fn faults(consumable: &ConsumableDesign) -> Vec<String> {
        consumable_faults(consumable, &test_items())
    }

    #[test]
    fn the_base_numbers_have_no_faults() {
        assert_eq!(faults(&kit(80.0, 10.0)), Vec::<String>::new());
    }

    #[test]
    fn an_unknown_or_engine_item_is_refused() {
        let mut unknown = kit(80.0, 10.0);
        unknown.item = ItemDesignId::from("NoSuchItem");
        assert_eq!(faults(&unknown).len(), 1);
        assert!(faults(&unknown)[0].contains("names no item"));
        let mut role = kit(80.0, 10.0);
        role.item = ItemDesignId::from(ItemRoleType::HullPlate.id());
        assert_eq!(faults(&role).len(), 1);
        assert!(faults(&role)[0].contains("already spends"));
    }

    #[test]
    fn bad_numbers_are_refused() {
        assert_eq!(faults(&kit(0.0, 10.0)).len(), 1);
        assert_eq!(faults(&kit(f32::NAN, 10.0)).len(), 1);
        assert_eq!(faults(&kit(80.0, 0.0)).len(), 1);
        assert_eq!(faults(&kit(80.0, 61.0)).len(), 1);
        assert!(faults(&kit(80.0, 60.0)).is_empty());
    }

    #[test]
    fn a_use_is_refused_in_check_order() {
        let repair = kit(80.0, 10.0);
        let facts = ConsumableUseFacts {
            held: 0,
            running: true,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::NoneHeld)
        );
        let facts = ConsumableUseFacts {
            held: 1,
            running: true,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::Running)
        );
        let facts = ConsumableUseFacts {
            held: 1,
            ..default()
        };
        assert_eq!(
            plan_consumable_use(&repair, &facts),
            Err(ConsumableRefusalType::NothingToRepair)
        );
        let damaged = ConsumableUseFacts {
            held: 1,
            damaged: true,
            ..default()
        };
        assert_eq!(plan_consumable_use(&repair, &damaged), Ok(()));
    }

    #[test]
    fn a_repair_heals_the_lowest_share_first_and_carries_at_max() {
        // Shares: 0.5, 0.25, lost, full, 0.9.
        let sections = [
            (50.0, 100.0),
            (50.0, 200.0),
            (0.0, 100.0),
            (100.0, 100.0),
            (90.0, 100.0),
        ];
        assert_eq!(plan_repair_split(10.0, &sections), vec![(1, 60.0)]);
        // 150 fills section 1 and carries 0 to the rest.
        assert_eq!(plan_repair_split(150.0, &sections), vec![(1, 200.0)]);
        // 210 fills 1 (150), then 0 (50), then 10 into 4; nothing reaches the
        // lost or full ones.
        assert_eq!(
            plan_repair_split(210.0, &sections),
            vec![(1, 200.0), (0, 100.0), (4, 100.0)]
        );
        assert!(plan_repair_split(10.0, &[(0.0, 100.0), (100.0, 100.0)]).is_empty());
    }
}
