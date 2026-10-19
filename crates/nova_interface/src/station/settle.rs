//! The Settle action: the fine that returns an Enemy civilization to
//! Neutral, paid at a docked station.
//!
//! The Map chart's stance block raises a [`StanceSettleCommand`] for an
//! Enemy neighbour of the selected station's civilization while the ship is
//! docked there. [`apply_station_settle_commands`] charges the fine
//! ([`fine_cr`](nova_world_base::prelude::fine_cr)) with a `Settle` credit
//! change on the player ship, pays it to the docked station, and settles the
//! civilization to Neutral ([`settle`]). A frozen station has no live
//! balance to pay, so a fine is never paid undocked. A station the player
//! owns keeps no civilization's book, and the docked station's own
//! civilization cannot be Enemy, or it would not have admitted the ship.
//!
//! Touch this module when changing what Settle moves.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::prelude::CreditChangeCauseType;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_world_base::prelude::{enemy_neighbours, settle};

use super::StationRuntime;
use crate::inventory::cr_text;

/// A request to settle with `civilization` at the docked station, raised by
/// the Map chart's Pay fine button.
#[derive(Message, Clone, Debug)]
pub struct StanceSettleCommand {
    /// The civilization to settle with: a living neighbour of the docked
    /// station's that stands Enemy to the player.
    pub civilization: CivilizationId,
}

/// The player ship and the station it is docked with, with the
/// civilization the station keeps the book for.
#[derive(SystemParam)]
pub(crate) struct SettleDock<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, &'static DockedShip),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    stations: Query<
        'w,
        's,
        (Option<&'static Name>, &'static TraderMarket),
        (
            With<StationMarker>,
            Without<DerelictShipMarker>,
            Without<OwnedStationMarker>,
        ),
    >,
}

impl SettleDock<'_, '_> {
    /// The player ship, the docked station, its name and its civilization,
    /// or `None` while the ship is not docked at a station a civilization
    /// keeps.
    pub(crate) fn docked(&self) -> Option<(Entity, Entity, String, CivilizationId)> {
        let (player, docked) = self.player.single().ok()?;
        let connection = self.connections.get(docked.connection).ok()?;
        let station = if connection.first_ship == player {
            connection.second_ship
        } else {
            connection.first_ship
        };
        let (name, market) = self.stations.get(station).ok()?;
        let name = name.map_or_else(|| "This station".to_string(), ToString::to_string);
        Some((player, station, name, market.civilization))
    }
}

/// Why the fine of `name` is refused: the ship holds less than the fine.
pub(crate) fn settle_refusal_text(name: &str, need: u32, have: u32) -> String {
    format!(
        "settle: {name} asks {}, the ship holds {}",
        cr_text(u64::from(need)),
        cr_text(u64::from(have))
    )
}

/// Apply [`StanceSettleCommand`]s: a civilization that is an Enemy
/// neighbour of the docked station's and whose fine the ship holds is
/// settled to Neutral, the fine moving with `Settle` credit changes on both
/// sides. A refusal changes nothing and shows its reason on the note line.
pub(crate) fn apply_station_settle_commands(
    mut messages: MessageReader<StanceSettleCommand>,
    mut runtime: ResMut<StationRuntime>,
    dock: SettleDock,
    mut stances: ResMut<CivilizationStances>,
    clock: Res<WorldClock>,
    mut q_credits: Query<(&mut ShipCredits, &mut ShipInventoryReports)>,
) {
    for command in messages.read() {
        let Some((player, station, station_name, home)) = dock.docked() else {
            runtime.say("settle: dock at a station a civilization keeps".to_string());
            continue;
        };
        let name = command.civilization.name();
        let Some((_, fine)) = enemy_neighbours(home, &stances)
            .into_iter()
            .find(|(civilization, _)| *civilization == command.civilization)
        else {
            runtime.say(format!("settle: {station_name} keeps no book for {name}"));
            continue;
        };
        let Ok([(mut own_cr, mut own_reports), (mut station_cr, mut station_reports)]) =
            q_credits.get_many_mut([player, station])
        else {
            panic!("settle: the player ship and station {station:?} each carry credits");
        };
        if own_cr.0 < fine {
            runtime.say(settle_refusal_text(&name, fine, own_cr.0));
            continue;
        }
        let Some(paid) = station_cr.0.checked_add(fine) else {
            runtime.say(format!(
                "settle: {station_name}'s balance after the fine does not fit"
            ));
            continue;
        };
        let left = own_cr.0 - fine;
        own_reports.change_credits(&mut own_cr, left, CreditChangeCauseType::Settle);
        station_reports.change_credits(&mut station_cr, paid, CreditChangeCauseType::Settle);
        let step = settle(&mut stances, command.civilization, clock.play_seconds);
        info!(
            "stance: settled with {name} for {fine} cr at {:.1} s of play: {step:?}",
            clock.play_seconds
        );
        runtime.say(format!(
            "settled with {name} for {}",
            cr_text(u64::from(fine))
        ));
    }
}
