//! The inspector's `Use` button: one click on the player's own row of an item
//! that has a `Consumable` uses one, docked or not.
//!
//! The click writes a [`ConsumableUseRequest`], which passes or refuses by
//! `use_consumables`. The verb sits at the right end of its own row, hidden
//! unless the selected row is the player's own row of a consumable item, and
//! disabled while that rule would refuse, read off the same facts
//! ([`ConsumableFactsQuery`]), so a press never does nothing. The row's count drops by one on a use. The interface holds the
//! game clock, so a repair started here runs once the interface closes.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::{
    plan_consumable_use, GameConsumables, ItemDesignId, PlayerSpaceshipMarker, ShipInventory,
};
use nova_ship::prelude::{ConsumableFactsQuery, ConsumableUseRequest};
use nova_ui::prelude::{button, verb_row, ButtonCueType, ButtonSpec, RowAlign, VerbGate};

use super::{InventoryRuntime, InventorySideType};

/// The name of the inspector's Use button.
pub const INVENTORY_USE_BUTTON: &str = "InventoryUse";

/// Marker for the inspector's Use button.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryUseButton;

/// Spawn the Use verb's row into the inspector's item part. The row takes no
/// space while [`update_inventory_use_button`] hides the verb.
pub(crate) fn spawn_inventory_use_button(item: &mut ChildSpawnerCommands) {
    let mut row = item.spawn((InventoryUseRow, verb_row(RowAlign::End)));
    row.entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    row.with_children(|row| {
        row.spawn((
            Name::new(INVENTORY_USE_BUTTON),
            InventoryUseButton,
            button(ButtonSpec::control("Use")),
            ButtonCueType::Owned,
            VerbGate::Hidden,
        ))
        .observe(on_inventory_use_button);
    });
}

/// The Use verb's row, out of the layout while the verb is hidden.
#[derive(Component)]
pub(crate) struct InventoryUseRow;

/// The consumable item of the selected row, while it is the player's own row
/// and the item has a consumable.
fn selected_consumable<'a>(
    runtime: &'a InventoryRuntime,
    consumables: &GameConsumables,
) -> Option<&'a ItemDesignId> {
    let (side, item) = runtime.selected.as_ref()?;
    (*side == InventorySideType::Own && consumables.get(item).is_some()).then_some(item)
}

/// Show the Use verb while the selected row is the player's own row of a
/// consumable item, and enable it while a use would pass.
pub(crate) fn update_inventory_use_button(
    runtime: Res<InventoryRuntime>,
    consumables: Option<Res<GameConsumables>>,
    player: Option<Single<(Entity, &ShipInventory), With<PlayerSpaceshipMarker>>>,
    q_facts: ConsumableFactsQuery,
    mut q_button: Query<&mut VerbGate, With<InventoryUseButton>>,
    mut q_row: Query<&mut Node, With<InventoryUseRow>>,
) {
    let usable = consumables.as_deref().and_then(|consumables| {
        let item = selected_consumable(&runtime, consumables)?;
        let design = consumables.design(item);
        let (ship, hold) = *player.as_deref()?;
        let facts = q_facts.facts(ship, design, hold)?;
        Some(plan_consumable_use(design, &facts).is_ok())
    });
    let display = if usable.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut q_row {
        if node.display != display {
            node.display = display;
        }
    }
    let wanted = match usable {
        None => VerbGate::Hidden,
        Some(false) => VerbGate::Disabled,
        Some(true) => VerbGate::Ready,
    };
    for mut gate in &mut q_button {
        gate.set_if_neq(wanted.clone());
    }
}

/// Ask to use one of the selected consumable item on the player ship. The
/// button widget drops a press on a disabled button, and this guard one on a
/// verb whose gate is not ready; a refusal that slips past both in the same
/// frame keeps the item and plays the deny cue.
fn on_inventory_use_button(
    activate: On<Activate>,
    q_button: Query<&VerbGate, With<InventoryUseButton>>,
    runtime: Res<InventoryRuntime>,
    consumables: Option<Res<GameConsumables>>,
    player: Option<Single<Entity, With<PlayerSpaceshipMarker>>>,
    mut requests: MessageWriter<ConsumableUseRequest>,
) {
    if q_button
        .get(activate.entity)
        .map_or(true, |gate| *gate != VerbGate::Ready)
    {
        return;
    }
    let (Some(player), Some(consumables)) = (player, consumables.as_deref()) else {
        return;
    };
    let Some(item) = selected_consumable(&runtime, consumables) else {
        return;
    };
    requests.write(ConsumableUseRequest {
        ship: *player,
        item: item.clone(),
    });
}
