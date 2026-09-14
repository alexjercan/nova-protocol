//! The deterministic hash every seeded look in nova is drawn from: FNV-1a in
//! 32 and 64 bits ([`Fnv32`], [`Fnv64`]), the seeded walk built on it
//! ([`SeedStream`]), and the one sphere rule a hash word is turned into a
//! direction by ([`unit_sphere_point`]).
//!
//! # Why a written-out hash and not a hasher or an RNG
//!
//! Generation here is REPRODUCIBLE, not random. The same scenario id, the same
//! hull cell, the same spark index must give the same answer in every process,
//! on every platform and after every toolchain bump - which is what a re-run
//! capture, a reloaded save and a replay all rely on. That rules out
//! `DefaultHasher`, whose output is not promised stable across releases of the
//! standard library: a ship that comes back wearing different antennae after a
//! toolchain bump is exactly the failure this derivation exists to avoid. It
//! also rules out the global RNG, whose draw order is a function of system
//! scheduling rather than of the thing being drawn.
//!
//! # Why it lives here
//!
//! `nova_ship`'s skin and damage sparks, `nova_scenario`'s rocks and planets
//! and this crate's own impact sparks all have to agree on it, and this is the
//! lowest crate the three of them share. The constants below are CONTENT: a
//! change to one silently regenerates every look derived from it, so the tests
//! pin them against the PUBLISHED FNV vectors rather than against whatever the
//! code happens to do.

use bevy::prelude::*;

/// `Fnv32`, `Fnv64`, `SeedStream` and `unit_sphere_point`.
pub mod prelude {
    pub use super::{unit_sphere_point, Fnv32, Fnv64, SeedStream};
}

/// 32-bit FNV-1a over bytes.
///
/// Consuming builder: `Fnv32::new().write(a).write(b).finish()` hashes `a`
/// then `b`, and hashes the same as one `write` over the two joined - the
/// walk carries no state but the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fnv32(u32);

impl Fnv32 {
    /// The published FNV-1a 32-bit offset basis. Content, not a tuning knob.
    pub const OFFSET: u32 = 0x811c_9dc5;

    /// The published FNV-1a 32-bit prime. Content, not a tuning knob.
    pub const PRIME: u32 = 0x0100_0193;

    /// An empty hash.
    #[must_use]
    pub fn new() -> Self {
        Self(Self::OFFSET)
    }

    /// Walks `bytes` into the hash.
    #[must_use]
    pub fn write(mut self, bytes: &[u8]) -> Self {
        for byte in bytes {
            self.0 ^= u32::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
        self
    }

    /// The hash word.
    #[must_use]
    pub fn finish(self) -> u32 {
        self.0
    }
}

impl Default for Fnv32 {
    fn default() -> Self {
        Self::new()
    }
}

/// 64-bit FNV-1a over bytes. [`Fnv32`]'s wider twin, for the hashes that split
/// one word into several draws or have to stay apart over long input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fnv64(u64);

impl Fnv64 {
    /// The published FNV-1a 64-bit offset basis. Content, not a tuning knob.
    pub const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

    /// The published FNV-1a 64-bit prime. Content, not a tuning knob.
    pub const PRIME: u64 = 0x0000_0100_0000_01b3;

    /// An empty hash.
    #[must_use]
    pub fn new() -> Self {
        Self(Self::OFFSET)
    }

    /// Walks `bytes` into the hash.
    #[must_use]
    pub fn write(mut self, bytes: &[u8]) -> Self {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
        self
    }

    /// The hash word.
    #[must_use]
    pub fn finish(self) -> u64 {
        self.0
    }
}

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

/// A deterministic stream of draws from one seed.
///
/// The [`Fnv32`] step with a shift-xor fold on top, walked with no further
/// input: not a general PRNG and not trying to be one. What it has to be is
/// identical in every process and on every platform, which an integer walk is
/// and a floating-point generator is not.
///
/// Draws come off in a FIXED ORDER, so a caller that adds a draw reshuffles
/// only the draws AFTER it - the property that lets a palette grow a slot
/// without invalidating the seeds already authored against its earlier ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedStream(u32);

impl SeedStream {
    /// How far the fold pulls the high bits down.
    ///
    /// A bare FNV step only ever carries low bits upward, so without this the
    /// bottom of the word barely moves between draws.
    const FOLD: u32 = 15;

    /// The stream `seed` opens.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self(Fnv32::OFFSET ^ seed)
    }

