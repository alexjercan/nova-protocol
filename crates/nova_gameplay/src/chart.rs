//! The player's chart of a streamed world: the cells the player entered, the
//! types of the clusters with bodies in each, and the stations and wrecks
//! the player's sensors found, each stamped with the play time of the find.
//!
//! The stamps make the chart the discovery log too: [`Chart::log`] orders
//! every find by its stamp, so the Map shows when and where without a second
//! record that could drift from the chart.
//!
//! [`Chart`] is a session resource. The open world crate feeds it while a
//! world streams, a world save carries it, and a Load puts it back. The Map
//! pane reads it for its chart view; it lives here because the interface
//! cannot depend on the world crates. It holds no entity: every station and
//! wreck is named by its [`GeneratedBodyId`], so the chart outlives the cell
//! that held the body.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, Meters, SectorCoord};

use crate::{
    civilization::CivilizationId, inventory::prelude::ItemDesignId,
    markers::prelude::StationServiceType,
};

/// Glob-import surface: `use nova_gameplay::chart::prelude::*`.
pub mod prelude {
    pub use super::{
        Chart, ChartCell, ChartLogEntry, ChartLogEntryType, ChartSectorEdge, ChartStation,
        ChartWreck, ClusterType, StationSummary,
    };
}

/// What a cluster of the open world is.
///
/// Here rather than beside the generator so the Map can name the type of a
/// visited cell without the world crates. What each type places is the
/// generator's policy, in `nova_world_base`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClusterType {
    /// Many rocks, and in quiet material one or two planetoids, and with
    /// traffic one derelict hull.
    AsteroidRich,
    /// Rocks and nothing else.
    RockOnly,
    /// Two to four planetoids with a few rocks around them.
    PlanetHeavy,
    /// Derelict hulls in a thin rock field.
    DerelictField,
}

impl ClusterType {
    /// Every type, in the order a readout and a legend list them.
    pub const ALL: [Self; 4] = [
        Self::AsteroidRich,
        Self::RockOnly,
        Self::PlanetHeavy,
        Self::DerelictField,
    ];

    /// What a readout calls the type.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AsteroidRich => "asteroid-rich",
            Self::RockOnly => "rock-only",
            Self::PlanetHeavy => "planet-heavy",
            Self::DerelictField => "derelict-field",
        }
    }
}

/// What the player last saw at a station while docked there: written on the
/// dock, refreshed while docked and on the undock.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationSummary {
    /// The station's descriptive name, as its contact panel shows it.
    pub name: String,
    /// The services its design offers. A dead station offers none of them.
    pub services: Vec<StationServiceType>,
    /// Its trade stock: the stacks in its hull's hold, in id order.
    pub stock: Vec<(ItemDesignId, u32)>,
    /// What the player stored there, in id order.
    pub stored: Vec<(ItemDesignId, u32)>,
}

/// One station the player's sensors found.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChartStation {
    /// The station's generated id.
    pub id: GeneratedBodyId,
    /// Dead when found: a derelict station offers no service.
    pub derelict: bool,
    /// The player's when last seen: bought from its civilization
    /// (`OwnedStationMarker`).
    pub owned: bool,
    /// The civilization that keeps it, read off its market when found.
    /// `None` on a station dead when found: it keeps no book and takes no
    /// fine.
    pub civilization: Option<CivilizationId>,
    /// What the player saw there on the last dock. `None` until the player
    /// docks: a station seen on sensors only shows no stock.
    pub summary: Option<StationSummary>,
    /// Play seconds on the world clock when the sensors found it.
    pub found_at: f64,
}

/// One derelict hull the player's sensors found.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChartWreck {
    /// The hull's generated id.
    pub id: GeneratedBodyId,
    /// Play seconds on the world clock when the sensors found it.
    pub found_at: f64,
}

/// One cell the player entered.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChartCell {
    /// The type of every cluster with a body in the cell, in the generator's
    /// order: zero, one or more.
    pub clusters: Vec<ClusterType>,
    /// Play seconds on the world clock when the player entered it.
    pub entered_at: f64,
}

/// What one discovery log entry points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChartLogEntryType {
    /// The player entered the cell.
    Cell(SectorCoord),
    /// The sensors found the station.
    Station(GeneratedBodyId),
    /// The sensors found the derelict hull.
    Wreck(GeneratedBodyId),
}

/// One line of the discovery log, derived from the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartLogEntry {
    /// Play seconds on the world clock when it happened.
    pub at: f64,
    /// What happened.
    pub entry: ChartLogEntryType,
}

/// The player's chart of the streamed world.
///
/// Visited cells are the cells the player stood in, never the streamed
/// window around them. Stations and wrecks are kept in the order they were
/// found, so a chart code minted from the position stays the same across a
/// save and a Load. Every entry carries the play time of its find.
///
/// Cleared with the frozen ledger whenever the world on hand goes: an
/// unload, a Retry, a replaced scenario. A Load restores the saved chart on
/// the frame the world arms.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Chart {
    visited: BTreeMap<SectorCoord, ChartCell>,
    stations: Vec<ChartStation>,
    wrecks: Vec<ChartWreck>,
}

