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
//! gravity pulls it and a ram shoves it. Its Controller section parks its
//! target attitude on the spawn rotation and drives the root with PD torque
//! alone (`controller_section::insert_controller_section_target` and
//! `sync_controller_section_forces`), so a hit that spins the hull is damped
//! back onto the parked attitude within [`STATION_LEVEL_TOLERANCE_RAD`].
//! Its place is held by its post: [`assign_station_post`] derives a
//! [`StationPost`] from the authored position and velocity on the first
//! fixed tick - a circular orbit of the well that holds it, or a fixed point
//! in flat space - and a living station rides back onto it on its RCS
//! (`nova_ship`'s `station_keeping_system`), pushing every hull docked at it
//! along. A dead station keeps its post and nothing reads it, so a docked
//! player tows it freely. Thrust stays zero because only a Thruster section
//! writes thrust and the station lint refuses one (`lint::station`).
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
//! itself integrates. A station authored in a well with any other velocity,
//! or out of every well with a velocity at all, has no post: the scenario
//! lint refuses it ([`derive_station_post`]) and the spawn panics.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use nova_events::prelude::{
    EntityId, EntityTypeName, MetersPerSecond3, DERELICT_STATION_TYPE_NAME,
    LIVING_STATION_TYPE_NAME,
};
use nova_gameplay::prelude::{
    circular_orbit_speed, well_accel, Allegiance, DerelictShipMarker, GravitySettings, GravityWell,
    ItemDesignId, LootableShipMarker, NeutralizedMarker, OwnedStationMarker, RetaliationTarget,
    SectionInactiveMarker, SectionMarker, ShipInventoryStock, UnsettledBody,
};
pub use nova_gameplay::prelude::{
    StationBuilds, StationDesignId, StationMarker, StationServiceType, StationServices,
    StationStorage,
};
use nova_ship::prelude::{AISpaceshipMarker, SectionBuildConfig, ShipCapabilities, StationPost};

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
        derive_station_post, freeze_station, station_orbit_velocity, station_scenario_object,
        station_well, thaw_station, FrozenStation, GameStationDesigns, StationBuilds,
        StationConfig, StationDesign, StationDesignId, StationDesignPrototype, StationDesignSource,
        StationMarker, StationServiceType, StationServices, StationStorage,
        STATION_COLLAPSE_THRESHOLD, STATION_LEVEL_TOLERANCE_RAD, STATION_POST_CIRCULAR_TOLERANCE,
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

/// How far an authored station velocity may differ from the circular speed
/// of the well that holds it, as a fraction of that speed, in magnitude and
/// in its radial component. Room for a number copied from
/// [`station_orbit_velocity`] and rounded in RON, and for one fixed step of
/// the pull before the post is read; a tenth off is an ellipse the keeper
/// would spend its magazine on, and is refused.
pub const STATION_POST_CIRCULAR_TOLERANCE: f32 = 0.05;

