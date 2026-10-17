//! Jobs: what a living generated station posts, what the player holds, has
//! done and has handed in, and the rules that move a job between those
//! states.
//!
//! A station's board has [`JOBS_PER_BOARD`] slots. Each slot carries a
//! posting number and the play time it was posted at ([`SlotPosting`], kept
//! per station in the [`JobPostings`] session resource); its offer is a
//! function of the world seed, the station's id, the slot, the posting, the
//! cluster types of its cell, the start role's [`JobMixType`] and the
//! [`JobField`] of wrecks and hostile hulls standing around it when the
//! board posts ([`station_job_board`]), so the same postings post the same
//! board while the world around it is the same, in every process and after
//! a Load; a looted wreck or a neutralized hull is off the next posting. An
//! offer the player does not hold expires [`JobOfferRule::offer_seconds`] of
//! play after its stamp; the open world crate then renews the slot
//! ([`JobPostings::renew`]), which posts a new offer under a new [`JobId`].
//! A held job never expires: its slot is pinned until it is handed in or
//! dropped. The open world crate puts the board on the station root as a
//! [`JobBoard`]; the Station pane lists it while the player is docked there,
//! with each offer's remaining time ([`offer_deadline_text`]), and offers
//! Accept and Hand in. Both happen docked only ([`plan_job_accept`],
//! [`plan_job_hand_in`]).
//!
//! Five kinds ([`JobType`]). A procurement is done at the hand-in, by the
//! goods in the hold. A located kind is done in the world: the open world
//! crate watches one existing event kind per type and marks the held job
//! done ([`Jobs::mark_done`]); the hand-in is refused until then. An Escort
//! and a Distress call put generated bodies in the world on the accept
//! ([`JobAccepted`]), named by the job ([`Job::spawned_bodies`]).
//!
//! [`Jobs`] is the player's record: the jobs held, the ids done and the jobs
//! completed. It is a session resource like the chart: a world save carries
//! it, a Load puts it back, and the Map's chart view marks every held job in
//! its target's cell. It lives here because the interface cannot depend on
//! the world crates. It holds no entity: a job names bodies by
//! [`GeneratedBodyId`], so it outlives the cell that holds its poster.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use nova_events::prelude::GeneratedBodyId;
use rand::{RngExt, SeedableRng};

use crate::{
    chart::prelude::ClusterType,
    hash::Fnv64,
    inventory::prelude::{GameItems, ItemCategoryType, ItemDesignId, ShipInventory},
    market::prelude::JobOfferRule,
};

/// Glob-import surface: `use nova_gameplay::jobs::prelude::*`.
pub mod prelude {
    pub use super::{
        escort_raiders, offer_deadline_text, plan_job_accept, plan_job_hand_in, station_job_board,
        Job, JobAcceptRefusalType, JobAccepted, JobBoard, JobField, JobHandInPartner,
        JobHandInPlan, JobHandInRefusalType, JobHull, JobId, JobMixType, JobPostings, JobType,
        JobWreck, Jobs, SlotPosting, ESCORT_RAIDERS_MAX, JOBS_PER_BOARD, RAIDERS_PER_CARGO_CR,
    };
}

/// How many jobs one station's board posts.
///
/// Agent decision 2026-10-16 for owner review: three, so the board reads at
/// a glance and a station never carries the player's whole plan.
pub const JOBS_PER_BOARD: u32 = 3;

/// The fewest and the most units one procurement asks for, both included.
const PROCURE_COUNT: (u32, u32) = (2, 8);
/// The flat fee a job adds over its base, in multiples of this many
/// credits, one to [`FEE_STEPS`] times.
const FEE_STEP_CR: u32 = 50;
/// How many fee steps a job may add.
const FEE_STEPS: u32 = 5;
/// The least and the most cargo an escorted hauler carries, at ask, both
/// included; the reward is half of it plus the fee.
const ESCORT_CARGO_CR: (u32, u32) = (300, 2_400);
/// What a distress call pays before the fee, doubled for the risk.
const DISTRESS_BASE_CR: u32 = 200;
/// What a neutralize pays before the fee, doubled for the risk.
const NEUTRALIZE_BASE_CR: u32 = 400;
/// One raider lies in wait per this many credits of hauler cargo, after the
/// first.
pub const RAIDERS_PER_CARGO_CR: u32 = 700;
/// The most raiders one escort meets.
pub const ESCORT_RAIDERS_MAX: u32 = 3;
/// One distress call in this many is a raider's trap.
const DISTRESS_FAKE_ONE_IN: u32 = 3;

/// The identity of a posted job: the station that posted it, its slot on
/// that station's board and the slot's posting number when it was posted.
/// The poster is read off the id, never kept twice. A renewed slot posts
/// under the next posting number, so the bodies an earlier offer of the
/// slot put in the world are never minted again.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct JobId {
    /// The generated station that posted the job.
    pub station: GeneratedBodyId,
    /// The job's slot on that station's board, below [`JOBS_PER_BOARD`].
    pub slot: u32,
    /// The slot's posting number: zero for the slot's first offer, one more
    /// for each renewal ([`JobPostings::renew`]).
    pub posting: u32,
}

impl JobId {
    /// The id of body `index` of the kind `part` this job puts in the world:
    /// in the poster's cell, under the stem
    /// `<station stem>_job_<slot>_<posting>_<part>`.
    fn spawned_body(&self, part: &str, index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(
            self.station.origin,
            format!(
                "{}_job_{}_{}_{part}",
                self.station.stem, self.slot, self.posting
            ),
            index,
        )
    }

    /// Every body id an offer under this id could have put in the world,
    /// whatever its kind: the hauler, [`ESCORT_RAIDERS_MAX`] raiders and the
    /// caller. A renewal takes the live ones out of the world first.
    pub fn possible_bodies(&self) -> Vec<GeneratedBodyId> {
        std::iter::once(self.spawned_body("hauler", 0))
            .chain((0..ESCORT_RAIDERS_MAX).map(|index| self.spawned_body("raider", index)))
            .chain(std::iter::once(self.spawned_body("caller", 0)))
            .collect()
    }
}

impl fmt::Display for JobId {
    /// `<station id>#<slot>.<posting>`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}.{}", self.station, self.slot, self.posting)
    }
}

/// One slot of a station's board: which posting it is on and when, in play
/// seconds, that posting went up.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SlotPosting {
    /// The posting number, zero for the first offer.
    pub posting: u32,
    /// The [`crate::clock::WorldClock`] reading when the posting went up.
    pub posted_at: f64,
}

impl SlotPosting {
    /// The play seconds the offer has left at `now` under `rule`: zero or
    /// below once it expired. The stamp is taken off `now` first, so a read
    /// on the posting frame gives the whole window exactly, with no rounding
    /// residue from the sum.
    pub fn remaining(self, rule: JobOfferRule, now: f64) -> f64 {
        (self.posted_at - now) + f64::from(rule.offer_seconds)
    }

    /// Whether the offer's window has passed at `now`: it has nothing left.
    pub fn is_expired(self, rule: JobOfferRule, now: f64) -> bool {
        self.remaining(rule, now) <= 0.0
    }
}

/// Every station's slot postings: the posting number and the play-time
/// stamp of each of the [`JOBS_PER_BOARD`] slots of every board the open
/// world has posted. A session resource like [`Jobs`]: a world save carries
/// it, a Load puts it back, and it is cleared with the chart.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct JobPostings {
    boards: BTreeMap<GeneratedBodyId, Vec<SlotPosting>>,
}

impl JobPostings {
    /// Whether no board has been posted.
    pub fn is_empty(&self) -> bool {
        self.boards.is_empty()
    }

    /// Forget every board.
    pub fn clear(&mut self) {
        self.boards.clear();
    }

    /// The slots of `station`'s board, if it has posted.
    pub fn board(&self, station: &GeneratedBodyId) -> Option<&[SlotPosting]> {
        self.boards.get(station).map(Vec::as_slice)
    }

    /// The slot the job `id` was posted in, if its station has posted and
    /// the slot is still on that posting.
    pub fn slot(&self, id: &JobId) -> Option<&SlotPosting> {
        self.board(&id.station)?
            .get(id.slot as usize)
            .filter(|slot| slot.posting == id.posting)
    }

