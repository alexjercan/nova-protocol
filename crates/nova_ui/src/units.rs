//! Player-facing distance/speed formatting: the single policy for turning raw
//! world units into the metres/kilometres/metres-per-second the HUD and NOVA OS
//! show. The policy: 1 world unit displays as 10 m; distance below 1000 m in
//! integer metres (`840 m`), at/above in kilometres to two decimals
//! (`1.24 km`); speed and closing speed in m/s to one decimal (`50.0 m/s`,
//! `+200.0 m/s`).
//!
//! The FORMATTING is display-only: world-space transforms, content RON and AI
//! tuning keep raw world units, and only the strings a player reads pass
//! through here. The SCALE itself is not - it is
//! [`nova_events::scale::METERS_PER_UNIT`], shared with the physics that turns
//! a G limit into a turn-rate ceiling, so a readout and a hull cannot disagree
//! about how long a metre is.

pub use nova_events::scale::METERS_PER_UNIT;

/// Distance threshold, in displayed metres, at/above which [`distance`]
/// switches from integer metres to kilometres. Exactly `1000 m` reads as
/// `1.00 km` (D6: `>= 1000 m` -> km).
pub const KM_THRESHOLD_M: f32 = 1000.0;

/// Format a world-space distance (in world units) for a player-facing readout:
/// integer metres below the [`KM_THRESHOLD_M`], kilometres to two decimals
/// at/above it.
///
/// ```
/// # use nova_ui::units::distance;
/// assert_eq!(distance(84.0), "840 m");
/// assert_eq!(distance(150.0), "1.50 km");
/// ```
pub fn distance(units: f32) -> String {
    let metres = units * METERS_PER_UNIT;
    // Switch on the ROUNDED metres, not the raw product, so the last
    // half-metre below the threshold reads `1.00 km` instead of printing a
    // four-digit `1000 m` the km branch exists to avoid.
    if metres.round() < KM_THRESHOLD_M {
        format!("{metres:.0} m")
    } else {
        format!("{:.2} km", metres / KM_THRESHOLD_M)
    }
}

/// Format a speed (world units per second) as metres per second to one
/// decimal.
///
/// ```
/// # use nova_ui::units::speed;
/// assert_eq!(speed(5.0), "50.0 m/s");
/// ```
pub fn speed(units_per_s: f32) -> String {
    format!("{:.1} m/s", units_per_s * METERS_PER_UNIT)
}

/// Format a speed against the rating it is flown against - a manual burn cap,
/// say - as one readout under one unit.
///
/// Two `speed` calls side by side would print the unit twice; the pair is one
/// number with a ceiling on it, not two numbers.
///
/// ```
/// # use nova_ui::units::speed_rated;
/// assert_eq!(speed_rated(5.0, 8.0), "50.0 / 80.0 m/s");
/// ```
pub fn speed_rated(units_per_s: f32, rated_units_per_s: f32) -> String {
    format!(
        "{:.1} / {}",
        units_per_s * METERS_PER_UNIT,
        speed(rated_units_per_s)
    )
}

/// Format a closing speed (world units per second) as signed metres per second
/// to one decimal - positive when the range is shrinking (approaching).
///
/// ```
/// # use nova_ui::units::closing_speed;
/// assert_eq!(closing_speed(20.0), "+200.0 m/s");
/// assert_eq!(closing_speed(-3.21), "-32.1 m/s");
/// ```
pub fn closing_speed(units_per_s: f32) -> String {
    format!("{:+.1} m/s", units_per_s * METERS_PER_UNIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_below_threshold_is_integer_metres() {
        // 1 u = 10 m: a sub-kilometre range reads in whole metres.
        assert_eq!(distance(0.0), "0 m");
        assert_eq!(distance(84.0), "840 m");
        // 99.9 u = 999 m, still under the 1000 m threshold.
        assert_eq!(distance(99.9), "999 m");
        // 99.94 u = 999.4 m rounds to 999 m, still metres.
        assert_eq!(distance(99.94), "999 m");
    }

    #[test]
    fn distance_at_and_above_threshold_is_kilometres() {
        // Exactly 100 u = 1000 m -> the km branch (D6: >= 1000 m).
        assert_eq!(distance(100.0), "1.00 km");
        assert_eq!(distance(124.0), "1.24 km");
        assert_eq!(distance(150.0), "1.50 km");
    }

    #[test]
    fn distance_boundary_never_prints_four_digit_metres() {
        // The switch is on ROUNDED metres, so the last half-metre below the
        // threshold reads km rather than "1000 m": 99.95 u = 999.5 m rounds to
        // 1000 -> km; 100.0 u is squarely km. No distance ever shows "1000 m".
        assert_eq!(distance(99.95), "1.00 km");
        assert_eq!(distance(99.99), "1.00 km");
        assert_eq!(distance(100.0), "1.00 km");
    }

    #[test]
    fn speed_is_metres_per_second_one_decimal() {
        assert_eq!(speed(0.0), "0.0 m/s");
        assert_eq!(speed(5.0), "50.0 m/s");
        assert_eq!(speed(12.34), "123.4 m/s");
    }

    #[test]
    fn closing_speed_is_signed_metres_per_second() {
        assert_eq!(closing_speed(20.0), "+200.0 m/s");
        assert_eq!(closing_speed(12.34), "+123.4 m/s");
        assert_eq!(closing_speed(-3.21), "-32.1 m/s");
    }
}
