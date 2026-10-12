//! The STATION content kind and the station scenario object.
//!
//! A station is a hull that never flies: a [`ShipDesign`] with a docking port
//! and a flight computer, no thruster, parked where the scenario placed it.
//! It is authored once as a [`StationDesign`] and spawned by id through
//! [`StationConfig`]. The spawn builds the same root a spaceship gets - the
//! dock, the trade, the sever and the freeze all query `SpaceshipRootMarker` -
//! and adds [`StationMarker`] and [`StationStorage`] over it. Nothing here
//! inserts a player or AI marker: a station has no driver.
//!
//! Invulnerability and the self-levelling tuning are the next task; a station
//! here takes damage like the hull it is built from.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond3;
use nova_gameplay::prelude::{
    Allegiance, DerelictShipMarker, ItemDesignId, ShipInventoryStock, UnsettledBody,
};
use nova_ship::prelude::ShipCapabilities;

use crate::objects::{
    ship_design::prelude::{ShipDesign, ShipDesignSource},
    spaceship::prelude::{
        freeze_ship, spaceship_scenario_object, thaw_ship, FrozenShip, SpaceshipConfig,
        SpaceshipController,
    },
};

/// Glob-import surface for the station content kind and object.
pub mod prelude {
    pub use super::{
        freeze_station, station_scenario_object, thaw_station, FrozenStation, GameStationDesigns,
        StationConfig, StationDesign, StationDesignId, StationMarker, StationServiceType,
        StationStorage,
    };
}

/// The id a station design is referenced by, from a scenario spawn or another
/// mod.
///
/// A `DesignId`: an authored catalog identifier, resolved against
/// [`GameStationDesigns`]. Serialized as the bare id string, so authored RON
/// writes `design: "block_waystation"`. An id no loaded design carries is
/// refused at lint and at load; nothing resolves it to another design.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct StationDesignId(String);

impl StationDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StationDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for StationDesignId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// What a docked ship can do at a station. The closed set the game knows;
/// a design lists the ones it offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StationServiceType {
    /// Rebuild destroyed sections of the docked ship.
    Rebuild,
    /// Change the docked ship's section layout.
    Refit,
    /// Buy from and sell to the station's stock.
    Trade,
    /// Leave items in the station's own storage, up to its cap.
    Storage,
    /// Take contracts posted at the station.
    Jobs,
    /// Hire and drop crew.
    Roster,
}

/// What a station IS, apart from any one spawn of it: its hull, what it
/// offers, what it holds and what it pays with.
///
/// Every field is required in RON. A station with nothing to offer or a
/// storage cap with no Storage service is refused at lint, see
/// `lint_station_design`.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationDesign {
    /// The id a scenario spawns this design by.
    pub id: StationDesignId,
    /// The name a picker shows. A spawned station is named by the scenario
    /// object that placed it.
    pub name: String,
    /// What this station is, for a reader.
    pub about: String,
    /// The hull: a docking port, a flight computer and plating, no thruster.
    pub hull: ShipDesign,
    /// The services a docked ship may use here. At least one.
    pub services: Vec<StationServiceType>,
    /// How many grams [`StationStorage`] takes. Above zero exactly when
    /// `services` lists [`StationServiceType::Storage`].
    pub storage_cap_g: u32,
    /// The stock the station's hold opens with, within the hull's hold.
    pub stock: ShipInventoryStock,
    /// The credits the station trades with at spawn.
    pub credits: u32,
}

/// The loaded station-design catalog: every [`StationDesign`] the enabled mods
/// registered, in merge order. Filled by the mod merge (`nova_assets`'s
/// `register_bundles`) and read through
/// [`get_design`](GameStationDesigns::get_design).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameStationDesigns(pub Vec<StationDesign>);

impl GameStationDesigns {
    /// The design `id` names, or `None` if nothing authored it.
    pub fn get_design(&self, id: &StationDesignId) -> Option<&StationDesign> {
        self.0.iter().find(|design| design.id == *id)
    }
}

/// The scenario/modding RON surface for a station object: WHICH design it
/// spawns, which side it is on, whether it is a derelict and how it moves.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationConfig {
    /// The catalog design, by id.
    pub design: StationDesignId,
    /// Which side the station belongs to. `None` keeps no side: a station
    /// is hostile to no one. In strict RON the `Option` keeps its variant:
    /// `allegiance: Some(Neutral)`.
    pub allegiance: Option<Allegiance>,
    /// A dead station: its systems spawn inactive and a docked ship may Take
    /// from its hold.
    pub derelict: bool,
    /// Explicit initial motion in meters per second. Zero means an intentional
    /// stationary start, not immunity to gravity.
    pub initial_velocity: MetersPerSecond3,
}

