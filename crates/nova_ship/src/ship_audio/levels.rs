//! How hard the ship's continuous loops read: the throttle-to-volume and
//! intent-to-volume curves for the engine hum and the RCS fine-adjust loop.
//! Ship tuning, not mixing - the generic rolloff and throttle live in
//! [`nova_gameplay::audio`].

/// Loudest the engine hum ever gets (at full thrust), on the linear scale.
pub const ENGINE_MAX_VOLUME: f32 = 0.3;

/// Loudest the RCS fine-adjust loop ever gets (at full-deflection intent).
///
/// Set by MEASUREMENT, not by comparing it to [`ENGINE_MAX_VOLUME`]. The two
/// loops are nothing alike spectrally - the drive is a 66 Hz spine, the RCS a
/// 1.6 kHz hiss sitting where the ear is most sensitive - so a ceiling that
/// looked comfortably under the drive's on paper (0.22 against 0.30) measured
/// 11 dB OVER it A-weighted, and the hiss drowned the burn it is supposed to
/// sit beneath. At 0.05 the two land within about a dB of each other, which is
/// what the old comment claimed all along.
///
/// The general trap, worth remembering before tuning any other pair here: a
/// linear volume is not a loudness. Two cues only compare directly when they
/// occupy the same part of the spectrum.
pub const RCS_MAX_VOLUME: f32 = 0.05;

/// Loudest ANOTHER ship's RCS hiss gets (at full deflection), before the
/// distance rolloff.
///
/// Above [`RCS_MAX_VOLUME`] on purpose. The pilot's own hiss is the hull they
/// sit in and is never attenuated; another ship's is out in the world, under
/// the rolloff, panned, and ranked against every engine hum in the scene by
/// the exterior-loop cap. At the hull level a quarter-deflection dodge at
/// 300 m resolved to about 0.01, which a pilot does not hear. This is about
/// 8 dB over the hull ceiling: an RCS push from another ship is
/// information the pilot needs, not a background bed.
pub const RCS_EXTERIOR_MAX_VOLUME: f32 = 0.12;

/// The share of [`RCS_EXTERIOR_MAX_VOLUME`] any delivered push from another
/// ship sounds at, however light. A station-keeping trim or a dodge is often a
/// small fraction of full deflection, and a linear curve put it under the
/// audible floor. The pilot's own curve stays linear: they feel their own push
/// through the stick.
pub const RCS_EXTERIOR_FLOOR: f32 = 0.4;

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

/// Another ship's RCS hiss volume from its burn effort: silent with no push,
/// then from [`RCS_EXTERIOR_FLOOR`] of [`RCS_EXTERIOR_MAX_VOLUME`] at the
/// lightest push to all of it at full deflection. Pure for tests.
pub(super) fn rcs_exterior_volume(effort: f32) -> f32 {
    if effort <= 0.0 {
        return 0.0;
    }
    let effort = effort.min(1.0);
    RCS_EXTERIOR_MAX_VOLUME * (RCS_EXTERIOR_FLOOR + (1.0 - RCS_EXTERIOR_FLOOR) * effort)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn rcs_volume_is_silent_at_rest_and_saturates_at_full_deflection() {
        assert_eq!(rcs_volume(0.0), 0.0);
        assert_eq!(rcs_volume(1.0), RCS_MAX_VOLUME);
        // A diagonal command can exceed 1; the clamp holds it at the ceiling.
        assert_eq!(rcs_volume(1.7), RCS_MAX_VOLUME);
        assert!((rcs_volume(0.5) - RCS_MAX_VOLUME * 0.5).abs() < f32::EPSILON);
    }
}
