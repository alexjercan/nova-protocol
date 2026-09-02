//! Player-facing distance/speed formatting: the single policy for turning the
//! meters the domain already carries into the strings the HUD and NOVA OS show.
//! Distance below 1000 m in integer meters (`840 m`), at/above in kilometers to
//! two decimals (`1.24 km`); speed and closing speed in m/s to one decimal
//! (`50.0 m/s`, `+200.0 m/s`).
//!
//! Nothing here converts. A [`Meters`] arrives in meters and leaves as a
//! string, so a readout cannot pick up a second factor of ten on the way to the
//! screen. Code holding a world-space Bevy or avian number crosses the seam
//! once, with [`Meters::from_engine`], and hands the result in.

use nova_events::units::prelude::*;

/// Distance threshold, in meters, at/above which [`distance`] switches from
/// integer meters to kilometers. Exactly `1000 m` reads as `1.00 km`.
pub const KM_THRESHOLD_M: Meters = Meters(1000.0);

/// Format a distance for a player-facing readout: integer meters below the
/// [`KM_THRESHOLD_M`], kilometers to two decimals at/above it.
///
/// ```
/// # use nova_ui::units::distance;
/// # use nova_events::units::Meters;
/// assert_eq!(distance(Meters(840.0)), "840 m");
/// assert_eq!(distance(Meters(1500.0)), "1.50 km");
/// ```
pub fn distance(meters: Meters) -> String {
    let meters = meters.get();
    // Switch on the ROUNDED meters, not the raw value, so the last half-meter
    // below the threshold reads `1.00 km` instead of printing a four-digit
    // `1000 m` the km branch exists to avoid.
    if meters.round() < KM_THRESHOLD_M.get() {
        format!("{meters:.0} m")
    } else {
        format!("{:.2} km", meters / KM_THRESHOLD_M.get())
    }
}

/// Format a speed as meters per second to one decimal.
///
/// ```
/// # use nova_ui::units::speed;
/// # use nova_events::units::MetersPerSecond;
/// assert_eq!(speed(MetersPerSecond(50.0)), "50.0 m/s");
/// ```
pub fn speed(speed: MetersPerSecond) -> String {
    format!("{:.1} m/s", speed.get())
}

/// Format a speed against the rating it is flown against - a manual burn cap,
/// say - as one readout under one unit.
///
/// Two `speed` calls side by side would print the unit twice; the pair is one
/// number with a ceiling on it, not two numbers.
///
/// ```
/// # use nova_ui::units::speed_rated;
/// # use nova_events::units::MetersPerSecond;
/// assert_eq!(
///     speed_rated(MetersPerSecond(50.0), MetersPerSecond(80.0)),
///     "50.0 / 80.0 m/s"
/// );
/// ```
pub fn speed_rated(flown: MetersPerSecond, rated: MetersPerSecond) -> String {
    format!("{:.1} / {}", flown.get(), speed(rated))
}

/// Format a closing speed as signed meters per second to one decimal -
/// positive when the range is shrinking (approaching).
///
/// ```
/// # use nova_ui::units::closing_speed;
/// # use nova_events::units::MetersPerSecond;
/// assert_eq!(closing_speed(MetersPerSecond(200.0)), "+200.0 m/s");
/// assert_eq!(closing_speed(MetersPerSecond(-32.1)), "-32.1 m/s");
/// ```
pub fn closing_speed(closing: MetersPerSecond) -> String {
    format!("{:+.1} m/s", closing.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_below_threshold_is_integer_meters() {
        assert_eq!(distance(Meters::ZERO), "0 m");
        assert_eq!(distance(Meters(840.0)), "840 m");
        assert_eq!(distance(Meters(999.0)), "999 m");
        assert_eq!(distance(Meters(999.4)), "999 m");
    }

    #[test]
    fn distance_at_and_above_threshold_is_kilometers() {
        assert_eq!(distance(Meters(1000.0)), "1.00 km");
        assert_eq!(distance(Meters(1240.0)), "1.24 km");
        assert_eq!(distance(Meters(1500.0)), "1.50 km");
    }

    #[test]
    fn distance_boundary_never_prints_four_digit_meters() {
        // The switch is on ROUNDED meters, so the last half-meter below the
        // threshold reads km rather than "1000 m".
        assert_eq!(distance(Meters(999.5)), "1.00 km");
        assert_eq!(distance(Meters(999.9)), "1.00 km");
        assert_eq!(distance(Meters(1000.0)), "1.00 km");
    }

    #[test]
    fn speed_is_meters_per_second_one_decimal() {
        assert_eq!(speed(MetersPerSecond::ZERO), "0.0 m/s");
        assert_eq!(speed(MetersPerSecond(50.0)), "50.0 m/s");
        assert_eq!(speed(MetersPerSecond(123.4)), "123.4 m/s");
    }

    #[test]
    fn closing_speed_is_signed_meters_per_second() {
        assert_eq!(closing_speed(MetersPerSecond(200.0)), "+200.0 m/s");
        assert_eq!(closing_speed(MetersPerSecond(123.4)), "+123.4 m/s");
        assert_eq!(closing_speed(MetersPerSecond(-32.1)), "-32.1 m/s");
    }

    /// The seam the whole migration turns on: an engine distance crosses once
    /// on the way in, and the formatter never multiplies again. A 30 world-unit
    /// blast - what `blast_radius: 300` now authors - still reads 300 m.
    #[test]
    fn an_engine_distance_reads_the_same_meters_it_was_authored_in() {
        assert_eq!(distance(Meters::from_engine(30.0)), "300 m");
        assert_eq!(speed(MetersPerSecond::from_engine(32.0)), "320.0 m/s");
    }
}