/// The well that holds a station at `position`: the centre and `mu` of the
/// well whose pull there is strongest, by the rule the force system applies
/// to a body with no incumbent well (`dominant_well_acceleration`, see
/// `nova_gameplay::gravity`). `None` in flat space. `wells` are each a
/// centre and its well, engine units.
pub fn station_well(
    position: Vec3,
    wells: &[(Vec3, GravityWell)],
    settings: &GravitySettings,
) -> Option<(Vec3, f32)> {
    wells
        .iter()
        .filter_map(|(centre, well)| {
            let offset = *centre - position;
            offset.try_normalize()?;
            let accel = well_accel(
                well.mu,
                offset.length(),
                well.body_radius,
                well.soi_radius,
                settings.fade_fraction,
                settings.surface_margin,
            );
            (accel > 0.0).then_some((*centre, well.mu, accel))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(centre, mu, _)| (centre, mu))
}

/// The post a station at `position` with `velocity` holds, engine units:
/// held by `well` (its centre and `mu`, see [`station_well`]) it rides a
/// circular orbit, so the velocity must be tangential at the circular speed
/// within [`STATION_POST_CIRCULAR_TOLERANCE`]; in flat space it stands at
/// rest, so the velocity must be zero. The lint and the spawn both apply
/// this rule, the lint as an error and the spawn as a panic.
///
/// # Errors
///
/// The authored fault, worded for the author.
pub fn derive_station_post(
    position: Vec3,
    velocity: Vec3,
    well: Option<(Vec3, f32)>,
) -> Result<StationPost, String> {
    let Some((centre, mu)) = well else {
        if velocity != Vec3::ZERO {
            return Err(format!(
                "stands outside every well with a velocity of {:?} m/s; a station out of a well \
                 authors (0.0, 0.0, 0.0)",
                MetersPerSecond3::from_engine(velocity).get()
            ));
        }
        return Ok(StationPost::Fixed { position });
    };
    let radial = position - centre;
    let radius = radial.length();
    let Some(radial_direction) = radial.try_normalize() else {
        return Err("stands on the centre of a well; no orbit exists there".to_string());
    };
    let circular = circular_orbit_speed(mu, radius);
    let speed = velocity.length();
    let tolerance = STATION_POST_CIRCULAR_TOLERANCE * circular;
    if (speed - circular).abs() > tolerance || velocity.dot(radial_direction).abs() > tolerance {
        return Err(format!(
            "moves at {:.2} m/s with {:.2} m/s along its radial, {:.0} m from a well whose \
             circular speed there is {:.2} m/s; a station in a well authors \
             station_orbit_velocity (within {:.0} percent, tangential)",
            MetersPerSecond3::from_engine(velocity).get().length(),
            MetersPerSecond3::from_engine(radial_direction * velocity.dot(radial_direction))
                .get()
                .length(),
            nova_events::prelude::Meters::from_engine(radius).0,
            MetersPerSecond3::from_engine(radial_direction * circular)
                .get()
                .length(),
            STATION_POST_CIRCULAR_TOLERANCE * 100.0
        ));
    }
    let Some(normal) = radial.cross(velocity).try_normalize() else {
        return Err("has no orbit plane: its velocity is not tangential".to_string());
    };
    Ok(StationPost::Orbit {
        centre,
        radius,
        normal,
    })
}

/// Give every station root without a post the one its authored position and
/// velocity define ([`derive_station_post`]), living or dead, so a thaw and a
/// freeze always find one. `FixedUpdate`, on the first tick a station's
/// avian pose is real: avian seeds `Position::PLACEHOLDER` until the body
/// resolves its `Transform`, and a post written from the placeholder would
/// be permanent. A thawed station carries its frozen post and is skipped.
///
/// # Panics
///
/// On a station whose authored motion defines no post. The scenario lint
/// refuses the same authoring first (`lint::scenario`); a generated station
/// is placed on its orbit or at rest by the generator. The wells are the
/// ones up on this tick: a station spawned before its well is authored
/// wrong (it flies straight until the well appears) and panics here, which
/// the lint cannot see because it does not order triggers.
pub(crate) fn assign_station_post(
    settings: Res<GravitySettings>,
    mut commands: Commands,
    q_station: Query<
        (
            Entity,
            &Position,
            &LinearVelocity,
            Option<&EntityId>,
            Option<&Name>,
        ),
        (With<StationMarker>, Without<StationPost>),
    >,
    q_wells: Query<(&Position, &GravityWell)>,
) {
    if q_station.is_empty() {
        return;
    }
    let wells: Vec<(Vec3, GravityWell)> = q_wells
        .iter()
        .map(|(position, well)| (position.0, well.clone()))
        .collect();
    for (station, position, velocity, id, name) in &q_station {
        if !position.0.is_finite() || position.0 == Position::PLACEHOLDER.0 {
            continue;
        }
        let well = station_well(position.0, &wells, &settings);
        match derive_station_post(position.0, velocity.0, well) {
            Ok(post) => {
                debug!("assign_station_post: station {station:?} holds {post:?}");
                commands.entity(station).insert(post);
            }
            Err(fault) => {
                let label = id
                    .map(|id| id.0.clone())
                    .or_else(|| name.map(ToString::to_string))
                    .unwrap_or_else(|| station.to_string());
                panic!("assign_station_post: station '{label}' {fault}");
            }
        }
    }
}

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
    /// The section prototype ids the station builds: what its catalogue
    /// lists and what a swap, an add or a rebuild may craft here, whatever
    /// parts the holds carry. Each a known prototype, listed once; nonempty
    /// exactly when `services` lists Refit or Rebuild.
    pub builds: Vec<String>,
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
        StationBuilds(design.builds.clone()),
        StationStorage::new(design.storage_cap_g),
    ));
    // Its own insert: the spaceship bundle above already names the hull
    // `spaceship`, and one bundle may not carry a component twice.
    entity.insert(station_type_name(living));
}

