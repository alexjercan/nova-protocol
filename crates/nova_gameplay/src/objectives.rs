//! Mission objectives: the [`GameObjectives`] list the game reasons about, and
//! the conveyance tag ([`ObjectiveMarkerTarget`]) the scenario side attaches to
//! world entities.
//!
//! Nova owns this because objectives are mission state, not a widget: the
//! scenario loader writes [`GameObjectives`], and the HUD reads it from three
//! places (the objective stack, the NOVA OS monitor and the objective-change
//! feedback) - this module renders nothing itself. The conveyance tag lives
//! here - not in nova_scenario with the action that inserts it - because the
//! HUD chip module (`hud/objective_markers.rs`) queries it and the crate
//! dependency runs nova_scenario -> nova_gameplay, the same split as
//! `BeaconMarker`.

use bevy::prelude::*;

/// `GameObjectives`, `Objective` and `ObjectiveMarkerTarget`.
pub mod prelude {
    pub use super::{GameObjectives, Objective, ObjectiveMarkerTarget};
}

/// A single objective line: an opaque `id` for game code to address, and the `message` shown.
#[derive(Clone, Debug)]
pub struct Objective {
    /// Opaque identifier for game code (not shown).
    pub id: String,
    /// The text shown for this objective.
    pub message: String,
}

impl Objective {
    /// Convenience constructor from string slices.
    pub fn new(id: &str, message: &str) -> Self {
        Self {
            id: id.to_string(),
            message: message.to_string(),
        }
    }
}

/// The current objectives. Replace the `objectives` vec to change what the panel shows.
#[derive(Resource, Clone, Debug, Default)]
pub struct GameObjectives {
    /// The objectives, rendered top to bottom.
    pub objectives: Vec<Objective>,
}

/// Marks an entity as the current objective: attaching this grows a gold
/// HUD marker chip (label + distance, edge-clamped as a direction cue) via
/// the objective-markers observer; removing it (or despawning the entity)
/// takes the chip down.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ObjectiveMarkerTarget {
    /// The short name the marker chip shows next to the distance.
    pub label: String,
}

impl ObjectiveMarkerTarget {
    /// Construct from a string slice.
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}
