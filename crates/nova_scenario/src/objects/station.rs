//! The STATION content kind and the station scenario object.
//!
//! A station is a hull that never flies: a [`ShipDesign`] with a docking port
//! and a flight computer, no thruster, parked where the scenario placed it.
//! It is authored once as a [`StationDesign`] and spawned by id through
//! [`StationConfig`]. The spawn builds the same root a spaceship gets - the
//! dock, the trade, the sever and the freeze all query `SpaceshipRootMarker` -
//! and adds [`StationMarker`] and [`StationStorage`] over it; the marker
//! brings the design's [`StationServices`] with it. Nothing here inserts a
//! player or AI marker: a station has no driver.
//!
//! The body is a dynamic thrusterless hull, never `RigidBody::Static`:
//! gravity pulls it and a ram shoves it. Its Controller section is the whole
//! of its station-keeping: the controller parks its target attitude on the
//! spawn rotation and drives the root with PD torque alone
//! (`controller_section::insert_controller_section_target` and
//! `sync_controller_section_forces`), so a hit that spins the hull is damped
//! back onto the parked attitude within [`STATION_LEVEL_TOLERANCE_RAD`], and
//! nothing ever pushes the body back to where it was placed. Thrust stays
//! zero because only a Thruster section writes thrust and the station lint
//! refuses one (`lint::station`). Station sections take no damage: the health
//! store refuses every hit under a [`StationMarker`] root
//! (`nova_gameplay::integrity::health::on_damage`).
//!
//! Where a cluster has a planet its station orbits it: the generator spawns
//! it with [`station_orbit_velocity`], the circular speed the gravity layer
//! itself integrates, and the orbit is held within
//! [`STATION_ORBIT_TOLERANCE`] of its radius.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond3;
use nova_gameplay::prelude::{
    circular_orbit_speed, Allegiance, DerelictShipMarker, ItemDesignId, ShipInventoryStock,
    UnsettledBody,
};
pub use nova_gameplay::prelude::{
    StationDesignId, StationMarker, StationServiceType, StationServices,
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
        freeze_station, station_orbit_velocity, station_scenario_object, thaw_station,
        FrozenStation, GameStationDesigns, StationConfig, StationDesign, StationDesignId,
        StationMarker, StationServiceType, StationServices, StationStorage,
        STATION_LEVEL_TOLERANCE_RAD, STATION_ORBIT_TOLERANCE,
    };
}

/// How far off its parked attitude a settled station may rest, in radians.
///
/// 0.03 rad is eight times the residual the controller leaves after a 0.5 rad
/// nudge (`system_station_body` settles 0.0038 rad off spawn), so float noise
/// and one fixed step of integration never fail it, and well under the
/// 0.1 rad a proof turns the hull before it counts as moved, so a hull that
/// never recovered cannot pass. On the 30 m waystation it is a 0.45 m sway of
/// the collar: not a thing a docking approach notices.
pub const STATION_LEVEL_TOLERANCE_RAD: f32 = 0.03;

/// How far a station's orbital radius may drift over three orbits, as a
/// fraction of the radius it was placed at.
///
/// Two things spend this budget. The pull is a force held constant over each
/// fixed step, so an orbit gains energy every step and widens: measured at
/// about 1.5 % of its binding energy per orbit on `system_station_orbit`'s
/// 30 s orbit, less on a generated planetoid's slower one, since the gain
/// scales with the angle swept per step. And a hit: a tangential kick of one
/// percent of orbital speed swings the radius about four percent (the
/// apoapsis of the new ellipse is roughly `r (1 + 4 f)`). Three orbits of
/// the first plus one of the second measure a tenth; 0.15 leaves the
/// integrator's per-orbit wobble room and still refuses an orbit that is
/// walking out of the well's unfaded core or down onto its surface clamp.
pub const STATION_ORBIT_TOLERANCE: f32 = 0.15;

/// The velocity a station spawns with to ride a circular orbit of the well at
/// `well_position` with strength `mu` (engine units, as `GravityWell::mu`),
/// placed at `station_position` (engine units).
///
/// The speed is the gravity layer's own [`circular_orbit_speed`], so the
/// station rides the rule the FixedUpdate pull integrates. The tangent is the
/// radial crossed with +Y, or +X when the radial is near +Y: the rule the
/// generated rocks seed with, without their per-id sign flip, so every
/// station in a cluster rides the same way round.
///
/// `None` when the station sits on the well's centre or the geometry is not
/// finite: no orbit exists there, and the caller refuses the placement.
pub fn station_orbit_velocity(
    mu: f32,
    well_position: Vec3,
    station_position: Vec3,
) -> Option<MetersPerSecond3> {
    let radial = (station_position - well_position).try_normalize()?;
    let axis = if radial.y.abs() < 0.8 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let tangent = radial.cross(axis).try_normalize()?;
    let speed = circular_orbit_speed(mu, station_position.distance(well_position));
    let velocity = MetersPerSecond3::from_engine(tangent * speed);
    velocity.is_finite().then_some(velocity)
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

/// Insert a station root's [`StationServices`] from its design whenever a
/// [`StationMarker`] lands, so a fresh spawn and a thaw read the same list.
///
/// # Panics
///
/// On a marker naming a design the catalog lacks: lint and the save load
/// refuse such an id before anything spawns it.
pub(crate) fn insert_station_services(
    add: On<Add, StationMarker>,
    q_marker: Query<&StationMarker>,
    designs: Res<GameStationDesigns>,
    mut commands: Commands,
) {
    let Ok(marker) = q_marker.get(add.entity) else {
        return;
    };
    let Some(design) = designs.get_design(&marker.0) else {
        panic!(
            "insert_station_services: station {:?} names design '{}', which the station \
             catalog lacks",
            add.entity, marker.0
        );
    };
    commands
        .entity(add.entity)
        .insert(StationServices(design.services.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The orbit velocity is the gravity layer's circular speed on a tangent:
    /// it is perpendicular to the radial, its magnitude is `sqrt(mu / r)`,
    /// and a station placed on the well's centre gets no orbit at all.
    #[test]
    fn station_orbit_velocity_is_the_circular_tangent_speed() {
        let well = Vec3::new(10.0, -5.0, 2.0);
        let station = well + Vec3::new(45.0, 0.0, 0.0);
        let mu = 4000.0;

        let velocity = station_orbit_velocity(mu, well, station)
            .expect("a station off the centre gets an orbit")
            .to_engine();

        let radial = station - well;
        assert!(
            velocity.dot(radial).abs() < 1e-3,
            "the orbit velocity {velocity:?} is not tangent to the radial {radial:?}"
        );
        let expected = circular_orbit_speed(mu, radial.length());
        assert!(
            (velocity.length() - expected).abs() < 1e-3,
            "the orbit speed is {} not {expected}",
            velocity.length()
        );
        assert!(
            station_orbit_velocity(mu, well, well).is_none(),
            "a station on the well's centre must get no orbit"
        );
    }
}
