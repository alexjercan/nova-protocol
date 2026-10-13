//! The player's chart of a streamed world: the cells the player entered, the
//! types of the clusters with bodies in each, and the stations and wrecks
//! the player's sensors found.
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

use crate::{inventory::prelude::ItemDesignId, markers::prelude::StationServiceType};

/// Glob-import surface: `use nova_gameplay::chart::prelude::*`.
pub mod prelude {
    pub use super::{Chart, ChartSectorEdge, ChartStation, ClusterType, StationSummary};
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
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChartStation {
    /// The station's generated id.
    pub id: GeneratedBodyId,
    /// Dead when found: a derelict station offers no service.
    pub derelict: bool,
    /// What the player saw there on the last dock. `None` until the player
    /// docks: a station seen on sensors only shows no stock.
    pub summary: Option<StationSummary>,
}

/// The player's chart of the streamed world.
///
/// Visited cells are the cells the player stood in, never the streamed
/// window around them. Each one keeps the type of every cluster with a body
/// in it, in the generator's order: zero, one or more. Stations and wrecks
/// are kept in the order they were found, so a chart code minted from the
/// position stays the same across a save and a Load.
///
/// Cleared with the frozen ledger whenever the world on hand goes: an
/// unload, a Retry, a replaced scenario. A Load restores the saved chart on
/// the frame the world arms.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Chart {
    visited: BTreeMap<SectorCoord, Vec<ClusterType>>,
    stations: Vec<ChartStation>,
    wrecks: Vec<GeneratedBodyId>,
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

    /// Record that the player entered `coord`, holding `clusters`. A cell
    /// already visited keeps the types it was recorded with.
    pub fn visit(&mut self, coord: SectorCoord, clusters: Vec<ClusterType>) {
        self.visited.entry(coord).or_insert(clusters);
    }

    /// Every visited cell with its cluster types, in coordinate order.
    pub fn visited(&self) -> impl Iterator<Item = (SectorCoord, &[ClusterType])> + '_ {
        self.visited
            .iter()
            .map(|(coord, clusters)| (*coord, clusters.as_slice()))
    }

    /// Every found station, in the order it was found.
    pub fn stations(&self) -> &[ChartStation] {
        &self.stations
    }

    /// The found station `id`, if the player found it.
    pub fn station(&self, id: &GeneratedBodyId) -> Option<&ChartStation> {
        self.stations.iter().find(|station| station.id == *id)
    }

    /// Record that the player found the station `id`. A station already
    /// found keeps its entry.
    pub fn find_station(&mut self, id: &GeneratedBodyId, derelict: bool) {
        if self.station(id).is_none() {
            self.stations.push(ChartStation {
                id: id.clone(),
                derelict,
                summary: None,
            });
        }
    }

    /// Record what the player saw at the station `id` while docked, finding
    /// it first if it was not found yet.
    pub fn record_station(
        &mut self,
        id: &GeneratedBodyId,
        derelict: bool,
        summary: StationSummary,
    ) {
        self.find_station(id, derelict);
        let station = self
            .stations
            .iter_mut()
            .find(|station| station.id == *id)
            .expect("found above");
        station.derelict = derelict;
        station.summary = Some(summary);
    }

    /// Every found wreck, in the order it was found.
    pub fn wrecks(&self) -> &[GeneratedBodyId] {
        &self.wrecks
    }

    /// Whether the player found the wreck `id`.
    pub fn has_wreck(&self, id: &GeneratedBodyId) -> bool {
        self.wrecks.contains(id)
    }

    /// Record that the player found the wreck `id`.
    pub fn find_wreck(&mut self, id: &GeneratedBodyId) {
        if !self.has_wreck(id) {
            self.wrecks.push(id.clone());
        }
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
    /// listed twice, or a summary stack of zero or listed twice.
    pub fn validate(&self) -> Result<(), String> {
        for (index, station) in self.stations.iter().enumerate() {
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
            if self.wrecks[..index].contains(wreck) {
                return Err(format!("the chart lists the wreck '{wreck}' twice"));
            }
        }
        Ok(())
    }
}

/// The cell edge of the streamed world the chart is drawn on. Present only
/// while a world streams, so the Map draws no chart over a fixed scene.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ChartSectorEdge(pub Meters);
