//! The engine seam for the figures the shell prints.
//!
//! A command that CHANGES a number and a command that READS it back must print
//! the same line, so the conversion and the format live here once rather than
//! at each site.

use nova_events::prelude::MetersPerSecond;

/// A manual speed cap as the shell prints it.
///
/// Engine boundary: [`FlightSpeedCap`](nova_gameplay::prelude::FlightSpeedCap)
/// holds world units because it is compared against an avian velocity every
/// tick; a figure a player reads is meters.
pub fn cap_label(cap: f32) -> String {
    format!("{:.0} m/s", MetersPerSecond::from_engine(cap).get())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cap crosses the seam once: what `speed-cap 300` sets is what
    /// `inspect` reads back.
    #[test]
    fn a_cap_reads_back_the_meters_it_was_set_in() {
        let engine = MetersPerSecond(300.0).to_engine();
        assert_eq!(cap_label(engine), "300 m/s");
    }
}
