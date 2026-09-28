//! The TAB interface's Inventory pane: what the player ship carries and, while
//! it is docked, what the docked ship carries.
//!
//! The pane reads the live [`ShipInventory`](nova_gameplay::prelude::ShipInventory)
//! of the player ship and of the partner its `DockingConnection` names: category
//! filters, one column per ship and an inspector for the clicked row. While
//! docked, the inspector opens a transfer form: Give from the player's row to
//! any partner, Take from the partner's row when the partner is neutralized or
//! lootable. Confirm writes an [`InventoryTransferCommand`], and
//! [`apply_inventory_transfer_commands`] moves the items by the
//! [`plan_item_transfer`](nova_gameplay::prelude::plan_item_transfer) rule or
//! refuses with no change. There is no price and no capacity.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `app` | The pane body, its refresh, the transfer form and its handler. |

mod app;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_gameplay::prelude::{ItemCategoryType, ItemTransferType, ItemType};

pub(crate) use self::app::*;

/// Drives the Inventory pane's columns and inspector.
pub(crate) struct InventoryPanePlugin;

impl Plugin for InventoryPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryRuntime>();
        app.add_message::<InventoryTransferCommand>();
        app.add_systems(
            Update,
            (
                apply_inventory_transfer_commands,
                type_inventory_draft,
                update_inventory_panel,
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
/// open transfer form and the result note.
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
    /// The transfer form the selected row opened, if it offers one.
    pub(crate) draft: Option<InventoryDraft>,
    /// The last transfer result and its remaining seconds on screen.
    pub(crate) note: Option<(String, f32)>,
}

/// A transfer waiting for its quantity to be confirmed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InventoryDraft {
    /// Take from the partner or Give to it.
    pub(crate) transfer: ItemTransferType,
    /// The item to move.
    pub(crate) item: ItemType,
    /// The one quantity the wheel, the slider, the field and All set. `None`
    /// while the field holds text that is not a whole number, which Confirm
    /// refuses.
    pub(crate) quantity: Option<u32>,
}

/// A confirmed transfer between the player ship and its docked partner,
/// written only by the form's Confirm and applied by
/// [`apply_inventory_transfer_commands`].
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InventoryTransferCommand {
    /// Take from the partner or Give to it.
    pub(crate) transfer: ItemTransferType,
    /// The item to move.
    pub(crate) item: ItemType,
    /// The quantity; `None` when the typed text is not a whole number.
    pub(crate) quantity: Option<u32>,
}
