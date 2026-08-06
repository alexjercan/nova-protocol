//! The mixing layer every cue goes through: where the listener is, how
//! far a sound falls off, how hard a throttle reads, and the per-source
//! throttle that keeps a burst of events from stacking into mud.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::prelude::*;

/// Distance-attenuation rolloff for positional cues, in world units. A cue
/// plays at full base volume within `SFX_NEAR_DISTANCE`, is inaudible beyond
/// `SFX_FAR_DISTANCE`, and rolls off between (see [`distance_attenuation`]).
/// Tune by ear for the scene scale (Nova ships are a few units across; combat
/// happens over dozens).
pub(super) const SFX_NEAR_DISTANCE: f32 = 20.0;
pub(super) const SFX_FAR_DISTANCE: f32 = 320.0;

/// Shape of the distance rolloff between NEAR and FAR. Loudness perception is
/// logarithmic, so a linear *amplitude* ramp sounds flat for most of the range
/// and then cliffs to silence near the end. Decaying the amplitude
/// geometrically toward this floor instead gives a roughly constant
/// dB-per-distance falloff, so the *perceived* volume fades evenly. Smaller
/// floor = steeper decay / more perceived range; 0.05 is about -26 dB at the
/// far end (before the final remap to true zero).
const SFX_ROLLOFF_FLOOR: f32 = 0.05;

/// Below this final (attenuated) linear volume a one-shot is not worth
/// spawning - it would be inaudible. Skipping it avoids audio-entity churn for
/// far events.
const SFX_AUDIBLE_THRESHOLD: f32 = 0.01;

/// Minimum seconds between successive turret-fire and impact one-shots. Without
/// this the ~100/s PDC and the many-collider blast hits would each spawn a
/// storm of overlapping audio entities that reads as a wall of noise;
/// collapsing them to a bounded rate keeps the cue legible and the entity churn
/// sane.
pub(super) const TURRET_FIRE_MIN_INTERVAL: f32 = 0.05;
pub(super) const IMPACT_MIN_INTERVAL: f32 = 0.04;

/// World-cell size (units) for grouping co-located area cues (impact,
/// explosion). A blast hitting many colliders of one ship, or a ship's sections
/// all destroyed at once, fall in the same cell and collapse to a single sound;
/// events far enough apart get their own. Small enough to keep distinct
/// ships/impacts separate. Turret fire is keyed by entity instead, so it does
/// not use this.
pub(super) const SFX_AREA_CELL: f32 = 6.0;

/// Drop throttle keys not touched within this many seconds, so the per-source
/// map stays bounded as ships move through new cells and turrets come and go.
const SFX_THROTTLE_PRUNE_WINDOW: f32 = 2.0;
/// A dying multi-section ship marks every section destroyed in the same frame;
/// this collapses that burst into a single explosion instead of N overlapping
/// ones (which would clip). Short enough that genuinely separate kills >60ms
/// apart still each sound.
pub(super) const EXPLOSION_MIN_INTERVAL: f32 = 0.06;

/// Loudest the engine hum ever gets (at full thrust), on the linear scale.
pub(super) const ENGINE_MAX_VOLUME: f32 = 0.3;

/// Loudest the RCS fine-adjust loop ever gets (at full-deflection intent).
/// Below [`ENGINE_MAX_VOLUME`]: RCS is a gentle nudge, not the main drive.
pub(super) const RCS_MAX_VOLUME: f32 = 0.22;

/// Per-source throttle key. Turret fire is keyed by the firing turret entity so
/// each gun sounds independently (even two guns on one ship); the area cues are
/// keyed by a quantized world cell so a co-located burst collapses to one sound
/// while distinct locations each sound. Keying globally (one timestamp per cue)
/// was the bug where a second gun firing in the same window was silenced.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum ThrottleKey {
    TurretFire(Entity),
    Impact(IVec3),
    Explosion(IVec3),
}

/// Quantize a world position to a [`SFX_AREA_CELL`]-sized integer cell, so
/// nearby events share a key and far ones do not.
pub(super) fn area_cell(pos: Vec3) -> IVec3 {
    (pos / SFX_AREA_CELL).floor().as_ivec3()
}

/// Last-played timestamp per throttle key, in seconds since startup. A key that
/// is absent has never played, so its first event always fires.
#[derive(Resource, Default)]
pub(super) struct SfxThrottle {
    pub(super) last: HashMap<ThrottleKey, f32>,
}

impl SfxThrottle {
    /// If `key` has not sounded within `min_interval` seconds, stamp it `now`
    /// and return true; otherwise false. Each key throttles independently.
    pub(super) fn allow(&mut self, key: ThrottleKey, now: f32, min_interval: f32) -> bool {
        let last = self.last.entry(key).or_insert(f32::NEG_INFINITY);
        if now - *last >= min_interval {
            *last = now;
            true
        } else {
            false
        }
    }

