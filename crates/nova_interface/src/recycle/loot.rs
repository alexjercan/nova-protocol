//! The Recycle pane's loot list: the wreck's hold and credits, each with a
//! take button.
//!
//! A take is the Inventory pane's own: a row's Take all writes the
//! [`InventoryActionCommand`] a Take of the whole stack writes there, and
//! Take credits writes its [`CreditTakeCommand`], so the same rule refuses or
//! moves, and the result shows on the same note line with the same cue.
//!
//! Touch this module when changing what the loot list shows.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::strip::StripSite;
use crate::{
    inventory::{CreditTakeCommand, InventoryActionCommand, InventoryActionType},
    pane::{compact_button, themed_label},
};

/// The name prefix of a hold row's Take all button, followed by the item
/// id: `RecycleTake-HullPlate`.
pub const RECYCLE_TAKE_PREFIX: &str = "RecycleTake-";

/// The name of the Take credits button.
pub const RECYCLE_TAKE_CREDITS: &str = "RecycleTakeCredits";

/// The loot rows' container, and what they were built from, so the rows
/// respawn only when the wreck's hold or credits change.
#[derive(Component, Default)]
pub(crate) struct RecycleLootRows(Option<LootView>);

/// The wreck's hold and credits as the list shows them.
#[derive(Clone, Debug, PartialEq)]
struct LootView {
    stacks: Vec<(ItemDesignId, String, u32)>,
    credits: u32,
}

/// Build the loot block: a header and the rows [`update_recycle_loot`]
/// fills.
pub(crate) fn spawn_recycle_loot(panel: &mut ChildSpawnerCommands) {
    panel.spawn(crate::pane::divider());
    panel.spawn(themed_label("The wreck's hold", 13.0, UiColor::Accent));
    panel.spawn((
        RecycleLootRows::default(),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
    ));
}

/// Respawn the loot rows when the docked wreck's hold or credits change.
pub(crate) fn update_recycle_loot(
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_hold: Query<(&ShipInventory, &ShipCredits)>,
    mut q_rows: Query<(Entity, &mut RecycleLootRows)>,
) {
    let Ok((rows, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let Some(items) = items else {
        return;
    };
    let view = site
        .partner()
        .filter(|partner| partner.recyclable())
        .and_then(|partner| q_hold.get(partner.entity).ok())
        .map(|(hold, credits)| LootView {
            stacks: hold
                .stacks()
                .map(|(item, count)| (item.clone(), items.design(item).name.clone(), count))
                .collect(),
            credits: credits.0,
        });
    if shown.0 == view {
        return;
    }
    commands.entity(rows).despawn_children();
    commands.entity(rows).with_children(|list| {
        let Some(view) = &view else {
            return;
        };
        if view.stacks.is_empty() && view.credits == 0 {
            list.spawn(themed_label("The hold is empty.", 12.0, UiColor::Label));
        }
        for (item, name, count) in &view.stacks {
            list.spawn(loot_row()).with_children(|row| {
                row.spawn(themed_label(
                    &format!("{count} {name}"),
                    12.0,
                    UiColor::Body,
                ));
                row.spawn((
                    Name::new(format!("{RECYCLE_TAKE_PREFIX}{item}")),
                    compact_button(ButtonSpec::new("Take all").fit()),
                ))
                .observe(on_take_button(item.clone(), *count));
            });
        }
        if view.credits > 0 {
            list.spawn(loot_row()).with_children(|row| {
                row.spawn(themed_label(
                    &format!("{} cr", view.credits),
                    12.0,
                    UiColor::Body,
                ));
                row.spawn((
                    Name::new(RECYCLE_TAKE_CREDITS),
                    compact_button(ButtonSpec::new("Take credits").fit()),
                ))
                .observe(on_take_credits_button);
            });
        }
    });
    shown.0 = view;
}

/// One loot row: the stack and its button side by side.
fn loot_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(12.0),
        ..default()
    }
}

/// Write a Take of `count` of `item`, the whole stack the row shows.
fn on_take_button(
    item: ItemDesignId,
    count: u32,
) -> impl FnMut(On<Activate>, MessageWriter<InventoryActionCommand>) {
    move |_activate, mut actions| {
        actions.write(InventoryActionCommand {
            action: InventoryActionType::Take,
            item: item.clone(),
            quantity: Some(count),
        });
    }
}

/// Write a Take credits.
fn on_take_credits_button(_activate: On<Activate>, mut actions: MessageWriter<CreditTakeCommand>) {
    actions.write(CreditTakeCommand);
}