    /// The next whole word off the stream.
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(Fnv32::PRIME);
        self.0 ^= self.0 >> Self::FOLD;
        self.0
    }

    /// The next draw in `[0, 1)`, taken off the high bits - the low bits of
    /// FNV-1a are its least mixed.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    /// The next draw in `[-1, 1)`.
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// A point on the unit sphere from one hash word.
///
/// The cylindrical-equal-area map - height uniform, turn uniform - so the
/// spread is even rather than bunched at the poles. `+Z` is the height axis
/// and the ring is laid out in X/Y.
///
/// # One slicing, owned here
///
/// Two callers wrote this out and disagreed on which bits mean what while both
/// claiming this rule. The slicing is therefore a decision rather than a
/// detail, and it is made once: the turn comes off bits 16..31 and the height
/// off bits 8..15.
///
/// The bottom byte is deliberately NOT geometry. FNV-1a's low bits are its
/// least mixed - bit 0 is the parity of the input's low bits and nothing more
/// - so a height drawn from them tracks whatever the caller counted last. It
/// is left free instead, for a caller that needs a third fraction out of the
/// same word: `impact_spark` draws a spark's speed from it.
#[must_use]
pub fn unit_sphere_point(hash: u32) -> Vec3 {
    let turn = (hash >> 16) as f32 / 65_536.0 * std::f32::consts::TAU;
    let height = ((hash >> 8) & 0xff) as f32 / 256.0 * 2.0 - 1.0;
    let ring = (1.0 - height * height).max(0.0).sqrt();
    Vec3::new(ring * turn.cos(), ring * turn.sin(), height).normalize_or(Vec3::Y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants are content. Pinned against the PUBLISHED FNV-1a vectors,
    /// not against whatever this file currently computes, so a typo in an
    /// offset or a prime cannot pass by agreeing with itself - and so a change
    /// to one is a failing test rather than a silent regeneration of every
    /// rock, plate and spark derived from it.
    #[test]
    fn the_hash_is_fnv_1a_and_stays_fnv_1a() {
        assert_eq!(Fnv32::new().finish(), 0x811c_9dc5, "32-bit offset basis");
        assert_eq!(Fnv32::new().write(b"a").finish(), 0xe40c_292c);
        assert_eq!(Fnv32::new().write(b"foobar").finish(), 0xbf9c_f968);

        assert_eq!(
            Fnv64::new().finish(),
            0xcbf2_9ce4_8422_2325,
            "64-bit offset basis"
        );
        assert_eq!(Fnv64::new().write(b"a").finish(), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(
            Fnv64::new().write(b"foobar").finish(),
            0x8594_4171_f739_67e8
        );
    }

    /// Callers feed a hash in pieces - a cell, then a face, then a salt - and
    /// the answer has to be the answer for the whole input, or splitting a
    /// write differently reshapes a ship.
    #[test]
    fn writing_in_pieces_hashes_the_same_as_writing_at_once() {
        assert_eq!(
            Fnv32::new().write(b"foo").write(b"bar").finish(),
            Fnv32::new().write(b"foobar").finish()
        );
        assert_eq!(
            Fnv64::new().write(b"foo").write(b"bar").finish(),
            Fnv64::new().write(b"foobar").finish()
        );
    }

    /// The seed picks the whole walk, and the walk is the same walk every
    /// time. A stream that drifted would reshuffle every planet palette and
    /// every rock's proportions at once.
    #[test]
    fn a_seed_opens_the_same_walk_every_time() {
        let mut once = SeedStream::new(0);
        let mut again = SeedStream::new(0);
        let drawn: Vec<u32> = (0..4).map(|_| once.next_u32()).collect();

        assert_eq!(
            drawn,
            vec![0x050c_5707, 0xf96c_f2df, 0x8583_5e09, 0x36cd_65b1],
            "the walk from seed 0"
        );
        assert_eq!(drawn[0], again.next_u32(), "and it is the same walk");
        assert_ne!(
            SeedStream::new(7).next_u32(),
            drawn[0],
            "a different seed is a different walk"
        );
    }

    /// A draw is a share of one, which is what every caller multiplies a range
    /// by.
    #[test]
    fn every_draw_lands_inside_its_declared_range() {
        let mut stream = SeedStream::new(20_260_913);
        for _ in 0..256 {
            let unit = stream.unit();
            assert!((0.0..1.0).contains(&unit), "unit draw {unit}");
        }
        let mut stream = SeedStream::new(20_260_913);
        for _ in 0..256 {
            let signed = stream.signed();
            assert!((-1.0..1.0).contains(&signed), "signed draw {signed}");
        }
    }

    /// THE divergence this module exists to close: the turn is bits 16..31 and
    /// the height is bits 8..15, pinned as exact points so the two callers
    /// that once disagreed cannot drift apart again.
    #[test]
    fn the_sphere_rule_takes_its_turn_and_its_height_from_fixed_bits() {
        // Height byte 0x00 is the south pole, whatever the turn says.
        let pole = unit_sphere_point(0x0000_0000);
        assert!(
            pole.abs_diff_eq(Vec3::NEG_Z, 1e-5),
            "height byte 0x00: {pole}"
        );
        // Height byte 0x80 is the equator; turn bits 0x0000 face +X.
        let start = unit_sphere_point(0x0000_8000);
        assert!(
            start.abs_diff_eq(Vec3::X, 1e-5),
            "turn bits 0x0000: {start}"
        );
        // A quarter of the turn range is a quarter turn, onto +Y.
        let quarter = unit_sphere_point(0x4000_8000);
        assert!(
            quarter.abs_diff_eq(Vec3::Y, 1e-5),
            "turn bits 0x4000: {quarter}"
        );
    }

    /// The bottom byte is left free ON PURPOSE, so a caller can draw a third
    /// fraction out of the same word without moving the direction. This is the
    /// assertion that fails if the height is ever moved back onto the low
    /// bits.
    #[test]
    fn the_sphere_rule_leaves_the_low_byte_free() {
        for hash in [0x0000_0000u32, 0x1234_5600, 0xffff_ff00, 0x8193_e200] {
            assert_eq!(
                unit_sphere_point(hash),
                unit_sphere_point(hash | 0xff),
                "the low byte steered the direction at {hash:#010x}"
            );
        }
    }

    /// An even spread is the whole claim of the equal-area map: a sphere's
    /// worth of directions averages out near the origin, while a cone's does
    /// not.
    #[test]
    fn the_sphere_rule_sprays_every_way_rather_than_one_way() {
        let points: Vec<Vec3> = (0..512u32)
            .map(|nth| unit_sphere_point(Fnv32::new().write(&nth.to_le_bytes()).finish()))
            .collect();
        for point in &points {
            assert!(
                (point.length() - 1.0).abs() < 1e-4,
                "every point is on the unit sphere: {point}"
            );
        }
        let mean = points.iter().sum::<Vec3>() / points.len() as f32;
        assert!(
            mean.length() < 0.2,
            "the spread leans one way (mean {mean})"
        );
    }
}
