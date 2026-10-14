//! The ship-structure vocabulary: what a ship root, a section and a fired
//! projectile ARE, with none of the behavior that builds or flies them.
//!
//! These are the markers that both sides of the ship seam read. The ship
//! crate spawns them; `integrity` and `gravity` here classify entities by them
//! (is this a ship root? a live section? a torpedo? a station?) without
//! depending on the sections that define one. They are plain components with no
//! systems and minimal requirements, which lets them sit below the seam instead
//! of straddling it. A ship root declares the generic integrity role and the
//! inventory every ship requires; a station root names the design it was built
//! from, which is the one datum the marker carries.
//!
//! The AI counterparts are deliberately NOT here: `AISpaceshipMarker` requires
//! the AI behavior state, and `AINonCombatant` is an AI directive, so both live
//! with the AI that owns them and the edges into this layer are inverted
//! instead (see `gravity`'s opt-in observers and `integrity::neutralize`).

use bevy::prelude::*;

use crate::{
    integrity::prelude::{DamageMarks, IntegrityRoot},
    inventory::prelude::{ShipCredits, ShipInventory, ShipInventoryReports},
    relations::prelude::Allegiance,
};

/// The whole module - every marker is part of the shared vocabulary.
pub mod prelude {
    pub use super::{
        station_offers, ControllerSectionMarker, DerelictShipMarker, GunRoundMarker,
        OwnedStationMarker, PlayerFleetMarker, PlayerSpaceshipMarker, RailgunSectionMarker,
        RailgunSlugProjectileMarker, SectionInactiveMarker, SectionMarker, SpaceshipRootMarker,
        StationDesignId, StationMarker, StationServiceType, StationServices, ThrusterSectionMarker,
        TorpedoProjectileMarker, TorpedoSectionMarker, TurretBulletProjectileMarker,
        TurretSectionMarker,
    };
}

/// The root component for an entire spaceship. Every section is a child of the
/// entity carrying this.
///
/// Records WHERE it was hit by requirement. A ship's hits belong to the ship
/// and not to whatever stopped them, because a crater has to cross the seams
/// between the plates and sections it reaches - and every ship has SOMETHING
/// that reads the list, whether or not it is wearing cladding. The skin builder
/// used to insert this, which meant an unclad ship recorded nothing and its
/// hull sections could not carve.
///
/// Every ship carries a [`ShipInventory`], empty until something fills it,
/// and [`ShipCredits`], zero until something pays it, and the
/// [`ShipInventoryReports`] queue those two report their changes through.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[require(
    IntegrityRoot,
    DamageMarks,
    ShipInventory,
    ShipCredits,
    ShipInventoryReports
)]
pub struct SpaceshipRootMarker;

/// Marks the player's spaceship root.
///
/// Carries [`Allegiance::Player`] by requirement, so every player-marked root
/// participates in the relation model without extra spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Player)]
pub struct PlayerSpaceshipMarker;

/// Marks a ship root in the player's fleet: a hired escort that follows the
/// player ship from sector to sector.
///
/// The streamed world never adopts or freezes a fleet escort, as it never
/// adopts the player ship: it travels with the player, and the world save
/// keeps it in the player's record, not in a sector's. Removed when the
/// escort is neutralized; the wreck is then a sector's body like any other.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Player)]
pub struct PlayerFleetMarker;

/// Marks a derelict ship root: every section but its hull and docking ports
/// carries [`SectionInactiveMarker`].
///
/// Inserted in the root's spawn bundle, so the section spawn sees it, or on
/// a living station the instant it is neutralized, which then switches its
/// live sections off the same way. A spawned derelict was never in a fight,
/// so neutralization never reads its inactive weapons or flight computers as
/// a defeat: it keeps its allegiance's marker and fires no defeat event.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct DerelictShipMarker;

/// The id a station design is referenced by, from a scenario spawn or another
/// mod.
///
/// A `DesignId`: an authored catalog identifier, resolved against the station
/// catalog the scenario crate owns. Serialized as the bare id string, so
/// authored RON writes `design: "block_waystation"`. An id no loaded design
/// carries is refused at lint and at load; nothing resolves it to another
/// design.
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