    /// The slots of `station`'s board, posted now at `now` on their first
    /// posting when the station has none yet.
    pub fn open(&mut self, station: &GeneratedBodyId, now: f64) -> &[SlotPosting] {
        self.boards.entry(station.clone()).or_insert_with(|| {
            vec![
                SlotPosting {
                    posting: 0,
                    posted_at: now,
                };
                JOBS_PER_BOARD as usize
            ]
        })
    }

    /// Move the slot of `id` to its next posting at `now` and give back the
    /// id the new offer posts under.
    ///
    /// # Panics
    ///
    /// When the station has not posted, the slot is past the board, or `id`
    /// is not the slot's current posting: the caller renews what it read off
    /// [`JobPostings::open`] this frame.
    pub fn renew(&mut self, id: &JobId, now: f64) -> JobId {
        let slot = self
            .boards
            .get_mut(&id.station)
            .and_then(|slots| slots.get_mut(id.slot as usize))
            .unwrap_or_else(|| panic!("JobPostings renews '{id}', which was never posted"));
        assert_eq!(
            slot.posting, id.posting,
            "JobPostings renews '{id}' but the slot is on posting {}",
            slot.posting
        );
        slot.posting += 1;
        slot.posted_at = now;
        JobId {
            station: id.station.clone(),
            slot: id.slot,
            posting: slot.posting,
        }
    }

    /// Refuse a record a Load could not have built: a board with other than
    /// [`JOBS_PER_BOARD`] slots, a stamp that is not finite or stands ahead
    /// of the clock at `clock_seconds`, or a held job of `jobs` whose slot is
    /// not posted or not on its posting.
    pub fn validate(&self, clock_seconds: f64, jobs: &Jobs) -> Result<(), String> {
        for (station, slots) in &self.boards {
            if slots.len() != JOBS_PER_BOARD as usize {
                return Err(format!(
                    "the postings of '{station}' hold {} slot(s), not {JOBS_PER_BOARD}",
                    slots.len()
                ));
            }
            for (index, slot) in slots.iter().enumerate() {
                if !slot.posted_at.is_finite() || slot.posted_at > clock_seconds {
                    return Err(format!(
                        "the postings of '{station}' stamp slot {index} at {} s of play but the \
                         clock reads {clock_seconds} s",
                        slot.posted_at
                    ));
                }
            }
        }
        for job in jobs.held() {
            if self.slot(&job.id).is_none() {
                return Err(format!(
                    "the held job '{}' is not its slot's current posting",
                    job.id
                ));
            }
        }
        Ok(())
    }
}

/// Which kinds a start role's boards lean to. Authored on the start role;
/// the board draws each slot's kind by the mix's weights ([`JobMixType::weights`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JobMixType {
    /// Procurements and escorts: the industrial and transport starts.
    Trade,
    /// Recoveries off wrecks: the scavenger start.
    Salvage,
    /// Distress calls and neutralizations: the bounty hunter, military and
    /// pirate starts.
    Combat,
}

/// The five kinds, for the weighted draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JobKindType {
    Procure,
    Recover,
    Escort,
    Distress,
    Neutralize,
}

impl JobMixType {
    /// The draw weight of each kind, in [`JobKindType`] order: Procure,
    /// Recover, Escort, Distress, Neutralize. Every kind keeps a weight, so
    /// no start sees one kind only.
    ///
    /// Agent decision 2026-10-16 for owner review.
    pub const fn weights(self) -> [u32; 5] {
        match self {
            JobMixType::Trade => [4, 1, 4, 1, 1],
            JobMixType::Salvage => [1, 5, 1, 2, 1],
            JobMixType::Combat => [1, 1, 1, 3, 4],
        }
    }

    /// Draw a kind off `rng` by this mix's weights.
    fn draw(self, rng: &mut WyRand) -> JobKindType {
        const KINDS: [JobKindType; 5] = [
            JobKindType::Procure,
            JobKindType::Recover,
            JobKindType::Escort,
            JobKindType::Distress,
            JobKindType::Neutralize,
        ];
        let weights = self.weights();
        let total: u32 = weights.iter().sum();
        let mut draw = rng.random_range(0..total);
        for (kind, weight) in KINDS.into_iter().zip(weights) {
            if draw < weight {
                return kind;
            }
            draw -= weight;
        }
        unreachable!("the draw is below the weight total")
    }
}

/// What a job asks the player to do.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum JobType {
    /// Bring `count` of `item` to the posting station. The hand-in takes the
    /// units out of the player's hold; the station consumes them.
    Procure {
        /// The item wanted.
        item: ItemDesignId,
        /// How many units, above zero.
        count: u32,
    },
    /// Take the whole stack of `item`, `count` units, off the derelict
    /// `wreck` and undock with it aboard. Done on the undock from the wreck
    /// with the hold carrying at least `count` and the wreck carrying none.
    Recover {
        /// The derelict that holds the stack.
        wreck: GeneratedBodyId,
        /// What the wreck is called, for the title.
        wreck_name: String,
        /// The item to take.
        item: ItemDesignId,
        /// The stack's units when the board posted, above zero.
        count: u32,
    },
    /// Escort the hauler the accept puts beside the poster to its mark, a
    /// leg into the poster's cell, past the raiders lying in wait on the
    /// way, [`escort_raiders`] of them for `cargo_cr`. Done when the hauler's
    /// helm order completes; a hauler neutralized or destroyed on the way
    /// drops the job.
    Escort {
        /// The hauler's id.
        hauler: GeneratedBodyId,
        /// What its hold is worth at ask, above zero.
        cargo_cr: u32,
    },
    /// Answer the call the accept puts in the world: a drifting hull to dock
    /// with when `genuine`, else a raider's trap, done when the raider is
    /// neutralized or destroyed. The board does not say which.
    Distress {
        /// The caller's id.
        caller: GeneratedBodyId,
        /// Whether a hull in need waits there, or a raider.
        genuine: bool,
    },
    /// Neutralize or destroy the hostile `hull`.
    Neutralize {
        /// The hull's id.
        hull: GeneratedBodyId,
        /// What the hull is called, for the title.
        hull_name: String,
    },
}

impl JobType {
    /// The verb a list calls the kind by.
    pub const fn label(&self) -> &'static str {
        match self {
            JobType::Procure { .. } => "Bring",
            JobType::Recover { .. } => "Recover",
            JobType::Escort { .. } => "Escort",
            JobType::Distress { .. } => "Answer",
            JobType::Neutralize { .. } => "Neutralize",
        }
    }

    /// Whether the kind is done at the hand-in by the goods in the hold,
    /// rather than in the world before it.
    pub const fn is_procurement(&self) -> bool {
        matches!(self, JobType::Procure { .. })
    }
}

/// One job: posted on a board, held by the player, or completed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Job {
    /// Its identity: the poster and the board slot.
    pub id: JobId,
    /// What it asks.
    pub kind: JobType,
    /// The station the job hands in at: the poster.
    pub target: GeneratedBodyId,
    /// What the poster pays on the hand-in, above zero.
    pub reward_cr: u32,
}

impl Job {
    /// The station that posted the job.
    pub fn posted_by(&self) -> &GeneratedBodyId {
        &self.id.station
    }

    /// The one line a board or a list calls the job by: `Bring 4 Iron ore`,
    /// `Recover 3 Frame spar from Vell derelict`, `Escort a hauler carrying
    /// 1200 cr`, `Answer a distress call`, `Neutralize Harrow armored`. An
    /// item the catalog lacks reads by its id; the save check refuses one
    /// before a Load.
    pub fn title(&self, items: &GameItems) -> String {
        let item_name = |item: &ItemDesignId| {
            items
                .get(item)
                .map_or_else(|| item.to_string(), |design| design.name.clone())
        };
        match &self.kind {
            JobType::Procure { item, count } => format!("Bring {count} {}", item_name(item)),
            JobType::Recover {
                wreck_name,
                item,
                count,
                ..
            } => format!("Recover {count} {} from {wreck_name}", item_name(item)),
            JobType::Escort { cargo_cr, .. } => format!("Escort a hauler carrying {cargo_cr} cr"),
            JobType::Distress { .. } => "Answer a distress call".to_string(),
            JobType::Neutralize { hull_name, .. } => format!("Neutralize {hull_name}"),
        }
    }