    /// Drop keys idle for longer than `window` seconds so the map stays
    /// bounded.
    pub(super) fn prune(&mut self, now: f32, window: f32) {
        self.last.retain(|_, &mut last| now - last < window);
    }
}

/// Map an average per-thruster throttle (0..1) to a linear engine-hum volume:
/// silent at rest, scaling linearly to [`ENGINE_MAX_VOLUME`] at full throttle.
/// The caller averages over each ship's active thrusters rather than summing,
/// so the hum tracks how hard that ship is burning instead of pinning to max
/// the moment more than one thruster fires. The clamp guards out-of-range
/// input. Pure for tests.
pub(super) fn engine_volume(avg_throttle: f32) -> f32 {
    avg_throttle.clamp(0.0, 1.0) * ENGINE_MAX_VOLUME
}

/// RCS fine-adjust loop volume from the ship's `RcsIntent` magnitude (the burn
/// effort, ~0..1 per axis; a diagonal command can exceed 1, hence the clamp).
/// A touch quieter than the main-drive hum ([`RCS_MAX_VOLUME`] < ENGINE_MAX):
/// RCS is a gentle station-keeping push, not a burn. Pure for tests.
pub(super) fn rcs_volume(effort: f32) -> f32 {
    effort.clamp(0.0, 1.0) * RCS_MAX_VOLUME
}

/// Distance rolloff in [0, 1]: full within [`SFX_NEAR_DISTANCE`], zero beyond
/// [`SFX_FAR_DISTANCE`]. Between them the amplitude decays *geometrically*
/// toward [`SFX_ROLLOFF_FLOOR`] (constant dB per distance), not linearly, so
/// the perceived loudness fades evenly instead of staying flat and then
/// cliffing - the fix for "same volume then instantly zero". The geometric
/// curve is remapped from `[floor, 1]` back to `[0, 1]` so it still reaches
/// exactly zero at FAR. Pure for unit testing.
pub(super) fn distance_attenuation(distance: f32) -> f32 {
    if distance <= SFX_NEAR_DISTANCE {
        1.0
    } else if distance >= SFX_FAR_DISTANCE {
        0.0
    } else {
        let t = (distance - SFX_NEAR_DISTANCE) / (SFX_FAR_DISTANCE - SFX_NEAR_DISTANCE);
        let decayed = SFX_ROLLOFF_FLOOR.powf(t);
        (decayed - SFX_ROLLOFF_FLOOR) / (1.0 - SFX_ROLLOFF_FLOOR)
    }
}

