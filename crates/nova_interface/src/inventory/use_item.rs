//! The inspector's `Use` button: one click on the player's own row of an item
//! that has a `Consumable` uses one, docked or not.
//!
//! The click writes the same [`ConsumableUseRequest`] the flight key writes,
//! so the use passes or refuses by the same rule (`use_consumables`). The
//! button is disabled while that rule would refuse, read off the same facts
//! ([`ConsumableFactsQuery`]), so a press never does nothing. The row's count
//! drops by one on a use. The interface holds the game clock, so a repair or
//! boost started here runs once the interface closes.

use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use nova_gameplay::prelude::{
    plan_consumable_use, GameConsumables, ItemDesignId, PlayerSpaceshipMarker, ShipInventory,
};
use nova_ship::prelude::{ConsumableFactsQuery, ConsumableUseRequest, ShipConsumableEffects};
use nova_ui::widget::{ButtonCueType, ButtonSpec};

use super::{InventoryRuntime, InventorySideType};
use crate::pane::compact_button;

/// The name of the inspector's Use button.
pub const INVENTORY_USE_BUTTON: &str = "InventoryUse";

/// Marker for the inspector's Use button.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct InventoryUseButton;

/// Spawn the Use button into the inspector's item part, out of the layout
/// until [`update_inventory_use_button`] shows it.
pub(crate) fn spawn_inventory_use_button(item: &mut ChildSpawnerCommands) {
    item.spawn((
        Name::new(INVENTORY_USE_BUTTON),
        InventoryUseButton,
        compact_button(ButtonSpec::new("Use").fit()),
        ButtonCueType::Owned,
        InteractionDisabled,
    ))
    .observe(on_inventory_use_button);
}

/// The consumable item of the selected row, while it is the player's own row
/// and the item has a consumable.
fn selected_consumable<'a>(
    runtime: &'a InventoryRuntime,
    consumables: &GameConsumables,
) -> Option<&'a ItemDesignId> {
    let (side, item) = runtime.selected.as_ref()?;
    (*side == InventorySideType::Own && consumables.get(item).is_some()).then_some(item)
}

/// Show the Use button while the selected row is the player's own row of a
/// consumable item, and enable it while a use would pass.
pub(crate) fn update_inventory_use_button(
    runtime: Res<InventoryRuntime>,
    consumables: Option<Res<GameConsumables>>,
    player: Option<
        Single<
            (Entity, &ShipInventory, Option<&ShipConsumableEffects>),
            With<PlayerSpaceshipMarker>,
        >,
    >,
    q_facts: ConsumableFactsQuery,
    mut q_button: Query<(Entity, &mut Node, Has<InteractionDisabled>), With<InventoryUseButton>>,
    mut commands: Commands,
) {
    let usable = consumables.as_deref().and_then(|consumables| {
        let item = selected_consumable(&runtime, consumables)?;
        let design = consumables.design(item);
        let (ship, hold, effects) = *player.as_deref()?;
        let running = effects
            .is_some_and(|effects| effects.in_slot(consumables, design.effect.slot()).is_some());
        let (facts, _) = q_facts.facts(ship, design, hold, running)?;
        Some(plan_consumable_use(design, &facts).is_ok())
    });
    let display = if usable.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for (button, mut node, disabled) in &mut q_button {
        if node.display != display {
            node.display = display;
        }
        match (usable == Some(true), disabled) {
            (true, true) => {
                commands.entity(button).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(button).insert(InteractionDisabled);
            }
            _ => {}
        }
    }
}

/// Ask to use one of the selected consumable item on the player ship. The
/// button widget drops a press on a disabled button; a refusal that slips
/// past it in the same frame keeps the item, as the key's does.
fn on_inventory_use_button(
    activate: On<Activate>,
    q_button: Query<Has<InteractionDisabled>, With<InventoryUseButton>>,
    runtime: Res<InventoryRuntime>,
    consumables: Option<Res<GameConsumables>>,
    player: Option<Single<Entity, With<PlayerSpaceshipMarker>>>,
    mut requests: MessageWriter<ConsumableUseRequest>,
) {
    if q_button.get(activate.entity).unwrap_or(true) {
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
        slot: consumables.design(item).effect.slot(),
    });
}