/// Marks a station root, carrying the design it was built from.
///
/// Sits beside [`SpaceshipRootMarker`] on the same root: a station is built
/// as a ship (sections, dock, hold, freeze) and this is what tells the two
/// apart. A station takes damage, collapses and is destroyed as a ship is.
/// What the marker changes: a living station that is neutralized turns into
/// a dead (derelict) station rather than a drifting wreck, and a sever
/// anchors on it so a cut never moves it off its orbit.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
pub struct StationMarker(pub StationDesignId);

/// Marks a station root the player owns: bought from its civilization with
/// the Ship pane's station buy.
///
/// The station's frozen record carries it (`FrozenStation`'s `owned`), so
/// ownership crosses a retire, a thaw and a Load with the body; nothing else
/// keeps a copy. What it changes: docking admission skips the side rule, so
/// the station refuses no hull (only its one held connection), and a Buy
/// from its hold pays bid, not ask. The station keeps its civilization's side
/// and its AI; a dead owned station keeps the marker and is a derelict.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct OwnedStationMarker;

/// What a docked ship can do at a station. The closed set the game knows; a
/// station design lists the ones it offers.
///
/// Here rather than beside the station design so the interface can read a
/// docked station's [`StationServices`] without the scenario crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StationServiceType {
    /// Rebuild destroyed sections of the docked ship.
    Rebuild,
    /// Swap a docked ship's section for another part of the same kind and
    /// footprint, or add a part at a free socket, paid in parts and credits
    /// by the part's build recipe.
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

/// The services a station root's design offers, inserted beside its
/// [`StationMarker`] whenever one lands, by spawn or thaw, from the loaded
/// station catalog. A derelict station keeps the list; a service reader
/// asks [`station_offers`], which checks [`DerelictShipMarker`] too.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub struct StationServices(pub Vec<StationServiceType>);

impl StationServices {
    /// Whether the design offers `service`.
    pub fn offers(&self, service: StationServiceType) -> bool {
        self.0.contains(&service)
    }
}

/// Whether a docked partner offers `service` now: it is a living station
/// whose design lists it. `services` is the partner's [`StationServices`],
/// `None` for a partner that is not a station, and `derelict` is whether it
/// carries [`DerelictShipMarker`].
///
/// The one gate every station service reads (Rebuild, Trade, Storage), so a
/// dead station offers none: a docked ship only loots it.
pub fn station_offers(
    services: Option<&StationServices>,
    derelict: bool,
    service: StationServiceType,
) -> bool {
    !derelict && services.is_some_and(|services| services.offers(service))
}

/// Marks a live section entity in a ship tree. Present on every spawned section
/// (added by `base_section`); its absence marks the editor preview
/// (`preview_section` omits it, carrying [`SectionInactiveMarker`] instead).
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionMarker;

/// Marks a section that is present in the world but not simulated - the editor
/// preview / palette section. Added by `preview_section` in place of
/// [`SectionMarker`] so gameplay systems skip it.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SectionInactiveMarker;

/// Marker component for turret sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TurretSectionMarker;

/// Marker for a spinal railgun section - the gun the HULL aims, as opposed to
/// the traversing [`TurretSectionMarker`].
#[derive(Component, Clone, Debug, Reflect)]
pub struct RailgunSectionMarker;

/// Marker component for thruster sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ThrusterSectionMarker;

/// Marker for a torpedo launch bay section entity; the fire system reads its
/// input and launches projectiles from its child spawner.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoSectionMarker;

/// Marker component for controller sections.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControllerSectionMarker;

/// Marker for a fired torpedo projectile (the guided flying entity spawned at
/// launch), as opposed to the stationary bay.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TorpedoProjectileMarker;

/// Marks anything the round sweep integrates: a straight-line-plus-gravity
/// path that is not a rigid body and interacts only with what it hits.
///
/// The vocabulary word `rounds::advance_rounds` queries on, so a second gun
/// does not have to wear the turret's marker to be swept. It carries none of
/// the turret bullet's render or audio wiring, which is exactly why the
/// railgun slug takes this and not [`TurretBulletProjectileMarker`].
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct GunRoundMarker;

/// Marker for turret bullet projectiles.
#[derive(Component, Clone, Debug, Reflect)]
#[require(GunRoundMarker)]
pub struct TurretBulletProjectileMarker;

/// Marker for a fired railgun slug - one shell in the air per gun, and the
/// only round in the game that outlives what it hits by design.
#[derive(Component, Clone, Debug, Reflect)]
#[require(GunRoundMarker)]
pub struct RailgunSlugProjectileMarker;
