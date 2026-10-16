//! The two consumers of the play-time clock: restock on a thaw, and the
//! refusal window a civilization holds against the player.
//!
//! Nothing frozen simulates, so a trader's stock stands still while its
//! cell is off the window and catches up the moment the cell thaws:
//! [`restock_thawed_traders`] reads the [`ThawedAfter`] stamp the thaw put
//! on the body and moves every targeted item of its [`TraderMarket`] toward
//! the target by the table's [`RestockRule`]
//! (`nova_gameplay::market::restocked_count`). A trader thaws through the
//! section spawn, which puts its hold on the root once, so the restock keys
//! on the hold landing.
//!
//! A hull the player neutralizes costs the player its civilization's
//! stations: [`record_neutralized_civilization`] stamps the hull's
//! civilization into [`NeutralizedCivilizations`] with the clock when its
//! [`NeutralizedMarker`] lands while the player's ship or a fleet escort
//! was the last to damage it ([`LastAttacker`]), and
//! [`forget_served_refusals`] drops each entry once the table's
//! [`RobberyRule`] window has passed since its stamp. Admission reads the
//! record alone (`DockRefusalType::NeutralizedHull`); it cannot see the
//! clock. The record is cleared with the frozen ledger and saved with the
//! player, so a window runs on across a retire, a thaw and a Load.

use bevy::prelude::*;
use nova_gameplay::prelude::{
    restocked_count, GameItems, GamePriceTable, NeutralizedCivilizations, NeutralizedMarker,
    PlayerFleetMarker, PlayerSpaceshipMarker, ShipInventory, TraderMarket,
};
use nova_ship::prelude::LastAttacker;
use nova_world::prelude::{ThawedAfter, WorldClock};

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

/// Record the civilization of a generated hull the player just neutralized:
/// the hull carries a [`TraderMarket`] (every intact generated ship and
/// living station does) and its [`LastAttacker`] is the player's ship or a
/// fleet escort. A hull thawed already neutralized carries no attacker and
/// records nothing.
pub(crate) fn record_neutralized_civilization(
    add: On<Add, NeutralizedMarker>,
    hulls: Query<(Option<&Name>, &TraderMarket, Option<&LastAttacker>)>,
    player: Query<(), Or<(With<PlayerSpaceshipMarker>, With<PlayerFleetMarker>)>>,
    clock: Res<WorldClock>,
    mut record: ResMut<NeutralizedCivilizations>,
) {
    let Ok((name, market, attacker)) = hulls.get(add.entity) else {
        return;
    };
    let Some(LastAttacker(attacker)) = attacker else {
        return;
    };
    if !player.contains(*attacker) {
        return;
    }
    let label = name.map_or_else(|| format!("{:?}", add.entity), |name| name.to_string());
    info!(
        "refusal: the player neutralized '{label}' of {} at {:.1} s of play; its stations refuse",
        market.civilization.name(),
        clock.play_seconds
    );
    record.record(market.civilization, clock.play_seconds);
}

/// Forget every recorded civilization whose refusal window has passed.
pub(crate) fn forget_served_refusals(
    clock: Res<WorldClock>,
    table: Res<GamePriceTable>,
    mut record: ResMut<NeutralizedCivilizations>,
) {
    if record.is_empty() {
        return;
    }
    let cutoff = clock.play_seconds - f64::from(table.0.robbery.refusal_seconds);
    let before = record.entries().count();
    record.forget_before(cutoff);
    if record.entries().count() < before {
        info!(
            "refusal: a civilization's window passed at {:.1} s of play",
            clock.play_seconds
        );
    }
}
