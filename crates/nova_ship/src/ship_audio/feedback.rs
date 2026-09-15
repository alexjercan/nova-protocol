//! The ship's own voice: the feedback sounds and alarm thresholds a hull
//! carries on its ROOT.
//!
//! These used to live on the controller section, on the theory that the
//! computer is what talks to the pilot. A controller is attitude hardware, and
//! a hull that loses one does not stop being able to warn about its own
//! structure - so the whole set is the SHIP's, snapshotted unresolved from the
//! design's presentation config at spawn and resolved per cue.

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;

/// The ship's authored feedback sounds and its hull alarm threshold.
pub mod prelude {
    pub use super::{ShipFeedbackSounds, ShipHullWarning, DEFAULT_WARN_HULL_FRACTION};
}

/// The hull fraction a ship warns at when nothing is authored.
///
/// Thirty percent of the built hull: late enough that an ordinary skirmish
/// does not trip it, and six times the structural-collapse floor, so the alarm
/// still leaves a pilot room to break off.
pub const DEFAULT_WARN_HULL_FRACTION: f32 = 0.30;

/// The ship's authored feedback sounds, snapshotted UNRESOLVED from the
/// design's presentation config onto the ship ROOT (one component for the
/// whole set - they share the same consumers). The audio module reads the
/// PLAYER ship's own component and resolves per cue.
///
/// AUTHORED-OR-SILENT throughout: a `None` plays nothing.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ShipFeedbackSounds {
    /// Lock acquired (once per radar gesture).
    #[reflect(ignore)]
    pub lock_on: Option<AssetRef<AudioSource>>,
    /// Lock cleared (tap-clear).
    #[reflect(ignore)]
    pub lock_off: Option<AssetRef<AudioSource>>,
    /// Radar hold denied (the ship has no LOCK capability).
    #[reflect(ignore)]
    pub radar_deny: Option<AssetRef<AudioSource>>,
    /// Held radar gesture re-designated to a new target.
    #[reflect(ignore)]
    pub radar_retarget: Option<AssetRef<AudioSource>>,
    /// Weapons safety re-engaged (hot -> cold edge).
    #[reflect(ignore)]
    pub safety_on: Option<AssetRef<AudioSource>>,
    /// A hostile has this ship in its combat lock - the threat alarm, on the
    /// rising edge of "somebody is aiming at me".
    #[reflect(ignore)]
    pub warn_lock: Option<AssetRef<AudioSource>>,
    /// A magazine ran dry - the GAUGE, inside the cockpit, as distinct from
    /// the gun's own dead-trigger click out on the mount. The two fire on the
    /// same edge and are meant to be heard as one event from two places; the
    /// gun's is per-turret, this one is per-SHIP.
    #[reflect(ignore)]
    pub ammo_dry: Option<AssetRef<AudioSource>>,
    /// The hull is critical - ONE alarm, on the falling edge through
    /// [`ShipHullWarning`].
    #[reflect(ignore)]
    pub warn_hull: Option<AssetRef<AudioSource>>,
    /// RCS fine-adjust LOOP: plays continuously while the ship is burning the
    /// RCS primitive. Unlike the one-shots above this is sustained, resolved
    /// and volume-tracked by the audio module (one loop per distinct handle),
    /// exactly like a thruster's `loop_sound`.
    #[reflect(ignore)]
    pub rcs_loop: Option<AssetRef<AudioSource>>,
}

/// The hull fraction this ship warns at, snapshotted from the design's
/// presentation config and clamped to `0..=1`.
///
/// Apart from [`ShipFeedbackSounds`] because it is a threshold, not a sound,
/// and the two are read by the same cue only by coincidence of this being the
/// first integrity instrument the ship has.
///
/// A SHIP decision, and the reason it is authored at all: a cheap civilian
/// build may warn late, or (at `0.0`) never warn until there is nothing left.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct ShipHullWarning(pub f32);

impl Default for ShipHullWarning {
    fn default() -> Self {
        Self(DEFAULT_WARN_HULL_FRACTION)
    }
}