    /// What the player has to do before the hand-in, for a list.
    pub fn task(&self) -> String {
        match &self.kind {
            JobType::Procure { .. } => "bring the goods".to_string(),
            JobType::Recover { wreck_name, .. } => {
                format!("dock at {wreck_name}, take the stack and undock")
            }
            JobType::Escort { .. } => "see the hauler to its mark".to_string(),
            JobType::Distress { .. } => "reach the caller".to_string(),
            JobType::Neutralize { hull_name, .. } => format!("neutralize {hull_name}"),
        }
    }

    /// Every item id the job names.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        match &self.kind {
            JobType::Procure { item, .. } | JobType::Recover { item, .. } => Some(item),
            JobType::Escort { .. } | JobType::Distress { .. } | JobType::Neutralize { .. } => None,
        }
        .into_iter()
    }

    /// The generated body the job is about in the world, or none for a
    /// procurement: the wreck, the hauler, the caller or the hull.
    pub fn located_body(&self) -> Option<&GeneratedBodyId> {
        match &self.kind {
            JobType::Procure { .. } => None,
            JobType::Recover { wreck, .. } => Some(wreck),
            JobType::Escort { hauler, .. } => Some(hauler),
            JobType::Distress { caller, .. } => Some(caller),
            JobType::Neutralize { hull, .. } => Some(hull),
        }
    }

    /// The raiders an escort meets, in index order; empty for every other
    /// kind.
    pub fn raider_ids(&self) -> Vec<GeneratedBodyId> {
        match &self.kind {
            JobType::Escort { cargo_cr, .. } => (0..escort_raiders(*cargo_cr))
                .map(|index| self.id.spawned_body("raider", index))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Every body the accept puts in the world: the hauler and its raiders,
    /// or the caller. Empty for a procurement, a recovery and a neutralize,
    /// which name bodies that already stand.
    pub fn spawned_bodies(&self) -> Vec<GeneratedBodyId> {
        match &self.kind {
            JobType::Escort { hauler, .. } => std::iter::once(hauler.clone())
                .chain(self.raider_ids())
                .collect(),
            JobType::Distress { caller, .. } => vec![caller.clone()],
            JobType::Procure { .. } | JobType::Recover { .. } | JobType::Neutralize { .. } => {
                Vec::new()
            }
        }
    }

    /// Refuse a job no board could have posted: a zero count, cargo or
    /// reward, a slot past the board, a hand-in elsewhere than the poster,
    /// or a spawned body not named by the job.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.slot >= JOBS_PER_BOARD {
            return Err(format!(
                "the job '{}' holds slot {}, past the {JOBS_PER_BOARD} a board posts",
                self.id, self.id.slot
            ));
        }
        if self.reward_cr == 0 {
            return Err(format!("the job '{}' pays nothing", self.id));
        }
        if self.target != self.id.station {
            return Err(format!(
                "the job '{}' hands in at '{}', not at its poster",
                self.id, self.target
            ));
        }
        match &self.kind {
            JobType::Procure { item, count } | JobType::Recover { item, count, .. } => {
                if *count == 0 {
                    return Err(format!("the job '{}' asks for 0 of '{item}'", self.id));
                }
            }
            JobType::Escort { hauler, cargo_cr } => {
                if *cargo_cr == 0 {
                    return Err(format!("the escort '{}' carries nothing", self.id));
                }
                if *hauler != self.id.spawned_body("hauler", 0) {
                    return Err(format!(
                        "the escort '{}' names the hauler '{hauler}', not its own",
                        self.id
                    ));
                }
            }
            JobType::Distress { caller, .. } => {
                if *caller != self.id.spawned_body("caller", 0) {
                    return Err(format!(
                        "the distress call '{}' names the caller '{caller}', not its own",
                        self.id
                    ));
                }
            }
            JobType::Neutralize { .. } => {}
        }
        Ok(())
    }
}

/// How many raiders lie in wait for a hauler carrying `cargo_cr`: one, plus
/// one per [`RAIDERS_PER_CARGO_CR`], at most [`ESCORT_RAIDERS_MAX`].
pub fn escort_raiders(cargo_cr: u32) -> u32 {
    (1 + cargo_cr / RAIDERS_PER_CARGO_CR).min(ESCORT_RAIDERS_MAX)
}

/// The board a living generated station posts, on its root. Put there by
/// the open world the frame the station stands under its cell's root with
/// the cells around it live, and posted again the same way after a thaw or
/// a Load. The pane hides the jobs the player holds or completed.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct JobBoard(pub Vec<Job>);

impl JobBoard {
    /// The posted job `id`, if this board posted it.
    pub fn job(&self, id: &JobId) -> Option<&Job> {
        self.0.iter().find(|job| job.id == *id)
    }
}

/// Raised by the Station pane once a job is held, so the open world puts
/// the job's bodies ([`Job::spawned_bodies`]) in the world.
#[derive(Event, Clone, Debug)]
pub struct JobAccepted {
    /// The job just held.
    pub job: Job,
}

/// The player's jobs: the jobs held, in accept order, the ids of the held
/// jobs whose work is done, and the jobs completed, in hand-in order. A
/// completed job keeps its kind, its poster and its names: the role goal
/// counts the hand-ins by kind and by the poster's cell.
///
/// Cleared with the chart whenever the world on hand goes: an unload, a
/// Retry, a replaced scenario. A Load restores the saved record on the
/// frame the world arms.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Jobs {
    held: Vec<Job>,
    done: Vec<JobId>,
    completed: Vec<Job>,
}

impl Jobs {
    /// Whether the player holds no job and completed none.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty() && self.completed.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.held.clear();
        self.done.clear();
        self.completed.clear();
    }

    /// The jobs held, in accept order.
    pub fn held(&self) -> &[Job] {
        &self.held
    }

    /// The held jobs whose work is done, in the order it was done.
    pub fn done(&self) -> &[JobId] {
        &self.done
    }

    /// The jobs completed, in hand-in order.
    pub fn completed(&self) -> &[Job] {
        &self.completed
    }

    /// The held job `id`, if the player holds it.
    pub fn job(&self, id: &JobId) -> Option<&Job> {
        self.held.iter().find(|job| job.id == *id)
    }

    /// Whether the player holds the job `id`.
    pub fn holds(&self, id: &JobId) -> bool {
        self.job(id).is_some()
    }

    /// Whether the held job `id` is done and waits for its hand-in.
    pub fn is_done(&self, id: &JobId) -> bool {
        self.done.contains(id)
    }

    /// Whether the player completed the job `id`.
    pub fn is_completed(&self, id: &JobId) -> bool {
        self.completed.iter().any(|job| job.id == *id)
    }

    /// Hold `job`.
    ///
    /// # Panics
    ///
    /// When the job is already held or completed: the caller plans an accept
    /// first ([`plan_job_accept`]).
    pub fn accept(&mut self, job: Job) {
        assert!(
            !self.holds(&job.id) && !self.is_completed(&job.id),
            "Jobs accepts '{}' twice",
            job.id
        );
        self.held.push(job);
    }

    /// Record that the held job `id`'s work is done.
    ///
    /// # Panics
    ///
    /// When the job is not held or is done already: the world crate marks a
    /// job once, on the event that does it.
    pub fn mark_done(&mut self, id: &JobId) {
        assert!(self.holds(id), "Jobs marks '{id}' done, which is not held");
        assert!(!self.is_done(id), "Jobs marks '{id}' done twice");
        self.done.push(id.clone());
    }

    /// Move the held job `id` to the completed list and give it back.
    ///
    /// # Panics
    ///
    /// When the job is not held: the caller plans a hand-in first
    /// ([`plan_job_hand_in`]).
    pub fn complete(&mut self, id: &JobId) -> Job {
        let job = self.drop(id);
        self.completed.push(job.clone());
        job
    }

    /// Take the held job `id` off the list, done or not, and give it back:
    /// the job is over without a hand-in, as when its hauler is lost. The
    /// board posts it again.
    ///
    /// # Panics
    ///
    /// When the job is not held.
    pub fn drop(&mut self, id: &JobId) -> Job {
        let at = self
            .held
            .iter()
            .position(|job| job.id == *id)
            .unwrap_or_else(|| panic!("Jobs drops '{id}', which is not held"));
        self.done.retain(|done| done != id);
        self.held.remove(at)
    }

    /// Every item id a held job names.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> + '_ {
        self.held.iter().flat_map(Job::item_ids)
    }

    /// Refuse a record a Load could not have built: a job held or completed
    /// twice, held and completed at once, done but not held, or a job
    /// [`Job::validate`] refuses.
    pub fn validate(&self) -> Result<(), String> {
        for (index, job) in self.held.iter().enumerate() {
            job.validate()?;
            if self.held[..index].iter().any(|other| other.id == job.id) {
                return Err(format!("the jobs hold '{}' twice", job.id));
            }
            if self.is_completed(&job.id) {
                return Err(format!("the jobs hold '{}' and list it completed", job.id));
            }
        }
        for (index, id) in self.done.iter().enumerate() {
            if !self.holds(id) {
                return Err(format!("the jobs list '{id}' done but not held"));
            }
            if self.done[..index].contains(id) {
                return Err(format!("the jobs list '{id}' done twice"));
            }
        }
        for (index, job) in self.completed.iter().enumerate() {
            job.validate()?;
            if self.completed[..index]
                .iter()
                .any(|other| other.id == job.id)
            {
                return Err(format!("the jobs list '{}' completed twice", job.id));
            }
        }
        Ok(())
    }
}

