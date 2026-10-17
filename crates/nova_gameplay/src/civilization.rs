//! Who a generated body belongs to, and where its civilization stands with
//! the player.
//!
//! A [`CivilizationId`] is the stable identity of one seeded civilization of
//! the open world: the generator draws every trait of the civilization off
//! streams keyed by it, a generated trader carries it in its
//! [`crate::market::TraderMarket`], and a frozen record keeps it with the
//! hull. A civilization holds one [`StanceType`] toward the player, and one
//! toward each living neighbour; never a float. The seed draws both; the
//! player's events move the first: [`CivilizationStances`] is the player's
//! saved table of the civilizations whose stance differs from the seed or
//! that have something pending (offences inside the window, merits toward a
//! step, a provocation, a decay clock). The world layer stamps it with the
//! play-time clock, prunes it as the clock passes each window, clears it
//! with the frozen ledger, saves it with the player and writes the effective
//! stance's side onto every live body of the civilization, so admission, the
//! AI and the HUD keep reading `Allegiance` alone.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;

use crate::{
    hash::prelude::{Fnv32, SeedStream},
    relations::Allegiance,
};

/// Glob-import surface: `use nova_gameplay::civilization::prelude::*`.
pub mod prelude {
    pub use super::{
        CivilizationId, CivilizationStances, StanceRecord, StanceStep, StanceType,
        ENEMY_DECAY_SECONDS, MERIT_STEP, OFFENCE_STEP, OFFENCE_WINDOW_SECONDS,
        PROVOKED_WINDOW_SECONDS,
    };
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

/// How many offences inside [`OFFENCE_WINDOW_SECONDS`] step a civilization
/// down one stance. The first two move nothing: a grace count.
pub const OFFENCE_STEP: u32 = 3;

/// The play seconds an offence counts toward a step: one hour. Longer than
/// a session's worth of raids on one civilization, so a paced pirate still
/// flips it; forgotten after, so three kills a week apart never add up.
pub const OFFENCE_WINDOW_SECONDS: f64 = 3_600.0;

/// How many jobs handed in at a civilization's stations step it up one.
pub const MERIT_STEP: u32 = 3;

/// The play seconds a provoked civilization reads Enemy: fifteen minutes.
/// Longer than a round trip to the next cluster, so waiting at the door
/// does not pay;
/// shorter than a session, so the sanction ends while the player still
/// knows what earned it.
pub const PROVOKED_WINDOW_SECONDS: f64 = 900.0;

/// The play seconds with no new offence after which a civilization the
/// player's events made Enemy returns to Neutral: two hours.
pub const ENEMY_DECAY_SECONDS: f64 = 7_200.0;

/// The stance a civilization holds toward the player, or two neighbouring
/// civilizations hold toward each other. Closed: never a float, never shown
/// as a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StanceType {
    /// Its crews fly the player's side; its stations sell escorts and
    /// themselves.
    Ally,
    /// Bystanders: its crews answer fire, its stations admit and trade.
    Neutral,
    /// Its crews engage on sight and its stations refuse the dock.
    Enemy,
}

impl StanceType {
    /// The side a body of a civilization on this stance flies.
    pub fn side(self) -> Allegiance {
        match self {
            Self::Ally => Allegiance::Player,
            Self::Neutral => Allegiance::Neutral,
            Self::Enemy => Allegiance::Enemy,
        }
    }

    /// The stance a side reads as: the seed draws a side, the table holds a
    /// stance.
    pub fn from_side(side: Allegiance) -> Self {
        match side {
            Allegiance::Player => Self::Ally,
            Allegiance::Neutral => Self::Neutral,
            Allegiance::Enemy => Self::Enemy,
        }
    }

    /// One step down (`-1`: Ally -> Neutral -> Enemy) or up (`+1`), saturating
    /// at both ends. Zero or a larger magnitude is a caller fault.
    pub fn stepped(self, by: i8) -> Self {
        let rank = match self {
            Self::Enemy => 0_i8,
            Self::Neutral => 1,
            Self::Ally => 2,
        };
        match (rank + by).clamp(0, 2) {
            0 => Self::Enemy,
            1 => Self::Neutral,
            _ => Self::Ally,
        }
    }
}

