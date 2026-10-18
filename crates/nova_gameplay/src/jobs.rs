//! Jobs: what a living generated station posts, what the player holds, has
//! done and has handed in, and the rules that move a job between those
//! states.
//!
//! A station's board has [`JOBS_PER_BOARD`] slots. Each slot carries a
//! posting number, the play time it was posted at and the offer it posted
//! ([`SlotPosting`], kept per station in the [`JobPostings`] session
//! resource). The offer is drawn once, when the slot posts, as a function of
//! the world seed, the station's id, the slot, the posting, the cluster
//! types of its cell, the start role's [`JobMixType`], the [`JobField`] of
//! wrecks and hostile hulls standing around it and the board's other offers
//! ([`station_job_offers`]); it is kept as drawn until the slot renews,
//! whatever the field does, and saved with the world. A new offer never
//! names a body a held job or another unexpired offer names
//! ([`JobPostings::named_bodies`]), so two boards never pay for one wreck or
//! one hull. An offer the player does not hold expires
//! [`JobOfferRule::offer_seconds`] of play after its stamp; the open world
//! crate then posts the slot's next offer under a new [`JobId`]
//! ([`JobPostings::post`]). A held job never expires: its slot is pinned
//! until it is handed in or dropped. A dropped job's offer stays posted,
//! refused at the accept once its target is gone, until its window passes. The open world crate puts the board on the station root as a
//! [`JobBoard`]; the Jobs pane lists it while the player is docked there,
//! with each offer's remaining time ([`offer_deadline_text`]), and offers
//! Accept and Hand in. Both happen docked only ([`plan_job_accept`],
//! [`plan_job_hand_in`]). Every job asks something of the hull
//! ([`plan_job_hull_fit`]): the starting station's board
//! ([`StarterBoardMarker`]) lists only the jobs the current hull can do, every
//! other board lists them all and refuses the accept with the reason.
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

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

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
        escort_raiders, offer_deadline_text, plan_job_accept, plan_job_hand_in, plan_job_hull_fit,
        station_job_offers, Job, JobAcceptRefusalType, JobAccepted, JobBoard, JobField,
        JobHandInPartner, JobHandInPlan, JobHandInRefusalType, JobHull, JobHullFit,
        JobHullRefusalType, JobId, JobMixType, JobPostings, JobType, JobWreck, Jobs, SlotPosting,
        StarterBoardMarker, ESCORT_RAIDERS_MAX, JOBS_PER_BOARD, RAIDERS_PER_CARGO_CR,
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
    /// for each renewal ([`JobPostings::post`]).
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

/// One slot of a station's board: which posting it is on, when, in play
/// seconds, that posting went up, and the offer it posted.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SlotPosting {
    /// The posting number, zero for the first offer.
    pub posting: u32,
    /// The [`crate::clock::WorldClock`] reading when the posting went up.
    pub posted_at: f64,
    /// The offer the slot posted on this posting, as drawn: it stands
    /// unchanged until the slot renews, whatever the world around it does.
    pub offer: Job,
}

impl SlotPosting {
    /// The play seconds the offer has left at `now` under `rule`: zero or
    /// below once it expired. The stamp is taken off `now` first, so a read
    /// on the posting frame gives the whole window exactly, with no rounding
    /// residue from the sum.
    pub fn remaining(&self, rule: JobOfferRule, now: f64) -> f64 {
        (self.posted_at - now) + f64::from(rule.offer_seconds)
    }

    /// Whether the offer's window has passed at `now`: it has nothing left.
    pub fn is_expired(&self, rule: JobOfferRule, now: f64) -> bool {
        self.remaining(rule, now) <= 0.0
    }
}

