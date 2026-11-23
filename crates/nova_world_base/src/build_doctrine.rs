//! How each civilization builds its hulls: a [`BuildDoctrine`] drawn once
//! from the civilization's own seeded stream, which the ship generator reads
//! inside each role's spine size, aspect, ceiling and section budget.
//!
//! The doctrine moves only the draws that keep a layout safe: the scale
//! centre, the share of broad and compact hulls, the mid-body waist or
//! shoulder, the stern taper, the cube pattern, the drive pair share and the
//! nose length. The role keeps its size, so berth fit, clearance and the
//! turn-rate band stay the role's. The ranges are the ones a sweep of 200
//! civilizations x 4 roles x 5 advancements x 32 seeds built with no layout
//! failure: `nova_authoring`'s `generated_doctrines` test.
//!
//! An escort raider, a [`ShipSizeType::Budget`] hull, reads only the cube
//! pattern: the band from advancement 0.5 has 108 sections for 3 turrets,
//! and any proportion shift left some seeds with no room for a cargo intake.
//!
//! [`ShipSizeType::Budget`]: nova_gameplay::prelude::ShipSizeType::Budget

use nova_gameplay::prelude::{CivilizationId, SeedStream};

/// How one civilization builds hulls: proportion habits the hull generator
/// reads inside each role's spine, aspect and ceiling. Drawn once per
/// civilization; every hull of every role reads the same doctrine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuildDoctrine {
    /// Centre of the per-hull scale draw on the role's spine length,
    /// 0.85..=1.0; each hull draws within [`Self::SCALE_SPREAD`] of it.
    pub scale: f32,
    /// Share of broad hulls, 0.0..=0.4: shorter, a cell wider and a cell
    /// deeper.
    pub broad: f32,
    /// Share of compact hulls on roles that allow them, 0.0..=0.4: shorter
    /// still and a cell wider. Roles without compact hulls build long ones
    /// instead.
    pub compact: f32,
    /// Mid-body bias in cells: negative a waist, positive a shoulder,
    /// -0.4..=0.4; each hull draws within [`Self::WAIST_SPREAD`] of it.
    pub waist: f32,
    /// Centre of the blend of the body's size toward the stern's, 0.0..=1.0;
    /// each hull draws within [`Self::TAPER_SPREAD`] of it, held to 0..=1.
    pub taper: f32,
    /// The cube pattern this civilization favours, drawn at
    /// [`Self::FAVOURED_PATTERN`]; the other two share the rest evenly.
    pub pattern: CubePatternType,
    /// Ring width in stations when a hull draws rings, 2..=4.
    pub ring_width: i32,
    /// Share of mirrored drive pairs where a single drive also fits,
    /// 0.1..=0.9.
    pub drive_pair: f32,
    /// Nose divisor, 3..=8: the nose takes `1 + length / nose` stations of
    /// a spine `length` long. Low is a long pointed bow, high a blunt one.
    pub nose: i32,
}

/// How a hull lays its second palette cube over the spine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubePatternType {
    /// Rings of [`BuildDoctrine::ring_width`] stations along the length,
    /// every other ring.
    Rings,
    /// Everything above the keel.
    Deck,
    /// The outer shell of every cross-section.
    Shell,
}

impl CubePatternType {
    /// Every pattern, in draw order.
    pub const ALL: [Self; 3] = [Self::Rings, Self::Deck, Self::Shell];
}

impl BuildDoctrine {
    /// How far each hull's scale draws from [`Self::scale`].
    pub const SCALE_SPREAD: f32 = 0.1;

    /// How far each hull's mid-body bias draws from [`Self::waist`], in
    /// cells.
    pub const WAIST_SPREAD: f32 = 0.5;

    /// How far each hull's stern blend draws from [`Self::taper`].
    pub const TAPER_SPREAD: f32 = 0.25;

    /// The share of hulls that wear [`Self::pattern`].
    pub const FAVOURED_PATTERN: f32 = 0.7;