/// One civilization's saved standing with the player. A civilization with
/// no record stands as the seed drew it with nothing pending.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StanceRecord {
    /// The table stance: the seed's, moved by the player's events.
    pub stance: StanceType,
    /// The play-time stamps of the offences inside the window, oldest first.
    pub offences: Vec<f64>,
    /// Jobs handed in toward the next step up.
    pub merits: u32,
    /// The play-time stamp of the latest provocation, while it holds.
    pub provoked_at: Option<f64>,
    /// The play time an event (an offence step or a propagated step) made
    /// the civilization Enemy, restarted by every later offence; `None` for
    /// a civilization that is not Enemy or is Enemy by the seed alone. Drives
    /// the decay and the pirate's outlaw count.
    pub enemy_since: Option<f64>,
    /// Intact hulls the player neutralized while the civilization was Enemy
    /// by events: the pirate charter's count.
    pub outlaw_hulls: u32,
}

impl StanceRecord {
    fn seeded(stance: StanceType) -> Self {
        Self {
            stance,
            offences: Vec::new(),
            merits: 0,
            provoked_at: None,
            enemy_since: None,
            outlaw_hulls: 0,
        }
    }

    /// Whether the record says anything the seed does not.
    fn is_seeded(&self, seeded: StanceType) -> bool {
        self.stance == seeded
            && self.offences.is_empty()
            && self.merits == 0
            && self.provoked_at.is_none()
            && self.enemy_since.is_none()
            && self.outlaw_hulls == 0
    }
}

/// One change of a civilization's table stance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StanceStep {
    /// The civilization whose stance moved.
    pub civilization: CivilizationId,
    /// Its stance before.
    pub from: StanceType,
    /// Its stance after.
    pub to: StanceType,
}

impl StanceStep {
    /// `-1` for a step down, `+1` for a step up.
    pub fn direction(self) -> i8 {
        if self.to < self.from {
            1
        } else {
            -1
        }
    }
}

/// The player's table of civilization stances: sparse, one record per
/// civilization whose standing differs from the seed or has something
/// pending. The world layer moves it; admission never reads it, since the
/// world layer writes each effective stance's side onto the bodies.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CivilizationStances {
    records: BTreeMap<CivilizationId, StanceRecord>,
}

impl CivilizationStances {
    /// The table stance of `civilization`, whose seed drew `seeded`.
    pub fn stance(&self, civilization: &CivilizationId, seeded: StanceType) -> StanceType {
        self.records
            .get(civilization)
            .map_or(seeded, |record| record.stance)
    }

    /// The stance the civilization's bodies act on at `now`: Enemy while a
    /// provocation holds, the table stance otherwise.
    pub fn effective(
        &self,
        civilization: &CivilizationId,
        seeded: StanceType,
        now: f64,
    ) -> StanceType {
        match self.records.get(civilization) {
            Some(record) => {
                let provoked = record
                    .provoked_at
                    .is_some_and(|at| now - at < PROVOKED_WINDOW_SECONDS);
                if provoked {
                    StanceType::Enemy
                } else {
                    record.stance
                }
            }
            None => seeded,
        }
    }

    /// The record of `civilization`, if it has one.
    pub fn record(&self, civilization: &CivilizationId) -> Option<&StanceRecord> {
        self.records.get(civilization)
    }

    fn entry(&mut self, civilization: CivilizationId, seeded: StanceType) -> &mut StanceRecord {
        self.records
            .entry(civilization)
            .or_insert_with(|| StanceRecord::seeded(seeded))
    }

    fn prune(&mut self, civilization: &CivilizationId, seeded: StanceType) {
        if self
            .records
            .get(civilization)
            .is_some_and(|record| record.is_seeded(seeded))
        {
            self.records.remove(civilization);
        }
    }

