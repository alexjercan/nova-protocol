//! The STATION content kind and the station scenario object.
//!
//! A station is a hull that never flies: a [`ShipDesign`] with a docking port
//! and a flight computer, no thruster, parked where the scenario placed it.
//! It is authored as a [`StationDesign`], either once in the catalog as a
//! [`StationDesignPrototype`] spawned by id or inline in the spawn, as the
//! open world writes its generated stations ([`StationDesignSource`]). The
//! spawn builds the same root a spaceship gets - the dock, the trade, the
//! sever and the freeze all query `SpaceshipRootMarker` - and adds
//! [`StationMarker`], the design's [`StationServices`] and
//! [`StationStorage`] over it. A living station is
//! AI-driven: its turrets aim, defend and answer fire, and with no thruster
//! the AI never flies it. A dead station has no driver.
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
//! refuses one (`lint::station`).
//!
//! A station takes damage, collapses and is destroyed as a ship is: below
//! the hull's authored `collapse_threshold` share of its pool (a sever that
//! leaves the kept half there included) it is peeled apart and its root
//! destroyed, and its hold and the player's storage drop as canisters
//! (`wreck_cargo`). A living station that is neutralized - its last turret
//! or its last flight computer lost - turns into a dead station
//! ([`on_station_neutralized_turn_derelict`]) first, and a dead station can
//! still be shot apart.
//!
//! Where a cluster has a planet its station orbits it: the generator spawns
//! it with [`station_orbit_velocity`], the circular speed the gravity layer
//! itself integrates, and the orbit is held within
//! [`STATION_ORBIT_TOLERANCE`] of its radius.

use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond3;
use nova_gameplay::prelude::{
    circular_orbit_speed, Allegiance, DerelictShipMarker, ItemDesignId, LootableShipMarker,
    NeutralizedMarker, OwnedStationMarker, RetaliationTarget, SectionInactiveMarker, SectionMarker,
    ShipInventoryStock, UnsettledBody,
};
pub use nova_gameplay::prelude::{
    StationDesignId, StationMarker, StationServiceType, StationServices, StationStorage,
};
use nova_ship::prelude::{AISpaceshipMarker, SectionBuildConfig, ShipCapabilities};

use crate::objects::{
    parked_hulls::prelude::{
        freeze_parked_hulls, FrozenParkedHull, PendingParkedHull, PendingParkedHulls,
    },
    ship_design::prelude::{ShipDesign, ShipDesignSource},
    spaceship::{
        derelict_keeps_live,
        prelude::{
            freeze_ship, spaceship_scenario_object, thaw_ship, AIControllerConfig, FrozenShip,
            SpaceshipConfig, SpaceshipController,
        },
    },
};

/// Glob-import surface for the station content kind and object.
pub mod prelude {
    pub use super::{
        freeze_station, station_orbit_velocity, station_scenario_object, thaw_station,
        FrozenStation, GameStationDesigns, StationConfig, StationDesign, StationDesignId,
        StationDesignPrototype, StationDesignSource, StationMarker, StationServiceType,
        StationServices, StationStorage, STATION_COLLAPSE_THRESHOLD, STATION_LEVEL_TOLERANCE_RAD,
        STATION_ORBIT_TOLERANCE,
    };
}

/// The share of its hull pool at which a station collapses: half, above the
/// ship default, so a station comes apart sooner, a sever that cuts away
/// half of it included. Every shipped station and every station the sandbox
/// editor writes authors this value.
pub const STATION_COLLAPSE_THRESHOLD: f32 = 0.5;

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

/// One authored station design in the catalog: an id, a name, what it is,
/// and the design it is.
///
/// Resolved by [`id`](StationDesignPrototype::id) out of
/// [`GameStationDesigns`], which the mod merge fills as it fills the ship
/// design catalog, so a mod's design with the id of a base one replaces it.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationDesignPrototype {
    /// The id a scenario spawns this design by.
    pub id: StationDesignId,
    /// The name a picker shows. A spawned station is named by the scenario
    /// object that placed it.
    pub name: String,
    /// What this station is, for a reader.
    pub about: String,
    /// The design itself.
    pub design: StationDesign,
}

/// Where a spawned station's [`StationDesign`] comes from.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// Inline carries a whole hull next to Prototype's small id, and boxing it
// cannot compile here because the enum derives Reflect and bevy_reflect 0.19
// has no Reflect impl for Box<T>: the trade `ShipDesignSource` makes.
#[expect(
    clippy::large_enum_variant,
    reason = "spawn-time config, and bevy_reflect 0.19 cannot box the variant"
)]
pub enum StationDesignSource {
    /// A catalog design, by id, resolved against [`GameStationDesigns`].
    Prototype(StationDesignId),
    /// The design, whole: the open world's generated stations spawn this
    /// way, and a save keeps what the spawn built.
    Inline(StationDesign),
}

