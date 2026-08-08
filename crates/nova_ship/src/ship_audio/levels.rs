//! How hard the ship's continuous loops read: the throttle-to-volume and
//! intent-to-volume curves for the engine hum and the RCS fine-adjust loop.
//! Ship tuning, not mixing - the generic rolloff and throttle live in
//! [`nova_gameplay::audio`].

/// Loudest the engine hum ever gets (at full thrust), on the linear scale.
pub const ENGINE_MAX_VOLUME: f32 = 0.3;

/// Loudest the RCS fine-adjust loop ever gets (at full-deflection intent).
/// Below [`ENGINE_MAX_VOLUME`]: RCS is a gentle nudge, not the main drive.
pub const RCS_MAX_VOLUME: f32 = 0.22;

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