/// Every station's slot postings: the posting number, the play-time stamp
/// and the offer of each of the [`JOBS_PER_BOARD`] slots of every board the
/// open world has posted. A session resource like [`Jobs`]: a world save
/// carries it, a Load puts it back, and it is cleared with the chart.
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

    /// Post `offers` at `now`, all of one station: a station with no board
    /// takes [`JOBS_PER_BOARD`] offers, one per slot in slot order, each on
    /// its first posting; a posted station takes each offer on its slot's
    /// next posting, and its other slots stand.
    ///
    /// # Panics
    ///
    /// On any other shape: the caller posts what [`station_job_offers`] drew
    /// for the ids it renews.
    pub fn post(&mut self, offers: Vec<Job>, now: f64) {
        let station = offers
            .first()
            .expect("JobPostings posts no offer")
            .id
            .station
            .clone();
        let Some(slots) = self.boards.get_mut(&station) else {
            assert!(
                offers.len() == JOBS_PER_BOARD as usize
                    && offers.iter().enumerate().all(|(slot, offer)| offer.id
                        == JobId {
                            station: station.clone(),
                            slot: slot as u32,
                            posting: 0,
                        }),
                "JobPostings opens '{station}' with offers other than its first postings"
            );
            let slots = offers
                .into_iter()
                .map(|offer| SlotPosting {
                    posting: 0,
                    posted_at: now,
                    offer,
                })
                .collect();
            self.boards.insert(station, slots);
            return;
        };
        for offer in offers {
            assert_eq!(
                offer.id.station, station,
                "JobPostings posts '{}' beside '{station}'",
                offer.id
            );
            let slot = slots
                .get_mut(offer.id.slot as usize)
                .unwrap_or_else(|| panic!("JobPostings posts '{}' off the board", offer.id));
            assert_eq!(
                offer.id.posting,
                slot.posting + 1,
                "JobPostings posts '{}' but the slot is on posting {}",
                offer.id,
                slot.posting
            );
            *slot = SlotPosting {
                posting: offer.id.posting,
                posted_at: now,
                offer,
            };
        }
    }

    /// Every body a new offer must not name at `now` under `rule`: what each
    /// job held in `jobs` is about and every body it put in the world, and
    /// what each offer on every board is about while its window stands and
    /// the player has not handed it in.
    pub fn named_bodies(
        &self,
        rule: JobOfferRule,
        now: f64,
        jobs: &Jobs,
    ) -> BTreeSet<GeneratedBodyId> {
        let held = jobs.held().iter().flat_map(|job| {
            job.located_body()
                .cloned()
                .into_iter()
                .chain(job.spawned_bodies())
        });
        let offered = self
            .boards
            .values()
            .flatten()
            .filter(|slot| !slot.is_expired(rule, now) && !jobs.is_completed(&slot.offer.id))
            .filter_map(|slot| slot.offer.located_body().cloned());
        held.chain(offered).collect()
    }

    /// Every item id a posted offer names.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> + '_ {
        self.boards
            .values()
            .flatten()
            .flat_map(|slot| slot.offer.item_ids())
    }

    /// Refuse a record a Load could not have built: a board with other than
    /// [`JOBS_PER_BOARD`] slots, a stamp that is not finite or stands ahead
    /// of the clock at `clock_seconds`, or an offer that is not its slot's
    /// posting or that [`Job::validate`] refuses. A held job is not read
    /// against its slot: it stands by its own rules ([`Jobs::validate`])
    /// after its posting expires or its slot renews.
    pub fn validate(&self, clock_seconds: f64) -> Result<(), String> {
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
                let id = JobId {
                    station: station.clone(),
                    slot: index as u32,
                    posting: slot.posting,
                };
                if slot.offer.id != id {
                    return Err(format!(
                        "the postings of '{station}' post '{}' on slot {index}, posting {}",
                        slot.offer.id, slot.posting
                    ));
                }
                slot.offer.validate()?;
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

/// The board a living generated station posts, on its root: its slots'
/// offers ([`JobPostings`]), in slot order. Put there by the open world the
/// frame the station stands under its cell's root with the cells around it
/// live, and put back from the same offers after a thaw or a Load. The pane
/// hides the jobs the player holds or completed.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct JobBoard(pub Vec<Job>);

impl JobBoard {
    /// The posted job `id`, if this board posted it.
    pub fn job(&self, id: &JobId) -> Option<&Job> {
        self.0.iter().find(|job| job.id == *id)
    }
}

/// The board of the station the player started docked at: the one board
/// whose list hides every job the current hull cannot do
/// ([`plan_job_hull_fit`]), so the first jobs are ones the start can finish.
/// Put beside the [`JobBoard`] by the open world from the session's starting
/// station id, and never on another board.
#[derive(Component, Debug, Default)]
pub struct StarterBoardMarker;

/// Raised by the Jobs pane once a job is held, so the open world puts
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

    /// The code the chart and the world marker call the held job `id` by:
    /// `JOB-n`, its 1-based place in the accept order. None when the player
    /// does not hold it.
    pub fn mark_code(&self, id: &JobId) -> Option<String> {
        self.held
            .iter()
            .position(|job| job.id == *id)
            .map(|index| format!("JOB-{}", index + 1))
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
    /// the job is over without a hand-in, as when its hauler is lost. Its
    /// offer stays on the board until its window passes.
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

/// The offers the slots `ids` of one station's board post in the world of
/// `seed`, the station standing in a cell whose clusters are `clusters`,
/// for a start whose boards lean by `mix`, with `field` standing around it,
/// beside the board's `standing` offers, which keep their slots: one offer
/// per id, in order, each kind drawn by the mix's weights. No offer names a
/// body a standing offer names, and a procurement asks for no item a
/// standing one asks for while the pool allows. The caller leaves out of
/// `field` every body another board or a held job names
/// ([`JobPostings::named_bodies`]).
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
/// the station id, the slot and its posting, in a fixed number, so the same
/// inputs draw the same offers in every process, and a renewed slot draws
/// afresh. A catalog with no item of the wanted categories draws from every
/// item. A reward past a credit balance saturates.
///
/// # Panics
///
/// When `ids` is empty, spans stations, repeats a slot or names one past
/// the board, and when the catalog is empty: a board always has an item to
/// ask for.
#[expect(
    clippy::too_many_arguments,
    reason = "one pure draw over the seed, the slots, the poster's cell, the catalog, the mix, the field and the standing offers"
)]
pub fn station_job_offers(
    seed: u32,
    ids: &[JobId],
    clusters: &[ClusterType],
    items: &GameItems,
    mix: JobMixType,
    field: &JobField,
    standing: &[Job],
) -> Vec<Job> {
    let station = &ids
        .first()
        .expect("station_job_offers: no slot to draw")
        .station;
    for (index, id) in ids.iter().enumerate() {
        assert!(
            id.station == *station
                && id.slot < JOBS_PER_BOARD
                && ids[..index].iter().all(|other| other.slot != id.slot),
            "station_job_offers: '{id}' is not a slot of its own on the board of '{station}'"
        );
    }
    let wanted = wanted_categories(clusters);
    let standing_items: Vec<&ItemDesignId> = standing.iter().flat_map(Job::item_ids).collect();
    let standing_bodies: Vec<&GeneratedBodyId> =
        standing.iter().filter_map(Job::located_body).collect();
    let mut pool: Vec<&ItemDesignId> = items
        .iter()
        .filter(|design| wanted.contains(&design.category))
        .map(|design| &design.id)
        .collect();
    if pool.is_empty() {
        pool = items.iter().map(|design| &design.id).collect();
    }
    assert!(
        !pool.is_empty(),
        "station_job_offers: '{station}' draws from an empty catalog"
    );
    let unasked: Vec<&ItemDesignId> = pool
        .iter()
        .copied()
        .filter(|item| !standing_items.contains(item))
        .collect();
    if !unasked.is_empty() {
        pool = unasked;
    }
    let mut wrecks: Vec<&JobWreck> = field
        .wrecks
        .iter()
        .filter(|wreck| {
            !standing_bodies.contains(&&wreck.id)
                && wreck.stacks.iter().any(|(_, count)| *count > 0)
        })
        .collect();
    let mut hostiles: Vec<&JobHull> = field
        .hostiles
        .iter()
        .filter(|hull| !standing_bodies.contains(&&hull.id))
        .collect();
    let reward = |base: u64, fee: u32| u32::try_from(base + u64::from(fee)).unwrap_or(u32::MAX);
    ids.iter()
        .map(|id| {
            let id = id.clone();
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

/// What the player's hull can do, read off its live sections and its hold
/// for the board's gate ([`plan_job_hull_fit`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobHullFit {
    /// The hold's whole capacity in grams, empty or not: the player can make
    /// room.
    pub hold_capacity_g: u32,
    /// A live docking section stands on the hull.
    pub has_clamp: bool,
    /// A live weapon section stands on the hull: a turret, a torpedo bay or
    /// a railgun.
    pub has_weapon: bool,
}

/// Why the current hull cannot do a job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobHullRefusalType {
    /// A procurement's whole stack outweighs the hold.
    HoldTooSmall {
        /// The stack's mass.
        need_g: u64,
        /// The hold's capacity.
        have_g: u32,
    },
    /// A recovery or a distress call docks with its target, and the hull has
    /// no docking section.
    NoClamp,
    /// An escort or a neutralize fights, and the hull has no weapon section.
    NoWeapon,
}

/// Plan whether a hull of `fit` can do `job`: a procurement needs hold
/// capacity for its whole stack, a recovery and a distress call need a
/// docking section, an escort and a neutralize need a weapon section. The
/// starter board hides a refused job; every other board lists it and greys
/// its Accept with the refusal.
///
/// Agent decision 2026-10-18 for owner review: the gate reads the hull, not
/// the start class and a timer, so an upgrade opens jobs by itself.
///
/// # Panics
///
/// When a procurement names an item the catalog lacks: the board drew it
/// from the catalog, and the save check refuses one before a Load.
pub fn plan_job_hull_fit(
    job: &Job,
    items: &GameItems,
    fit: JobHullFit,
) -> Result<(), JobHullRefusalType> {
    match &job.kind {
        JobType::Procure { item, count } => {
            let need_g = items.design(item).stack_mass_g(*count);
            if need_g > u64::from(fit.hold_capacity_g) {
                return Err(JobHullRefusalType::HoldTooSmall {
                    need_g,
                    have_g: fit.hold_capacity_g,
                });
            }
        }
        JobType::Recover { .. } | JobType::Distress { .. } => {
            if !fit.has_clamp {
                return Err(JobHullRefusalType::NoClamp);
            }
        }
        JobType::Escort { .. } | JobType::Neutralize { .. } => {
            if !fit.has_weapon {
                return Err(JobHullRefusalType::NoWeapon);
            }
        }
    }
    Ok(())
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

    /// The ids of `station`'s first postings, in slot order.
    fn first_ids(station: &GeneratedBodyId) -> Vec<JobId> {
        (0..JOBS_PER_BOARD)
            .map(|slot| JobId {
                station: station.clone(),
                slot,
                posting: 0,
            })
            .collect()
    }

    /// A board on the first postings with nothing standing.
    fn board(
        seed: u32,
        station: &GeneratedBodyId,
        clusters: &[ClusterType],
        items: &GameItems,
        mix: JobMixType,
        field: &JobField,
    ) -> Vec<Job> {
        station_job_offers(seed, &first_ids(station), clusters, items, mix, field, &[])
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
    fn a_small_pool_repeats_an_item_and_an_empty_catalog_is_a_fault() {
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
        let empty = std::panic::catch_unwind(|| {
            board(
                3,
                &station(1),
                &[],
                &GameItems::default(),
                JobMixType::Trade,
                &JobField::default(),
            )
        });
        assert!(empty.is_err(), "an empty catalog has nothing to post");
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
    /// body ids differ from every earlier posting's, beside the board's
    /// standing offers, which keep their slots and their bodies.
    #[test]
    fn a_renewed_slot_posts_a_new_offer_under_a_new_id_beside_the_standing_offers() {
        let items = catalog();
        let poster = station(1);
        let field = field();
        let before = board(10, &poster, &[], &items, JobMixType::Combat, &field);
        let mut postings = JobPostings::default();
        postings.post(before.clone(), 0.0);
        let first: Vec<(u32, f64, &Job)> = postings
            .board(&poster)
            .unwrap()
            .iter()
            .map(|slot| (slot.posting, slot.posted_at, &slot.offer))
            .collect();
        assert_eq!(
            first,
            before
                .iter()
                .map(|offer| (0, 0.0, offer))
                .collect::<Vec<_>>()
        );
        let renewed = JobId {
            station: poster.clone(),
            slot: 0,
            posting: 1,
        };
        let standing = before[1..].to_vec();
        let drawn = station_job_offers(
            10,
            std::slice::from_ref(&renewed),
            &[],
            &items,
            JobMixType::Combat,
            &field,
            &standing,
        );
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].id, renewed);
        assert_ne!(
            (drawn[0].kind.clone(), drawn[0].reward_cr),
            (before[0].kind.clone(), before[0].reward_cr),
            "a renewed slot draws afresh"
        );
        drawn[0].validate().unwrap();
        for job in &standing {
            if let Some(body) = job.located_body() {
                assert_ne!(
                    drawn[0].located_body(),
                    Some(body),
                    "the new offer names no body a standing offer names"
                );
            }
        }
        postings.post(drawn.clone(), 30.0);
        assert_eq!(
            postings.slot(&renewed),
            Some(&SlotPosting {
                posting: 1,
                posted_at: 30.0,
                offer: drawn[0].clone(),
            })
        );
        assert_eq!(
            postings.slot(&before[0].id),
            None,
            "the old posting is off the slot"
        );
        for job in &standing {
            assert_eq!(
                postings.slot(&job.id).map(|slot| &slot.offer),
                Some(job),
                "a standing slot keeps its offer"
            );
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
        postings.validate(30.0).unwrap();
    }

    /// A posted offer is the record: a dropped recovery whose wreck is gone
    /// stays on its slot as it was posted, refused at the accept, while
    /// another slot renews over the changed field; at its own expiry the
    /// slot renews and the new offer names the wreck no more.
    #[test]
    fn a_standing_offer_keeps_its_job_until_its_slot_renews_whatever_the_field_does() {
        let items = catalog();
        let poster = station(1);
        let field = field();
        let (before, k) = (0..200)
            .find_map(|seed| {
                let posted = board(seed, &poster, &[], &items, JobMixType::Salvage, &field);
                let k = posted
                    .iter()
                    .position(|job| matches!(job.kind, JobType::Recover { .. }))?;
                Some((posted, k))
            })
            .expect("a salvage board posts a recovery");
        let recovery = before[k].clone();
        let wreck = recovery.located_body().unwrap().clone();
        let mut postings = JobPostings::default();
        postings.post(before.clone(), 0.0);
        let mut jobs = Jobs::default();
        jobs.accept(recovery.clone());
        assert_eq!(jobs.drop(&recovery.id), recovery);

        let mut gone = field.clone();
        gone.wrecks.retain(|candidate| candidate.id != wreck);
        let other = (k + 1) % JOBS_PER_BOARD as usize;
        let next = JobId {
            posting: 1,
            ..before[other].id.clone()
        };
        let standing: Vec<Job> = before
            .iter()
            .filter(|job| job.id != before[other].id)
            .cloned()
            .collect();
        let drawn = station_job_offers(
            7,
            std::slice::from_ref(&next),
            &[],
            &items,
            JobMixType::Salvage,
            &gone,
            &standing,
        );
        postings.post(drawn, 30.0);
        for job in &standing {
            assert_eq!(
                postings.slot(&job.id).map(|slot| &slot.offer),
                Some(job),
                "a slot that did not renew keeps its offer over a changed field"
            );
        }
        assert_eq!(
            plan_job_accept(&recovery, Some(&poster), &jobs, false, false),
            Err(JobAcceptRefusalType::TargetGone),
            "the dropped recovery stays posted and greys on its gone wreck"
        );

        let renewed = JobId {
            posting: 1,
            ..recovery.id.clone()
        };
        let standing: Vec<Job> = postings
            .board(&poster)
            .unwrap()
            .iter()
            .map(|slot| slot.offer.clone())
            .filter(|job| job.id != recovery.id)
            .collect();
        let drawn = station_job_offers(
            7,
            std::slice::from_ref(&renewed),
            &[],
            &items,
            JobMixType::Salvage,
            &gone,
            &standing,
        );
        assert_ne!(drawn[0].located_body(), Some(&wreck));
        postings.post(drawn, 120.0);
        assert!(postings.slot(&recovery.id).is_none());
        postings.validate(120.0).unwrap();
    }

    /// A new offer leaves out what each held job is about and every body it
    /// put in the world, and what each unexpired offer on every board names;
    /// an expired or handed-in offer frees its body, and a board left with
    /// no candidate posts procurements.
    #[test]
    fn no_new_offer_names_a_body_a_held_job_or_an_unexpired_offer_names() {
        let items = catalog();
        let rule = JobOfferRule {
            offer_seconds: 100.0,
        };
        let a = station(1);
        let b = station(2);
        let raider = |job: &Job| job.raider_ids();
        let escort = (0..200)
            .flat_map(|seed| board(seed, &station(3), &[], &items, JobMixType::Trade, &field()))
            .find(|job| matches!(job.kind, JobType::Escort { .. }))
            .expect("a trade board posts an escort");
        let mut field = field();
        for id in raider(&escort) {
            field.hostiles.push(JobHull {
                name: id.to_string(),
                id,
            });
        }
        let posted_a = board(4, &a, &[], &items, JobMixType::Salvage, &field);
        let mut postings = JobPostings::default();
        postings.post(posted_a.clone(), 0.0);
        let mut jobs = Jobs::default();
        jobs.accept(escort.clone());

        let named = postings.named_bodies(rule, 50.0, &jobs);
        for body in escort.spawned_bodies() {
            assert!(named.contains(&body), "a held job's body is named: {body}");
        }
        for body in posted_a.iter().filter_map(Job::located_body) {
            assert!(
                named.contains(body),
                "an unexpired offer's body is named: {body}"
            );
        }
        let mut open = field.clone();
        open.wrecks.retain(|wreck| !named.contains(&wreck.id));
        open.hostiles.retain(|hull| !named.contains(&hull.id));
        for seed in 0..60 {
            for job in board(seed, &b, &[], &items, JobMixType::Salvage, &open) {
                if let Some(body) = job.located_body() {
                    assert!(!named.contains(body), "'{}' names '{body}' again", job.id);
                }
            }
        }

        let none = JobField::default();
        for job in board(4, &b, &[], &items, JobMixType::Combat, &none) {
            job.validate().unwrap();
            assert!(
                matches!(
                    job.kind,
                    JobType::Procure { .. } | JobType::Escort { .. } | JobType::Distress { .. }
                ),
                "a board with no candidate names no body: {:?}",
                job.kind
            );
        }

        let expired = postings.named_bodies(rule, 100.0, &Jobs::default());
        assert!(expired.is_empty(), "an expired offer frees its body");
        if let Some(offer) = posted_a.iter().find(|job| job.located_body().is_some()) {
            let mut handed = Jobs::default();
            handed.accept(offer.clone());
            handed.complete(&offer.id);
            assert!(
                !postings
                    .named_bodies(rule, 50.0, &handed)
                    .contains(offer.located_body().unwrap()),
                "a handed-in offer frees its body"
            );
        }
    }

    /// An offer expires exactly at its stamp plus the window; an accept of an
    /// expired offer is refused before the target check.
    #[test]
    fn an_offer_expires_at_its_stamp_plus_the_window_and_the_accept_refuses_it() {
        let (_, poster, job) = a_procurement();
        let rule = JobOfferRule {
            offer_seconds: 100.0,
        };
        let slot = SlotPosting {
            posting: 0,
            posted_at: 50.0,
            offer: job.clone(),
        };
        assert_eq!(slot.remaining(rule, 60.0), 90.0);
        let noisy = SlotPosting {
            posted_at: 40.081_234_5 + 1_201.0,
            ..slot.clone()
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
    /// clock, a board of the wrong width, an offer off its slot's posting or
    /// one no board could post; and a post of any other shape than a whole
    /// first board or a slot's next posting is a fault.
    #[test]
    fn postings_validate_refuses_an_offer_off_its_slot_and_a_stamp_ahead_of_the_clock() {
        let items = catalog();
        let poster = station(1);
        let first = board(10, &poster, &[], &items, JobMixType::Trade, &field());
        let mut postings = JobPostings::default();
        postings.post(first.clone(), 10.0);
        postings.validate(10.0).unwrap();
        assert!(
            postings.validate(9.0).unwrap_err().contains("clock reads"),
            "a stamp ahead of the clock is refused"
        );
        let mut moved = postings.clone();
        moved.boards.get_mut(&poster).unwrap()[0].offer = first[1].clone();
        assert!(
            moved.validate(10.0).unwrap_err().contains("on slot 0"),
            "an offer off its slot is refused"
        );
        let mut unpaid = postings.clone();
        unpaid.boards.get_mut(&poster).unwrap()[0].offer.reward_cr = 0;
        assert!(
            unpaid.validate(10.0).unwrap_err().contains("pays nothing"),
            "an offer no board could post is refused"
        );
        let mut narrow = postings.clone();
        narrow.boards.get_mut(&poster).unwrap().pop();
        assert!(
            narrow.validate(10.0).unwrap_err().contains("slot(s)"),
            "a board of the wrong width is refused"
        );
        let again = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            postings.clone().post(first.clone(), 20.0);
        }));
        assert!(
            again.is_err(),
            "posting a slot's current posting again panics"
        );
        let partial = std::panic::catch_unwind(|| {
            JobPostings::default().post(first[..1].to_vec(), 20.0);
        });
        assert!(partial.is_err(), "opening a board with one slot panics");
        let nothing = std::panic::catch_unwind(|| JobPostings::default().post(Vec::new(), 20.0));
        assert!(nothing.is_err(), "posting nothing panics");
        assert!(JobPostings::default().is_empty());
        postings.clear();
        assert!(postings.is_empty());
    }

    #[test]
    fn the_hull_gate_refuses_each_kind_the_hull_cannot_do_and_passes_a_fit_hull() {
        let items = catalog();
        let poster = station(0);
        let id = |slot| JobId {
            station: poster.clone(),
            slot,
            posting: 0,
        };
        let job = |slot, kind| Job {
            id: id(slot),
            kind,
            target: poster.clone(),
            reward_cr: 100,
        };
        let procure = job(
            0,
            JobType::Procure {
                item: "IronOre".into(),
                count: 4,
            },
        );
        let recover = job(
            1,
            JobType::Recover {
                wreck: body("wreck", 0),
                wreck_name: "Vell derelict".into(),
                item: "FrameSpar".into(),
                count: 2,
            },
        );
        let escort = job(
            2,
            JobType::Escort {
                hauler: id(2).spawned_body("hauler", 0),
                cargo_cr: 600,
            },
        );
        let distress = job(
            0,
            JobType::Distress {
                caller: id(0).spawned_body("caller", 0),
                genuine: true,
            },
        );
        let neutralize = job(
            1,
            JobType::Neutralize {
                hull: body("hull", 0),
                hull_name: "Harrow".into(),
            },
        );
        let bare = JobHullFit {
            hold_capacity_g: 3_999,
            has_clamp: false,
            has_weapon: false,
        };
        assert_eq!(
            plan_job_hull_fit(&procure, &items, bare),
            Err(JobHullRefusalType::HoldTooSmall {
                need_g: 4_000,
                have_g: 3_999
            }),
            "four 1 kg units outweigh a 3,999 g hold"
        );
        assert_eq!(
            plan_job_hull_fit(&recover, &items, bare),
            Err(JobHullRefusalType::NoClamp)
        );
        assert_eq!(
            plan_job_hull_fit(&distress, &items, bare),
            Err(JobHullRefusalType::NoClamp)
        );
        assert_eq!(
            plan_job_hull_fit(&escort, &items, bare),
            Err(JobHullRefusalType::NoWeapon)
        );
        assert_eq!(
            plan_job_hull_fit(&neutralize, &items, bare),
            Err(JobHullRefusalType::NoWeapon)
        );
        let fit = JobHullFit {
            hold_capacity_g: 4_000,
            has_clamp: true,
            has_weapon: true,
        };
        for job in [&procure, &recover, &escort, &distress, &neutralize] {
            assert_eq!(
                plan_job_hull_fit(job, &items, fit),
                Ok(()),
                "a hull with the hold, a clamp and a weapon does '{}'",
                job.id
            );
        }
        let armed_only = JobHullFit {
            hold_capacity_g: 4_000,
            has_clamp: false,
            has_weapon: true,
        };
        assert_eq!(plan_job_hull_fit(&escort, &items, armed_only), Ok(()));
        assert_eq!(
            plan_job_hull_fit(&recover, &items, armed_only),
            Err(JobHullRefusalType::NoClamp),
            "a weapon does not stand in for a clamp"
        );
    }
}
