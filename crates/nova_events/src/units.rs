//! The physical quantities Nova authors and reasons in: [`Meters`],
//! [`MetersPerSecond`], [`MetersPerSecondSquared`] and the [`Meters3`] offset.
//!
//! Everything a creator writes into a content file and everything gameplay
//! code names as a constant is SI. The engine underneath is not: Bevy
//! transforms and meshes, avian colliders and velocities, shaders, and the
//! build grid all count in world units, and one world unit is
//! [`METERS_PER_UNIT`] meters. These types are the seam. They cross it in
//! exactly two directions - `from_engine` reads a Bevy or physics number into
//! SI, `to_engine` hands one back - and they carry no `Deref` to `f32`, so a
//! meter cannot be passed where the engine wants a unit by accident.
//!
//! Serialization is deliberately plain: every quantity is `#[serde
//! (transparent)]`, so a content file keeps writing `blast_radius: 300.0` and
//! the RON stays readable. The TYPE is what documents the unit, not a wrapper
//! in the file.

use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Meters in one engine world unit.
///
/// The one number that separates what a creator writes from what Bevy and
/// avian receive. Nothing outside an engine boundary should name it: the
/// quantity types below already carry it, and code that multiplies by it by
/// hand is code that can forget to.
pub const METERS_PER_UNIT: f32 = 10.0;

/// A length in meters.
///
/// The unit of every authored distance, every named gameplay range, and every
/// player-facing readout. Cross to engine world units only at a Bevy, physics,
/// rendering or build-grid boundary, and only through
/// [`to_engine`](Self::to_engine).
///
/// ```
/// # use nova_events::units::Meters;
/// // A 300 m blast is authored as 300 and reaches avian as 30 world units.
/// assert_eq!(Meters(300.0).to_engine(), 30.0);
/// assert_eq!(Meters::from_engine(30.0), Meters(300.0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Reflect, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Meters(pub f32);

/// A speed in meters per second.
///
/// ```
/// # use nova_events::units::MetersPerSecond;
/// assert_eq!(MetersPerSecond(2000.0).to_engine(), 200.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Reflect, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetersPerSecond(pub f32);

/// An acceleration in meters per second squared.
///
/// ```
/// # use nova_events::units::MetersPerSecondSquared;
/// assert_eq!(MetersPerSecondSquared(90.0).to_engine(), 9.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Reflect, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MetersPerSecondSquared(pub f32);

/// A displacement in meters on all three axes - an authored world position, a
/// mount offset, a muzzle stand-off.
///
/// Held as a [`Vec3`] so the axis convention stays Bevy's, but it is not one: a
/// `Meters3` never reaches a `Transform` without [`to_engine`](Self::to_engine).
///
/// ```
/// # use nova_events::units::Meters3;
/// # use bevy::prelude::Vec3;
/// assert_eq!(Meters3(Vec3::new(0.0, 0.0, 400.0)).to_engine(), Vec3::new(0.0, 0.0, 40.0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Meters3(pub Vec3);

/// Glob-import surface for the quantity types and the scale they cross at.
pub mod prelude {
    pub use super::{Meters, Meters3, MetersPerSecond, MetersPerSecondSquared, METERS_PER_UNIT};
}

