//! What posts a [`JobBoard`] on every living generated station the open
//! world streams.
//!
//! The board is a pure function of the world seed, the station's id and the
//! cluster types of its cell, so a station posts the same board on every
//! visit, after a thaw and after a Load. It is put on the station root once,
//! on the first frame the root stands under its cell's root; nothing on it
//! changes afterwards. The player's record of the jobs taken off it is
//! [`nova_gameplay::prelude::Jobs`], which `chart::clear_chart` clears.

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::{
    station_job_board, station_offers, DerelictShipMarker, GameItems, JobBoard, StationMarker,
    StationServiceType, StationServices,
};
use nova_world::prelude::{SectorClusterTypes, SectorRoot, WorldConfig};

use crate::NovaLayeredWorld;

/// Post the board on every generated station that offers
/// [`StationServiceType::Jobs`] and has none yet. A dead station posts
/// nothing; a station whose cell root is not live yet waits a frame.
pub(crate) fn post_station_boards(
    mut commands: Commands,
    config: Res<WorldConfig<NovaLayeredWorld>>,
    items: Res<GameItems>,
    stations: Query<
        (
            Entity,
            &GeneratedBodyId,
            &ChildOf,
            Option<&StationServices>,
            Has<DerelictShipMarker>,
        ),
        (With<StationMarker>, Without<JobBoard>),
    >,
    roots: Query<&SectorClusterTypes, With<SectorRoot>>,
) {
    for (entity, id, parent, services, derelict) in &stations {
        if !station_offers(services, derelict, StationServiceType::Jobs) {
            continue;
        }
        let Ok(clusters) = roots.get(parent.parent()) else {
            continue;
        };
        let board = station_job_board(config.seed, id, &clusters.0, &items);
        info!(
            "nova_world_base: the station '{id}' posts {} job(s)",
            board.len()
        );
        commands.entity(entity).insert(JobBoard(board));
    }
}