/// A derelict standing near a station when its board posts, with what its
/// hold carries: a Recover candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobWreck {
    /// The wreck's generated id.
    pub id: GeneratedBodyId,
    /// What it is called.
    pub name: String,
    /// Its hold's stacks, each above zero.
    pub stacks: Vec<(ItemDesignId, u32)>,
}

/// A hostile intact hull standing near a station when its board posts: a
/// Neutralize candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobHull {
    /// The hull's generated id.
    pub id: GeneratedBodyId,
    /// What it is called.
    pub name: String,
}

/// What stands around a station when its board posts: the located bodies
/// its jobs may name. The poster lists them in id order, so the board does
/// not depend on query order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JobField {
    /// The derelicts with something in their hold.
    pub wrecks: Vec<JobWreck>,
    /// The hostile hulls still standing and not neutralized.
    pub hostiles: Vec<JobHull>,
}

/// The item categories a station in a cell with `clusters` asks for. A cell
/// with no cluster body asks for every category but ammunition.
fn wanted_categories(clusters: &[ClusterType]) -> Vec<ItemCategoryType> {
    let mut wanted = Vec::new();
    let mut want = |category: ItemCategoryType| {
        if !wanted.contains(&category) {
            wanted.push(category);
        }
    };
    if clusters.is_empty() {
        for category in [
            ItemCategoryType::Raw,
            ItemCategoryType::Food,
            ItemCategoryType::Parts,
            ItemCategoryType::Repair,
        ] {
            want(category);
        }
    }
    for cluster in clusters {
        match cluster {
            ClusterType::AsteroidRich | ClusterType::RockOnly => want(ItemCategoryType::Raw),
            ClusterType::PlanetHeavy => {
                want(ItemCategoryType::Food);
                want(ItemCategoryType::Raw);
            }
            ClusterType::DerelictField => {
                want(ItemCategoryType::Parts);
                want(ItemCategoryType::Repair);
            }
        }
    }
    wanted
}

/// The board the station `station` posts in the world of `seed`, standing
/// in a cell whose clusters are `clusters`, for a start whose boards lean
/// by `mix`, with `field` standing around it, on the slot postings `slots`:
/// [`JOBS_PER_BOARD`] jobs, each slot's kind drawn by the mix's weights. A
/// slot whose posting the player holds in `held` posts the held job as it
/// is, and no slot names a body a held job is about.
///
/// A procurement asks for an item from the categories the cell asks for
/// ([`wanted_categories`]), distinct while the pool allows, [`PROCURE_COUNT`]
/// units at the item's ask plus half. A recovery names a wreck of the field
/// and one of its stacks, whole, at ask twice; an escort a hauler carrying
/// [`ESCORT_CARGO_CR`] at ask, for half of it; a distress call pays
/// [`DISTRESS_BASE_CR`] and is a trap one time in [`DISTRESS_FAKE_ONE_IN`];
/// a neutralize names a hostile of the field for [`NEUTRALIZE_BASE_CR`].
/// Every job adds a flat fee of one to [`FEE_STEPS`] times [`FEE_STEP_CR`],
/// doubled for a distress call and a neutralize. A kind drawn with no
/// candidate left in the field posts a procurement instead; a wreck or a
/// hull is named once per board.
///
/// Each slot's draws come off its own `WyRand`, seeded from the world seed,
/// the station id, the slot and its posting, in a fixed number, so the board
/// is the same in every process and after a Load while the postings and the
/// field are, and a renewed slot draws afresh. A catalog with no item of the
/// wanted categories draws from every item; an empty catalog posts nothing.
/// A reward past a credit balance saturates.
///
/// # Panics
///
/// When `slots` is not [`JOBS_PER_BOARD`] long: the caller reads them off
/// [`JobPostings::open`].
#[expect(
    clippy::too_many_arguments,
    reason = "one pure draw over the seed, the poster, its cell, the catalog, the mix, the field, the postings and the held jobs"
)]
pub fn station_job_board(
    seed: u32,
    station: &GeneratedBodyId,
    clusters: &[ClusterType],
    items: &GameItems,
    mix: JobMixType,
    field: &JobField,
    slots: &[SlotPosting],
    held: &Jobs,
) -> Vec<Job> {
    assert_eq!(
        slots.len(),
        JOBS_PER_BOARD as usize,
        "station_job_board: '{station}' posts on {} slot(s)",
        slots.len()
    );
    let taken: Vec<&GeneratedBodyId> = held.held().iter().filter_map(Job::located_body).collect();
    let wanted = wanted_categories(clusters);
    let mut pool: Vec<&ItemDesignId> = items
        .iter()
        .filter(|design| wanted.contains(&design.category))
        .map(|design| &design.id)
        .collect();
    if pool.is_empty() {
        pool = items.iter().map(|design| &design.id).collect();
    }
    if pool.is_empty() {
        return Vec::new();
    }
    let mut wrecks: Vec<&JobWreck> = field
        .wrecks
        .iter()
        .filter(|wreck| {
            !taken.contains(&&wreck.id) && wreck.stacks.iter().any(|(_, count)| *count > 0)
        })
        .collect();
    let mut hostiles: Vec<&JobHull> = field
        .hostiles
        .iter()
        .filter(|hull| !taken.contains(&&hull.id))
        .collect();
    let reward = |base: u64, fee: u32| u32::try_from(base + u64::from(fee)).unwrap_or(u32::MAX);
    slots
        .iter()
        .enumerate()
        .map(|(slot, posting)| {
            let id = JobId {
                station: station.clone(),
                slot: slot as u32,
                posting: posting.posting,
            };
            if let Some(job) = held.job(&id) {
                return job.clone();
            }
            let key = Fnv64::new()
                .write(&seed.to_le_bytes())
                .write(b"station_jobs")
                .write(station.to_string().as_bytes())
                .write(&id.slot.to_le_bytes())
                .write(&id.posting.to_le_bytes())
                .finish();
            let mut rng = WyRand::from_seed(key.to_le_bytes());
            // A fixed number of draws per slot, whatever the kind decides.
            let mut kind = mix.draw(&mut rng);
            let fee = rng.random_range(1..=FEE_STEPS) * FEE_STEP_CR;
            let pick = rng.random_range(0..u32::MAX);
            let count: u32 = rng.random_range(PROCURE_COUNT.0..=PROCURE_COUNT.1);
            let cargo_cr: u32 = rng.random_range(ESCORT_CARGO_CR.0..=ESCORT_CARGO_CR.1);
            let fake = rng.random_range(0..DISTRESS_FAKE_ONE_IN) == 0;
            if kind == JobKindType::Recover && wrecks.is_empty()
                || kind == JobKindType::Neutralize && hostiles.is_empty()
            {
                kind = JobKindType::Procure;
            }
            let (kind, reward_cr) = match kind {
                JobKindType::Procure => {
                    let at = pick as usize % pool.len();
                    let item = if pool.len() > 1 {
                        pool.swap_remove(at)
                    } else {
                        pool[at]
                    };
                    let goods = u64::from(count) * u64::from(items.design(item).ask_cr) * 3 / 2;
                    (
                        JobType::Procure {
                            item: item.clone(),
                            count,
                        },
                        reward(goods, fee),
                    )
                }
                JobKindType::Recover => {
                    let wreck = wrecks.swap_remove(pick as usize % wrecks.len());
                    let stacks: Vec<&(ItemDesignId, u32)> = wreck
                        .stacks
                        .iter()
                        .filter(|(_, count)| *count > 0)
                        .collect();
                    let (item, count) = stacks[(pick / 7) as usize % stacks.len()].clone();
                    let goods = u64::from(count) * u64::from(items.design(&item).ask_cr) * 2;
                    (
                        JobType::Recover {
                            wreck: wreck.id.clone(),
                            wreck_name: wreck.name.clone(),
                            item,
                            count,
                        },
                        reward(goods, fee),
                    )
                }
                JobKindType::Escort => (
                    JobType::Escort {
                        hauler: id.spawned_body("hauler", 0),
                        cargo_cr,
                    },
                    reward(u64::from(cargo_cr / 2), fee),
                ),
                JobKindType::Distress => (
                    JobType::Distress {
                        caller: id.spawned_body("caller", 0),
                        genuine: !fake,
                    },
                    reward(u64::from(DISTRESS_BASE_CR), fee * 2),
                ),
                JobKindType::Neutralize => {
                    let hull = hostiles.swap_remove(pick as usize % hostiles.len());
                    (
                        JobType::Neutralize {
                            hull: hull.id.clone(),
                            hull_name: hull.name.clone(),
                        },
                        reward(u64::from(NEUTRALIZE_BASE_CR), fee * 2),
                    )
                }
            };
            Job {
                id,
                kind,
                target: station.clone(),
                reward_cr,
            }
        })
        .collect()
}

