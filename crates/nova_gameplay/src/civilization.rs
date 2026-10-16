//! Who a generated body belongs to, and which of them refuse the player.
//!
//! A [`CivilizationId`] is the stable identity of one seeded civilization of
//! the open world: the generator draws every trait of the civilization off
//! streams keyed by it, a generated trader carries it in its
//! [`crate::market::TraderMarket`], and a frozen record keeps it with the
//! hull. It lives here, below the ship crate, because docking admission
//! reads it: [`NeutralizedCivilizations`] is the player's record of the
//! civilizations whose hull the player neutralized, and a living station of
//! one refuses the player's dock until the refusal window of the price table
//! (`PriceTable::robbery`) has passed. The world layer stamps the record on
//! the neutralize with the play-time clock, prunes it as the clock passes
//! each window, clears it with the frozen ledger and saves it with the
//! player; admission reads membership alone.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;

use crate::hash::prelude::{Fnv32, SeedStream};

/// Glob-import surface: `use nova_gameplay::civilization::prelude::*`.
pub mod prelude {
    pub use super::{CivilizationId, NeutralizedCivilizations};
}

/// The chance a name has three syllables rather than two.
const NAME_THREE_SYLLABLE_CHANCE: f32 = 0.5;

/// The syllables a name is composed from. Content: a change renames every
/// civilization in every world.
const NAME_SYLLABLES: [&str; 24] = [
    "an", "bel", "cor", "da", "el", "fen", "gar", "hal", "is", "jor", "ka", "lun", "mar", "nor",
    "os", "pra", "quel", "ren", "sol", "tev", "ur", "vas", "wen", "zo",
];

/// A civilization's stable machine identity: the world seed and its signed
/// lattice node.
///
/// Never a hash and never a name. Two nodes whose draw streams collide still
/// have distinct identities, and two civilizations may share a display name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CivilizationId {
    /// The world seed the civilization was drawn from.
    pub world_seed: u32,
    /// Its node on the civilization lattice.
    pub node: [i32; 3],
}

impl CivilizationId {
    /// The display name: two or three syllables, first letter capitalized.
    ///
    /// A pure function of the identity. Names repeat across a world, so a
    /// name must not be used to find a civilization.
    pub fn name(self) -> String {
        let mut stream = self.stream(b"name");
        let syllables = if stream.unit() < 1.0 - NAME_THREE_SYLLABLE_CHANCE {
            2
        } else {
            3
        };
        // Three syllable draws every time, so the syllable count does not
        // shift any later draw.
        let drawn: [&str; 3] = std::array::from_fn(|_| {
            let index = (stream.unit() * NAME_SYLLABLES.len() as f32) as usize;
            NAME_SYLLABLES[index.min(NAME_SYLLABLES.len() - 1)]
        });
        let joined = drawn[..syllables].concat();
        let mut letters = joined.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    /// The draw stream for one aspect of this civilization: the FNV-1a 32
    /// of the world seed, `civilization`, `aspect` and the node, each index
    /// little-endian.
    ///
    /// Public so the generator that draws a civilization's centroid, status,
    /// advancement and roles keys them the same way [`name`](Self::name)
    /// is keyed. Each aspect has its own stream, so retuning one aspect
    /// cannot move another: a new name inventory keeps every centroid and
    /// status. A change to this keying moves every civilization in every
    /// world.
    pub fn stream(self, aspect: &[u8]) -> SeedStream {
        SeedStream::new(
            Fnv32::new()
                .write(&self.world_seed.to_le_bytes())
                .write(b"civilization")
                .write(aspect)
                .write(&self.node[0].to_le_bytes())
                .write(&self.node[1].to_le_bytes())
                .write(&self.node[2].to_le_bytes())
                .finish(),
        )
    }
}

impl fmt::Display for CivilizationId {
    /// `civ_<x>_<y>_<z>@<seed>`, a negative index written `n<abs>`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "civ")?;
        for index in self.node {
            if index < 0 {
                write!(formatter, "_n{}", index.unsigned_abs())?;
            } else {
                write!(formatter, "_{index}")?;
            }
        }
        write!(formatter, "@{}", self.world_seed)
    }
}

/// The player's record of the civilizations whose hull the player
/// neutralized: each with the play-time reading of the latest neutralize.
///
/// A living station of a recorded civilization refuses the player's dock
/// (`DockRefusalType::NeutralizedHull`). The world layer records a
/// neutralize, forgets an entry once the refusal window has passed since
/// its stamp, clears the record with the frozen ledger and saves it with the
/// player, so the window runs on across a retire, a thaw and a Load.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct NeutralizedCivilizations {
    latest: BTreeMap<CivilizationId, f64>,
}

impl NeutralizedCivilizations {
    /// Stamp `civilization` with `play_seconds`, the clock at the neutralize.
    /// A later neutralize of the same civilization replaces the stamp, so
    /// the window restarts.
    pub fn record(&mut self, civilization: CivilizationId, play_seconds: f64) {
        self.latest.insert(civilization, play_seconds);
    }

    /// Whether a living station of `civilization` refuses the player.
    pub fn refuses(&self, civilization: &CivilizationId) -> bool {
        self.latest.contains_key(civilization)
    }

    /// The civilizations recorded, each with its stamp, in id order.
    pub fn entries(&self) -> impl Iterator<Item = (&CivilizationId, f64)> + '_ {
        self.latest.iter().map(|(id, at)| (id, *at))
    }

    /// Forget every civilization stamped before `play_seconds`: the clock
    /// less the refusal window.
    pub fn forget_before(&mut self, play_seconds: f64) {
        self.latest.retain(|_, at| *at >= play_seconds);
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.latest.clear();
    }

    /// Whether a saved record is one a session could have written: every
    /// stamp finite, at least zero and at most `clock_seconds`, the saved
    /// clock.
    ///
    /// # Errors
    ///
    /// The first stamp that is not, named.
    pub fn validate(&self, clock_seconds: f64) -> Result<(), String> {
        for (id, at) in &self.latest {
            if !(at.is_finite() && *at >= 0.0 && *at <= clock_seconds) {
                return Err(format!(
                    "civilization {id} is stamped neutralized at {at} s of play, outside 0..={clock_seconds}"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civilization(node: [i32; 3]) -> CivilizationId {
        CivilizationId {
            world_seed: 7,
            node,
        }
    }

    /// A recorded civilization refuses until the cutoff passes its stamp; a
    /// later neutralize restarts its window; a stamp from the future or
    /// below zero is a fault the save refuses.
    #[test]
    fn a_record_refuses_until_its_stamp_falls_behind_the_cutoff() {
        let mut record = NeutralizedCivilizations::default();
        let (first, second) = (civilization([0, 0, 0]), civilization([1, 0, 0]));
        assert!(!record.refuses(&first));

        record.record(first, 10.0);
        record.record(second, 20.0);
        assert!(record.refuses(&first) && record.refuses(&second));

        record.forget_before(15.0);
        assert!(!record.refuses(&first), "stamped before the cutoff");
        assert!(record.refuses(&second), "stamped after the cutoff");

        record.record(second, 40.0);
        record.forget_before(30.0);
        assert!(
            record.refuses(&second),
            "the later neutralize restarted the window"
        );

        assert_eq!(record.validate(40.0), Ok(()));
        assert!(record.validate(39.0).is_err(), "a stamp ahead of the clock");
        record.record(first, -1.0);
        assert!(record.validate(40.0).is_err(), "a stamp below zero");
    }
}
