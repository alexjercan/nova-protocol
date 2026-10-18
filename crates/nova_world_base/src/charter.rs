//! Charter 0: the open world's first two objectives. The player starts
//! docked at the session's starting station, so the harness scenario posts
//! "undock from your starting station" on a fresh start and, on the
//! player's undock, completes it and posts "dock at another living station"
//! ([`CHARTER_ZERO_OBJECTIVE_ID`]). That second step no author can complete:
//! the station it must not count is the seed's, so
//! [`complete_charter_zero`] completes it here on the player's dock at a
//! living station other than the starting one, and [`point_charter_zero`]
//! keeps the gold objective marker pointed at the station the objective
//! means.

use avian3d::prelude::Position;
use bevy::prelude::*;
use nova_events::prelude::{
    EntityId, EventKind, GameEvent, GeneratedBodyId, OnDockedEvent, LIVING_STATION_TYPE_NAME,
};
use nova_gameplay::prelude::{
    Allegiance, DerelictShipMarker, GameObjectives, ObjectiveMarkerTarget, PlayerSpaceshipMarker,
    StationMarker,
};
use nova_scenario::prelude::{GeneratedMarks, NovaEventWorld, OpenWorldSession};

/// The objective id the open-world harness posts for charter 0's first
/// step, undocking from the starting station. The harness completes it
/// itself on the player's undock.
pub const CHARTER_ZERO_UNDOCK_OBJECTIVE_ID: &str = "charter_0_undock";

/// The objective id the open-world harness posts for charter 0's second
/// step, docking at a living station other than the starting one.
pub const CHARTER_ZERO_OBJECTIVE_ID: &str = "charter_0";

/// The objective line charter 0's first step shows: the base content posts
/// it, and the New Game role step lists it as the first milestone.
pub const CHARTER_ZERO_UNDOCK_MESSAGE: &str = "Charter 0: undock from your starting station";

/// The objective line charter 0's second step shows.
pub const CHARTER_ZERO_MESSAGE: &str = "Charter 0: dock at another living station";

/// The marker chip label the pointer puts on the nearest station.
const CHARTER_ZERO_MARK: &str = "STATION";

/// Complete [`CHARTER_ZERO_OBJECTIVE_ID`] on the `ondocked` of the player
/// ship with a living station other than the session's starting station.
/// A dock with a dead station, a wreck or the starting station, and a dock
/// by another ship, leave it standing.
pub(crate) fn complete_charter_zero(
    event: On<GameEvent>,
    session: Option<Res<OpenWorldSession>>,
    mut scenario: ResMut<NovaEventWorld>,
    player: Query<&EntityId, With<PlayerSpaceshipMarker>>,
) {
    if event.name() != OnDockedEvent::name() {
        return;
    }
    let (Some(session), Some(data), Ok(player)) =
        (session, event.info().data.as_ref(), player.single())
    else {
        return;
    };
    let field = |name: &str| data.get(name).and_then(|value| value.as_str());
    if field("type_name") != Some(LIVING_STATION_TYPE_NAME)
        || field("other_id") != Some(player.0.as_str())
    {
        return;
    }
    let Some(station) = field("id") else {
        return;
    };
    if station == session.start.id.to_string()
        || !scenario
            .objectives()
            .iter()
            .any(|objective| objective.id == CHARTER_ZERO_OBJECTIVE_ID)
    {
        return;
    }
    info!("nova_world_base: charter 0 is done: the player docked at '{station}'");
    scenario.remove_objective(CHARTER_ZERO_OBJECTIVE_ID);
}