/// Why an accept changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobAcceptRefusalType {
    /// The ship is not docked at a living station that posts jobs.
    NotDocked,
    /// The docked station is not the job's poster.
    NotPostedHere,
    /// The player holds the job already.
    AlreadyHeld,
    /// The player completed the job already.
    AlreadyDone,
    /// The offer's window passed, or its slot renewed since the board was
    /// read: the station no longer offers it.
    Expired,
    /// The wreck no longer carries the stack, or the hull is gone or
    /// neutralized already: the job cannot be done.
    TargetGone,
}

/// Plan accepting `job` while docked at the generated station `docked`,
/// `None` while the ship is not docked at a living station posting jobs,
/// against the player's `jobs`. `expired` is whether the offer's window has
/// passed ([`SlotPosting::is_expired`]); `target_standing` is whether the
/// wreck of a recovery still carries its stack, or the hull of a neutralize
/// still stands un-neutralized, read for those two kinds only. A refusal
/// changes nothing.
pub fn plan_job_accept(
    job: &Job,
    docked: Option<&GeneratedBodyId>,
    jobs: &Jobs,
    expired: bool,
    target_standing: bool,
) -> Result<(), JobAcceptRefusalType> {
    let docked = docked.ok_or(JobAcceptRefusalType::NotDocked)?;
    if docked != job.posted_by() {
        return Err(JobAcceptRefusalType::NotPostedHere);
    }
    if jobs.holds(&job.id) {
        return Err(JobAcceptRefusalType::AlreadyHeld);
    }
    if jobs.is_completed(&job.id) {
        return Err(JobAcceptRefusalType::AlreadyDone);
    }
    if expired {
        return Err(JobAcceptRefusalType::Expired);
    }
    let located = matches!(
        job.kind,
        JobType::Recover { .. } | JobType::Neutralize { .. }
    );
    if located && !target_standing {
        return Err(JobAcceptRefusalType::TargetGone);
    }
    Ok(())
}

/// What a list says about an offer with `remaining_seconds` of play left:
/// `expires in 20 min` from a minute up, rounded up to the minute, `expires
/// in 45 s` under a minute, rounded up to the second, `expired` at zero and
/// below.
pub fn offer_deadline_text(remaining_seconds: f64) -> String {
    if remaining_seconds <= 0.0 {
        "expired".to_string()
    } else if remaining_seconds >= 60.0 {
        // Whole seconds first: float residue a hair above a full minute
        // would otherwise round the minutes up by one.
        format!(
            "expires in {} min",
            (remaining_seconds.round() / 60.0).ceil()
        )
    } else {
        format!("expires in {} s", remaining_seconds.ceil())
    }
}

/// The docked station a hand-in reads: its id and its balance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobHandInPartner {
    /// The station's generated id.
    pub id: GeneratedBodyId,
    /// The station's credits.
    pub credits_cr: u32,
}

/// Why a hand-in changes nothing, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobHandInRefusalType {
    /// The ship is not docked at a living station that posts jobs.
    NotDocked,
    /// The docked station is not the job's target.
    WrongStation {
        /// The station the job hands in at.
        needed: GeneratedBodyId,
    },
    /// The hold carries fewer units than the procurement asks.
    ShortItems {
        /// The item wanted.
        item: ItemDesignId,
        /// Units wanted.
        need: u32,
        /// Units in the hold.
        have: u32,
    },
    /// The located job's work is not done yet.
    NotDone,
    /// The station's balance does not cover the reward.
    StationCannotPay {
        /// The reward.
        need: u32,
        /// The station's balance.
        have: u32,
    },
    /// The player's balance after the reward would not fit.
    CreditOverflow,
}

/// What an allowed hand-in does: the goods it takes and the balances after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobHandInPlan {
    /// The item and units taken out of the hold, for a procurement; a
    /// located job takes nothing.
    pub goods: Option<(ItemDesignId, u32)>,
    /// The reward paid.
    pub reward_cr: u32,
    /// The player's balance after: before, plus the reward.
    pub own_cr: u32,
    /// The station's balance after: before, less the reward.
    pub station_cr: u32,
}

