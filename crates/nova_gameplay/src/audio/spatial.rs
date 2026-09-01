//! Directional sound: how an [`AudioRoute::Exterior`](super::AudioRoute::Exterior)
//! voice reaches the listener's two ears.
//!
//! bevy's spatial audio hands the mixing to rodio, which pans by comparing the
//! emitter's distance to each ear and folds its own `1/d^2` loudness law into
//! the same number. That law is not the tuned rolloff
//! [`distance_attenuation`](super::distance_attenuation) owns, so the emitter is
//! NOT parked at the source. It is parked on a sphere of FIXED radius
//! [`SPATIAL_EMITTER_RADIUS`] around the listener, in the true bearing
//! direction: rodio's distance term is then the same for every cue at every
//! range, its output is a pure left/right split, and [`pan_compensation`]
//! divides even that split's own loudness back out. Every dB of falloff is
//! ours.
//!
//! The radius is not free. rodio's "difference" term is inverted - it gives the
//! FAR ear the larger factor - so the correct side only wins because the
//! `1/d^2` term is steeper, which it stops being once the emitter is far enough
//! out that the two ear distances are nearly equal. Keeping the radius close to
//! the ear gap is what makes the pan land on the right side at all; see
//! `a_source_on_the_left_is_louder_in_the_left_ear`.

use bevy::prelude::*;

/// Distance between the listener's two ears, in world units, on the listener's
/// local X axis. Paired with [`SPATIAL_EMITTER_RADIUS`]: the two together are
/// the pan law, and neither is meaningful alone.
pub const SPATIAL_EAR_GAP: f32 = 2.0;

/// Radius of the sphere every exterior emitter is parked on, in world units.
///
/// Held at `1.25 x` the ear gap. Larger and rodio's inverted difference term
/// starts to win, panning cues to the wrong side; smaller and the near ear
/// crosses the `1/d^2` clamp at distance 1, which flattens the pan to nothing.
pub const SPATIAL_EMITTER_RADIUS: f32 = 2.5;

/// The listener's ear rig, inserted on whatever carries
/// [`SfxListenerMarker`](super::SfxListenerMarker).
pub fn listener_ears() -> SpatialListener {
    SpatialListener::new(SPATIAL_EAR_GAP)
}

/// The unit direction from the listener to `source`, in LISTENER-LOCAL space
/// (`+X` right, `+Y` up, `-Z` forward). A source sitting exactly on the
/// listener reads as dead ahead.
pub fn local_bearing(listener: &GlobalTransform, source: Vec3) -> Vec3 {
    listener
        .affine()
        .inverse()
        .transform_point3(source)
        .normalize_or(Vec3::NEG_Z)
}

/// The world point to park the emitter at for a source on `bearing`: fixed
/// radius, true direction.
pub fn emitter_point(listener: &GlobalTransform, bearing: Vec3) -> Vec3 {
    listener.transform_point(bearing * SPATIAL_EMITTER_RADIUS)
}

/// The `(left, right)` gains rodio applies to an emitter parked by
/// [`emitter_point`] on `bearing`.
///
/// A mirror of `rodio::source::Spatial::set_positions`, kept here so
/// [`pan_compensation`] can divide the bearing-dependent part of them back out
/// and so the pan law is something a test can read rather than something only
/// the speakers know.
pub fn pan_gains(bearing: Vec3) -> (f32, f32) {
    let emitter = bearing * SPATIAL_EMITTER_RADIUS;
    let half_gap = SPATIAL_EAR_GAP / 2.0;
    let left_distance = (Vec3::X * -half_gap).distance(emitter);
    let right_distance = (Vec3::X * half_gap).distance(emitter);

    let lean = (left_distance - right_distance) / SPATIAL_EAR_GAP;
    let left_lean = ((lean + 1.0) / 4.0 + 0.5).min(1.0);
    let right_lean = ((-lean + 1.0) / 4.0 + 0.5).min(1.0);

    (
        left_lean * (1.0 / left_distance.powi(2)).min(1.0),
        right_lean * (1.0 / right_distance.powi(2)).min(1.0),
    )
}