/// Generate the shared scalar surface of a one-component quantity: the
/// engine-boundary crossings, the ordering helpers, and the arithmetic that
/// stays inside the quantity's own dimension.
macro_rules! scalar_quantity {
    ($name:ident, $engine_doc:literal) => {
        impl $name {
            /// The zero of this quantity.
            pub const ZERO: Self = Self(0.0);

            /// The raw SI magnitude. Use it for formatting and for arithmetic
            /// this type does not model; never to feed an engine API.
            pub const fn get(self) -> f32 {
                self.0
            }

            #[doc = $engine_doc]
            pub fn to_engine(self) -> f32 {
                self.0 / METERS_PER_UNIT
            }

            /// Read a value that came out of a Bevy transform, an avian body or
            /// a shader back into SI.
            pub fn from_engine(engine: f32) -> Self {
                Self(engine * METERS_PER_UNIT)
            }

            /// Whether this is a real, finite quantity - what content
            /// validation asks before it trusts an authored number.
            pub fn is_finite(self) -> bool {
                self.0.is_finite()
            }

            /// The magnitude without its sign.
            pub fn abs(self) -> Self {
                Self(self.0.abs())
            }

            /// The smaller of the two.
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0))
            }

            /// The larger of the two.
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }

            /// Clamped into `[min, max]`.
            pub fn clamp(self, min: Self, max: Self) -> Self {
                Self(self.0.clamp(min.0, max.0))
            }

            /// Linear interpolation towards `other` at `t`.
            pub fn lerp(self, other: Self, t: f32) -> Self {
                Self(self.0 + (other.0 - self.0) * t)
            }
        }

        impl Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        impl Mul<f32> for $name {
            type Output = Self;
            fn mul(self, rhs: f32) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl Mul<$name> for f32 {
            type Output = $name;
            fn mul(self, rhs: $name) -> $name {
                $name(self * rhs.0)
            }
        }

        impl Div<f32> for $name {
            type Output = Self;
            fn div(self, rhs: f32) -> Self {
                Self(self.0 / rhs)
            }
        }

        /// Dividing like by like leaves a dimensionless ratio.
        impl Div for $name {
            type Output = f32;
            fn div(self, rhs: Self) -> f32 {
                self.0 / rhs.0
            }
        }

        impl Sum for $name {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::ZERO, Add::add)
            }
        }
    };
}

scalar_quantity!(
    Meters,
    "The same length in engine world units, for a Bevy transform, a mesh \
     primitive, an avian collider or a shader."
);
scalar_quantity!(
    MetersPerSecond,
    "The same speed in engine world units per second, for an avian \
     `LinearVelocity` or a spawn impulse."
);
scalar_quantity!(
    MetersPerSecondSquared,
    "The same acceleration in engine world units per second squared, for an \
     avian force or a thruster integration."
);

impl Meters {
    /// The length covered per second at this speed - the dimensional step from
    /// a distance and a duration to a speed.
    pub fn per_second(self, seconds: f32) -> MetersPerSecond {
        MetersPerSecond(self.0 / seconds)
    }

    /// The square of the length, in square meters. Kept raw because Nova only
    /// ever compares two of them (a squared-range test that skips a `sqrt`).
    pub fn squared(self) -> f32 {
        self.0 * self.0
    }
}

impl MetersPerSecond {
    /// The distance covered in `seconds` at this speed.
    pub fn over(self, seconds: f32) -> Meters {
        Meters(self.0 * seconds)
    }

    /// The acceleration needed to reach this speed in `seconds`.
    pub fn per_second(self, seconds: f32) -> MetersPerSecondSquared {
        MetersPerSecondSquared(self.0 / seconds)
    }
}

impl MetersPerSecondSquared {
    /// The speed gained in `seconds` at this acceleration.
    pub fn over(self, seconds: f32) -> MetersPerSecond {
        MetersPerSecond(self.0 * seconds)
    }
}

impl Meters3 {
    /// The zero displacement.
    pub const ZERO: Self = Self(Vec3::ZERO);

    /// Build a displacement from its three meter components.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }

    /// The same displacement in engine world units, for a Bevy `Transform` or
    /// an avian `Position`.
    pub fn to_engine(self) -> Vec3 {
        self.0 / METERS_PER_UNIT
    }

    /// Read a Bevy or avian world-space vector back into meters.
    pub fn from_engine(engine: Vec3) -> Self {
        Self(engine * METERS_PER_UNIT)
    }

    /// The raw SI components. Use it for formatting and for vector math this
    /// type does not model; never to feed an engine API.
    pub const fn get(self) -> Vec3 {
        self.0
    }

    /// The displacement's length.
    pub fn length(self) -> Meters {
        Meters(self.0.length())
    }
}

