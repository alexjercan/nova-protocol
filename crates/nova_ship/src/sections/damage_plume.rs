//! PLUME: a drive that still pushes, badly.
//!
//! The damage effect for a thruster. Like
//! [`SPARKS`](super::damage_sparks) it takes no geometry - the bell has to stay
//! a bell to read as a drive - but where sparks are a thing added beside the
//! section, this grades something the section already has: the exhaust cone
//! [`thruster_section`](super::thruster_section) draws from its own throttle.
//!
//! A damaged drive runs SHORT and UNSTEADY. The plume is cut back toward
//! nothing as the level rises and guttered by a flicker on top, so a failing
//! thruster reads as failing from across the field while it is still firing -
//! which is the point, because a drive is the one section whose state a pilot
//! chasing the ship most wants to know.
//!
//! Nothing here touches thrust. The plume is a look and the section keeps
//! delivering exactly the impulse it authored: a section that pushed less as it
//! took damage would be a balance change hiding inside a damage effect, and
//! `SectionInactiveMarker` is already the one thing that stops a drive.

use bevy::prelude::*;

/// `DamagePlume`, `plume_scale` and `DamagePlumePlugin`.
pub mod prelude {
    pub use super::{plume_scale, DamagePlume, DamagePlumePlugin};
}

/// The level below which a plume runs clean.
///
/// A drive that guts the moment it is scratched reads as broken rather than as
/// hit, and every fight starts with a scratch. The same threshold sparks use,
/// and deliberately so: a thruster wearing both effects should start showing
/// them together rather than stuttering into one and then the other.
const PLUME_THRESHOLD: f32 = 0.35;

/// How much of its plume a drive still runs at the worst level a living section
/// reaches. Not zero: a drive with no plume at all reads as SHUT DOWN, which is
/// what `SectionInactiveMarker` means and must not be confused with.
const PLUME_FLOOR: f32 = 0.25;

/// How deep the flicker cuts at the worst level, as a share of what is left.
const FLICKER_DEPTH: f32 = 0.45;

/// How fast the flicker runs, in radians per second. Deliberately not a round
/// number of hertz: two beating sines at close frequencies never repeat on a
/// tidy period, so the gutter does not read as a pulse.
const FLICKER_RATE: f32 = 37.0;
/// The second flicker frequency, beating against [`FLICKER_RATE`].
const FLICKER_BEAT: f32 = 23.5;

/// Makes a thruster's exhaust gut and flicker as its damage level rises,
/// WITHOUT changing the thrust it delivers.
///
/// Carried by thrusters, authored as
/// [`DamageEffect::Plume`](super::damage_effects::DamageEffect::Plume).
/// Compose it freely - a drive that sparks and guts is two effects on one
/// section, which is the point of effects being components.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamagePlume;

/// What share of its authored plume a drive at `level` runs at time `seconds`.
///
/// `1.0` up to [`PLUME_THRESHOLD`], then falling toward [`PLUME_FLOOR`] with a
/// flicker that deepens as it goes. Pure, so the curve can be read without a
/// running app - and so the one thing that must never happen (a plume reading
/// as shut down while the drive is still firing) is a unit test rather than a
/// screenshot.
pub fn plume_scale(level: f32, seconds: f32) -> f32 {
    if level < PLUME_THRESHOLD {
        return 1.0;
    }
    // How far into the failing range this level sits, 0 at the threshold and 1
    // at destruction.
    let into = ((level - PLUME_THRESHOLD) / (1.0 - PLUME_THRESHOLD)).clamp(0.0, 1.0);
    let steady = 1.0 + (PLUME_FLOOR - 1.0) * into;

    // Two beating sines rather than one, so the gutter never settles into a
    // rhythm a player could read as intentional.
    let beat = (seconds * FLICKER_RATE).sin() * 0.6 + (seconds * FLICKER_BEAT).sin() * 0.4;
    // Mapped to [0, 1] so the flicker only ever cuts INTO what is left: a
    // damaged drive must not flare brighter than a healthy one.
    let gutter = 1.0 - FLICKER_DEPTH * into * (0.5 - beat * 0.5);
    (steady * gutter).clamp(0.0, 1.0)
}

/// Registers the plume effect's reflected type.
///
/// No systems: the grading happens where the plume is already written, in
/// `thruster_section`'s shader update, because two systems writing one material
/// would fight over it every frame.
pub struct DamagePlumePlugin;

impl Plugin for DamagePlumePlugin {
    fn build(&self, app: &mut App) {
        trace!("DamagePlumePlugin: build");

        app.register_type::<DamagePlume>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lightly scratched drive is not a failing one, and every fight starts
    /// with a scratch.
    #[test]
    fn a_lightly_damaged_drive_runs_clean() {
        for seconds in [0.0, 0.3, 1.7, 9.0] {
            assert_eq!(plume_scale(0.0, seconds), 1.0);
            assert_eq!(plume_scale(PLUME_THRESHOLD - 0.01, seconds), 1.0);
        }
    }

    /// THE thing that must never happen: a drive that is still firing must
    /// never show nothing, because nothing is what a shut-down drive shows and
    /// the two must not be confusable.
    #[test]
    fn a_failing_drive_never_reads_as_shut_down() {
        for step in 0..=40 {
            let seconds = step as f32 / 4.0;
            let scale = plume_scale(1.0, seconds);
            assert!(
                scale > 0.0,
                "a firing drive showed nothing at t={seconds}: {scale}"
            );
        }
    }

    /// A damaged drive must not flare BRIGHTER than a healthy one - the
    /// flicker cuts into what is left and never adds to it.
    #[test]
    fn a_damaged_drive_never_outshines_a_healthy_one() {
        for step in 0..=40 {
            let seconds = step as f32 / 4.0;
            for level in [0.4, 0.6, 0.8, 1.0] {
                let scale = plume_scale(level, seconds);
                assert!(
                    scale <= 1.0,
                    "level {level} flared to {scale} at t={seconds}"
                );
            }
        }
    }

    /// The worse it gets the less it runs, measured on the steady part so the
    /// flicker cannot mask a curve going the wrong way.
    #[test]
    fn a_worse_drive_runs_a_shorter_plume() {
        // The flicker is at its shallowest where `beat` peaks; averaging over a
        // whole beat is the honest way to compare two levels.
        let average = |level: f32| -> f32 {
            let samples = 400;
            (0..samples)
                .map(|step| plume_scale(level, step as f32 / 40.0))
                .sum::<f32>()
                / samples as f32
        };

        let mut previous = average(PLUME_THRESHOLD);
        for step in 1..=10 {
            let level = PLUME_THRESHOLD + (1.0 - PLUME_THRESHOLD) * (step as f32 / 10.0);
            let scale = average(level);
            assert!(
                scale <= previous,
                "level {level} ran a longer plume than the level before it"
            );
            previous = scale;
        }
    }
}
