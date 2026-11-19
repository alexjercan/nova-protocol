//! What feeds the player's [`Chart`] while the open world streams: the cells
//! the player enters, the stations and derelict hulls the player's sensors
//! find, each stamped with the [`WorldClock`] of the find, and what the
//! player sees at a station while docked there.
//!
//! The chart, the player's [`Jobs`], the [`JobPostings`], the
//! [`StationVisits`], the player's [`CivilizationStances`] and the role
//! goal's [`GoalRecord`] are cleared with the frozen ledger, in
//! `NovaWorldSystems::Cleanup`, so a New Game, a Retry, an unload or a
//! replaced scenario starts new ones. A Load puts the saved records back
//! after that clear, on the frame the world arms.

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::{
    Chart, ChartSectorEdge, CivilizationStances, DerelictShipMarker, Encounters, GoalProgress,
    GoalRecord, JobPostings, Jobs, OwnedStationMarker, PlayerSpaceshipMarker, ShipInventory,
    StationMarker, StationServices, StationStorage, StationSummary, StationVisits, TraderMarket,
    WorldClock,
};
use nova_ship::prelude::{DockedShip, DockingConnection, SensorContacts};
use nova_world::prelude::{CurrentSector, SectorClusterTypes, SectorRoot, WorldConfig};

use crate::NovaLayeredWorld;

/// Forget the chart, the jobs, the job postings, the station visits, the
/// civilization stances, the encounters and the role goal's record and progress on every
/// frame `Cleanup` drops the frozen ledger, and keep [`ChartSectorEdge`] on
/// the armed world's edge.
#[expect(
    clippy::too_many_arguments,
    reason = "one system clearing every session record the ledger owns"
)]
pub(crate) fn clear_chart(
    mut commands: Commands,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    edge: Option<Res<ChartSectorEdge>>,
    mut chart: ResMut<Chart>,
    mut jobs: ResMut<Jobs>,
    mut postings: ResMut<JobPostings>,
    mut visits: ResMut<StationVisits>,
    mut stances: ResMut<CivilizationStances>,
    mut goal: ResMut<GoalRecord>,
    mut progress: ResMut<GoalProgress>,
    mut encounters: ResMut<Encounters>,
) {
    if *encounters != Encounters::default() {
        *encounters = Encounters::default();
    }
    if !chart.is_empty() {
        chart.clear();
    }
    if !jobs.is_empty() {
        jobs.clear();
    }
    if !postings.is_empty() {
        postings.clear();
    }
    if !visits.is_empty() {
        visits.clear();
    }
    if !stances.is_empty() {
        stances.clear();
    }
    // The whole record, not the stamp alone: an unmet goal still carries
    // its count of hulls taken.
    if *goal != GoalRecord::default() {
        goal.clear();
    }
    if !progress.is_empty() {
        *progress = GoalProgress::default();
    }
    let armed = config.map(|config| config.sector_edge);
    let shown = edge.map(|edge| edge.0);
    if armed != shown {
        match armed {
            Some(edge) => commands.insert_resource(ChartSectorEdge(edge)),
            None => commands.remove_resource::<ChartSectorEdge>(),
        }
    }
}

/// Record the cell the player stands in, with its cluster types, once
/// its root is live. A cell the player crosses before its root spawns is
/// recorded on the first frame the root stands while the player is still in
/// it.
pub(crate) fn chart_visited_cell(
    current: Res<CurrentSector>,
    roots: Query<(&SectorRoot, &SectorClusterTypes)>,
    clock: Res<WorldClock>,
    mut chart: ResMut<Chart>,
) {
    let coord = current.0;
    if chart.has_visited(coord) {
        return;
    }
    let Some((_, clusters)) = roots.iter().find(|(root, _)| root.0 == coord) else {
        return;
    };
    info!(
        "nova_world_base: the chart records {coord} with clusters {:?}",
        clusters.0
    );
    chart.visit(coord, clusters.0.clone(), clock.play_seconds);
}

/// Record every generated station and derelict hull the player's sensors
/// see this frame. A wreck fragment and a destroyed root wear no generated
/// id, so they are never found.
pub(crate) fn chart_sensor_finds(
    player: Query<&SensorContacts, With<PlayerSpaceshipMarker>>,
    bodies: Query<
        (
            &GeneratedBodyId,
            Has<StationMarker>,
            Has<DerelictShipMarker>,
            Has<OwnedStationMarker>,
            Option<&TraderMarket>,
        ),
        Or<(With<StationMarker>, With<DerelictShipMarker>)>,
    >,
    clock: Res<WorldClock>,
    mut chart: ResMut<Chart>,
) {
    let Ok(contacts) = player.single() else {
        return;
    };
    for contact in contacts.iter() {
        let Ok((id, station, derelict, owned, market)) = bodies.get(contact.entity) else {
            continue;
        };
        if station {
            if chart.station(id).is_none() {
                info!("nova_world_base: the chart finds the station '{id}'");
                let civilization = market.map(|market| market.civilization);
                chart.find_station(id, derelict, owned, civilization, clock.play_seconds);
            }
        } else if chart.wreck(id).is_none() {
            info!("nova_world_base: the chart finds the wreck '{id}'");
            chart.find_wreck(id, clock.play_seconds);
        }
    }
}

