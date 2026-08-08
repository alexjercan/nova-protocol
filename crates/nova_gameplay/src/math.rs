//! Vector math nova's rigs are written in: frame-rate-independent smoothing
//! ([`LerpSnap`]), the spherical-coordinate conversions the camera, transform
//! and mesh rigs share, and the thrust-direction predicate `flight` and `camera`
//! must agree on ([`is_forward_aligned`]).
//!
//! Nova owns this because it is the shared floor under `camera`, `transform`,
//! `flight` and `mesh` - modules that must agree on what "theta" and "main
//! drive" mean - and it is reachable only inside `nova_gameplay`.

use bevy::prelude::*;

/// Glob-import surface for the shared math floor.
pub mod prelude {
    pub use super::{
        direction_to_spherical, is_forward_aligned, slerp, spherical_to_cartesian, LerpSnap,
        FORWARD_ALIGNMENT_COS,
    };
}

/// A thruster counts as part of the main drive when its thrust direction
/// aligns with the ship's forward axis at least this much (cosine). Everything
/// less aligned is left alone - the flight layer only commands the engines
/// that actually push the ship forward.
pub const FORWARD_ALIGNMENT_COS: f32 = 0.9;

/// Whether a thruster's world thrust direction counts as main drive for a
/// ship facing `forward`. Pure for unit testing.
pub fn is_forward_aligned(thrust_dir: Vec3, forward: Vec3) -> bool {
    thrust_dir.dot(forward) >= FORWARD_ALIGNMENT_COS
}

/// Trait for interpolating a value smoothly towards a target with optional snapping.
///
/// The `lerp_and_snap` method allows for smooth interpolation over time while ensuring
/// that the value snaps exactly to the target when it is very close. This is useful for
/// camera movement, smoothing physics values, or any gradual transitions.
pub trait LerpSnap {
    /// Interpolates `self` towards `to` using the given `smoothness` and `dt` (delta time),
    /// and snaps to `to` if close enough.
    ///
    /// - `to`: The target value to interpolate towards.
    /// - `smoothness`: A value between 0.0 and 1.0 controlling how smooth the interpolation is.
    ///   0.0 means instant change, 1.0 means very smooth.
    /// - `dt`: The delta time since the last update.
    ///
    /// Returns the new interpolated (and possibly snapped) value.
    fn lerp_and_snap(&self, to: Self, smoothness: f32, dt: f32) -> Self;
}

impl LerpSnap for f32 {
    fn lerp_and_snap(&self, to: Self, smoothness: f32, dt: f32) -> Self {
        // NOTE: powi(7) turns the 0..1 `smoothness` dial into a per-second retention
        // factor; `powf(dt)` then makes the result frame-rate independent.
        let t = smoothness.powi(7);
        let mut new_value = self.lerp(to, 1.0 - t.powf(dt));
        if smoothness < 1.0 && (new_value - to).abs() < f32::EPSILON {
            new_value = to;
        }

        new_value
    }
}

impl LerpSnap for Vec3 {
    fn lerp_and_snap(&self, to: Self, smoothness: f32, dt: f32) -> Self {
        let t = smoothness.powi(7);
        let mut new_value = self.lerp(to, 1.0 - t.powf(dt));
        if smoothness < 1.0 && (new_value - to).length() < f32::EPSILON {
            new_value = to;
        }

        new_value
    }
}

/// Convert spherical coordinates to a 3D Cartesian vector.
///
/// - `radius`: Distance from the origin.
/// - `theta`: Horizontal angle in radians, measured from -Z axis around Y axis.
/// - `phi`: Vertical angle in radians from the horizontal plane.
///
/// Returns a `Vec3` representing the Cartesian coordinates.
pub fn spherical_to_cartesian(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let x = radius * phi.cos() * theta.sin();
    let y = radius * phi.sin();
    let z = -radius * phi.cos() * theta.cos();
    Vec3::new(x, y, z)
}

/// Convert a direction vector to spherical coordinates (theta, phi).
///
/// - `direction`: The 3D direction vector to convert.
///
/// Returns `(theta, phi)` where:
/// - `theta` is the horizontal angle in radians around the Y axis from -Z,
/// - `phi` is the vertical angle in radians from the horizontal plane.
pub fn direction_to_spherical(direction: Vec3) -> (f32, f32) {
    let r = direction.length();
    if r == 0.0 {
        return (0.0, 0.0);
    }

    let x = direction.x / r;
    let y = direction.y / r;
    let z = direction.z / r;

    let horiz = (x * x + z * z).sqrt();
    let eps = 1e-6_f32;

    let theta = if horiz <= eps { 0.0 } else { x.atan2(-z) };
    let phi = y.atan2(horiz);

    (theta, phi)
}

/// Spherically interpolate between two vectors along the great circle.
///
/// - `a`: Start vector.
/// - `b`: End vector.
/// - `t`: Interpolation factor (0.0 = `a`, 1.0 = `b`).
///
/// Returns a unit vector on the great circle path from `a` to `b`.
pub fn slerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    let dot = a.dot(b).clamp(-1.0, 1.0);
    let theta = dot.acos() * t;
    let relative = (b - a * dot).normalize();
    (a * theta.cos() + relative * theta.sin()).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_alignment_selects_main_drive_thrusters() {
        let forward = Vec3::NEG_Z;
        assert!(is_forward_aligned(Vec3::NEG_Z, forward));
        assert!(!is_forward_aligned(Vec3::Z, forward));
        assert!(!is_forward_aligned(Vec3::X, forward));
    }

    #[test]
    fn test_spherical_to_cartesian() {
        let radius = 1.0;

        let theta = 0.0;
        let phi = 0.0;
        let pos = spherical_to_cartesian(radius, theta, phi);
        assert!(pos.abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-6));

        let theta = std::f32::consts::FRAC_PI_2;
        let phi = 0.0;
        let pos = spherical_to_cartesian(radius, theta, phi);
        assert!(pos.abs_diff_eq(Vec3::new(1.0, 0.0, 0.0), 1e-6));

        let theta = 0.0;
        let phi = std::f32::consts::FRAC_PI_2;
        let pos = spherical_to_cartesian(radius, theta, phi);
        assert!(pos.abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-6));
    }

    #[test]
    fn test_direction_to_spherical() {
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let (theta, phi) = direction_to_spherical(dir);
        assert!(theta.abs() <= 1e-6);
        assert!(phi.abs() <= 1e-6);

        let dir = Vec3::new(1.0, 0.0, 0.0);
        let (theta, phi) = direction_to_spherical(dir);
        assert!((theta - std::f32::consts::FRAC_PI_2).abs() <= 1e-6);
        assert!(phi.abs() <= 1e-6);

        let dir = Vec3::new(0.0, 1.0, 0.0);
        let (theta, phi) = direction_to_spherical(dir);
        assert!(theta.abs() <= 1e-6);
        assert!((phi - std::f32::consts::FRAC_PI_2).abs() <= 1e-6);
    }
}