/// The loaded station-design catalog: every [`StationDesignPrototype`] the
/// enabled mods registered, in merge order. Filled by the mod merge
/// (`nova_assets`'s `register_bundles`) and read through
/// [`get_design`](GameStationDesigns::get_design).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameStationDesigns(pub Vec<StationDesignPrototype>);

impl GameStationDesigns {
    /// The design `id` names, or `None` if nothing authored it.
    pub fn get_design(&self, id: &StationDesignId) -> Option<&StationDesignPrototype> {
        self.0.iter().find(|design| design.id == *id)
    }

    /// The design `source` spawns: the catalog design it names, or its
    /// inline design. `None` when it names an id nothing authored.
    pub fn resolve<'a>(&'a self, source: &'a StationDesignSource) -> Option<&'a StationDesign> {
        match source {
            StationDesignSource::Prototype(id) => {
                self.get_design(id).map(|prototype| &prototype.design)
            }
            StationDesignSource::Inline(design) => Some(design),
        }
    }
}

/// The scenario/modding RON surface for a station object: WHICH design it
/// spawns, which side it is on, whether it is a derelict and how it moves.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationConfig {
    /// The design: a catalog design by id, or one written inline.
    pub design: StationDesignSource,
    /// Which side the station belongs to. An Enemy or Player station fires
    /// on the sides hostile to it; a Neutral one only answers fire. A dead
    /// station is Neutral whatever this says.
    pub allegiance: Allegiance,
    /// A dead station: its systems spawn inactive and a docked ship may Take
    /// from its hold.
    pub derelict: bool,
    /// Explicit initial motion in meters per second. Zero means an intentional
    /// stationary start, not immunity to gravity.
    pub initial_velocity: MetersPerSecond3,
}

/// Build the station `design` on `entity` as `config` spawns it: the ship
/// root with the design's stock and credits, plus [`StationMarker`], the
/// design's [`StationServices`] and an empty [`StationStorage`]. The caller
/// resolves `config.design` to `design`. A living station is AI-driven and defends
/// itself; a dead one has no driver, takes its loot marker and is Neutral.
///
/// A derelict's marker lands before the root marker, so the section spawn
/// reads it and builds the station's systems inactive.
///
/// # Panics
///
/// On a design whose hull authors no `integrity.collapse_threshold`: a
/// station states the share of its hull it collapses at, and the lint
/// refuses the design first.
pub fn station_scenario_object(
    entity: &mut EntityCommands,
    design: &StationDesign,
    config: &StationConfig,
) {
    assert!(
        design.hull.integrity.collapse_threshold.is_some(),
        "station_scenario_object: a station design authors no hull \
         integrity.collapse_threshold, the share it collapses at"
    );
    if config.derelict {
        entity.insert(DerelictShipMarker);
    }
    let living = !config.derelict;
    entity.insert((
        spaceship_scenario_object(SpaceshipConfig {
            design: ShipDesignSource::Inline(design.hull.clone()),
            controller: if living {
                station_ai()
            } else {
                SpaceshipController::None
            },
            initial_velocity: config.initial_velocity,
            allegiance: Some(if living {
                config.allegiance
            } else {
                Allegiance::Neutral
            }),
            capabilities: station_capabilities(living),
            inventory: design.stock.clone(),
            lootable: config.derelict,
            credits: design.credits,
        }),
        StationMarker,
        StationServices(design.services.clone()),
        StationStorage::new(design.storage_cap_g),
    ));
}

/// The AI a living station is driven by. No leash: the leash measures how far
/// the hull has moved from its spawn, and an orbiting station moves all the
/// time without leaving its post.
fn station_ai() -> SpaceshipController {
    SpaceshipController::AI(AIControllerConfig {
        leash: None,
        ..default()
    })
}

/// Turn a living station that has just been neutralized into a dead station:
/// derelict, lootable, Neutral, with no driver and every section but its
/// structure and docking ports switched off, as a station spawned dead is.
/// It keeps its place, its hold and its storage; nothing despawns.
///
/// Neutralization is the generic rule (`nova_gameplay::integrity::neutralize`):
/// an armed station loses its last weapon or its last flight computer, an
/// unarmed one its last flight computer. A station thawed already dead
/// carries [`DerelictShipMarker`] before its neutralized marker and is left
/// alone.
pub(crate) fn on_station_neutralized_turn_derelict(
    add: On<Add, NeutralizedMarker>,
    mut commands: Commands,
    q_station: Query<Option<&Children>, (With<StationMarker>, Without<DerelictShipMarker>)>,
    q_section: Query<&SectionBuildConfig, (With<SectionMarker>, Without<SectionInactiveMarker>)>,
) {
    let station = add.entity;
    let Ok(children) = q_station.get(station) else {
        return;
    };
    info!("on_station_neutralized_turn_derelict: station {station:?} is dead");
    for child in children.into_iter().flatten() {
        if let Ok(config) = q_section.get(*child) {
            if !derelict_keeps_live(&config.0.kind) {
                commands.entity(*child).insert(SectionInactiveMarker);
            }
        }
    }
    commands
        .entity(station)
        .remove::<(AISpaceshipMarker, RetaliationTarget)>()
        .insert((
            DerelictShipMarker,
            LootableShipMarker,
            Allegiance::Neutral,
            SpaceshipController::None,
            station_capabilities(false),
        ));
}