/// Plan handing `job` in while docked with `partner`, `None` while the ship
/// is not docked at a living station posting jobs, from `hold` with a
/// balance of `own_cr`; `done` is whether the player's record lists the job
/// done ([`Jobs::is_done`]), read for a located job only. Checks run in
/// [`JobHandInRefusalType`] order. A refusal changes nothing.
pub fn plan_job_hand_in(
    job: &Job,
    partner: Option<&JobHandInPartner>,
    hold: &ShipInventory,
    own_cr: u32,
    done: bool,
) -> Result<JobHandInPlan, JobHandInRefusalType> {
    let partner = partner.ok_or(JobHandInRefusalType::NotDocked)?;
    if partner.id != job.target {
        return Err(JobHandInRefusalType::WrongStation {
            needed: job.target.clone(),
        });
    }
    let goods = match &job.kind {
        JobType::Procure { item, count } => {
            let have = hold.count(item);
            if have < *count {
                return Err(JobHandInRefusalType::ShortItems {
                    item: item.clone(),
                    need: *count,
                    have,
                });
            }
            Some((item.clone(), *count))
        }
        JobType::Recover { .. }
        | JobType::Escort { .. }
        | JobType::Distress { .. }
        | JobType::Neutralize { .. } => {
            if !done {
                return Err(JobHandInRefusalType::NotDone);
            }
            None
        }
    };
    let Some(station_cr) = partner.credits_cr.checked_sub(job.reward_cr) else {
        return Err(JobHandInRefusalType::StationCannotPay {
            need: job.reward_cr,
            have: partner.credits_cr,
        });
    };
    let own_cr = own_cr
        .checked_add(job.reward_cr)
        .ok_or(JobHandInRefusalType::CreditOverflow)?;
    Ok(JobHandInPlan {
        goods,
        reward_cr: job.reward_cr,
        own_cr,
        station_cr,
    })
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;
    use crate::inventory::prelude::ItemDesign;

    fn item(id: &str, category: ItemCategoryType, ask_cr: u32) -> ItemDesign {
        ItemDesign {
            id: id.into(),
            name: id.to_lowercase(),
            about: String::new(),
            category,
            mass_g: 1_000,
            ask_cr,
            bid_cr: ask_cr * 3 / 4,
        }
    }

    fn catalog() -> GameItems {
        GameItems::new([
            item("IronOre", ItemCategoryType::Raw, 16),
            item("StoneOre", ItemCategoryType::Raw, 4),
            item("WaterIce", ItemCategoryType::Raw, 12),
            item("Rations", ItemCategoryType::Food, 8),
            item("FrameSpar", ItemCategoryType::Parts, 60),
            item("HullPlate", ItemCategoryType::Repair, 40),
            item("PdcRound", ItemCategoryType::Ammo, 4),
        ])
    }

    fn station(x: i32) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_0_0_0_station", 0)
    }

    fn body(stem: &str, index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(1, 0, 0), stem, index)
    }

    /// Two wrecks and two hostiles to name.
    fn field() -> JobField {
        JobField {
            wrecks: vec![
                JobWreck {
                    id: body("cluster_1_0_0_hull", 2),
                    name: "Vell derelict".to_string(),
                    stacks: vec![("FrameSpar".into(), 3), ("Rations".into(), 5)],
                },
                JobWreck {
                    id: body("cluster_1_0_0_hull", 4),
                    name: "Harrow derelict".to_string(),
                    stacks: vec![("HullPlate".into(), 2)],
                },
            ],
            hostiles: vec![
                JobHull {
                    id: body("cluster_1_0_0_hull", 0),
                    name: "Kessel armored".to_string(),
                },
                JobHull {
                    id: body("cluster_1_0_0_hull", 1),
                    name: "Kessel scavenger".to_string(),
                },
            ],
        }
    }

    /// The first posting of every slot, posted at zero play.
    fn first_postings() -> Vec<SlotPosting> {
        let mut postings = JobPostings::default();
        postings.open(&station(1), 0.0).to_vec()
    }

    /// A board on the first postings with nothing held.
    fn board(
        seed: u32,
        station: &GeneratedBodyId,
        clusters: &[ClusterType],
        items: &GameItems,
        mix: JobMixType,
        field: &JobField,
    ) -> Vec<Job> {
        station_job_board(
            seed,
            station,
            clusters,
            items,
            mix,
            field,
            &first_postings(),
            &Jobs::default(),
        )
    }

    /// The boards of many seeds for one mix, flattened.
    fn boards(mix: JobMixType, field: &JobField) -> Vec<Job> {
        let items = catalog();
        (0..60)
            .flat_map(|seed| {
                board(
                    seed,
                    &station(1),
                    &[ClusterType::AsteroidRich],
                    &items,
                    mix,
                    field,
                )
            })
            .collect()
    }

    fn kind_counts(jobs: &[Job]) -> [usize; 5] {
        let mut counts = [0; 5];
        for job in jobs {
            let at = match job.kind {
                JobType::Procure { .. } => 0,
                JobType::Recover { .. } => 1,
                JobType::Escort { .. } => 2,
                JobType::Distress { .. } => 3,
                JobType::Neutralize { .. } => 4,
            };
            counts[at] += 1;
        }
        counts
    }

    #[test]
    fn a_board_is_the_same_for_its_seed_station_and_field_and_asks_the_cell_s_categories() {
        let items = catalog();
        let rocks = [ClusterType::AsteroidRich];
        let field = field();
        let posted = board(10, &station(1), &rocks, &items, JobMixType::Trade, &field);
        assert_eq!(posted.len(), JOBS_PER_BOARD as usize);
        assert_eq!(
            posted,
            board(10, &station(1), &rocks, &items, JobMixType::Trade, &field),
            "the same seed, station and field post the same board"
        );
        assert_ne!(
            posted,
            board(11, &station(1), &rocks, &items, JobMixType::Trade, &field),
            "another seed posts another board"
        );
        assert_ne!(
            posted,
            board(10, &station(2), &rocks, &items, JobMixType::Trade, &field),
            "another station posts another board"
        );
        for (slot, job) in posted.iter().enumerate() {
            job.validate().unwrap();
            assert_eq!(job.id.slot, slot as u32);
            assert_eq!(job.id.posting, 0);
            assert_eq!(job.posted_by(), &station(1));
            assert_eq!(&job.target, &station(1));
        }
        let mut seen = Vec::new();
        for job in boards(JobMixType::Trade, &JobField::default()) {
            match &job.kind {
                JobType::Procure { item, count } => {
                    assert_eq!(
                        items.design(item).category,
                        ItemCategoryType::Raw,
                        "an asteroid cell asks for raw material"
                    );
                    assert!((PROCURE_COUNT.0..=PROCURE_COUNT.1).contains(count));
                    let goods = count * items.design(item).ask_cr * 3 / 2;
                    assert!(
                        job.reward_cr > goods && job.reward_cr <= goods + FEE_STEPS * FEE_STEP_CR,
                        "the reward pays the goods and a fee: {job:?}"
                    );
                    seen.push(item.clone());
                }
                JobType::Recover { .. } | JobType::Neutralize { .. } => {
                    panic!("an empty field names no wreck and no hull: {job:?}")
                }
                JobType::Escort { .. } | JobType::Distress { .. } => {}
            }
        }
        assert!(seen.contains(&"IronOre".into()) && seen.contains(&"StoneOre".into()));
        let derelicts = board(
            10,
            &station(1),
            &[ClusterType::DerelictField],
            &items,
            JobMixType::Trade,
            &JobField::default(),
        );
        for job in &derelicts {
            if let JobType::Procure { item, .. } = &job.kind {
                assert!(
                    matches!(
                        items.design(item).category,
                        ItemCategoryType::Parts | ItemCategoryType::Repair
                    ),
                    "a derelict field asks for parts and plates: {job:?}"
                );
            }
        }
    }

    #[test]
    fn the_mix_weights_the_kinds_and_a_located_kind_names_the_field_once_per_board() {
        let field = field();
        let trade = kind_counts(&boards(JobMixType::Trade, &field));
        let salvage = kind_counts(&boards(JobMixType::Salvage, &field));
        let combat = kind_counts(&boards(JobMixType::Combat, &field));
        assert!(
            trade[0] + trade[2] > trade[1] + trade[3] + trade[4],
            "a trade start sees mostly procurements and escorts: {trade:?}"
        );
        assert!(
            salvage[1] > salvage[0] && salvage[1] > salvage[2],
            "a salvage start sees mostly recoveries: {salvage:?}"
        );
        assert!(
            combat[3] + combat[4] > combat[0] + combat[1] + combat[2],
            "a combat start sees mostly calls and neutralizes: {combat:?}"
        );
        for counts in [trade, salvage, combat] {
            assert!(
                counts.iter().all(|count| *count > 0),
                "every kind is posted somewhere: {counts:?}"
            );
        }
        let items = catalog();
        for seed in 0..60 {
            let posted = board(seed, &station(1), &[], &items, JobMixType::Combat, &field);
            let mut named = Vec::new();
            for job in &posted {
                job.validate().unwrap();
                match &job.kind {
                    JobType::Recover {
                        wreck,
                        wreck_name,
                        item,
                        count,
                    } => {
                        let source = field.wrecks.iter().find(|w| w.id == *wreck).unwrap();
                        assert_eq!(wreck_name, &source.name);
                        assert!(source.stacks.contains(&(item.clone(), *count)));
                        assert_eq!(
                            job.reward_cr,
                            count * items.design(item).ask_cr * 2 + {
                                job.reward_cr - count * items.design(item).ask_cr * 2
                            }
                        );
                        assert!(!named.contains(wreck), "a wreck is named once per board");
                        named.push(wreck.clone());
                        assert!(job.located_body() == Some(wreck));
                        assert!(job.spawned_bodies().is_empty());
                        assert_eq!(job.item_ids().count(), 1);
                        assert!(job.title(&items).starts_with("Recover "));
                    }
                    JobType::Neutralize { hull, hull_name } => {
                        let source = field.hostiles.iter().find(|h| h.id == *hull).unwrap();
                        assert_eq!(hull_name, &source.name);
                        assert!(!named.contains(hull), "a hull is named once per board");
                        named.push(hull.clone());
                        assert_eq!(job.title(&items), format!("Neutralize {hull_name}"));
                        assert!(job.reward_cr >= NEUTRALIZE_BASE_CR + 2 * FEE_STEP_CR);
                    }
                    JobType::Escort { hauler, cargo_cr } => {
                        assert_eq!(
                            job.spawned_bodies().len() as u32,
                            1 + escort_raiders(*cargo_cr)
                        );
                        assert_eq!(job.spawned_bodies()[0], *hauler);
                        assert_eq!(
                            hauler.to_string(),
                            format!(
                                "sector_1_0_0_cluster_0_0_0_station_job_{}_0_hauler_0",
                                job.id.slot
                            )
                        );
                        assert!(job.reward_cr > cargo_cr / 2);
                    }
                    JobType::Distress { caller, .. } => {
                        assert_eq!(job.spawned_bodies(), vec![caller.clone()]);
                        assert_eq!(job.title(&items), "Answer a distress call");
                    }
                    JobType::Procure { .. } => {}
                }
            }
        }
        let calls: Vec<bool> = boards(JobMixType::Combat, &field)
            .into_iter()
            .filter_map(|job| match job.kind {
                JobType::Distress { genuine, .. } => Some(genuine),
                _ => None,
            })
            .collect();
        assert!(
            calls.iter().any(|genuine| *genuine) && calls.iter().any(|genuine| !genuine),
            "some calls are genuine and some are traps: {calls:?}"
        );
    }

    #[test]
    fn raiders_scale_with_the_cargo_up_to_the_cap() {
        assert_eq!(escort_raiders(0), 1);
        assert_eq!(escort_raiders(RAIDERS_PER_CARGO_CR - 1), 1);
        assert_eq!(escort_raiders(RAIDERS_PER_CARGO_CR), 2);
        assert_eq!(escort_raiders(u32::MAX), ESCORT_RAIDERS_MAX);
    }

    #[test]
    fn a_small_pool_repeats_an_item_and_an_empty_catalog_posts_nothing() {
        let only_food = GameItems::new([item("Rations", ItemCategoryType::Food, 8)]);
        let posted = board(
            3,
            &station(1),
            &[ClusterType::PlanetHeavy],
            &only_food,
            JobMixType::Trade,
            &JobField::default(),
        );
        assert_eq!(posted.len(), JOBS_PER_BOARD as usize);
        assert!(posted.iter().all(|job| match &job.kind {
            JobType::Procure { item, .. } => item.as_str() == "Rations",
            _ => true,
        }));
        let ammo_only = GameItems::new([item("PdcRound", ItemCategoryType::Ammo, 4)]);
        let posted = board(
            3,
            &station(1),
            &[ClusterType::RockOnly],
            &ammo_only,
            JobMixType::Trade,
            &JobField::default(),
        );
        assert_eq!(
            posted.len(),
            JOBS_PER_BOARD as usize,
            "no wanted item draws from every item"
        );
        assert!(board(
            3,
            &station(1),
            &[],
            &GameItems::default(),
            JobMixType::Trade,
            &JobField::default()
        )
        .is_empty());
    }

    /// The first procurement a trade board posts on seed 10.
    fn a_procurement() -> (GameItems, GeneratedBodyId, Job) {
        let items = catalog();
        let poster = station(1);
        let job = (0..100)
            .flat_map(|seed| {
                board(
                    seed,
                    &poster,
                    &[ClusterType::AsteroidRich],
                    &items,
                    JobMixType::Trade,
                    &JobField::default(),
                )
            })
            .find(|job| job.kind.is_procurement())
            .expect("a trade board posts a procurement");
        (items, poster, job)
    }

    #[test]
    fn accept_and_hand_in_move_a_procurement_through_held_to_completed_and_refuse_in_order() {
        let (items, poster, job) = a_procurement();
        let JobType::Procure { item, count } = job.kind.clone() else {
            unreachable!()
        };
        let mut jobs = Jobs::default();

        assert_eq!(
            plan_job_accept(&job, None, &jobs, false, true),
            Err(JobAcceptRefusalType::NotDocked)
        );
        assert_eq!(
            plan_job_accept(&job, Some(&station(2)), &jobs, false, true),
            Err(JobAcceptRefusalType::NotPostedHere)
        );
        plan_job_accept(&job, Some(&poster), &jobs, false, false)
            .expect("a procurement reads no target");
        jobs.accept(job.clone());
        assert_eq!(jobs.held(), &[job.clone()]);
        assert_eq!(
            plan_job_accept(&job, Some(&poster), &jobs, false, true),
            Err(JobAcceptRefusalType::AlreadyHeld)
        );

        let mut hold = ShipInventory::new(&items, 1_000_000, []);
        let here = JobHandInPartner {
            id: poster.clone(),
            credits_cr: 10_000,
        };
        assert_eq!(
            plan_job_hand_in(&job, None, &hold, 100, false),
            Err(JobHandInRefusalType::NotDocked)
        );
        assert_eq!(
            plan_job_hand_in(
                &job,
                Some(&JobHandInPartner {
                    id: station(2),
                    credits_cr: 10_000
                }),
                &hold,
                100,
                false
            ),
            Err(JobHandInRefusalType::WrongStation {
                needed: poster.clone()
            })
        );
        assert_eq!(
            plan_job_hand_in(&job, Some(&here), &hold, 100, false),
            Err(JobHandInRefusalType::ShortItems {
                item: item.clone(),
                need: count,
                have: 0
            })
        );
        hold.add(&items, &item, count);
        assert_eq!(
            plan_job_hand_in(
                &job,
                Some(&JobHandInPartner {
                    id: poster.clone(),
                    credits_cr: job.reward_cr - 1
                }),
                &hold,
                100,
                false
            ),
            Err(JobHandInRefusalType::StationCannotPay {
                need: job.reward_cr,
                have: job.reward_cr - 1
            })
        );
        assert_eq!(
            plan_job_hand_in(&job, Some(&here), &hold, u32::MAX, false),
            Err(JobHandInRefusalType::CreditOverflow)
        );
        let plan = plan_job_hand_in(&job, Some(&here), &hold, 100, false).unwrap();
        assert_eq!(
            plan,
            JobHandInPlan {
                goods: Some((item.clone(), count)),
                reward_cr: job.reward_cr,
                own_cr: 100 + job.reward_cr,
                station_cr: 10_000 - job.reward_cr,
            }
        );
        let done = jobs.complete(&job.id);
        assert_eq!(done, job);
        assert!(jobs.held().is_empty());
        assert_eq!(jobs.completed(), &[job.clone()]);
        assert_eq!(
            plan_job_accept(&job, Some(&poster), &jobs, false, true),
            Err(JobAcceptRefusalType::AlreadyDone)
        );
        jobs.validate().unwrap();
    }

    #[test]
    fn a_located_job_refuses_the_accept_of_a_gone_target_and_the_hand_in_until_done() {
        let items = catalog();
        let poster = station(1);
        let field = field();
        let located: Vec<Job> = (0..100)
            .flat_map(|seed| board(seed, &poster, &[], &items, JobMixType::Combat, &field))
            .filter(|job| !job.kind.is_procurement())
            .collect();
        let recover = located
            .iter()
            .find(|job| matches!(job.kind, JobType::Recover { .. }))
            .unwrap()
            .clone();
        let neutralize = located
            .iter()
            .find(|job| matches!(job.kind, JobType::Neutralize { .. }))
            .unwrap()
            .clone();
        let escort = located
            .iter()
            .find(|job| matches!(job.kind, JobType::Escort { .. }) && job.id != recover.id)
            .unwrap()
            .clone();
        let mut jobs = Jobs::default();
        for job in [&recover, &neutralize] {
            assert_eq!(
                plan_job_accept(job, Some(&poster), &jobs, false, false),
                Err(JobAcceptRefusalType::TargetGone),
                "{job:?}"
            );
            plan_job_accept(job, Some(&poster), &jobs, false, true).unwrap();
        }
        plan_job_accept(&escort, Some(&poster), &jobs, false, false)
            .expect("an escort reads no target: its hauler spawns on the accept");
        jobs.accept(recover.clone());
        jobs.accept(escort.clone());

        let hold = ShipInventory::new(&items, 1_000_000, []);
        let here = JobHandInPartner {
            id: poster.clone(),
            credits_cr: 10_000,
        };
        assert_eq!(
            plan_job_hand_in(&recover, Some(&here), &hold, 100, jobs.is_done(&recover.id)),
            Err(JobHandInRefusalType::NotDone)
        );
        jobs.mark_done(&recover.id);
        assert!(jobs.is_done(&recover.id));
        jobs.validate().unwrap();
        let plan =
            plan_job_hand_in(&recover, Some(&here), &hold, 100, jobs.is_done(&recover.id)).unwrap();
        assert_eq!(plan.goods, None, "a recovery takes nothing at the hand-in");
        assert_eq!(plan.own_cr, 100 + recover.reward_cr);
        jobs.complete(&recover.id);
        assert!(
            !jobs.is_done(&recover.id),
            "a completed job is no longer done"
        );
        assert_eq!(jobs.completed(), &[recover.clone()]);

        let dropped = jobs.drop(&escort.id);
        assert_eq!(dropped, escort);
        assert!(jobs.held().is_empty());
        assert!(
            !jobs.is_completed(&escort.id),
            "a dropped job is posted again"
        );
        plan_job_accept(&escort, Some(&poster), &jobs, false, false).unwrap();
        jobs.validate().unwrap();
    }

    #[test]
    fn validate_refuses_a_job_held_twice_held_and_completed_or_done_but_not_held() {
        let (_, poster, job) = a_procurement();
        let mut twice = Jobs::default();
        twice.held.push(job.clone());
        twice.held.push(job.clone());
        assert!(twice.validate().unwrap_err().contains("twice"));
        let mut both = Jobs::default();
        both.held.push(job.clone());
        both.completed.push(job.clone());
        assert!(both.validate().unwrap_err().contains("completed"));
        let mut loose = Jobs::default();
        loose.done.push(job.id.clone());
        assert!(loose.validate().unwrap_err().contains("not held"));
        let mut zero = job.clone();
        zero.reward_cr = 0;
        assert!(zero.validate().unwrap_err().contains("pays nothing"));
        let mut elsewhere = job.clone();
        elsewhere.target = station(2);
        assert!(elsewhere.validate().unwrap_err().contains("poster"));
        let mut foreign = Job {
            id: JobId {
                station: poster.clone(),
                slot: 1,
                posting: 0,
            },
            kind: JobType::Escort {
                hauler: body("other_hauler", 0),
                cargo_cr: 500,
            },
            target: poster,
            reward_cr: 300,
        };
        assert!(foreign.validate().unwrap_err().contains("not its own"));
        foreign.kind = JobType::Escort {
            hauler: foreign.id.spawned_body("hauler", 0),
            cargo_cr: 0,
        };
        assert!(foreign.validate().unwrap_err().contains("carries nothing"));
    }

    /// A renewed slot posts under the next posting, a different offer whose
    /// body ids differ from every earlier posting's; a slot the player holds
    /// posts the held job as it is, and no slot names the body it is about.
    #[test]
    fn a_renewed_slot_posts_a_new_offer_under_a_new_id_and_a_held_job_pins_its_slot() {
        let items = catalog();
        let poster = station(1);
        let field = field();
        let mut postings = JobPostings::default();
        let first = postings.open(&poster, 0.0).to_vec();
        assert_eq!(
            first,
            vec![
                SlotPosting {
                    posting: 0,
                    posted_at: 0.0
                };
                JOBS_PER_BOARD as usize
            ]
        );
        let mut held = Jobs::default();
        let before = station_job_board(
            10,
            &poster,
            &[],
            &items,
            JobMixType::Combat,
            &field,
            &first,
            &held,
        );
        let pinned = before[1].clone();
        held.accept(pinned.clone());
        let renewed = postings.renew(&before[0].id, 30.0);
        assert_eq!(
            renewed,
            JobId {
                station: poster.clone(),
                slot: 0,
                posting: 1
            }
        );
        assert_eq!(
            postings.slot(&renewed),
            Some(&SlotPosting {
                posting: 1,
                posted_at: 30.0
            })
        );
        assert_eq!(
            postings.slot(&before[0].id),
            None,
            "the old posting is off the slot"
        );
        let slots = postings.board(&poster).unwrap().to_vec();
        let after = station_job_board(
            10,
            &poster,
            &[],
            &items,
            JobMixType::Combat,
            &field,
            &slots,
            &held,
        );
        assert_eq!(after.len(), JOBS_PER_BOARD as usize);
        assert_eq!(after[0].id, renewed);
        assert_ne!(
            (after[0].kind.clone(), after[0].reward_cr),
            (before[0].kind.clone(), before[0].reward_cr),
            "a renewed slot draws afresh"
        );
        assert_eq!(after[1], pinned, "the held job pins its slot");
        assert_eq!(
            after[2], before[2],
            "an untouched slot posts the same offer"
        );
        for job in &after {
            job.validate().unwrap();
        }
        let old_bodies = before[0].id.possible_bodies();
        for body in renewed.possible_bodies() {
            assert!(
                !old_bodies.contains(&body),
                "a new posting mints no body id of the old one: {body}"
            );
        }
        assert_eq!(
            renewed.possible_bodies()[0].to_string(),
            "sector_1_0_0_cluster_0_0_0_station_job_0_1_hauler_0"
        );
        assert_eq!(renewed.to_string(), format!("{poster}#0.1"));
        if let Some(body) = pinned.located_body() {
            for job in &after {
                if job.id != pinned.id {
                    assert_ne!(
                        job.located_body(),
                        Some(body),
                        "no other slot names the held job's body"
                    );
                }
            }
        }
        postings.validate(30.0, &held).unwrap();
    }

    /// An offer expires exactly at its stamp plus the window; an accept of an
    /// expired offer is refused before the target check.
    #[test]
    fn an_offer_expires_at_its_stamp_plus_the_window_and_the_accept_refuses_it() {
        let rule = JobOfferRule {
            offer_seconds: 100.0,
        };
        let slot = SlotPosting {
            posting: 2,
            posted_at: 50.0,
        };
        assert_eq!(slot.remaining(rule, 60.0), 90.0);
        let noisy = SlotPosting {
            posting: 0,
            posted_at: 40.081_234_5 + 1_201.0,
        };
        assert_eq!(
            noisy.remaining(
                JobOfferRule {
                    offer_seconds: 1_200.0
                },
                noisy.posted_at
            ),
            1_200.0,
            "a read on the posting frame has the whole window, with no residue"
        );
        assert!(!slot.is_expired(rule, 149.9));
        assert!(slot.is_expired(rule, 150.0));
        assert!(slot.is_expired(rule, 200.0));

        let (_, poster, job) = a_procurement();
        let jobs = Jobs::default();
        assert_eq!(
            plan_job_accept(&job, Some(&poster), &jobs, true, false),
            Err(JobAcceptRefusalType::Expired),
            "an expired offer is refused before its target is read"
        );
        plan_job_accept(&job, Some(&poster), &jobs, false, true).unwrap();

        assert_eq!(offer_deadline_text(1_200.0), "expires in 20 min");
        assert_eq!(
            offer_deadline_text(1_200.0 + 1e-12),
            "expires in 20 min",
            "float residue on a whole minute does not add a minute"
        );
        assert_eq!(offer_deadline_text(60.0), "expires in 1 min");
        assert_eq!(offer_deadline_text(61.0), "expires in 2 min");
        assert_eq!(offer_deadline_text(59.5), "expires in 60 s");
        assert_eq!(offer_deadline_text(0.4), "expires in 1 s");
        assert_eq!(offer_deadline_text(0.0), "expired");
        assert_eq!(offer_deadline_text(-5.0), "expired");
    }

    /// The postings refuse a record no session wrote: a stamp ahead of the
    /// clock, a board of the wrong width, a held job off its slot's posting;
    /// and a renewal of a posting the slot is not on is a fault.
    #[test]
    fn postings_validate_refuses_a_stale_held_posting_and_a_stamp_ahead_of_the_clock() {
        let (_, poster, job) = a_procurement();
        let mut postings = JobPostings::default();
        postings.open(&poster, 10.0);
        let mut held = Jobs::default();
        held.accept(job.clone());
        postings.validate(10.0, &held).unwrap();
        assert!(
            postings
                .validate(9.0, &held)
                .unwrap_err()
                .contains("clock reads"),
            "a stamp ahead of the clock is refused"
        );
        postings.renew(&job.id, 20.0);
        assert!(
            postings
                .validate(20.0, &held)
                .unwrap_err()
                .contains("not its slot's current posting"),
            "a held job behind its slot's posting is refused"
        );
        let stale = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            postings.clone().renew(&job.id, 30.0)
        }));
        assert!(stale.is_err(), "renewing a stale posting panics");
        let unposted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            JobPostings::default().renew(&job.id, 30.0)
        }));
        assert!(unposted.is_err(), "renewing an unposted slot panics");
        let mut narrow = JobPostings::default();
        narrow.boards.insert(
            poster.clone(),
            vec![SlotPosting {
                posting: 0,
                posted_at: 0.0,
            }],
        );
        assert!(
            narrow
                .validate(0.0, &Jobs::default())
                .unwrap_err()
                .contains("slot(s)"),
            "a board of the wrong width is refused"
        );
        assert!(JobPostings::default().is_empty());
        postings.clear();
        assert!(postings.is_empty());
    }
}