impl Chart {
    /// Whether the chart holds nothing.
    pub fn is_empty(&self) -> bool {
        self.visited.is_empty() && self.stations.is_empty() && self.wrecks.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.visited.clear();
        self.stations.clear();
        self.wrecks.clear();
    }

    /// Whether the player entered `coord`.
    pub fn has_visited(&self, coord: SectorCoord) -> bool {
        self.visited.contains_key(&coord)
    }

    /// Record that the player entered `coord`, holding `clusters`, at `at`
    /// play seconds. A cell already visited keeps the types and the time it
    /// was recorded with.
    pub fn visit(&mut self, coord: SectorCoord, clusters: Vec<ClusterType>, at: f64) {
        self.visited.entry(coord).or_insert(ChartCell {
            clusters,
            entered_at: at,
        });
    }

    /// Every visited cell, in coordinate order.
    pub fn visited(&self) -> impl Iterator<Item = (SectorCoord, &ChartCell)> + '_ {
        self.visited.iter().map(|(coord, cell)| (*coord, cell))
    }

    /// The visited cell `coord`, if the player entered it.
    pub fn cell(&self, coord: SectorCoord) -> Option<&ChartCell> {
        self.visited.get(&coord)
    }

    /// Every found station, in the order it was found.
    pub fn stations(&self) -> &[ChartStation] {
        &self.stations
    }

    /// The found station `id`, if the player found it.
    pub fn station(&self, id: &GeneratedBodyId) -> Option<&ChartStation> {
        self.stations.iter().find(|station| station.id == *id)
    }

    /// The code the Map, the HUD and the save call the found station `id`
    /// by: `STN-n`, its 1-based place in the found order. None while the
    /// player has not found it.
    pub fn station_code(&self, id: &GeneratedBodyId) -> Option<String> {
        self.stations
            .iter()
            .position(|station| station.id == *id)
            .map(|index| format!("STN-{}", index + 1))
    }

    /// Record that the player found the station `id` of `civilization` at
    /// `at` play seconds, dead or alive, owned or not. A station already
    /// found keeps its entry.
    pub fn find_station(
        &mut self,
        id: &GeneratedBodyId,
        derelict: bool,
        owned: bool,
        civilization: Option<CivilizationId>,
        at: f64,
    ) {
        if self.station(id).is_none() {
            self.stations.push(ChartStation {
                id: id.clone(),
                derelict,
                owned,
                civilization,
                summary: None,
                found_at: at,
            });
        }
    }

    /// Record what the player saw at the station `id` while docked, finding
    /// it first at `at` play seconds if it was not found yet.
    pub fn record_station(
        &mut self,
        id: &GeneratedBodyId,
        derelict: bool,
        owned: bool,
        civilization: Option<CivilizationId>,
        summary: StationSummary,
        at: f64,
    ) {
        self.find_station(id, derelict, owned, civilization, at);
        let station = self
            .stations
            .iter_mut()
            .find(|station| station.id == *id)
            .expect("found above");
        station.derelict = derelict;
        station.owned = owned;
        station.summary = Some(summary);
    }

    /// Every found wreck, in the order it was found.
    pub fn wrecks(&self) -> &[ChartWreck] {
        &self.wrecks
    }

    /// The found wreck `id`, if the player found it.
    pub fn wreck(&self, id: &GeneratedBodyId) -> Option<&ChartWreck> {
        self.wrecks.iter().find(|wreck| wreck.id == *id)
    }

    /// Record that the player found the wreck `id` at `at` play seconds. A
    /// wreck already found keeps its entry.
    pub fn find_wreck(&mut self, id: &GeneratedBodyId, at: f64) {
        if self.wreck(id).is_none() {
            self.wrecks.push(ChartWreck {
                id: id.clone(),
                found_at: at,
            });
        }
    }

    /// The discovery log: every visited cell, found station and found wreck
    /// by play time, oldest first. Finds on the same stamp list the cell
    /// first, then the station, then the wreck, each in found order, so the
    /// log reads the same across a save and a Load.
    pub fn log(&self) -> Vec<ChartLogEntry> {
        let mut log: Vec<ChartLogEntry> = self
            .visited
            .iter()
            .map(|(coord, cell)| ChartLogEntry {
                at: cell.entered_at,
                entry: ChartLogEntryType::Cell(*coord),
            })
            .chain(self.stations.iter().map(|station| ChartLogEntry {
                at: station.found_at,
                entry: ChartLogEntryType::Station(station.id.clone()),
            }))
            .chain(self.wrecks.iter().map(|wreck| ChartLogEntry {
                at: wreck.found_at,
                entry: ChartLogEntryType::Wreck(wreck.id.clone()),
            }))
            .collect();
        // A stable sort keeps the chain order inside one stamp: cells in
        // coordinate order, then stations and wrecks in found order.
        log.sort_by(|a, b| a.at.total_cmp(&b.at));
        log
    }

    /// Every item id a station summary names: its stock and the stored
    /// stacks.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> + '_ {
        self.stations
            .iter()
            .filter_map(|station| station.summary.as_ref())
            .flat_map(|summary| summary.stock.iter().chain(&summary.stored))
            .map(|(item, _)| item)
    }

    /// Refuse a chart a Load could not have built: a station or a wreck
    /// listed twice, a summary stack of zero or listed twice, or a find
    /// stamped off the play-time clock.
    pub fn validate(&self) -> Result<(), String> {
        for (coord, cell) in &self.visited {
            validate_stamp(cell.entered_at, || format!("the cell {coord}"))?;
        }
        for (index, station) in self.stations.iter().enumerate() {
            validate_stamp(station.found_at, || format!("the station '{}'", station.id))?;
            if self.stations[..index]
                .iter()
                .any(|other| other.id == station.id)
            {
                return Err(format!(
                    "the chart lists the station '{}' twice",
                    station.id
                ));
            }
            let Some(summary) = &station.summary else {
                continue;
            };
            for (what, stacks) in [("stock", &summary.stock), ("storage", &summary.stored)] {
                for (at, (item, count)) in stacks.iter().enumerate() {
                    if *count == 0 {
                        return Err(format!(
                            "the chart's {what} of '{}' holds 0 of '{item}'",
                            station.id
                        ));
                    }
                    if stacks[..at].iter().any(|(other, _)| other == item) {
                        return Err(format!(
                            "the chart's {what} of '{}' lists '{item}' twice",
                            station.id
                        ));
                    }
                }
            }
        }
        for (index, wreck) in self.wrecks.iter().enumerate() {
            validate_stamp(wreck.found_at, || format!("the wreck '{}'", wreck.id))?;
            if self.wrecks[..index]
                .iter()
                .any(|other| other.id == wreck.id)
            {
                return Err(format!("the chart lists the wreck '{}' twice", wreck.id));
            }
        }
        Ok(())
    }
}