    /// Move `civilization` by `by` (`-1` down, `+1` up) at `now`, saturating.
    /// A step to Enemy stamps `enemy_since`; a step out of Enemy clears it
    /// and the outlaw count. A saturated step is `None`.
    fn step(
        &mut self,
        civilization: CivilizationId,
        seeded: StanceType,
        by: i8,
        now: f64,
    ) -> Option<StanceStep> {
        let record = self.entry(civilization, seeded);
        let from = record.stance;
        let to = from.stepped(by);
        if to == from {
            self.prune(&civilization, seeded);
            return None;
        }
        record.stance = to;
        if to == StanceType::Enemy {
            record.enemy_since = Some(now);
        } else if from == StanceType::Enemy {
            record.enemy_since = None;
            record.outlaw_hulls = 0;
        }
        Some(StanceStep {
            civilization,
            from,
            to,
        })
    }

    /// Record an offence against `civilization` at `now`: the player
    /// neutralized an intact hull of it. The window's offences are kept; at
    /// [`OFFENCE_STEP`] the civilization steps down one and the tally
    /// clears. An offence while Enemy by events restarts its decay. The step
    /// it caused, for propagation.
    pub fn offence(
        &mut self,
        civilization: CivilizationId,
        seeded: StanceType,
        now: f64,
    ) -> Option<StanceStep> {
        let record = self.entry(civilization, seeded);
        record
            .offences
            .retain(|at| now - at < OFFENCE_WINDOW_SECONDS);
        record.offences.push(now);
        if record.enemy_since.is_some() {
            record.enemy_since = Some(now);
        }
        if record.offences.len() < OFFENCE_STEP as usize {
            return None;
        }
        record.offences.clear();
        self.step(civilization, seeded, -1, now)
    }

    /// Record a merit for `civilization`: the player handed in a job one of
    /// its stations posted. At [`MERIT_STEP`] the civilization steps up one
    /// and the count clears. Merits never propagate.
    pub fn merit(
        &mut self,
        civilization: CivilizationId,
        seeded: StanceType,
        now: f64,
    ) -> Option<StanceStep> {
        let record = self.entry(civilization, seeded);
        record.merits += 1;
        if record.merits < MERIT_STEP {
            return None;
        }
        record.merits = 0;
        self.step(civilization, seeded, 1, now)
    }

    /// Move a neighbour by `by` because a neighbour of its stepped: an Ally
    /// edge follows, an Enemy edge warms. Never propagates further.
    pub fn propagate(
        &mut self,
        civilization: CivilizationId,
        seeded: StanceType,
        by: i8,
        now: f64,
    ) -> Option<StanceStep> {
        self.step(civilization, seeded, by, now)
    }

    /// Stamp a provocation at `now`: the player's damage to a body of the
    /// civilization crossed the threshold. Its bodies read Enemy for
    /// [`PROVOKED_WINDOW_SECONDS`]; a later provocation restarts the window.
    pub fn provoke(&mut self, civilization: CivilizationId, seeded: StanceType, now: f64) {
        self.entry(civilization, seeded).provoked_at = Some(now);
    }

    /// Settle with `civilization`: the fine paid, an Enemy returns to
    /// Neutral and every offence and provocation clears. The step, or `None`
    /// when it was not Enemy.
    pub fn settle(
        &mut self,
        civilization: CivilizationId,
        seeded: StanceType,
        now: f64,
    ) -> Option<StanceStep> {
        let record = self.entry(civilization, seeded);
        record.offences.clear();
        record.provoked_at = None;
        let step = if record.stance == StanceType::Enemy {
            self.step(civilization, seeded, 1, now)
        } else {
            None
        };
        self.prune(&civilization, seeded);
        step
    }

    /// Count an intact hull of `civilization` the player neutralized while
    /// it is Enemy by events. Nothing while it is not.
    pub fn count_outlaw_hull(&mut self, civilization: &CivilizationId) {
        if let Some(record) = self.records.get_mut(civilization) {
            if record.stance == StanceType::Enemy && record.enemy_since.is_some() {
                record.outlaw_hulls += 1;
            }
        }
    }

