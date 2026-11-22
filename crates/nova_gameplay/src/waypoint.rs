//! The player's waypoint: a chart target the autopilot flies to while it is
//! not streamed in.
//!
//! A chart station or a chart cell can sit cells out of the live window, so
//! no entity stands for it. The Map engages `GotoPos` at the target's mark
//! and puts a [`Waypoint`] on the player ship, which names what the mark
//! stands for. The world crate hands a station waypoint over to a GOTO of
//! the live station when the station streams in, and clears a waypoint the
//! autopilot no longer flies. The HUD chip and the Map read it here because
//! the HUD cannot depend on the world crates.

use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, SectorCoord};

use crate::chart::Chart;

/// Glob-import surface: `use nova_gameplay::waypoint::prelude::*`.
pub mod prelude {
    pub use super::{Waypoint, WaypointTargetType};
}

/// What a waypoint stands for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WaypointTargetType {
    /// A charted station, by its generated id. The mark is where the station
    /// froze; the GOTO hands over to the station once it streams in.
    Station(GeneratedBodyId),
    /// A chart cell. The mark is the cell centre, and the leg rests there.
    Sector(SectorCoord),
}

impl WaypointTargetType {
    /// The chart code the target reads by, on the Map and on the HUD chip:
    /// its station's `STN-n`, or its cell's coordinate. `None` for a station
    /// the chart does not hold.
    pub fn chart_code(&self, chart: &Chart) -> Option<String> {
        match self {
            Self::Station(id) => chart.station_code(id),
            Self::Sector(coord) => Some(coord.to_string()),
        }
    }
}

/// The chart target the player's autopilot flies to. On the player ship only
/// while its autopilot flies the target: `GotoPos` at [`Self::mark`], or
/// `Goto` of the station after the hand-off.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Waypoint {
    /// What the waypoint stands for.
    pub target: WaypointTargetType,
    /// The engine-space point the `GotoPos` leg flies to. Kept so a `GotoPos`
    /// at another point, such as a scenario move order, does not read as
    /// this waypoint.
    pub mark: Vec3,
}