/// The type name a station wears: a living one and a dead one are two kinds
/// to an event filter, where a ship of any state is one. Inserted after the
/// spaceship bundle, which names every hull `spaceship`.
fn station_type_name(living: bool) -> EntityTypeName {
    EntityTypeName::new(if living {
        LIVING_STATION_TYPE_NAME
    } else {
        DERELICT_STATION_TYPE_NAME
    })
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
            station_type_name(false),
        ));
}

/// What a station is permitted to do: receive a dock, and while it is
/// `living` defend itself and hold its post on its RCS; nothing a hull under
/// way does. The RCS flag gates `rcs_burn_system` alone: no helm reaches a
/// living station, and a dead one is towed, never pushed.
fn station_capabilities(living: bool) -> ShipCapabilities {
    ShipCapabilities {
        stop_enabled: false,
        goto_enabled: false,
        orbit_enabled: false,
        lock_enabled: false,
        rcs_enabled: living,
        point_defense_enabled: living,
        dock_enabled: true,
    }
}

/// A station frozen with its cell: the services it offers, the hull record
/// a ship freezes to, its storage, the hulls it holds (parked at its
/// ports or coasting beside it for hire), whether the player owns it and
/// the post it holds.
///
/// `owned` is the one record of ownership: the live [`OwnedStationMarker`]
/// is frozen into it and thawed out of it, so a bought station stays the
/// player's across a retire, a thaw and a Load with no list beside it.
/// `post` is the one record of the post: a station frozen mid-return thaws
/// with its live pose and motion and finishes the return.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenStation {
    services: Vec<StationServiceType>,
    builds: Vec<String>,
    ship: FrozenShip,
    storage: StationStorage,
    parked: Vec<FrozenParkedHull>,
    owned: bool,
    post: StationPost,
}

impl FrozenStation {
    /// The post the station holds.
    pub fn post(&self) -> StationPost {
        self.post
    }

    /// The services the station offers.
    pub fn services(&self) -> &[StationServiceType] {
        &self.services
    }

