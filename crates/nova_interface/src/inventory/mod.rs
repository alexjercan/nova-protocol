//! The TAB interface's Inventory pane: what the player ship carries and, while
//! it is docked, what the docked ship carries.
//!
//! The pane reads the live [`ShipInventory`](nova_gameplay::prelude::ShipInventory)
//! of the player ship and of the partner its `DockingConnection` names: category
//! filters, one column per ship with its credits, the player's load against its
//! capacity, and an inspector for the clicked row. While docked, the inspector
//! opens an action form. The player's row opens Sell when the partner trades:
//! it is neither neutralized nor lootable. The form switches a Sell to Give and
//! back. The player's row opens Give when the partner is neutralized or
//! lootable. The partner's row
//! opens Take when the partner is neutralized or lootable, and Buy otherwise.
//! While undocked with a live cargo intake, the player's row opens a Jettison
//! form. Confirm writes an [`InventoryActionCommand`], and
//! [`apply_inventory_action_commands`] moves the items, and for a trade the
//! credits, by the
//! [`plan_item_transfer`](nova_gameplay::prelude::plan_item_transfer),
//! [`plan_item_trade`](nova_gameplay::prelude::plan_item_trade) or
//! [`plan_item_jettison`](nova_gameplay::prelude::plan_item_jettison) rule, or
//! refuses with no change. The partner column header also carries a
//! separate Take credits button, shown only while the partner is neutralized
//! or lootable and holds a credit above zero, even with an empty hold; one
//! click writes a [`CreditTakeCommand`] and
//! [`apply_credit_take_commands`] moves its whole balance by the
//! [`plan_credit_take`](nova_gameplay::prelude::plan_credit_take) rule, or
//! refuses with no change.
//!
//! The keyboard reaches the same selection: `viewer_next` and `viewer_prev`
//! step through the shown rows as a click would, and `inventory_details`
//! opens the full description the inspector clips to four lines.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `app` | The pane body, its refresh, the action form and its handler. |

mod app;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_gameplay::prelude::{ItemCategoryType, ItemTradeType, ItemTransferType, ItemType};
use nova_ui::input_mode::prelude::{in_input_mode, InputMode};

pub(crate) use self::app::*;

/// Drives the Inventory pane's columns and inspector.
pub(crate) struct InventoryPanePlugin;

impl Plugin for InventoryPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryRuntime>();
        app.add_message::<InventoryActionCommand>();
        app.add_message::<CreditTakeCommand>();
        app.add_systems(
            Update,
            (
                inventory_keys.run_if(in_input_mode(InputMode::Normal)),
                apply_inventory_action_commands,
                apply_credit_take_commands,
                type_inventory_draft,
                update_inventory_panel,
                update_inventory_about,
                sync_inventory_draft_controls,
            )
                .chain()
                .in_set(InventoryPaneSystems),
        );
    }
}

/// System set for the Inventory pane's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InventoryPaneSystems;

/// Which column a row belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InventorySideType {
    /// The player ship.
    Own,
    /// The ship the player ship is docked with.
    Partner,
}

/// What the Inventory pane shows: the category filter, the selected row, the
/// open action form and the result note.
///
/// Kept while the interface is closed, like the pane itself. A selection that
/// no longer names a shown row, and a draft the selection no longer offers, are
/// cleared by [`update_inventory_panel`].
#[derive(Resource, Clone, PartialEq, Debug, Default)]
pub(crate) struct InventoryRuntime {
    /// The one category the rows show; `None` shows every category.
    pub(crate) filter: Option<ItemCategoryType>,
    /// The row the inspector shows.
    pub(crate) selected: Option<(InventorySideType, ItemType)>,
    /// The action form the selected row opened, if it offers one.
    pub(crate) draft: Option<InventoryDraft>,
    /// The last action result and its remaining seconds on screen.
    pub(crate) note: Option<(String, f32)>,
    /// Whether `inventory_details` holds the selected item's full description
    /// open. A new selection closes it.
    pub(crate) details: bool,
}

/// What an Inventory pane form does with its items.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InventoryActionType {
    /// Move items from the docked partner to the player ship.
    Take,
    /// Move items from the player ship to the docked partner.
    Give,
    /// Move items from the trading partner to the player ship, paying its ask.
    Buy,
    /// Move items from the player ship to the trading partner, paid its bid.
    Sell,
    /// Queue items from the player ship as canisters on its cargo intake.
    Jettison,
}

impl InventoryActionType {
    /// The transfer this action is, or `None` for a trade or a jettison.
    pub(crate) fn transfer(self) -> Option<ItemTransferType> {
        match self {
            Self::Take => Some(ItemTransferType::Take),
            Self::Give => Some(ItemTransferType::Give),
            Self::Buy | Self::Sell | Self::Jettison => None,
        }
    }

    /// The trade this action is, or `None` for a transfer or a jettison.
    pub(crate) fn trade(self) -> Option<ItemTradeType> {
        match self {
            Self::Buy => Some(ItemTradeType::Buy),
            Self::Sell => Some(ItemTradeType::Sell),
            Self::Take | Self::Give | Self::Jettison => None,
        }
    }
}

/// An action waiting for its quantity to be confirmed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InventoryDraft {
    /// Take, Give, Buy, Sell or Jettison.
    pub(crate) action: InventoryActionType,
    /// The item to move.
    pub(crate) item: ItemType,
    /// The one quantity the wheel, the slider, the field and All set. `None`
    /// while the field holds text that is not a whole number, which Confirm
    /// refuses.
    pub(crate) quantity: Option<u32>,
}

/// A confirmed action on the player ship's inventory, written only by the
/// form's Confirm and applied by [`apply_inventory_action_commands`].
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InventoryActionCommand {
    /// Take, Give, Buy, Sell or Jettison.
    pub(crate) action: InventoryActionType,
    /// The item to move.
    pub(crate) item: ItemType,
    /// The quantity; `None` when the typed text is not a whole number.
    pub(crate) quantity: Option<u32>,
}

/// Take the docked partner's whole credit balance, written only by the
/// partner column header's Take credits button and applied by
/// [`apply_credit_take_commands`]. No item and no quantity: the balance moves
/// whole on one click, unlike an item Take or Give.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CreditTakeCommand;
