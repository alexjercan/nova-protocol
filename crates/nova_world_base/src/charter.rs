//! Charter 0: the open-world harness's first objective, "dock at the nearest
//! station". The harness scenario posts the objective on a fresh start and
//! completes it on the dock, through its own `OnDocked` handler; this module
//! keeps the gold objective marker pointed at the station the objective
//! means, which no author can name because the world is generated.

use avian3d::prelude::Position;
use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::{
    Allegiance, DerelictShipMarker, GameObjectives, ObjectiveMarkerTarget, PlayerSpaceshipMarker,
    StationMarker,
};
use nova_scenario::prelude::GeneratedMarks;

/// The objective id the open-world harness posts for charter 0.
pub const CHARTER_ZERO_OBJECTIVE_ID: &str = "charter_0";

/// The marker chip label the pointer puts on the nearest station.
const CHARTER_ZERO_MARK: &str = "STATION";

/// While [`CHARTER_ZERO_OBJECTIVE_ID`] is listed, keep the objective marker
/// on the living, non-Enemy streamed station nearest the player and remember
/// it in [`GeneratedMarks`], so a thaw puts the chip back; take both off
/// every other station and once the charter is gone. No such station in the
/// window: no mark, and the objective stands until one streams in. No one
/// player ship to measure from: the marks stand as they are.
#[expect(
    clippy::type_complexity,
    reason = "a station's identity, position, liveness, side and mark in one query"
)]
pub(crate) fn point_charter_zero(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
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
            .filter(|(_, _, _, derelict, allegiance, _)| {
                !derelict && *allegiance != Some(&Allegiance::Enemy)
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