/// What a station is permitted to do: receive a dock, defend itself while
/// it is `living`, and nothing a hull under way does.
fn station_capabilities(living: bool) -> ShipCapabilities {
    ShipCapabilities {
        stop_enabled: false,
        goto_enabled: false,
        orbit_enabled: false,
        lock_enabled: false,
        rcs_enabled: false,
        point_defense_enabled: living,
        dock_enabled: true,
    }
}

/// A station frozen with its cell: the services it offers, the hull record
/// a ship freezes to, its storage, the hulls it holds (parked at its
/// ports or coasting beside it for hire) and whether the player owns it.
///
/// `owned` is the one record of ownership: the live [`OwnedStationMarker`]
/// is frozen into it and thawed out of it, so a bought station stays the
/// player's across a retire, a thaw and a Load with no list beside it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenStation {
    services: Vec<StationServiceType>,
    ship: FrozenShip,
    storage: StationStorage,
    parked: Vec<FrozenParkedHull>,
    owned: bool,
}

impl FrozenStation {
    /// The services the station offers.
    pub fn services(&self) -> &[StationServiceType] {
        &self.services
    }

    /// The hull record: sections, hold, damage and balance.
    pub fn ship(&self) -> &FrozenShip {
        &self.ship
    }

    /// The store the station froze with.
    pub fn storage(&self) -> &StationStorage {
        &self.storage
    }

    /// The hulls the station holds: for sale or the player's own at its ports, and
    /// for hire beside it.
    pub fn parked(&self) -> &[FrozenParkedHull] {
        &self.parked
    }

    /// Whether the player owns the station.
    pub fn owned(&self) -> bool {
        self.owned
    }

    /// Every item id the station holds: its hull's, its storage's and its
    /// parked hulls'.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        self.ship
            .item_ids()
            .chain(self.storage.item_ids())
            .chain(self.parked.iter().flat_map(|hull| hull.ship().item_ids()))
    }
}

/// Freeze a station: its hull as [`freeze_ship`] records it, with its
/// services, storage, parked hulls ([`freeze_parked_hulls`]) and whether it carries
/// [`OwnedStationMarker`].
///
/// # Errors
///
/// [`UnsettledBody`] while the hull is collapsing, being destroyed or
/// mid-sever, see [`freeze_ship`], or while a parked hull cannot freeze.
///
/// # Panics
///
/// On an entity that carries no [`StationMarker`], no [`StationServices`] or
/// no [`StationStorage`]: not a station root this module spawned.
pub fn freeze_station(world: &World, station: Entity) -> Result<FrozenStation, UnsettledBody> {
    let root = world.entity(station);
    if !root.contains::<StationMarker>() {
        panic!("freeze_station: {station} is not a station root");
    }
    let Some(services) = root.get::<StationServices>() else {
        panic!("freeze_station: station {station} has no services");
    };
    let Some(storage) = root.get::<StationStorage>() else {
        panic!("freeze_station: station {station} has no storage");
    };
    Ok(FrozenStation {
        services: services.0.clone(),
        ship: freeze_ship(world, station)?,
        storage: storage.clone(),
        parked: freeze_parked_hulls(world, station)?,
        owned: root.contains::<OwnedStationMarker>(),
    })
}

/// Rebuild a frozen station on `entity`, which the caller spawned with its
/// pose, identity and parent. The hull thaws through [`thaw_ship`]; the
/// marker, the services, the storage, the parked hulls to park again
/// ([`PendingParkedHulls`]) and, for an owned station,
/// [`OwnedStationMarker`] land beside it. The caller inserts the
/// velocity after this.
pub fn thaw_station(entity: &mut EntityCommands, frozen: FrozenStation) {
    let FrozenStation {
        services,
        ship,
        storage,
        parked,
        owned,
    } = frozen;
    thaw_ship(entity, ship);
    entity.insert((StationMarker, StationServices(services), storage));
    if !parked.is_empty() {
        entity.insert(PendingParkedHulls(
            parked.into_iter().map(PendingParkedHull::Frozen).collect(),
        ));
    }
    if owned {
        entity.insert(OwnedStationMarker);
    }
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