/// A play-time stamp the clock could have written: finite and not negative.
fn validate_stamp(at: f64, what: impl FnOnce() -> String) -> Result<(), String> {
    if at.is_finite() && at >= 0.0 {
        Ok(())
    } else {
        Err(format!(
            "the chart stamps {} at {at} s of play, which no clock reads",
            what()
        ))
    }
}

/// The cell edge of the streamed world the chart is drawn on. Present only
/// while a world streams, so the Map draws no chart over a fixed scene.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ChartSectorEdge(pub Meters);

#[cfg(test)]
mod tests {
    use super::*;

    fn body(stem: &str) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::ORIGIN, stem, 0)
    }

    /// The log lists every find by its play-time stamp, oldest first, and
    /// on one stamp the cell before the station before the wreck, so a Load
    /// reads the same log the session wrote.
    #[test]
    fn the_log_orders_every_find_by_play_time() {
        let mut chart = Chart::default();
        chart.visit(SectorCoord::new(0, 0, 0), vec![], 0.0);
        chart.find_wreck(&body("wreck_a"), 12.5);
        chart.find_station(&body("station_a"), false, false, None, 12.5);
        chart.visit(SectorCoord::new(1, 0, 0), vec![ClusterType::RockOnly], 12.5);
        chart.find_station(&body("station_b"), true, false, None, 4.0);
        // A repeat find keeps the first stamp.
        chart.find_wreck(&body("wreck_a"), 40.0);
        chart.visit(
            SectorCoord::new(0, 0, 0),
            vec![ClusterType::PlanetHeavy],
            40.0,
        );

        let log: Vec<(f64, ChartLogEntryType)> = chart
            .log()
            .into_iter()
            .map(|entry| (entry.at, entry.entry))
            .collect();
        assert_eq!(
            log,
            vec![
                (0.0, ChartLogEntryType::Cell(SectorCoord::new(0, 0, 0))),
                (4.0, ChartLogEntryType::Station(body("station_b"))),
                (12.5, ChartLogEntryType::Cell(SectorCoord::new(1, 0, 0))),
                (12.5, ChartLogEntryType::Station(body("station_a"))),
                (12.5, ChartLogEntryType::Wreck(body("wreck_a"))),
            ]
        );
        assert!(chart.validate().is_ok());
    }

    /// A stamp no clock reads is refused on Load.
    #[test]
    fn a_find_stamped_off_the_clock_is_refused() {
        let mut chart = Chart::default();
        chart.find_wreck(&body("wreck_a"), -1.0);
        let error = chart.validate().expect_err("a negative stamp is refused");
        assert!(error.contains("wreck_a"), "{error}");

        let mut chart = Chart::default();
        chart.visit(SectorCoord::new(0, 0, 0), vec![], f64::NAN);
        assert!(chart.validate().is_err(), "a NaN stamp is refused");
    }
}