    /// The pirate charter's facts: how many civilizations the player's
    /// events made Enemy and stand Enemy now, and the most outlaw hulls
    /// counted on one of them.
    pub fn outlaw_progress(&self) -> (u64, u64) {
        let outlawed = self
            .records
            .values()
            .filter(|record| record.stance == StanceType::Enemy && record.enemy_since.is_some());
        let count = outlawed.clone().count() as u64;
        let hulls = outlawed
            .map(|record| u64::from(record.outlaw_hulls))
            .max()
            .unwrap_or(0);
        (count, hulls)
    }

    /// Let the clock pass at `now`: offences older than the window are
    /// forgotten, an expired provocation clears, and a civilization Enemy by
    /// events with no offence for [`ENEMY_DECAY_SECONDS`] returns to
    /// Neutral. `seeded` gives each civilization's seed stance. The steps
    /// the decay made; they never propagate.
    pub fn expire(
        &mut self,
        now: f64,
        seeded: impl Fn(&CivilizationId) -> StanceType,
    ) -> Vec<StanceStep> {
        let ids: Vec<CivilizationId> = self.records.keys().copied().collect();
        let mut steps = Vec::new();
        for id in ids {
            let seed = seeded(&id);
            let decayed = {
                let record = self.entry(id, seed);
                record
                    .offences
                    .retain(|at| now - at < OFFENCE_WINDOW_SECONDS);
                if record
                    .provoked_at
                    .is_some_and(|at| now - at >= PROVOKED_WINDOW_SECONDS)
                {
                    record.provoked_at = None;
                }
                record.stance == StanceType::Enemy
                    && record
                        .enemy_since
                        .is_some_and(|since| now - since >= ENEMY_DECAY_SECONDS)
            };
            if decayed {
                if let Some(step) = self.step(id, seed, 1, now) {
                    steps.push(step);
                }
            }
            self.prune(&id, seed);
        }
        steps
    }

