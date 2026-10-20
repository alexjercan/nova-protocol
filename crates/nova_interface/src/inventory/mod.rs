//! The TAB interface's Inventory pane: what the player ship carries and, while
//! it is docked, what the docked ship carries and, at a living station that
//! offers Storage, what the station stores for the player.
//!
//! The pane reads the live [`ShipInventory`](nova_gameplay::prelude::ShipInventory)
//! of the player ship and of the partner its `DockingConnection` names: category
//! filters, one column per ship with its credits, the player's load against its
//! capacity, and an inspector for the clicked row. While docked, the inspector
//! opens an action form. A ship partner trades while it is neither neutralized
//! nor lootable; a station partner trades only while it is living and offers
//! Trade, by [`station_offers`](nova_gameplay::prelude::station_offers). The
//! player's row opens Give when the partner is neutralized or lootable, Sell
//! when it trades, Store when it is a living station that offers Storage, and
//! Give otherwise; the form switches between Give, Sell and each Store where
//! the partner takes them. The partner's row opens Take when the partner is
//! neutralized or lootable, Buy when it trades, and nothing otherwise. A
//! storage column lists the station's [`StationStorage`](nova_gameplay::prelude::StationStorage)
//! while it offers Storage, and its row opens Withdraw. Under it, one section
//! per own ship hold: the player's own ships docked at the station, or the
//! docked partner itself when it is one of them, each with its own capacity.
//! Its row opens Withdraw from that hold, and the player's row switches to a
//! `To <ship>` Store into it. A partner that is an own ship shows no partner
//! column and does not trade. A derelict station is lootable and offers no
//! service. While undocked with a live cargo intake,
//! the player's row opens a Jettison form. Confirm writes an
//! [`InventoryActionCommand`], and [`apply_inventory_action_commands`] moves
//! the items, and for a trade the credits, by the
//! [`plan_item_transfer`](nova_gameplay::prelude::plan_item_transfer),
//! [`plan_item_trade`](nova_gameplay::prelude::plan_item_trade),
//! [`plan_item_storage`](nova_gameplay::prelude::plan_item_storage) or
//! [`plan_item_jettison`](nova_gameplay::prelude::plan_item_jettison) rule, or
//! refuses with no change. The partner column's Credits row also carries a
//! separate Take credits button, shown only while the partner is neutralized
//! or lootable and holds a credit above zero, even with an empty hold; one
//! click writes a [`CreditTakeCommand`] and
//! [`apply_credit_take_commands`] moves its whole balance by the
//! [`plan_credit_take`](nova_gameplay::prelude::plan_credit_take) rule, or
//! refuses with no change.
//!
//! A double click on a row selects it and confirms the form it opens, at the
//! whole stack, or one unit with Shift held: Sell or Store (whichever chip the
//! form shows) or Give from the hold, Withdraw from storage, Buy or Take from
//! the partner. It writes the same [`InventoryActionCommand`] Confirm does, so
//! it refuses exactly as Confirm would. Jettison has no double click.
//!
//! The keyboard reaches the same selection: `viewer_next` and `viewer_prev`
//! step through the shown rows as a click would, and `inventory_details`
//! opens the full description the inspector clips to four lines.
//!
//! Docked with a wreck the player may loot (neutralized, or a derelict
//! carrying `LootableShipMarker`) that is not a living station, the partner
//! column carries two more controls. Take all, in its Credits row, writes a
//! [`TakeAllCommand`]: [`apply_take_all_commands`] takes as much of every
//! stack as the hold has room for through the Take path, then the credits,
//! and names on the note line what stayed. Over the column, the strip block
//! strips the whole hull (see the `strip` module).
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `app` | The pane body, its refresh, the action form and its handler. |
//! | `strip` | The strip block, the strip action handler and the strip timer. |
//! | `market` | The MARKET strip: the trader's leads, restock and refusal. |

mod app;
mod market;
mod strip;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    GameItems, ItemCategoryType, ItemDesignId, ItemStorageType, ItemTradeType, ItemTransferType,
};
use nova_ui::{
    input_mode::prelude::{in_input_mode, InputMode},
    widget::DoubleClick,
};

pub(crate) use self::{app::*, market::*, strip::*};
pub use self::{
    app::{INVENTORY_BUY_ALL, INVENTORY_SELL_ALL, INVENTORY_TAKE_ALL},
    market::{INVENTORY_MARKET_REFUSED, INVENTORY_MARKET_STRIP},
    strip::{
        HullStripCommand, INVENTORY_STRIP_BLOCK, INVENTORY_STRIP_BUTTON, INVENTORY_STRIP_PROGRESS,
    },
};

/// Glob-import surface: `use nova_interface::inventory::prelude::*`.
pub mod prelude {
    pub use super::{
        app::{INVENTORY_BUY_ALL, INVENTORY_SELL_ALL, INVENTORY_TAKE_ALL},
        market::{INVENTORY_MARKET_REFUSED, INVENTORY_MARKET_STRIP},
        strip::{
            HullStripCommand, INVENTORY_STRIP_BLOCK, INVENTORY_STRIP_BUTTON,
            INVENTORY_STRIP_PROGRESS,
        },
        InventoryRuntime,
    };
}

/// Drives the Inventory pane's columns and inspector.
pub(crate) struct InventoryPanePlugin;

