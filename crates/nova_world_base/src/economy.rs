//! Restock on a thaw: the economy's consumer of the play-time clock.
//!
//! Nothing frozen simulates, so a trader's stock stands still while its
//! cell is off the window and catches up the moment the cell thaws:
//! [`restock_thawed_traders`] reads the [`ThawedAfter`] stamp the thaw put
//! on the body and moves every targeted item of its [`TraderMarket`] toward
//! the target by the table's [`RestockRule`]
//! (`nova_gameplay::market::restocked_count`). A trader thaws through the
//! section spawn, which puts its hold on the root once, so the restock keys
//! on the hold landing. A hull the player neutralizes is the stance graph's
//! business (`stance`).

use bevy::prelude::*;
use nova_gameplay::prelude::{
    restocked_count, GameItems, GamePriceTable, ShipInventory, TraderMarket,
};
use nova_world::prelude::ThawedAfter;

/// Move every targeted item of a trader whose hold just landed after a thaw
/// toward its target by the play time it stood frozen. An item adds only
/// what the hold has room for: a hold the player filled with sold goods
/// restocks what fits.
pub(crate) fn restock_thawed_traders(
    items: Res<GameItems>,
    table: Res<GamePriceTable>,
    mut traders: Query<
        (
            Entity,
            Option<&Name>,
            &ThawedAfter,
            &TraderMarket,
            &mut ShipInventory,
        ),
        Added<ShipInventory>,
    >,
) {
    for (entity, name, thawed, market, mut hold) in &mut traders {
        let elapsed = thawed.play_seconds;
        let label = name.map_or_else(|| format!("{entity:?}"), |name| name.to_string());
        for (item, target) in &market.target {
            let held = hold.count(item);
            let wanted = restocked_count(held, *target, elapsed, table.0.restock);
            if wanted > held {
                let fits = items.design(item).mass_g.max(1);
                let room = hold.free_g(&items) / fits;
                let added = (wanted - held).min(room);
                if added > 0 {
                    hold.add(&items, item, added);
                }
                info!(
                    "restock: '{label}' stood frozen {elapsed:.1} s, {item} {held} -> {} of {target}",
                    held + added
                );
            } else if wanted < held {
                hold.remove(item, held - wanted);
                info!(
                    "restock: '{label}' stood frozen {elapsed:.1} s, {item} {held} -> {wanted} of {target}"
                );
            }
        }
    }
}
