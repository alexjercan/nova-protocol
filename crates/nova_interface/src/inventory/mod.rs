//! The TAB interface's Inventory pane: what the player ship carries and, while
//! it is docked, what the docked ship carries.
//!
//! The pane reads the live [`ShipInventory`](nova_gameplay::prelude::ShipInventory)
//! of the player ship and of the partner its `DockingConnection` names. It
//! draws only that state: category filters, one column per ship and an
//! inspector for the clicked row. It moves no items; nothing here claims a
//! transfer, a price or a capacity.
//!
//! # Module layout
//!
//! | Module | Concern |
//! | --- | --- |
//! | `app` | The pane body, its refresh and its click observers. |

mod app;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_gameplay::prelude::{ItemCategoryType, ItemType};

pub(crate) use self::app::*;

/// Drives the Inventory pane's columns and inspector.
pub(crate) struct InventoryPanePlugin;

impl Plugin for InventoryPanePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryRuntime>();
        app.add_systems(Update, update_inventory_panel.in_set(InventoryPaneSystems));
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

/// What the Inventory pane shows: the category filter and the selected row.
///
/// Kept while the interface is closed, like the pane itself. A selection that
/// no longer names a shown row is cleared by [`update_inventory_panel`].
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct InventoryRuntime {
    /// The one category the rows show; `None` shows every category.
    pub(crate) filter: Option<ItemCategoryType>,
    /// The row the inspector shows.
    pub(crate) selected: Option<(InventorySideType, ItemType)>,
}