/// The station the player was docked to on the last frame.
#[derive(Default)]
pub(crate) struct LastDockedStation(Option<Entity>);

/// Write the summary of the station the player is docked to: on the dock,
/// whenever its hold or storage changes or the player buys it while docked,
/// and once more on the undock from the station as it was left.
#[expect(
    clippy::type_complexity,
    reason = "a station's identity, services, ownership, hold and storage in one query"
)]
pub(crate) fn chart_docked_station(
    player: Query<Option<&DockedShip>, With<PlayerSpaceshipMarker>>,
    connections: Query<&DockingConnection>,
    stations: Query<
        (
            &GeneratedBodyId,
            Option<&Name>,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
            Has<OwnedStationMarker>,
            Option<&TraderMarket>,
            Ref<ShipInventory>,
            Ref<StationStorage>,
        ),
        With<StationMarker>,
    >,
    mut last: Local<LastDockedStation>,
    clock: Res<WorldClock>,
    mut chart: ResMut<Chart>,
) {
    let Ok(docked) = player.single() else {
        last.0 = None;
        return;
    };
    let partner = docked
        .and_then(|docked| connections.get(docked.connection).ok())
        .map(|connection| {
            if player.contains(connection.first_ship) {
                connection.second_ship
            } else {
                connection.first_ship
            }
        })
        .filter(|partner| stations.contains(*partner));
    let (station, changed) = match (partner, last.0) {
        (Some(partner), Some(previous)) if partner == previous => (Some(partner), false),
        (Some(partner), _) => (Some(partner), true),
        (None, Some(previous)) => (Some(previous), true),
        (None, None) => (None, false),
    };
    last.0 = partner;
    let Some(Ok((id, name, services, derelict, owned, market, hold, storage))) =
        station.map(|station| stations.get(station))
    else {
        return;
    };
    // A buy lands the marker, the side and the name together, so a station
    // the chart does not yet hold as owned is recorded again.
    let bought = chart.station(id).is_some_and(|known| known.owned != owned);
    if !changed && !bought && !hold.is_changed() && !storage.is_changed() {
        return;
    }
    let summary = StationSummary {
        name: name.map_or_else(|| id.to_string(), |name| name.as_str().to_string()),
        services: services
            .filter(|_| !derelict)
            .map(|services| services.0.clone())
            .unwrap_or_default(),
        stock: hold
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect(),
        stored: storage
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect(),
    };
    if chart.station(id).is_some_and(|known| {
        known.summary.as_ref() == Some(&summary)
            && known.derelict == derelict
            && known.owned == owned
    }) {
        return;
    }
    info!(
        "nova_world_base: the chart records the station '{id}' as last seen: {} stock stack(s), \
         {} stored stack(s)",
        summary.stock.len(),
        summary.stored.len()
    );
    let civilization = market.map(|market| market.civilization);
    chart.record_station(
        id,
        derelict,
        owned,
        civilization,
        summary,
        clock.play_seconds,
    );
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;
    use nova_gameplay::prelude::{Job, JobId, JobReward, JobType};

    use super::*;

    /// The frame `Cleanup` drops a world, as a New Game or a Retry does,
    /// the next one opens with an empty goal record and no dropped job:
    /// an unmet goal's count of hulls taken goes with the world that
    /// counted it, and so does the id of a job the player dropped while
    /// holding and completing no other.
    #[test]
    fn a_new_world_opens_with_no_hull_taken_and_no_dropped_job() {
        let mut app = App::new();
        app.init_resource::<Chart>();
        app.init_resource::<Jobs>();
        app.init_resource::<JobPostings>();
        app.init_resource::<StationVisits>();
        app.init_resource::<CivilizationStances>();
        app.init_resource::<GoalRecord>();
        app.init_resource::<GoalProgress>();
        app.init_resource::<Encounters>();
        app.add_systems(Update, clear_chart);

        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let dropped = Job {
            id: JobId::slot(station.clone(), 0, 0),
            kind: JobType::Procure {
                item: "ore".into(),
                count: 4,
            },
            target: station,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(100),
        };
        let world = app.world_mut();
        world.resource_mut::<GoalRecord>().hulls_taken = 2;
        let mut jobs = world.resource_mut::<Jobs>();
        jobs.accept(dropped.clone());
        jobs.drop(&dropped.id);
        assert!(!world.resource::<GoalRecord>().is_met());

        app.update();

        assert_eq!(
            *app.world().resource::<GoalRecord>(),
            GoalRecord::default(),
            "the unmet record's count of hulls taken is gone"
        );
        assert!(
            !app.world().resource::<Jobs>().is_dropped(&dropped.id),
            "the dropped job's id is gone"
        );
    }
}
