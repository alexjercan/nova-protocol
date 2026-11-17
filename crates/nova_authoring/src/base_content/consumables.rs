//! Built-in CONSUMABLE content: the base items a pilot uses in flight.
//!
//! `content gen` serializes this builder into
//! `assets/base/consumables/base.content.ron`. One consumable per effect
//! type, each on its own flight key: the repair kit on 1, the boost cell on
//! 2, the decoy flare on 3. The numbers are provisional, like the item
//! prices. The flare lure stands 800 m off, above the 300 m blast of every
//! base torpedo bay.

use nova_events::prelude::Meters;
use nova_gameplay::prelude::{ConsumableDesign, ConsumableType, ItemDesignId};

use super::items::{BOOST_CELL, DECOY_FLARE, REPAIR_KIT};

/// The base game's consumables, in key order.
pub(crate) fn consumable_catalog() -> Vec<ConsumableDesign> {
    let consumable = |item: &str, effect| ConsumableDesign {
        item: ItemDesignId::from(item),
        effect,
    };
    vec![
        // Four hull plates of health in one key press, no section choice.
        consumable(
            REPAIR_KIT,
            ConsumableType::Repair {
                health: 80.0,
                duration_secs: 10.0,
            },
        ),
        consumable(
            BOOST_CELL,
            ConsumableType::ThrustBoost {
                factor: 1.5,
                duration_secs: 8.0,
            },
        ),
        // Torpedoes inside 400 m keep the lock: the PDCs are the last layer.
        consumable(
            DECOY_FLARE,
            ConsumableType::Flare {
                near_m: Meters(400.0),
                far_m: Meters(3000.0),
                lure_m: Meters(800.0),
            },
        ),
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
                consumable_faults(&consumable, &items, Meters(300.0)),
                Vec::<String>::new(),
                "{}",
                consumable.item
            );
        }
    }
}
