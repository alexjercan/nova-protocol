//! The ship-structure vocabulary: what a ship root, a section and a fired
//! projectile ARE, with none of the behavior that builds or flies them.
//!
//! These are the fifteen markers that both sides of the ship seam read. The ship
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
        ControllerSectionMarker, DerelictShipMarker, GunRoundMarker, PlayerSpaceshipMarker,
        RailgunSectionMarker, RailgunSlugProjectileMarker, SectionInactiveMarker, SectionMarker,
        SpaceshipRootMarker, StationDesignId, StationMarker, ThrusterSectionMarker,
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

/// Marks a ship root that was spawned as a derelict: every section but its
/// hull and docking ports spawns carrying [`SectionInactiveMarker`].
///
/// Inserted in the root's spawn bundle, so the section spawn sees it. A
/// derelict was never in a fight, so neutralization never reads its inactive
/// weapons or flight computers as a defeat: it keeps its allegiance's marker
/// and fires no defeat event.
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
/// apart. The health store reads it to refuse every hit under it, see
/// `integrity::health::on_damage`: a station's sections never lose health,
/// so it is never neutralized and never destroyed. That holds for a living
/// and a dead (derelict) station alike. Station integrity - a living station
/// that can be fought into a derelict - is a later rule, not a gap here.
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
pub struct StationMarker(pub StationDesignId);

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