/// While [`CHARTER_ZERO_OBJECTIVE_ID`] is listed, keep the objective marker
/// on the living, non-Enemy streamed station nearest the player that is not
/// the session's starting station, and remember it in [`GeneratedMarks`], so
/// a thaw puts the chip back; take both off every other station and once the
/// charter is gone. No such station in the window: no mark, and the
/// objective stands until one streams in. No one player ship to measure
/// from: the marks stand as they are.
#[expect(
    clippy::type_complexity,
    reason = "a station's identity, position, liveness, side and mark in one query"
)]
pub(crate) fn point_charter_zero(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    session: Option<Res<OpenWorldSession>>,
    player: Query<&Position, With<PlayerSpaceshipMarker>>,
    stations: Query<
        (
            Entity,
            &GeneratedBodyId,
            &Position,
            Has<DerelictShipMarker>,
            Option<&Allegiance>,
            Option<&ObjectiveMarkerTarget>,
        ),
        With<StationMarker>,
    >,
    mut marks: ResMut<GeneratedMarks>,
) {
    let listed = objectives
        .objectives
        .iter()
        .any(|objective| objective.id == CHARTER_ZERO_OBJECTIVE_ID);
    let nearest = if listed {
        let Ok(player) = player.single() else {
            return;
        };
        stations
            .iter()
            .filter(|(_, id, _, derelict, allegiance, _)| {
                !derelict
                    && *allegiance != Some(&Allegiance::Enemy)
                    && !session
                        .as_deref()
                        .is_some_and(|session| session.is_starting_station(id))
            })
            .min_by(|a, b| {
                a.2 .0
                    .distance_squared(player.0)
                    .total_cmp(&b.2 .0.distance_squared(player.0))
            })
            .map(|(entity, id, ..)| (entity, id))
    } else {
        None
    };
    for (entity, id, _, _, _, mark) in &stations {
        let charter_mark = mark.is_some_and(|mark| mark.label == CHARTER_ZERO_MARK);
        if nearest.is_some_and(|(target, _)| target == entity) {
            if !charter_mark {
                info!("nova_world_base: charter 0 points at the station '{id}'");
                commands
                    .entity(entity)
                    .insert(ObjectiveMarkerTarget::new(CHARTER_ZERO_MARK));
                marks.insert(id.clone(), CHARTER_ZERO_MARK.to_string());
            }
        } else if charter_mark {
            commands.entity(entity).remove::<ObjectiveMarkerTarget>();
            marks.remove(id);
        }
    }
    // A marked station that left the window keeps its remembered mark while
    // the charter stands, so a thaw brings the chip back; once the charter
    // is done, no station carries the charter mark.
    if !listed {
        marks.retain(|_, label| label != CHARTER_ZERO_MARK);
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{
        DockingEventInfo, GameEventInfo, Meters3, SectorCoord, DERELICT_STATION_TYPE_NAME,
        SPACESHIP_TYPE_NAME,
    };
    use nova_scenario::prelude::{ObjectiveActionConfig, StartingStation};

    use super::*;

    const PLAYER_ID: &str = "player";

    fn start_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_1_0_0_station", 0)
    }

    fn other_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 1, 0), "cluster_0_1_0_station", 0)
    }

    fn charter_app() -> App {
        let mut app = App::new();
        app.init_resource::<NovaEventWorld>();
        app.insert_resource(OpenWorldSession {
            seed: 7,
            role: "under_test".into(),
            start: StartingStation {
                id: start_id(),
                position: Meters3::new(40_000.0, 0.0, 0.0),
                spawn: Meters3::new(41_000.0, 0.0, 0.0),
            },
        });
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .push_objective(ObjectiveActionConfig {
                id: CHARTER_ZERO_OBJECTIVE_ID.to_string(),
                message: "Charter 0".to_string(),
            });
        app.world_mut()
            .spawn((EntityId::new(PLAYER_ID), PlayerSpaceshipMarker));
        app.add_observer(complete_charter_zero);
        app
    }

    fn dock(app: &mut App, station: &GeneratedBodyId, station_type_name: &str, ship: &str) {
        let info = DockingEventInfo {
            id: station.to_string(),
            type_name: station_type_name.to_string(),
            other_id: ship.to_string(),
            other_type_name: SPACESHIP_TYPE_NAME.to_string(),
        };
        app.world_mut().trigger(GameEvent::new(
            OnDockedEvent::name(),
            GameEventInfo::from_data(info),
        ));
        app.world_mut().flush();
    }

    fn charter_stands(app: &App) -> bool {
        app.world()
            .resource::<NovaEventWorld>()
            .objectives()
            .iter()
            .any(|objective| objective.id == CHARTER_ZERO_OBJECTIVE_ID)
    }

    #[test]
    fn only_the_players_dock_at_another_living_station_completes_charter_zero() {
        let mut app = charter_app();
        assert!(charter_stands(&app), "a fresh start lists charter 0");

        dock(&mut app, &start_id(), LIVING_STATION_TYPE_NAME, PLAYER_ID);
        assert!(charter_stands(&app), "the starting station does not count");

        dock(&mut app, &other_id(), DERELICT_STATION_TYPE_NAME, PLAYER_ID);
        assert!(charter_stands(&app), "a dead station does not complete it");

        dock(&mut app, &other_id(), LIVING_STATION_TYPE_NAME, "hauler");
        assert!(
            charter_stands(&app),
            "another ship's dock does not complete it"
        );

        dock(&mut app, &other_id(), LIVING_STATION_TYPE_NAME, PLAYER_ID);
        assert!(
            !charter_stands(&app),
            "the player's dock at another living station completes it"
        );
    }
}