    /// The section prototype ids the station builds.
    pub fn builds(&self) -> &[String] {
        &self.builds
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
/// mid-sever, see [`freeze_ship`], while a parked hull cannot freeze, or
/// while the station has no [`StationPost`] yet: a station spawned and
/// retired in one frame has not run the fixed tick that derives it, and the
/// cell holds it live until it has.
///
/// # Panics
///
/// On an entity that carries no [`StationMarker`], no [`StationServices`],
/// no [`StationBuilds`] or no [`StationStorage`]: not a station root this
/// module spawned.
pub fn freeze_station(world: &World, station: Entity) -> Result<FrozenStation, UnsettledBody> {
    let root = world.entity(station);
    if !root.contains::<StationMarker>() {
        panic!("freeze_station: {station} is not a station root");
    }
    let Some(post) = root.get::<StationPost>() else {
        return Err(UnsettledBody {
            reason: "the station post is not derived yet",
        });
    };
    let Some(services) = root.get::<StationServices>() else {
        panic!("freeze_station: station {station} has no services");
    };
    let Some(builds) = root.get::<StationBuilds>() else {
        panic!("freeze_station: station {station} has no builds");
    };
    let Some(storage) = root.get::<StationStorage>() else {
        panic!("freeze_station: station {station} has no storage");
    };
    Ok(FrozenStation {
        services: services.0.clone(),
        builds: builds.0.clone(),
        ship: freeze_ship(world, station)?,
        storage: storage.clone(),
        parked: freeze_parked_hulls(world, station)?,
        owned: root.contains::<OwnedStationMarker>(),
        post: *post,
    })
}

/// Rebuild a frozen station on `entity`, which the caller spawned with its
/// pose, identity and parent. The hull thaws through [`thaw_ship`]; the
/// marker, the services, the storage, the post, the parked hulls to park
/// again ([`PendingParkedHulls`]) and, for an owned station,
/// [`OwnedStationMarker`] land beside it. The caller inserts the
/// velocity after this.
pub fn thaw_station(entity: &mut EntityCommands, frozen: FrozenStation) {
    let FrozenStation {
        services,
        builds,
        ship,
        storage,
        parked,
        owned,
        post,
    } = frozen;
    let living = !ship.is_derelict();
    thaw_ship(entity, ship);
    entity.insert((
        StationMarker,
        StationServices(services),
        StationBuilds(builds),
        storage,
        post,
    ));
    entity.insert(station_type_name(living));
    if !parked.is_empty() {
        entity.insert(PendingParkedHulls(
            parked
                .into_iter()
                .map(|hull| PendingParkedHull::Frozen(Box::new(hull)))
                .collect(),
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

    /// The post follows the authored motion: the generator's circular
    /// velocity in a well is an orbit of that well on that radius and plane,
    /// rest in flat space is a fixed point, and anything else is refused.
    #[test]
    fn a_station_post_is_an_orbit_in_a_well_a_fixed_point_at_rest_and_refused_otherwise() {
        let settings = GravitySettings::default();
        let well = GravityWell::from_mass(4000.0, 21.1, &settings);
        let centre = Vec3::new(10.0, -5.0, 2.0);
        let wells = [(centre, well.clone())];
        let station = centre + Vec3::new(45.0, 0.0, 0.0);
        let velocity = station_orbit_velocity(4000.0, centre, station)
            .expect("an orbit exists off the centre")
            .to_engine();

        let held = station_well(station, &wells, &settings);
        assert_eq!(held, Some((centre, 4000.0)), "the planet holds the station");
        let post = derive_station_post(station, velocity, held).expect("circular is a post");
        let StationPost::Orbit {
            centre: at,
            radius,
            normal,
        } = post
        else {
            panic!("a station in a well orbits, got {post:?}");
        };
        assert_eq!(at, centre);
        assert!((radius - 45.0).abs() < 1e-3, "radius {radius}");
        assert!(
            normal.dot((station - centre).cross(velocity).normalize()) > 0.999,
            "the plane is r x v: {normal:?}"
        );

        let far = centre + Vec3::X * (well.soi_radius * 2.0);
        assert_eq!(
            station_well(far, &wells, &settings),
            None,
            "outside the reach no well holds it"
        );
        assert_eq!(
            derive_station_post(far, Vec3::ZERO, None),
            Ok(StationPost::Fixed { position: far }),
            "at rest in flat space the post is the point"
        );
        assert!(
            derive_station_post(far, Vec3::X, None).is_err(),
            "a velocity out of a well has no post"
        );
        assert!(
            derive_station_post(station, velocity * 1.1, held).is_err(),
            "a tenth over circular has no post"
        );
        assert!(
            derive_station_post(station, Vec3::X * velocity.length(), held).is_err(),
            "a radial velocity at circular speed has no post"
        );
        assert!(
            derive_station_post(station, Vec3::ZERO, held).is_err(),
            "rest in a well has no post: the station would fall"
        );
    }
}