/// The sink-volume factor that cancels the bearing-dependent LOUDNESS of
/// [`pan_gains`] while leaving its left/right RATIO - the pan itself - intact.
///
/// The two ears end up at RMS 1, which is exactly where a non-spatial voice
/// sits, so routing a cue `Exterior` changes where it sits in the stereo field
/// and nothing else.
pub fn pan_compensation(bearing: Vec3) -> f32 {
    let (left, right) = pan_gains(bearing);
    let power = ((left * left + right * right) / 2.0).sqrt();
    if power > f32::EPSILON {
        1.0 / power
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The compensated pair actually written to the two channels.
    fn ears(bearing: Vec3) -> (f32, f32) {
        let (left, right) = pan_gains(bearing);
        let scale = pan_compensation(bearing);
        (left * scale, right * scale)
    }

    #[test]
    fn a_source_on_the_left_is_louder_in_the_left_ear() {
        // The whole point of the fixed radius: rodio's difference term is
        // inverted, so this only holds while the radius stays near the ear gap.
        let (left, right) = ears(Vec3::NEG_X);
        assert!(
            left > right * 2.0,
            "a source to port must lean hard to the left ear, got L {left} R {right}"
        );
        let (left, right) = ears(Vec3::X);
        assert!(
            right > left * 2.0,
            "and to starboard, hard to the right, got L {left} R {right}"
        );
    }

    #[test]
    fn a_source_dead_ahead_or_astern_is_centred() {
        for bearing in [Vec3::NEG_Z, Vec3::Z, Vec3::Y, Vec3::NEG_Y] {
            let (left, right) = ears(bearing);
            assert!(
                (left - right).abs() < 1e-5,
                "{bearing:?} is on the median plane and must be centred, got L {left} R {right}"
            );
        }
    }

    #[test]
    fn compensation_holds_the_two_ears_at_unit_rms_whatever_the_bearing() {
        // The amplitude law stays `distance_attenuation`'s: turning a cue
        // Exterior must not make it louder or quieter, only place it.
        for bearing in [
            Vec3::NEG_Z,
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::new(1.0, 1.0, -1.0).normalize(),
            Vec3::new(-0.3, 0.2, 0.9).normalize(),
        ] {
            let (left, right) = ears(bearing);
            let rms = ((left * left + right * right) / 2.0).sqrt();
            assert!(
                (rms - 1.0).abs() < 1e-4,
                "{bearing:?}: expected unit RMS, got {rms} (L {left} R {right})"
            );
        }
    }

    #[test]
    fn the_emitter_sits_at_a_fixed_radius_however_far_the_true_source_is() {
        // The trick the pan rests on: rodio's own distance term must be the
        // same for a cue at 30 u and a cue at 3000 u, so it contributes no
        // loudness of its own.
        let listener =
            GlobalTransform::from(Transform::from_translation(Vec3::new(10.0, -4.0, 25.0)));
        for far in [30.0_f32, 300.0, 3000.0] {
            let source = listener.translation() + Vec3::new(far, 0.0, 0.0);
            let bearing = local_bearing(&listener, source);
            let emitter = emitter_point(&listener, bearing);
            assert!(
                (emitter.distance(listener.translation()) - SPATIAL_EMITTER_RADIUS).abs() < 1e-4,
                "a source at {far} u must still park the emitter on the sphere"
            );
        }
    }

    #[test]
    fn the_bearing_follows_the_listeners_own_facing() {
        // Panning is relative to where the camera is LOOKING, not to world axes:
        // turn the listener and a fixed source crosses the stereo field.
        let source = Vec3::new(0.0, 0.0, -100.0);
        let ahead = GlobalTransform::from(Transform::from_translation(Vec3::ZERO));
        let bearing = local_bearing(&ahead, source);
        assert!(
            bearing.z < -0.99 && bearing.x.abs() < 1e-5,
            "unrotated, the source is dead ahead, got {bearing:?}"
        );

        // Yaw 90 degrees to port. The source has not moved, but the pilot has
        // turned away from it, so it is now off the starboard beam.
        let turned = GlobalTransform::from(
            Transform::from_translation(Vec3::ZERO)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        );
        let bearing = local_bearing(&turned, source);
        assert!(
            bearing.x > 0.99,
            "yawed to port, the source must swing to starboard, got {bearing:?}"
        );
        let (left, right) = ears(bearing);
        assert!(
            right > left,
            "and the mix must follow it, L {left} R {right}"
        );
    }

    #[test]
    fn the_compensation_stays_bounded() {
        // It divides by a gain, so an unbounded value would be a click. Sweep
        // the sphere and pin the range.
        let mut worst: f32 = 0.0;
        for i in 0..64 {
            for j in 0..64 {
                let theta = std::f32::consts::TAU * i as f32 / 64.0;
                let phi = std::f32::consts::PI * j as f32 / 63.0;
                let bearing =
                    Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                worst = worst.max(pan_compensation(bearing));
            }
        }
        assert!(
            (1.0..=32.0).contains(&worst),
            "compensation must stay in a sane band, worst was {worst}"
        );
    }
}
