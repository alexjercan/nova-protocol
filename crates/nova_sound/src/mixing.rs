//! Shared mixing pieces: the listener camera's marker, the audible threshold,
//! and the world-cell grouping and burst budget that the juice layer shares
//! with the audio layer's own grouping.

use std::{collections::HashMap, hash::Hash};

use bevy::prelude::*;

/// Below this final (faded) linear level a one-shot is not worth an audio sink
/// - it would be inaudible - and a loop holds its sink paused.
pub const SFX_AUDIBLE_THRESHOLD: f32 = 0.01;

/// World-cell size (units) for grouping co-located area cues that have no body
/// over them - a hit on open space, a cue from a despawned emitter.
///
/// A cue that CAN name a body is grouped by the body instead (see
/// [`CueGroup`]): six units is a gunship, so a cell groups a skiff's whole
/// death and a carrier's into two dozen separate ones, which is the same
/// mistake in both directions.
pub const SFX_AREA_CELL: f32 = 6.0;

/// What a body-bound cue is grouped with.
///
/// One physical structure is one event. A hit and a death happen TO something -
/// a ship, a wreck fragment, a rock, a torpedo - and that thing is what a
/// listener hears, so a capital shedding two hundred sections is one collapse
/// however far apart the sections are, and two ships dying in the same place
/// are two deaths however close they are. A world cell can say neither.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CueGroup {
    /// The physical body the event happened on: the nearest rigid body at or
    /// above the entity (`nova_gameplay`'s `cue_body` finds it).
    Body(Entity),
    /// Where it happened, for an event with nothing physical over it.
    Cell(IVec3),
}

/// Quantize a world position to a [`SFX_AREA_CELL`]-sized integer cell, so
/// nearby events share a key and far ones do not.
pub fn area_cell(pos: Vec3) -> IVec3 {
    (pos / SFX_AREA_CELL).floor().as_ivec3()
}

/// What a cue at `at` on `body` groups with: the body when there is one, and
/// the world cell when there is not.
pub fn cue_group(body: Option<Entity>, at: Vec3) -> CueGroup {
    body.map_or_else(|| CueGroup::Cell(area_cell(at)), CueGroup::Body)
}

/// Last-fired timestamp per throttle key, in seconds since startup. A key that
/// is absent has never fired, so its first event always passes.
///
/// The juice layer's camera-kick budget runs on this, grouped by [`CueGroup`],
/// so a frame that is one body's bang is one kick. The sound engine's own
/// repeat interval is [`SoundRepeat`](super::SoundRepeat), keyed by sound and
/// `ChildOf` root.
#[derive(Resource)]
pub struct CueThrottle<K: Copy + Eq + Hash + Send + Sync + 'static> {
    last: HashMap<K, f32>,
}

impl<K: Copy + Eq + Hash + Send + Sync + 'static> Default for CueThrottle<K> {
    fn default() -> Self {
        Self {
            last: HashMap::new(),
        }
    }
}

impl<K: Copy + Eq + Hash + Send + Sync + 'static> CueThrottle<K> {
    /// If `key` has not fired within `min_interval` seconds, stamp it `now`
    /// and return true; otherwise false. Each key throttles independently.
    pub fn allow(&mut self, key: K, now: f32, min_interval: f32) -> bool {
        let last = self.last.entry(key).or_insert(f32::NEG_INFINITY);
        if now - *last >= min_interval {
            *last = now;
            true
        } else {
            false
        }
    }

    /// The keys currently being throttled - what has fired recently and has
    /// not yet been pruned. Read-only; `allow` is the only way in.
    pub fn tracked_keys(&self) -> impl ExactSizeIterator<Item = K> + '_ {
        self.last.keys().copied()
    }

    /// Drop keys idle for longer than `window` seconds so the map stays
    /// bounded.
    pub fn prune(&mut self, now: f32, window: f32) {
        self.last.retain(|_, &mut last| now - last < window);
    }
}

/// Marks the camera that acts as the SFX/juice listener: distance attenuation
/// for the one-shot cues, camera-shake trauma, and the flash-ring facing all
/// key off this entity. Exactly one camera should carry it at a time - the
/// gameplay (scenario) camera, tagged where it is spawned. "First `Camera3d`"
/// was the old signal, but ECS query order is unspecified, so a second camera
/// (minimap, render-to-texture, a leftover editor camera) could flip the
/// listener frame to frame; the explicit marker makes it stable. The editor
/// camera deliberately does not carry it: no gameplay cues fire in the editor,
/// and the shake component should never attach there.
///
/// (Checked at introduction time: the editor -> scenario transition never has
/// two `Camera3d` alive at once - the editor camera is `DespawnOnExit(Editor)`,
/// applied before `OnEnter(Scenario)` spawns the scenario camera - so the old
/// assumption was latent, not a live bug.)
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SfxListenerMarker;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Key {
        Impact(CueGroup),
        Destroy(CueGroup),
    }

    #[test]
    fn throttle_blocks_one_key_until_the_interval_elapses() {
        let key = Key::Destroy(CueGroup::Cell(IVec3::ZERO));
        let mut state = CueThrottle::<Key>::default();
        // First event of a key always fires (absent -> NEG_INFINITY).
        assert!(state.allow(key, 0.0, 0.05));
        // Too soon: blocked.
        assert!(!state.allow(key, 0.03, 0.05));
        // Exactly at the interval: fires again.
        assert!(state.allow(key, 0.05, 0.05));
        // Immediately after: blocked.
        assert!(!state.allow(key, 0.06, 0.05));
    }

    #[test]
    fn throttle_is_independent_per_key() {
        let mut state = CueThrottle::<Key>::default();
        assert!(state.allow(Key::Impact(CueGroup::Cell(IVec3::ZERO)), 0.0, 0.04));
        assert!(state.allow(Key::Impact(CueGroup::Cell(IVec3::ONE)), 0.0, 0.04));
        assert!(state.allow(Key::Destroy(CueGroup::Cell(IVec3::ZERO)), 0.0, 0.06));
    }

    #[test]
    fn prune_drops_only_idle_keys() {
        let mut state = CueThrottle::<Key>::default();
        state.allow(Key::Impact(CueGroup::Cell(IVec3::ZERO)), 0.0, 0.04); // last = 0.0
        state.allow(Key::Impact(CueGroup::Cell(IVec3::ONE)), 9.5, 0.04); // last = 9.5
        state.prune(10.0, 2.0); // window 2s at now=10 -> keep >8.0
        assert!(state
            .tracked_keys()
            .eq([Key::Impact(CueGroup::Cell(IVec3::ONE))]));
    }

    #[test]
    fn area_cell_groups_nearby_and_separates_distant() {
        // Points within one cell share a key; points cells apart do not.
        assert_eq!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(SFX_AREA_CELL * 0.5))
        );
        assert_ne!(
            area_cell(Vec3::ZERO),
            area_cell(Vec3::splat(SFX_AREA_CELL * 1.5))
        );
    }
}
