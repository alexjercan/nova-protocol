//! Nav beacon components (task 20260712-093044, spike
//! docs/spikes/20260712-092926-starter-scenario.md): the marker and label a
//! scenario-spawned beacon carries. They live in nova_gameplay - not with
//! the beacon's scenario object in nova_scenario - because the HUD chip
//! module (hud/beacon_chips.rs) queries them and the crate dependency runs
//! nova_scenario -> nova_gameplay, the same split as SpaceshipRootMarker.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::beacon::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{BeaconLabel, BeaconMarker};
}

/// Marks a nav beacon root: a player-facing waypoint. Spawning one grows a
/// HUD chip (label + distance, edge-clamped as a direction cue) via the
/// beacon-chips observer.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BeaconMarker;

/// The short name the beacon's HUD chip shows ("BEACON 1").
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct BeaconLabel(pub String);

impl BeaconLabel {
    /// Construct from a string slice.
    pub fn new(label: &str) -> Self {
        Self(label.to_string())
    }
}
