//! The shipped torpedo TYPES - what a bay launches, as opposed to the tube it
//! launches from.
//!
//! A type is defined by how the torpedo FLIES. Blast damage, warhead
//! toughness, blast radius and magazine all stay on the bay, and the two
//! assault types below are deliberately identical in every one of them (owner
//! direction: "they both deal same blast damage but one type is more evasive
//! than the other"), which
//! `sections::ordnance_tests::torpedo_types_differ_only_in_how_the_ordnance_flies`
//! keeps true. What a type owns is the run-in - the weave, and the cruise cap
//! that pays for it:
//!
//! | | Lance | Serpent |
//! |---|---|---|
//! | cruise cap | 35.0 u/s | 32.0 u/s |
//! | weave half-angle | 0.00 rad | 0.44 rad |
//! | rounds one PDC spends to stop it | ~116 | ~390 |
//! | where that PDC finally kills it | ~114 u out | ~40 u out |
//! | time over a 300 u run-in | 9.10 s | 9.78 s |
//! | speed along the line | 31.3 u/s | 29.1 u/s |
//! | reach at the bay's 100 s lifetime | 3130 u | 2914 u |
//!
//! So the Lance is the torpedo you fire at something that will not shoot back:
//! it arrives ~7% sooner, and against a target RUNNING at the player's 25 u/s
//! speed cap it closes at 6.3 u/s where a Serpent manages 4.1 - half again as
//! fast, which is where "long range at something that cannot answer" actually
//! bites. And a defender meeting Lances is a defender whose point defense
//! WORKS, which is what makes the type the campaign's difficulty lever.
//!
//! **The cruise cap is AUTHORED as that price, because the weave does not pay
//! for itself.** A corkscrew is a longer path, so an evasive torpedo was
//! expected to arrive later for free. Measured on the real body it does not:
//! the path stretch is ~1.7% rather than the ideal 11%, and thrust is capped on
//! the ALONG-NOSE speed, so a weaving torpedo never reaches the taper band,
//! keeps its engine lit and settles FASTER. Give the Serpent the Lance's cap
//! and it arrives 1.5% SOONER, longer path and all - which is the control arm
//! in `bay::tests::evasion_costs_time_because_the_type_is_slower_not_because_the_path_is_longer`.
//! Owner decision (task 20260816-113221): a weaving torpedo should BE slower
//! rather than fly just as fast behind a shorter timer, so the price is the cap
//! and not `projectile_lifetime`.

use bevy::prelude::Color;
use nova_ship::prelude::TorpedoTypeConfig;

/// Cruise cap of the straight-running type, in units per second, and the
/// reference every other torpedo speed in this module is quoted against. It is
/// the pre-types number, so the Lance flies exactly as the one shipped torpedo
/// always did.
const LANCE_MAX_SPEED: f32 = 35.0;

/// **Lance** - the straight-running bombardment torpedo: no weave at all, so
/// it flies the bare proportional-navigation intercept.
///
/// The cheaper torpedo to stop, and it does not pretend otherwise: a lead
/// solution solves a straight line exactly, so point defense kills it a full
/// envelope out instead of on its own doorstep. What it buys is the fastest
/// cruise of any assault torpedo and the shortest path to the target, which is
/// most of a second over a 300 u run and half again the closing speed on
/// anything running away. Bombardment ordnance for an enemy that cannot
/// answer - and the campaign's way of showing a player a torpedo they can
/// screen before showing them one they cannot.
pub(crate) fn lance() -> TorpedoTypeConfig {
    TorpedoTypeConfig {
        name: "Lance".to_string(),
        // Pale steel: cold, plain, unmistakably not the hot ordnance below.
        tint: Color::srgb(0.7, 0.78, 0.86),
        // The reference cruise every other torpedo speed is quoted against.
        max_speed: LANCE_MAX_SPEED,
        // The bare intercept. The rate is unread at zero amplitude and is
        // authored zero so the config says "no weave" rather than "a weave
        // nobody spins".
        weave_angle: 0.0,
        weave_rate: 0.0,
    }
}

/// **Serpent** - the assault torpedo: the terminal weave that costs a defender
/// roughly three times the ammunition per intercept, bought with a cruise cap
/// 3 u/s under the [`lance`]'s.
///
/// The same warhead as the Lance, flown differently and more slowly. See
/// [`TorpedoTypeConfig`] for the sweep behind both numbers; this is the engine
/// default, so an un-authored bay carries the ordnance the rest of the combat
/// model is balanced against.
pub(crate) fn serpent() -> TorpedoTypeConfig {
    TorpedoTypeConfig::default()
}

/// **Breaker** - the capital siege warhead.
///
/// Half the Serpent's amplitude at twice the cruise, so the flight path swings
/// about as wide (the swing scales with both) and a capital round still reads
/// as a committed run rather than a dance. It needs the evasion least: its
/// armor already beats point defense outright.
pub(crate) fn breaker() -> TorpedoTypeConfig {
    TorpedoTypeConfig {
        name: "Breaker".to_string(),
        // Deep crimson: the one in the air that ends a ship.
        tint: Color::srgb(0.75, 0.1, 0.12),
        // Twice the Lance's cruise: the closing window through a point-defense
        // envelope is what makes a siege round hard to stop. Unchanged by the
        // per-type move - it was already the bay's own number, and nothing
        // about the capital round's balance was being asked about.
        max_speed: 70.0,
        weave_angle: 0.22,
        weave_rate: 1.4,
    }
}
