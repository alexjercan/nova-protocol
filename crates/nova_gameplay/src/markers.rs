//! The ship-structure vocabulary: what a ship root, a section and a fired
//! projectile ARE, with none of the behavior that builds or flies them.
//!
//! These are the ten markers that both sides of the ship seam read. The ship
//! crate spawns them; `integrity` and `gravity` here classify entities by them
//! (is this a ship root? a live section? a torpedo?) without depending on the
//! sections that define one. They are plain unit components with no systems and
//! minimal requirements, which lets them sit below the seam instead of
//! straddling it. A ship root declares the generic integrity role every ship
//! requires.
//!
//! The AI counterparts are deliberately NOT here: `AISpaceshipMarker` requires
//! the AI behavior state, and `AINonCombatant` is an AI directive, so both live
//! with the AI that owns them and the edges into this layer are inverted
//! instead (see `gravity`'s opt-in observers and `integrity::neutralize`).

use bevy::prelude::*;

use crate::{
    integrity::prelude::{DamageMarks, IntegrityRoot},
    relations::prelude::Allegiance,
};

/// The whole module - every marker is part of the shared vocabulary.
pub mod prelude {
    pub use super::{
        ControllerSectionMarker, PlayerSpaceshipMarker, SectionInactiveMarker, SectionMarker,
        SpaceshipRootMarker, ThrusterSectionMarker, TorpedoProjectileMarker, TorpedoSectionMarker,
        TurretBulletProjectileMarker, TurretSectionMarker,
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
#[derive(Component, Clone, Debug, Default, Reflect)]
#[require(IntegrityRoot, DamageMarks)]
pub struct SpaceshipRootMarker;

/// Marks the player's spaceship root.
///
/// Carries [`Allegiance::Player`] by requirement, so every player-marked root
/// participates in the relation model without extra spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Player)]
pub struct PlayerSpaceshipMarker;

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

/// Marker for turret bullet projectiles.
#[derive(Component, Clone, Debug, Reflect)]
pub struct TurretBulletProjectileMarker;