/// The handle-taking core of [`play_positional`]: same distance attenuation and
/// audible-threshold gate, but for an already-resolved [`Handle<AudioSource>`]
/// rather than a bank key. Lets a caller play a section's own authored sound (a
/// resolved [`AssetRef<AudioSource>`]) through the exact same positional path
/// the bank cues use.
pub(super) fn play_positional_handle(
    commands: &mut Commands,
    handle: Handle<AudioSource>,
    base_volume: f32,
    source: Vec3,
    listener: Option<Vec3>,
) {
    let attenuation = listener.map_or(1.0, |l| distance_attenuation(l.distance(source)));
    let volume = base_volume * attenuation;
    if volume < SFX_AUDIBLE_THRESHOLD {
        return;
    }
    commands.play_sfx_volume(handle, volume);
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

/// The listener position for distance attenuation: the marked gameplay camera's
/// world translation, or `None` if no listener exists yet (early startup, or
/// the editor).
pub(super) fn listener_position(
    q_camera: &Query<&GlobalTransform, With<SfxListenerMarker>>,
) -> Option<Vec3> {
    q_camera.iter().next().map(|t| t.translation())
}

/// Keep the per-source throttle map bounded by dropping idle keys.
pub(super) fn prune_sfx_throttle(time: Res<Time>, mut throttle_state: ResMut<SfxThrottle>) {
    throttle_state.prune(time.elapsed_secs(), SFX_THROTTLE_PRUNE_WINDOW);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_blocks_one_key_until_the_interval_elapses() {
        let key = ThrottleKey::Explosion(IVec3::ZERO);
        let mut state = SfxThrottle::default();
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
        // The bug fix: two distinct sources firing in the same instant both
        // play.
        let mut world = World::new();
        let gun_a = ThrottleKey::TurretFire(world.spawn_empty().id());
        let gun_b = ThrottleKey::TurretFire(world.spawn_empty().id());
        let mut state = SfxThrottle::default();
        assert!(state.allow(gun_a, 0.0, 0.05));
        assert!(
            state.allow(gun_b, 0.0, 0.05),
            "a second gun must not be silenced"
        );
        // Same gun again in the same window is still throttled.
        assert!(!state.allow(gun_a, 0.0, 0.05));
        // Different cue kinds at the same cell are independent too.
        assert!(state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04));
        assert!(state.allow(ThrottleKey::Explosion(IVec3::ZERO), 0.0, 0.06));
    }

    #[test]
    fn prune_drops_only_idle_keys() {
        let mut state = SfxThrottle::default();
        state.allow(ThrottleKey::Impact(IVec3::ZERO), 0.0, 0.04); // last = 0.0
        state.allow(ThrottleKey::Impact(IVec3::ONE), 9.5, 0.04); // last = 9.5
        state.prune(10.0, 2.0); // window 2s at now=10 -> keep >8.0
        assert_eq!(state.last.len(), 1);
        assert!(state.last.contains_key(&ThrottleKey::Impact(IVec3::ONE)));
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

    #[test]
    fn engine_volume_is_silent_at_rest_and_saturates_at_full_thrust() {
        assert_eq!(engine_volume(0.0), 0.0);
        assert_eq!(engine_volume(1.0), ENGINE_MAX_VOLUME);
        // Multiple thrusters cannot push past the ceiling.
        assert_eq!(engine_volume(3.5), ENGINE_MAX_VOLUME);
        // Partial thrust scales linearly.
        assert!((engine_volume(0.5) - ENGINE_MAX_VOLUME * 0.5).abs() < f32::EPSILON);
        // Negative input (reverse) is treated by magnitude at the call site,
        // but guard the clamp here too.
        assert_eq!(engine_volume(-1.0), 0.0);
    }

    #[test]
    fn distance_attenuation_rolls_off_between_near_and_far() {
        // Full within the near radius (including at exactly near).
        assert_eq!(distance_attenuation(0.0), 1.0);
        assert_eq!(distance_attenuation(SFX_NEAR_DISTANCE), 1.0);
        // Silent at/beyond the far radius (endpoints are clean 1 and 0).
        assert_eq!(distance_attenuation(SFX_FAR_DISTANCE), 0.0);
        assert_eq!(distance_attenuation(SFX_FAR_DISTANCE + 100.0), 0.0);

        // Monotonic decreasing in the rolloff band.
        let mid = (SFX_NEAR_DISTANCE + SFX_FAR_DISTANCE) / 2.0;
        let a = distance_attenuation(SFX_NEAR_DISTANCE + 10.0);
        let m = distance_attenuation(mid);
        let b = distance_attenuation(SFX_FAR_DISTANCE - 10.0);
        assert!(a > m && m > b, "attenuation should decrease with distance");

        // Convex/perceptual: the geometric curve sits *below* the old linear
        // line (which would be 0.5 at the midpoint), so loudness is already
        // clearly reduced by the middle distances instead of staying flat then
        // cliffing.
        assert!(
            m < 0.5,
            "midpoint should be well below the linear 0.5, got {m}"
        );
        // Values stay in range.
        for d in [30.0, 100.0, 200.0, 300.0] {
            let v = distance_attenuation(d);
            assert!(
                (0.0..=1.0).contains(&v),
                "attenuation out of range at {d}: {v}"
            );
        }
    }

    #[test]
    fn listener_position_uses_the_marked_camera_not_any_camera3d() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        // An unmarked Camera3d must not be the listener...
        world.spawn((
            Camera3d::default(),
            GlobalTransform::from(Transform::from_translation(Vec3::new(5.0, 0.0, 0.0))),
        ));
        let mut state: SystemState<Query<&GlobalTransform, With<SfxListenerMarker>>> =
            SystemState::new(&mut world);
        assert_eq!(
            listener_position(&state.get(&world).unwrap()),
            None,
            "no marked listener -> None (graceful full-volume fallback)"
        );

        // ...only the camera carrying the marker is.
        let pos = Vec3::new(0.0, 3.0, -7.0);
        world.spawn((
            Camera3d::default(),
            SfxListenerMarker,
            GlobalTransform::from(Transform::from_translation(pos)),
        ));
        assert_eq!(listener_position(&state.get(&world).unwrap()), Some(pos));
    }

    #[test]
    fn rcs_volume_is_silent_at_rest_and_saturates_at_full_deflection() {
        assert_eq!(rcs_volume(0.0), 0.0);
        assert_eq!(rcs_volume(1.0), RCS_MAX_VOLUME);
        // A diagonal command can exceed 1; the clamp holds it at the ceiling.
        assert_eq!(rcs_volume(1.7), RCS_MAX_VOLUME);
        assert!((rcs_volume(0.5) - RCS_MAX_VOLUME * 0.5).abs() < f32::EPSILON);
    }
}
