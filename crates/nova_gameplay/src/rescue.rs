//! The SOS rescue rules: when a player ship is stranded, how long a rescue
//! call takes and what its bill is.
//!
//! A stranded player in the open world can call for rescue from the Ship
//! pane. The call runs [`RESCUE_SECS`] of game time; the answer moves the ship
//! to a station, rebuilds what it needs to dock and fly, and takes the bill
//! [`plan_rescue_bill`] plans. `nova_ship` owns the call and
//! `nova_world_base` the answer; this module holds only the pure rules both
//! and the pane read.

use std::collections::BTreeMap;

use crate::{
    inventory::{BuildRecipe, GameItems, ItemDesignId, ShipInventory},
    jobs::DISTRESS_BASE_CR,
};

/// The rescue rules.
pub mod prelude {
    pub use super::{
        plan_rescue_bill, stranded_reasons, RescueBill, RescueSectionBill, StrandedFacts,
        StrandedType, RESCUE_FEE_CR, RESCUE_SECS,
    };
}

/// Game seconds a rescue call takes from the click to the answer. The TAB
/// interface and the pause menu stop the game clock, so they hold the call.
pub const RESCUE_SECS: f32 = 60.0;

/// The fee every answered call takes before the sections it rebuilds: what
/// answering a distress call pays.
pub const RESCUE_FEE_CR: u32 = DISTRESS_BASE_CR;

/// What the stranded rule reads off a player ship. A ship that lost only
/// its guns can still fly and dock, so it is not stranded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StrandedFacts {
    /// At least one thruster section is live and not inactive.
    pub working_thruster: bool,
    /// At least one flight computer section is live and not inactive.
    pub working_computer: bool,
    /// At least one docking port is live, not inactive and holds no dock.
    pub free_port: bool,
}

/// Why a ship is stranded, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrandedType {
    /// No thruster works.
    NoThruster,
    /// No flight computer works.
    NoFlightComputer,
    /// No docking port is free and working.
    NoDockingPort,
}

/// Every reason `facts` make a ship stranded, in [`StrandedType`] order.
/// Empty for a ship that can fly and dock: it is offered no rescue.
pub fn stranded_reasons(facts: StrandedFacts) -> Vec<StrandedType> {
    [
        (!facts.working_thruster, StrandedType::NoThruster),
        (!facts.working_computer, StrandedType::NoFlightComputer),
        (!facts.free_port, StrandedType::NoDockingPort),
    ]
    .into_iter()
    .filter_map(|(holds, reason)| holds.then_some(reason))
    .collect()
}

/// A planned rescue bill.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RescueBill {
    /// Parts the rebuilt sections take from the ship's hold.
    pub from_hold: BTreeMap<ItemDesignId, u32>,
    /// Each rebuilt section's share, in recipe order.
    pub sections: Vec<RescueSectionBill>,
    /// The whole bill: [`RESCUE_FEE_CR`] plus each recipe's credits and the
    /// parts the hold lacks at reference ask.
    pub owed_cr: u64,
    /// What the ship pays: the bill, or its whole balance when it holds
    /// less. A broke ship is still rescued.
    pub charged_cr: u32,
}

/// One rebuilt section's share of a [`RescueBill`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RescueSectionBill {
    /// Parts its recipe takes from the ship's hold.
    pub from_hold: BTreeMap<ItemDesignId, u32>,
    /// Its recipe's credits plus the parts the hold lacks at reference ask.
    pub owed_cr: u64,
}

impl RescueBill {
    /// The credits each section's `Rebuilt` report states: the charge goes
    /// to the [`RESCUE_FEE_CR`] fee first, then to each section in order,
    /// so the shares never sum above [`charged_cr`](Self::charged_cr).
    pub fn section_charges_cr(&self) -> Vec<u32> {
        let mut left = u64::from(self.charged_cr).saturating_sub(u64::from(RESCUE_FEE_CR));
        self.sections
            .iter()
            .map(|section| {
                let share = section.owed_cr.min(left);
                left -= share;
                share as u32
            })
            .collect()
    }
}