impl Plugin for InventoryPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryRuntime>();
        app.add_message::<InventoryActionCommand>();
        app.add_message::<CreditTakeCommand>();
        app.add_message::<TakeAllCommand>();
        app.add_message::<HullStripCommand>();
        // Every pane system reads the item catalog, which lands only at the mod
        // merge and never on a refused base game; the set runs from boot.
        app.configure_sets(
            Update,
            InventoryPaneSystems.run_if(resource_exists::<GameItems>),
        );
        app.add_systems(
            Update,
            (
                inventory_keys.run_if(in_input_mode(InputMode::Normal)),
                apply_inventory_action_commands,
                apply_credit_take_commands,
                apply_take_all_commands,
                apply_hull_strip_commands,
                // Runs with the pane shut too: a strip goes on in flight.
                advance_hull_strip,
                type_inventory_draft,
                update_inventory_panel,
                update_inventory_trade_controls,
                update_market_strip,
                update_strip_block,
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

/// Which column a row belongs to, and for the storage column which section.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InventorySideType {
    /// The player ship.
    Own,
    /// The ship the player ship is docked with.
    Partner,
    /// The storage of the living station the player ship is docked with.
    Storage,
    /// The hold of one of the player's own ships, listed in the storage
    /// column (see `InventoryShips::own_holds`). Names a row's section, never
    /// a column.
    Hold(Entity),
}

/// Where a Store puts items and a Withdraw takes them from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InventoryStoreType {
    /// The docked station's [`StationStorage`](nova_gameplay::prelude::StationStorage).
    Station,
    /// The hold of one of the player's own ships.
    Ship(Entity),
}

/// What the Inventory pane shows: the category filter, the selected row, the
/// open action form and the result note.
///
/// Kept while the interface is closed, like the pane itself. A selection that
/// no longer names a shown row, and a draft the selection no longer offers, are
/// cleared by [`update_inventory_panel`].
#[derive(Resource, Clone, PartialEq, Debug, Default)]
pub struct InventoryRuntime {
    /// The one category the rows show; `None` shows every category.
    pub(crate) filter: Option<ItemCategoryType>,
    /// The row the inspector shows.
    pub(crate) selected: Option<(InventorySideType, ItemDesignId)>,
    /// The action form the selected row opened, if it offers one.
    pub(crate) draft: Option<InventoryDraft>,
    /// The last action result and its remaining seconds on screen.
    pub(crate) note: Option<(String, f32)>,
    /// Whether `inventory_details` holds the selected item's full description
    /// open. A new selection closes it.
    pub(crate) details: bool,
    /// The Give, Sell or Store chip last clicked; a player's row opens it
    /// while the partner takes it, else the action the row offers.
    pub(crate) own_action: Option<InventoryActionType>,
    /// The row the last click landed on, for a double click.
    pub(crate) click: DoubleClick<(InventorySideType, ItemDesignId)>,
}

impl InventoryRuntime {
    /// The last action result on the note line, while it shows: a take, a
    /// trade, or why a strip was refused or stopped.
    pub fn note(&self) -> Option<&str> {
        self.note.as_ref().map(|(note, _)| note.as_str())
    }

    /// Show `note` on the note line for [`NOTE_SECONDS`].
    pub(crate) fn say(&mut self, note: String) {
        self.note = Some((note, NOTE_SECONDS));
    }
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
    /// Move items from the player ship into the station's storage or an own
    /// ship's hold.
    Store(InventoryStoreType),
    /// Move items from the station's storage or an own ship's hold to the
    /// player ship.
    Withdraw(InventoryStoreType),
}

impl InventoryActionType {
    /// The transfer this action is, or `None` for any other action.
    pub(crate) fn transfer(self) -> Option<ItemTransferType> {
        match self {
            Self::Take => Some(ItemTransferType::Take),
            Self::Give => Some(ItemTransferType::Give),
            Self::Buy | Self::Sell | Self::Jettison | Self::Store(_) | Self::Withdraw(_) => None,
        }
    }

    /// The trade this action is, or `None` for any other action.
    pub(crate) fn trade(self) -> Option<ItemTradeType> {
        match self {
            Self::Buy => Some(ItemTradeType::Buy),
            Self::Sell => Some(ItemTradeType::Sell),
            Self::Take | Self::Give | Self::Jettison | Self::Store(_) | Self::Withdraw(_) => None,
        }
    }

    /// The storage move this action is and where, or `None` for any other
    /// action.
    pub(crate) fn storage(self) -> Option<(ItemStorageType, InventoryStoreType)> {
        match self {
            Self::Store(target) => Some((ItemStorageType::Store, target)),
            Self::Withdraw(target) => Some((ItemStorageType::Withdraw, target)),
            Self::Take | Self::Give | Self::Buy | Self::Sell | Self::Jettison => None,
        }
    }
}

/// An action waiting for its quantity to be confirmed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct InventoryDraft {
    /// What the draft or command does with its items.
    pub(crate) action: InventoryActionType,
    /// The item to move.
    pub(crate) item: ItemDesignId,
    /// The one quantity the wheel, the slider, the field and All set. `None`
    /// while the field holds text that is not a whole number, which Confirm
    /// refuses.
    pub(crate) quantity: Option<u32>,
}

/// A confirmed action on the player ship's inventory, written only by the
/// form's Confirm and applied by [`apply_inventory_action_commands`].
#[derive(Message, Clone, PartialEq, Eq, Debug)]
pub(crate) struct InventoryActionCommand {
    /// What the draft or command does with its items.
    pub(crate) action: InventoryActionType,
    /// The item to move.
    pub(crate) item: ItemDesignId,
    /// The quantity; `None` when the typed text is not a whole number.
    pub(crate) quantity: Option<u32>,
}

/// Take the docked partner's whole credit balance, written only by the
/// partner column's Take credits button and applied by
/// [`apply_credit_take_commands`]. No item and no quantity: the balance moves
/// whole on one click, unlike an item Take or Give.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CreditTakeCommand;

/// Take everything the docked lootable partner holds, written only by the
/// partner column's Take all button and applied by
/// [`apply_take_all_commands`]: as much of each stack as fits, then the
/// credits.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TakeAllCommand;
