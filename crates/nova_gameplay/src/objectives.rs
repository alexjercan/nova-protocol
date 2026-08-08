//! Mission objectives: the [`GameObjectives`] list the game reasons about, and
//! the conveyance tags ([`ObjectiveMarkerTarget`], [`ItemHighlight`]) the
//! scenario side attaches to world entities.
//!
//! Nova owns this because objectives are mission state, not a widget: the
//! scenario loader writes [`GameObjectives`], and the HUD reads it from three
//! places (the objective stack, the NOVA OS monitor and the objective-change
//! feedback) - this module renders nothing itself. The conveyance tags live
//! here - not in nova_scenario with the actions that insert them - because the
//! HUD chip modules (`hud/objective_markers.rs`, `hud/item_highlights.rs`)
//! query them and the crate dependency runs nova_scenario -> nova_gameplay, the
//! same split as `BeaconMarker`.

use bevy::prelude::*;

/// `GameObjectives`, `Objective`, `ItemHighlight` and `ObjectiveMarkerTarget`.
pub mod prelude {
    pub use super::{GameObjectives, ItemHighlight, Objective, ObjectiveMarkerTarget};
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

/// Marks an interactable/collectible prop the player is meant to notice:
/// the item-highlights observer grows a bracket chip over it that tracks
/// the prop's on-screen size (hidden off-screen - pointing at off-screen
/// items is the objective marker's job). Spawned intrinsically by pickup
/// objects (salvage crates); a pickup that does not advertise itself is a
/// bug, not a policy.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ItemHighlight {
    /// The prop's VISIBLE bounding-sphere radius (world units) - what the
    /// bracket sizes to. Authored, not collider-derived: a pickup's only
    /// collider is its oversized sensor sphere, which would balloon the
    /// bracket to the trigger volume.
    pub world_radius: f32,
}

impl ItemHighlight {
    /// Construct from the visible bounding radius.
    pub fn new(world_radius: f32) -> Self {
        Self { world_radius }
    }
}