/// Plan the bill of a rescue that rebuilds one section per recipe of
/// `recipes`, for a ship with the hold `own` and the balance `own_cr`.
///
/// Parts come from `own` first, across all recipes in order; the rest are
/// priced at [`ItemPrice::buy_cr`](crate::market::ItemPrice::buy_cr) of the
/// reference price. The ship pays the bill or its whole balance, whichever is
/// less: credits never go below zero, and nothing refuses.
///
/// # Panics
///
/// When `items` lacks a recipe part; content lint refuses that recipe first.
pub fn plan_rescue_bill(
    items: &GameItems,
    recipes: &[&BuildRecipe],
    own: &ShipInventory,
    own_cr: u32,
) -> RescueBill {
    let mut from_hold: BTreeMap<ItemDesignId, u32> = BTreeMap::new();
    let mut owed = u64::from(RESCUE_FEE_CR);
    let mut sections = Vec::with_capacity(recipes.len());
    for recipe in recipes {
        let mut section = RescueSectionBill {
            owed_cr: u64::from(recipe.credits),
            ..RescueSectionBill::default()
        };
        for (item, &count) in &recipe.parts {
            let taken = from_hold.get(item).copied().unwrap_or(0);
            let held = own.count(item).saturating_sub(taken).min(count);
            if held > 0 {
                *from_hold.entry(item.clone()).or_default() += held;
                section.from_hold.insert(item.clone(), held);
            }
            let missing = count - held;
            if missing > 0 {
                section.owed_cr += items.design(item).reference_price().buy_cr(missing);
            }
        }
        owed += section.owed_cr;
        sections.push(section);
    }
    RescueBill {
        from_hold,
        sections,
        owed_cr: owed,
        charged_cr: owed.min(u64::from(own_cr)) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_items;

    fn port_recipe() -> BuildRecipe {
        BuildRecipe {
            credits: 50,
            parts: [
                (ItemDesignId::from("DockingCollar"), 1),
                (ItemDesignId::from("SectionPanel"), 2),
            ]
            .into(),
            strip_yield: 0.5,
        }
    }

    fn drive_recipe() -> BuildRecipe {
        BuildRecipe {
            credits: 30,
            parts: [
                (ItemDesignId::from("DriveBell"), 1),
                (ItemDesignId::from("SectionPanel"), 2),
            ]
            .into(),
            strip_yield: 0.5,
        }
    }

    /// A ship that flies and docks is offered nothing, guns or not; each
    /// missing ability is one reason, in order.
    #[test]
    fn a_ship_is_stranded_only_while_it_cannot_fly_or_dock() {
        let fine = StrandedFacts {
            working_thruster: true,
            working_computer: true,
            free_port: true,
        };
        assert!(stranded_reasons(fine).is_empty());
        assert_eq!(
            stranded_reasons(StrandedFacts {
                free_port: false,
                ..fine
            }),
            vec![StrandedType::NoDockingPort]
        );
        assert_eq!(
            stranded_reasons(StrandedFacts::default()),
            vec![
                StrandedType::NoThruster,
                StrandedType::NoFlightComputer,
                StrandedType::NoDockingPort,
            ]
        );
    }

    /// The bill is the fee plus each recipe; parts in the hold are used
    /// first, across recipes, and only the rest is priced at ask.
    #[test]
    fn a_rescue_bill_uses_hold_parts_first_and_prices_the_rest() {
        let items = test_items();
        assert_eq!(
            plan_rescue_bill(
                &items,
                &[],
                &ShipInventory::new(&items, 1_000_000, []),
                5_000
            ),
            RescueBill {
                from_hold: BTreeMap::new(),
                sections: Vec::new(),
                owed_cr: u64::from(RESCUE_FEE_CR),
                charged_cr: RESCUE_FEE_CR,
            }
        );
        let own = ShipInventory::new(
            &items,
            1_000_000,
            [
                (ItemDesignId::from("SectionPanel"), 3),
                (ItemDesignId::from("DockingCollar"), 1),
            ],
        );
        let (port, drive) = (port_recipe(), drive_recipe());
        let bill = plan_rescue_bill(&items, &[&port, &drive], &own, 5_000);
        assert_eq!(
            bill.from_hold,
            [
                (ItemDesignId::from("DockingCollar"), 1),
                (ItemDesignId::from("SectionPanel"), 3),
            ]
            .into()
        );
        // Fee, both recipes' credits, one panel and the drive bell at ask.
        let ask = |id: &str, count| {
            items
                .design(&ItemDesignId::from(id))
                .reference_price()
                .buy_cr(count)
        };
        let owed =
            u64::from(RESCUE_FEE_CR) + 50 + 30 + ask("SectionPanel", 1) + ask("DriveBell", 1);
        assert_eq!(bill.owed_cr, owed);
        assert_eq!(u64::from(bill.charged_cr), owed);
        assert_eq!(
            bill.sections.iter().map(|s| s.owed_cr).collect::<Vec<_>>(),
            [50, 30 + ask("SectionPanel", 1) + ask("DriveBell", 1)],
            "the port takes its parts from the hold; the drive buys the rest"
        );
        assert_eq!(
            bill.section_charges_cr(),
            bill.sections
                .iter()
                .map(|s| s.owed_cr as u32)
                .collect::<Vec<_>>()
        );
    }

    /// A ship short of the bill pays its whole balance and no more; a broke
    /// ship pays nothing and is still rescued.
    #[test]
    fn a_rescue_bill_takes_at_most_the_whole_balance() {
        let items = test_items();
        let empty = ShipInventory::new(&items, 1_000_000, []);
        let port = port_recipe();
        let short = plan_rescue_bill(&items, &[&port], &empty, 120);
        assert!(short.owed_cr > 120);
        assert_eq!(short.charged_cr, 120);
        assert_eq!(
            short.section_charges_cr(),
            [0],
            "a charge under the fee leaves the sections nothing"
        );
        assert_eq!(plan_rescue_bill(&items, &[&port], &empty, 0).charged_cr, 0);
    }
}
