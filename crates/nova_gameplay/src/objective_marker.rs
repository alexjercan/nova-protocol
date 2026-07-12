//! Objective conveyance components (task 20260712-093831, spike
//! docs/spikes/20260712-140842-objective-conveyance-visuals.md): the marker
//! and highlight tags the scenario side attaches to world entities. They
//! live in nova_gameplay - not in nova_scenario with the actions that
//! insert them - because the HUD chip modules (hud/objective_markers.rs,
//! hud/item_highlights.rs) query them and the crate dependency runs
//! nova_scenario -> nova_gameplay, the same split as BeaconMarker.

use bevy::prelude::*;

pub mod prelude {
    pub use super::{ItemHighlight, ObjectiveMarkerTarget};
}

/// Marks an entity as the current objective: attaching this grows a gold
/// HUD marker chip (label + distance, edge-clamped as a direction cue) via
/// the objective-markers observer; removing it (or despawning the entity)
/// takes the chip down. Named `ObjectiveMarkerTarget`, not
/// `ObjectiveMarker` - bevy_common_systems already uses that name for the
/// objectives panel's text lines.
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
    /// bracket to the trigger volume (review R1.1).
    pub world_radius: f32,
}

impl ItemHighlight {
    /// Construct from the visible bounding radius.
    pub fn new(world_radius: f32) -> Self {
        Self { world_radius }
    }
}