impl Add for Meters3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Meters3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Meters3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Mul<f32> for Meters3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self(self.0 * rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_crosses_to_engine_units_by_the_scale() {
        assert_eq!(Meters(300.0).to_engine(), 30.0);
        assert_eq!(Meters(0.0).to_engine(), 0.0);
        assert_eq!(Meters(-45.0).to_engine(), -4.5);
    }

    #[test]
    fn a_length_read_back_from_the_engine_is_the_same_length() {
        for engine in [0.0_f32, 1.0, 30.0, -4.5, 1234.5] {
            assert_eq!(Meters::from_engine(engine).to_engine(), engine);
        }
    }

    #[test]
    fn a_speed_and_an_acceleration_cross_at_the_same_scale() {
        assert_eq!(MetersPerSecond(2000.0).to_engine(), 200.0);
        assert_eq!(MetersPerSecond::from_engine(200.0), MetersPerSecond(2000.0));
        assert_eq!(MetersPerSecondSquared(90.0).to_engine(), 9.0);
        assert_eq!(
            MetersPerSecondSquared::from_engine(9.0),
            MetersPerSecondSquared(90.0)
        );
    }

    #[test]
    fn a_displacement_crosses_component_wise() {
        let offset = Meters3::new(0.0, 25.0, -400.0);
        assert_eq!(offset.to_engine(), Vec3::new(0.0, 2.5, -40.0));
        assert_eq!(Meters3::from_engine(Vec3::new(0.0, 2.5, -40.0)), offset);
    }

    #[test]
    fn lengths_add_subtract_and_scale_within_their_dimension() {
        assert_eq!(Meters(300.0) + Meters(45.0), Meters(345.0));
        assert_eq!(Meters(300.0) - Meters(45.0), Meters(255.0));
        assert_eq!(Meters(300.0) * 0.5, Meters(150.0));
        assert_eq!(2.0 * Meters(300.0), Meters(600.0));
        assert_eq!(Meters(300.0) / 2.0, Meters(150.0));
        assert_eq!(-Meters(300.0), Meters(-300.0));
    }

    #[test]
    fn dividing_two_lengths_leaves_a_bare_ratio() {
        assert_eq!(Meters(300.0) / Meters(150.0), 2.0);
    }

    #[test]
    fn the_dimensional_steps_between_distance_speed_and_acceleration_hold() {
        assert_eq!(MetersPerSecond(50.0).over(4.0), Meters(200.0));
        assert_eq!(Meters(200.0).per_second(4.0), MetersPerSecond(50.0));
        assert_eq!(
            MetersPerSecondSquared(10.0).over(3.0),
            MetersPerSecond(30.0)
        );
        assert_eq!(
            MetersPerSecond(30.0).per_second(3.0),
            MetersPerSecondSquared(10.0)
        );
    }

    #[test]
    fn ordering_helpers_compare_within_the_dimension() {
        assert!(Meters(300.0) > Meters(45.0));
        assert_eq!(Meters(300.0).min(Meters(45.0)), Meters(45.0));
        assert_eq!(Meters(300.0).max(Meters(45.0)), Meters(300.0));
        assert_eq!(Meters(-300.0).abs(), Meters(300.0));
        assert_eq!(
            Meters(300.0).clamp(Meters(0.0), Meters(100.0)),
            Meters(100.0)
        );
        assert_eq!(
            [Meters(1.0), Meters(2.0), Meters(3.0)]
                .into_iter()
                .sum::<Meters>(),
            Meters(6.0)
        );
    }

    #[test]
    fn a_quantity_serializes_as_a_bare_number() {
        assert_eq!(
            ron::ser::to_string(&Meters(300.0)).expect("serialize"),
            "300.0"
        );
        assert_eq!(
            ron::ser::to_string(&MetersPerSecond(2000.0)).expect("serialize"),
            "2000.0"
        );
        assert_eq!(
            ron::from_str::<Meters>("300.0").expect("deserialize"),
            Meters(300.0)
        );
        assert_eq!(
            ron::from_str::<Meters>("300").expect("deserialize"),
            Meters(300.0)
        );
    }

    #[test]
    fn a_displacement_serializes_as_a_bare_triple() {
        assert_eq!(
            ron::ser::to_string(&Meters3::new(0.0, 0.0, 400.0)).expect("serialize"),
            "(0.0,0.0,400.0)"
        );
        assert_eq!(
            ron::from_str::<Meters3>("(0.0, 0.0, 400.0)").expect("deserialize"),
            Meters3::new(0.0, 0.0, 400.0)
        );
    }
}
