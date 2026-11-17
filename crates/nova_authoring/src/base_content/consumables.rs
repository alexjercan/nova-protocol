//! Built-in CONSUMABLE content: the base items a pilot uses from the
//! Inventory pane.
//!
//! `content gen` serializes this builder into
//! `assets/base/consumables/base.content.ron`. The one consumable is the
//! repair kit. Its numbers are provisional, like the item prices.

use nova_gameplay::prelude::{ConsumableDesign, ConsumableType, ItemDesignId};

use super::items::REPAIR_KIT;

/// The base game's consumables.
pub(crate) fn consumable_catalog() -> Vec<ConsumableDesign> {
    vec![
        // Four hull plates of health in one use, no section choice.
        ConsumableDesign {
            item: ItemDesignId::from(REPAIR_KIT),
            effect: ConsumableType::Repair {
                health: 80.0,
                duration_secs: 10.0,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use nova_gameplay::{prelude::consumable_faults, test_support::test_items};

    use super::*;

    #[test]
    fn base_consumables_have_no_faults() {
        let items = test_items();
        for consumable in consumable_catalog() {
            assert_eq!(
                consumable_faults(&consumable, &items),
                Vec::<String>::new(),
                "{}",
                consumable.item
            );
        }
    }
}