    /// The doctrine `id` builds with, from its own seeded stream.
    pub fn of(id: CivilizationId) -> Self {
        let mut stream = id.stream(b"doctrine");
        Self {
            scale: 0.85 + 0.15 * stream.unit(),
            broad: 0.4 * stream.unit(),
            compact: 0.4 * stream.unit(),
            waist: 0.4 * stream.signed(),
            taper: stream.unit(),
            pattern: CubePatternType::ALL[pick(&mut stream, 3) as usize],
            ring_width: 2 + pick(&mut stream, 3),
            drive_pair: 0.1 + 0.8 * stream.unit(),
            nose: 3 + pick(&mut stream, 6),
        }
    }

    /// One hull's cube pattern, from a `pick` in `[0, 1)`: the favoured
    /// pattern below [`Self::FAVOURED_PATTERN`], else one of the other two.
    pub fn hull_pattern(&self, pick: f32) -> CubePatternType {
        if pick < Self::FAVOURED_PATTERN {
            return self.pattern;
        }
        let others: Vec<CubePatternType> = CubePatternType::ALL
            .into_iter()
            .filter(|pattern| *pattern != self.pattern)
            .collect();
        let rest = (pick - Self::FAVOURED_PATTERN) / (1.0 - Self::FAVOURED_PATTERN);
        others[((rest * others.len() as f32) as usize).min(others.len() - 1)]
    }
}

/// A whole draw in `0..count`.
fn pick(stream: &mut SeedStream, count: i32) -> i32 {
    ((stream.unit() * count as f32) as i32).min(count - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every civilization's doctrine stays inside the ranges the sweep
    /// proved, and the lattice draws every pattern and nose.
    #[test]
    fn every_doctrine_stays_inside_its_proven_ranges() {
        let mut patterns = Vec::new();
        let mut noses = Vec::new();
        for world_seed in 0..16 {
            for x in -3..=3 {
                for y in -3..=3 {
                    let doctrine = BuildDoctrine::of(CivilizationId {
                        world_seed,
                        node: [x, y, 0],
                    });
                    assert!((0.85..=1.0).contains(&doctrine.scale), "{doctrine:?}");
                    assert!((0.0..=0.4).contains(&doctrine.broad), "{doctrine:?}");
                    assert!((0.0..=0.4).contains(&doctrine.compact), "{doctrine:?}");
                    assert!((-0.4..=0.4).contains(&doctrine.waist), "{doctrine:?}");
                    assert!((0.0..=1.0).contains(&doctrine.taper), "{doctrine:?}");
                    assert!((2..=4).contains(&doctrine.ring_width), "{doctrine:?}");
                    assert!((0.1..=0.9).contains(&doctrine.drive_pair), "{doctrine:?}");
                    assert!((3..=8).contains(&doctrine.nose), "{doctrine:?}");
                    patterns.push(doctrine.pattern);
                    noses.push(doctrine.nose);
                }
            }
        }
        for pattern in CubePatternType::ALL {
            assert!(patterns.contains(&pattern), "{pattern:?} never drawn");
        }
        for nose in 3..=8 {
            assert!(noses.contains(&nose), "nose {nose} never drawn");
        }
    }

    /// A hull wears its civilization's favoured pattern at the favoured
    /// share and each other pattern at half the rest.
    #[test]
    fn a_hull_wears_the_favoured_pattern_at_its_share() {
        let doctrine = BuildDoctrine::of(CivilizationId {
            world_seed: 3,
            node: [0, 0, 0],
        });
        let draws = 10_000;
        let mut counts = [0; 3];
        for draw in 0..draws {
            let pattern = doctrine.hull_pattern(draw as f32 / draws as f32);
            let index = CubePatternType::ALL
                .iter()
                .position(|other| *other == pattern)
                .expect("a pattern");
            counts[index] += 1;
        }
        for (pattern, count) in CubePatternType::ALL.into_iter().zip(counts) {
            let share = count as f32 / draws as f32;
            let expected = if pattern == doctrine.pattern {
                0.7
            } else {
                0.15
            };
            assert!((share - expected).abs() < 0.01, "{pattern:?}: {share}");
        }
    }
}