    /// Every record, in id order.
    pub fn entries(&self) -> impl Iterator<Item = (&CivilizationId, &StanceRecord)> + '_ {
        self.records.iter()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Whether a saved table is one a session could have written: every
    /// stamp finite, at least zero and at most `clock_seconds`, the saved
    /// clock; offences oldest first and fewer than [`OFFENCE_STEP`]; merits
    /// below [`MERIT_STEP`]; `enemy_since` only on an Enemy.
    ///
    /// # Errors
    ///
    /// The first record that is not, named.
    pub fn validate(&self, clock_seconds: f64) -> Result<(), String> {
        let in_clock = |at: f64| at.is_finite() && at >= 0.0 && at <= clock_seconds;
        for (id, record) in &self.records {
            let stamps = record
                .offences
                .iter()
                .chain(record.provoked_at.iter())
                .chain(record.enemy_since.iter());
            for at in stamps {
                if !in_clock(*at) {
                    return Err(format!(
                        "civilization {id} carries a stance stamp at {at} s of play, outside 0..={clock_seconds}"
                    ));
                }
            }
            if record.offences.windows(2).any(|pair| pair[0] > pair[1]) {
                return Err(format!("civilization {id} lists its offences out of order"));
            }
            if record.offences.len() >= OFFENCE_STEP as usize {
                return Err(format!(
                    "civilization {id} holds {} offences, at or past the step of {OFFENCE_STEP}",
                    record.offences.len()
                ));
            }
            if record.merits >= MERIT_STEP {
                return Err(format!(
                    "civilization {id} holds {} merits, at or past the step of {MERIT_STEP}",
                    record.merits
                ));
            }
            if record.enemy_since.is_some() && record.stance != StanceType::Enemy {
                return Err(format!(
                    "civilization {id} is {:?} but stamped Enemy by events",
                    record.stance
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

    /// Three offences inside the window step a Neutral civilization to
    /// Enemy, the first two move nothing, and an offence outside the window
    /// does not count toward the three.
    #[test]
    fn three_offences_inside_the_window_step_a_civilization_down_once() {
        let mut table = CivilizationStances::default();
        let x = civilization([0, 0, 0]);
        assert_eq!(table.offence(x, StanceType::Neutral, 10.0), None);
        assert_eq!(table.offence(x, StanceType::Neutral, 20.0), None);
        assert_eq!(table.stance(&x, StanceType::Neutral), StanceType::Neutral);
        let step = table.offence(x, StanceType::Neutral, 30.0);
        assert_eq!(
            step,
            Some(StanceStep {
                civilization: x,
                from: StanceType::Neutral,
                to: StanceType::Enemy,
            })
        );
        assert_eq!(step.unwrap().direction(), -1);
        assert_eq!(table.record(&x).unwrap().enemy_since, Some(30.0));
        assert!(table.record(&x).unwrap().offences.is_empty());

        // From Ally, two stamps fall out of the window before the third.
        let y = civilization([1, 0, 0]);
        table.offence(y, StanceType::Ally, 0.0);
        table.offence(y, StanceType::Ally, 1.0);
        assert_eq!(
            table.offence(y, StanceType::Ally, OFFENCE_WINDOW_SECONDS + 1.0),
            None
        );
        assert_eq!(table.record(&y).unwrap().offences.len(), 1);

        // Already Enemy: a third offence is a saturated step and restarts
        // the decay clock.
        table.offence(x, StanceType::Neutral, 40.0);
        table.offence(x, StanceType::Neutral, 41.0);
        assert_eq!(table.offence(x, StanceType::Neutral, 42.0), None);
        assert_eq!(table.record(&x).unwrap().enemy_since, Some(42.0));
    }

    /// Three merits step up once; a settled Enemy is Neutral with nothing
    /// pending; a civilization back on its seed with nothing pending has no
    /// record.
    #[test]
    fn merits_step_up_and_a_settlement_returns_an_enemy_to_neutral() {
        let mut table = CivilizationStances::default();
        let x = civilization([0, 0, 0]);
        assert_eq!(table.merit(x, StanceType::Neutral, 1.0), None);
        assert_eq!(table.merit(x, StanceType::Neutral, 2.0), None);
        assert_eq!(
            table.merit(x, StanceType::Neutral, 3.0).map(|step| step.to),
            Some(StanceType::Ally)
        );
        assert_eq!(table.merit(x, StanceType::Neutral, 4.0), None);
        assert_eq!(table.merit(x, StanceType::Neutral, 5.0), None);
        assert_eq!(
            table.merit(x, StanceType::Neutral, 6.0),
            None,
            "a saturated step is no step"
        );

        let e = civilization([2, 0, 0]);
        table.provoke(e, StanceType::Enemy, 10.0);
        assert_eq!(
            table.settle(e, StanceType::Enemy, 11.0).map(|step| step.to),
            Some(StanceType::Neutral)
        );
        let record = table.record(&e).unwrap();
        assert_eq!(record.stance, StanceType::Neutral);
        assert_eq!(record.provoked_at, None);
        assert!(record.offences.is_empty());

        let n = civilization([3, 0, 0]);
        table.provoke(n, StanceType::Neutral, 10.0);
        assert_eq!(table.settle(n, StanceType::Neutral, 11.0), None);
        assert_eq!(table.record(&n), None, "back on the seed, nothing pending");
    }

    /// A provoked civilization reads Enemy until its window passes, whatever
    /// its table stance; the expiry clears the stamp; an Enemy by events
    /// decays to Neutral and a seeded Enemy does not.
    #[test]
    fn a_provocation_reads_enemy_for_its_window_and_an_enemy_by_events_decays() {
        let mut table = CivilizationStances::default();
        let a = civilization([0, 0, 0]);
        table.provoke(a, StanceType::Ally, 100.0);
        assert_eq!(table.stance(&a, StanceType::Ally), StanceType::Ally);
        assert_eq!(
            table.effective(&a, StanceType::Ally, 100.0 + PROVOKED_WINDOW_SECONDS - 1.0),
            StanceType::Enemy
        );
        assert_eq!(
            table.effective(&a, StanceType::Ally, 100.0 + PROVOKED_WINDOW_SECONDS),
            StanceType::Ally
        );
        assert!(table
            .expire(100.0 + PROVOKED_WINDOW_SECONDS, |_| StanceType::Ally)
            .is_empty());
        assert_eq!(table.record(&a), None);

        let x = civilization([1, 0, 0]);
        for at in [0.0, 1.0, 2.0] {
            table.offence(x, StanceType::Neutral, at);
        }
        table.count_outlaw_hull(&x);
        assert_eq!(table.outlaw_progress(), (1, 1));
        let steps = table.expire(2.0 + ENEMY_DECAY_SECONDS - 1.0, |_| StanceType::Neutral);
        assert!(steps.is_empty());
        let steps = table.expire(2.0 + ENEMY_DECAY_SECONDS, |_| StanceType::Neutral);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].to, StanceType::Neutral);
        assert_eq!(table.record(&x), None, "decayed to its seed");
        assert_eq!(table.outlaw_progress(), (0, 0));

        let seeded_enemy = civilization([2, 0, 0]);
        table.provoke(seeded_enemy, StanceType::Enemy, 0.0);
        assert!(table
            .expire(ENEMY_DECAY_SECONDS * 2.0, |_| StanceType::Enemy)
            .is_empty());
        table.count_outlaw_hull(&seeded_enemy);
        assert_eq!(
            table.outlaw_progress(),
            (0, 0),
            "a seeded Enemy is no outlaw mark"
        );
    }

    /// A stamp ahead of the clock, below zero, offences out of order or at
    /// the step, merits at the step, or an Enemy-by-events stamp on a
    /// non-Enemy is a save no session wrote.
    #[test]
    fn a_saved_table_is_refused_when_no_session_could_have_written_it() {
        let mut table = CivilizationStances::default();
        let x = civilization([0, 0, 0]);
        table.offence(x, StanceType::Neutral, 10.0);
        table.provoke(x, StanceType::Neutral, 20.0);
        assert_eq!(table.validate(20.0), Ok(()));
        assert!(table.validate(19.0).is_err(), "a stamp ahead of the clock");

        let mut bad = table.clone();
        bad.records.get_mut(&x).unwrap().offences = vec![5.0, 4.0];
        assert!(bad.validate(20.0).is_err(), "offences out of order");
        let mut bad = table.clone();
        bad.records.get_mut(&x).unwrap().offences = vec![1.0, 2.0, 3.0];
        assert!(bad.validate(20.0).is_err(), "offences at the step");
        let mut bad = table.clone();
        bad.records.get_mut(&x).unwrap().merits = MERIT_STEP;
        assert!(bad.validate(20.0).is_err(), "merits at the step");
        let mut bad = table.clone();
        bad.records.get_mut(&x).unwrap().enemy_since = Some(1.0);
        assert!(bad.validate(20.0).is_err(), "Enemy by events on a Neutral");
        let mut bad = table;
        bad.records.get_mut(&x).unwrap().provoked_at = Some(-1.0);
        assert!(bad.validate(20.0).is_err(), "a stamp below zero");
    }

    /// Steps saturate at both ends and a side maps to its stance and back.
    #[test]
    fn a_stance_steps_and_saturates_and_maps_to_a_side() {
        assert_eq!(StanceType::Ally.stepped(-1), StanceType::Neutral);
        assert_eq!(StanceType::Neutral.stepped(-1), StanceType::Enemy);
        assert_eq!(StanceType::Enemy.stepped(-1), StanceType::Enemy);
        assert_eq!(StanceType::Enemy.stepped(1), StanceType::Neutral);
        assert_eq!(StanceType::Ally.stepped(1), StanceType::Ally);
        for stance in [StanceType::Ally, StanceType::Neutral, StanceType::Enemy] {
            assert_eq!(StanceType::from_side(stance.side()), stance);
        }
    }
}