/// The station root's own marker, carrying the design it was built from.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
pub struct StationMarker(pub StationDesignId);

/// What a station keeps for the ships that dock with it: a capped store
/// beside the hull's own hold. Empty at spawn.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationStorage {
    cap_g: u32,
    items: BTreeMap<ItemDesignId, u32>,
}

impl StationStorage {
    /// An empty store that takes `cap_g` grams.
    pub fn new(cap_g: u32) -> Self {
        Self {
            cap_g,
            items: BTreeMap::new(),
        }
    }

    /// How many grams the store takes.
    pub fn cap_g(&self) -> u32 {
        self.cap_g
    }

    /// Every stack held, by item.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemDesignId, u32)> + '_ {
        self.items.iter().map(|(item, count)| (item, *count))
    }

    /// Every item id held.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        self.items.keys()
    }
}

/// Build the station `design` on `entity` as `config` spawns it: the ship
/// root with no driver, docking as its one capability, the design's stock
/// and credits, plus [`StationMarker`] and an empty [`StationStorage`].
///
/// A derelict's marker lands before the root marker, so the section spawn
/// reads it and builds the station's systems inactive.
pub fn station_scenario_object(
    entity: &mut EntityCommands,
    design: &StationDesign,
    config: &StationConfig,
) {
    if config.derelict {
        entity.insert(DerelictShipMarker);
    }
    entity.insert((
        spaceship_scenario_object(SpaceshipConfig {
            design: ShipDesignSource::Inline(design.hull.clone()),
            controller: SpaceshipController::None,
            initial_velocity: config.initial_velocity,
            allegiance: config.allegiance,
            capabilities: station_capabilities(),
            inventory: design.stock.clone(),
            lootable: config.derelict,
            credits: design.credits,
        }),
        StationMarker(design.id.clone()),
        StationStorage::new(design.storage_cap_g),
    ));
}

/// What a station is permitted to do: receive a dock, and nothing a hull
/// under way does.
fn station_capabilities() -> ShipCapabilities {
    ShipCapabilities {
        stop_enabled: false,
        goto_enabled: false,
        orbit_enabled: false,
        lock_enabled: false,
        rcs_enabled: false,
        point_defense_enabled: false,
        dock_enabled: true,
    }
}

/// A station frozen with its cell: the design it was built from, the hull
/// record a ship freezes to, and its storage.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenStation {
    design: StationDesignId,
    ship: FrozenShip,
    storage: StationStorage,
}

impl FrozenStation {
    /// The design the station was built from.
    pub fn design(&self) -> &StationDesignId {
        &self.design
    }

    /// The hull record: sections, hold, damage and balance.
    pub fn ship(&self) -> &FrozenShip {
        &self.ship
    }

    /// The store the station froze with.
    pub fn storage(&self) -> &StationStorage {
        &self.storage
    }

    /// Every item id the station holds: its hull's and its storage's.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        self.ship.item_ids().chain(self.storage.item_ids())
    }
}

/// Freeze a station: its hull as [`freeze_ship`] records it, with its design
/// id and storage.
///
/// # Errors
///
/// [`UnsettledBody`] while the hull is collapsing, being destroyed or
/// mid-sever, see [`freeze_ship`].
///
/// # Panics
///
/// On an entity that carries no [`StationMarker`] or no [`StationStorage`]:
/// not a station root this module spawned.
pub fn freeze_station(world: &World, station: Entity) -> Result<FrozenStation, UnsettledBody> {
    let root = world.entity(station);
    let Some(marker) = root.get::<StationMarker>() else {
        panic!("freeze_station: {station} is not a station root");
    };
    let Some(storage) = root.get::<StationStorage>() else {
        panic!("freeze_station: station {station} has no storage");
    };
    Ok(FrozenStation {
        design: marker.0.clone(),
        ship: freeze_ship(world, station)?,
        storage: storage.clone(),
    })
}

/// Rebuild a frozen station on `entity`, which the caller spawned with its
/// pose, identity and parent. The hull thaws through [`thaw_ship`]; the
/// marker and the storage land beside it. The caller inserts the velocity
/// after this.
pub fn thaw_station(entity: &mut EntityCommands, frozen: FrozenStation) {
    let FrozenStation {
        design,
        ship,
        storage,
    } = frozen;
    thaw_ship(entity, ship);
    entity.insert((StationMarker(design), storage));
}
