//! Jobs: what a living generated station posts, what the player holds and
//! has completed, and the rules that move a job between those states.
//!
//! A station's board is a pure function of the world seed, the station's id
//! and the cluster types of its cell ([`station_job_board`]), so the same
//! station posts the same board on every visit, in every process and after
//! a Load. The open world crate puts the board on the station root as a
//! [`JobBoard`]; the Station pane lists it while the player is docked there
//! and offers Accept and Hand in. Both happen docked only
//! ([`plan_job_accept`], [`plan_job_hand_in`]).
//!
//! [`Jobs`] is the player's record: the jobs held and the ids completed. It
//! is a session resource like the chart: a world save carries it, a Load
//! puts it back, and the Map's chart view marks every held job in its
//! target's cell. It lives here because the interface cannot depend on the
//! world crates. It holds no entity: a job names bodies by
//! [`GeneratedBodyId`], so it outlives the cell that holds its poster.

use std::fmt;

use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use nova_events::prelude::GeneratedBodyId;
use rand::{RngExt, SeedableRng};

use crate::{
    chart::prelude::ClusterType,
    hash::Fnv64,
    inventory::prelude::{GameItems, ItemCategoryType, ItemDesignId, ShipInventory},
};

/// Glob-import surface: `use nova_gameplay::jobs::prelude::*`.
pub mod prelude {
    pub use super::{
        plan_job_accept, plan_job_hand_in, station_job_board, Job, JobAcceptRefusalType, JobBoard,
        JobHandInPartner, JobHandInPlan, JobHandInRefusalType, JobId, JobType, Jobs,
        JOBS_PER_BOARD,
    };
}

/// How many jobs one station's board posts.
///
/// Agent decision 2026-10-16 for owner review: three, so the board reads at
/// a glance and a station never carries the player's whole plan. No job
/// expires until the play-time clock lands.
pub const JOBS_PER_BOARD: u32 = 3;

/// The fewest and the most units one procurement asks for, both included.
const PROCURE_COUNT: (u32, u32) = (2, 8);
/// The flat fee a job adds over its goods, in multiples of this many credits,
/// one to [`PROCURE_FEE_STEPS`] times.
const PROCURE_FEE_STEP_CR: u32 = 50;
/// How many fee steps a job may add.
const PROCURE_FEE_STEPS: u32 = 5;

/// The identity of a posted job: the station that posted it and its slot on
/// that station's board. The poster is read off the id, never kept twice.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct JobId {
    /// The generated station that posted the job.
    pub station: GeneratedBodyId,
    /// The job's slot on that station's board, below [`JOBS_PER_BOARD`].
    pub slot: u32,
}

impl fmt::Display for JobId {
    /// `<station id>#<slot>`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}#{}", self.station, self.slot)
    }
}

/// What a job asks the player to do.
///
/// One kind here: the located kinds (recover, escort, distress call,
/// neutralize) are the next strand's and add variants with a target of
/// their own.
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
    /// The body the job is about. For a procurement, the station the goods
    /// are handed in at: the poster.
    pub target: GeneratedBodyId,
    /// What the poster pays on the hand-in, above zero.
    pub reward_cr: u32,
}

impl Job {
    /// The station that posted the job.
    pub fn posted_by(&self) -> &GeneratedBodyId {
        &self.id.station
    }

    /// The one line a board or a list calls the job by: `Bring 4 Iron ore`.
    /// An item the catalog lacks reads by its id; the save check refuses one
    /// before a Load.
    pub fn title(&self, items: &GameItems) -> String {
        match &self.kind {
            JobType::Procure { item, count } => {
                let name = items
                    .get(item)
                    .map_or_else(|| item.to_string(), |design| design.name.clone());
                format!("Bring {count} {name}")
            }
        }
    }

    /// Every item id the job names.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> {
        match &self.kind {
            JobType::Procure { item, .. } => std::iter::once(item),
        }
    }

    /// Refuse a job no board could have posted: a zero count, a zero reward,
    /// a slot past the board, or a procurement whose target is not its
    /// poster.
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
        match &self.kind {
            JobType::Procure { item, count } => {
                if *count == 0 {
                    return Err(format!("the job '{}' asks for 0 of '{item}'", self.id));
                }
                if self.target != self.id.station {
                    return Err(format!(
                        "the procurement '{}' hands in at '{}', not at its poster",
                        self.id, self.target
                    ));
                }
            }
        }
        Ok(())
    }
}

/// The board a living generated station posts, on its root. Put there by
/// the open world the frame the station stands under its cell's root, and
/// regenerated the same after a thaw or a Load. The pane hides the jobs the
/// player holds or completed; the board itself never changes.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct JobBoard(pub Vec<Job>);

impl JobBoard {
    /// The posted job `id`, if this board posted it.
    pub fn job(&self, id: &JobId) -> Option<&Job> {
        self.0.iter().find(|job| job.id == *id)
    }
}

/// The player's jobs: the jobs held, in accept order, and the ids
/// completed, in hand-in order.
///
/// Cleared with the chart whenever the world on hand goes: an unload, a
/// Retry, a replaced scenario. A Load restores the saved record on the
/// frame the world arms.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Jobs {
    held: Vec<Job>,
    completed: Vec<JobId>,
}

impl Jobs {
    /// Whether the player holds no job and completed none.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty() && self.completed.is_empty()
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.held.clear();
        self.completed.clear();
    }

    /// The jobs held, in accept order.
    pub fn held(&self) -> &[Job] {
        &self.held
    }

    /// The jobs completed, in hand-in order.
    pub fn completed(&self) -> &[JobId] {
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

    /// Whether the player completed the job `id`.
    pub fn is_completed(&self, id: &JobId) -> bool {
        self.completed.contains(id)
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

    /// Move the held job `id` to the completed list and give it back.
    ///
    /// # Panics
    ///
    /// When the job is not held: the caller plans a hand-in first
    /// ([`plan_job_hand_in`]).
    pub fn complete(&mut self, id: &JobId) -> Job {
        let at = self
            .held
            .iter()
            .position(|job| job.id == *id)
            .unwrap_or_else(|| panic!("Jobs completes '{id}', which is not held"));
        let job = self.held.remove(at);
        self.completed.push(id.clone());
        job
    }

    /// Every item id a held job names.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemDesignId> + '_ {
        self.held.iter().flat_map(Job::item_ids)
    }

    /// Refuse a record a Load could not have built: a job held or completed
    /// twice, held and completed at once, or a job [`Job::validate`] refuses.
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
        for (index, id) in self.completed.iter().enumerate() {
            if self.completed[..index].contains(id) {
                return Err(format!("the jobs list '{id}' completed twice"));
            }
        }
        Ok(())
    }
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
/// in a cell whose clusters are `clusters`: [`JOBS_PER_BOARD`]
/// procurements of items from the categories the cell asks for
/// ([`wanted_categories`]), distinct while the pool allows, each for
/// [`PROCURE_COUNT`] units at the item's ask plus half, plus a flat fee of
/// one to [`PROCURE_FEE_STEPS`] times [`PROCURE_FEE_STEP_CR`].
///
/// Every draw comes off one `WyRand` seeded from the world seed and the
/// station id, so the board is the same in every process and after a Load.
/// A catalog with no item of the wanted categories draws from every item;
/// an empty catalog posts nothing. A reward past a credit balance saturates.
pub fn station_job_board(
    seed: u32,
    station: &GeneratedBodyId,
    clusters: &[ClusterType],
    items: &GameItems,
) -> Vec<Job> {
    let key = Fnv64::new()
        .write(&seed.to_le_bytes())
        .write(b"station_jobs")
        .write(station.to_string().as_bytes())
        .finish();
    let mut rng = WyRand::from_seed(key.to_le_bytes());
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
    (0..JOBS_PER_BOARD)
        .map(|slot| {
            let at = rng.random_range(0..pool.len());
            let item = if pool.len() > 1 {
                pool.swap_remove(at)
            } else {
                pool[at]
            };
            let count: u32 = rng.random_range(PROCURE_COUNT.0..=PROCURE_COUNT.1);
            let fee = rng.random_range(1..=PROCURE_FEE_STEPS) * PROCURE_FEE_STEP_CR;
            let goods = u64::from(count) * u64::from(items.design(item).ask_cr) * 3 / 2;
            let reward_cr = u32::try_from(goods + u64::from(fee)).unwrap_or(u32::MAX);
            Job {
                id: JobId {
                    station: station.clone(),
                    slot,
                },
                kind: JobType::Procure {
                    item: item.clone(),
                    count,
                },
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
}

/// Plan accepting `job` while docked at the generated station `docked`,
/// `None` while the ship is not docked at a living station posting jobs,
/// against the player's `jobs`. A refusal changes nothing.
pub fn plan_job_accept(
    job: &Job,
    docked: Option<&GeneratedBodyId>,
    jobs: &Jobs,
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
    /// The hold carries fewer units than the job asks.
    ShortItems {
        /// The item wanted.
        item: ItemDesignId,
        /// Units wanted.
        need: u32,
        /// Units in the hold.
        have: u32,
    },
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

/// What an allowed hand-in does: the units it takes and the balances after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobHandInPlan {
    /// The item taken out of the hold.
    pub item: ItemDesignId,
    /// Units taken.
    pub count: u32,
    /// The reward paid.
    pub reward_cr: u32,
    /// The player's balance after: before, plus the reward.
    pub own_cr: u32,
    /// The station's balance after: before, less the reward.
    pub station_cr: u32,
}

/// Plan handing `job` in while docked with `partner`, `None` while the ship
/// is not docked at a living station posting jobs, from `hold` with a
/// balance of `own_cr`. Checks run in [`JobHandInRefusalType`] order. A
/// refusal changes nothing.
pub fn plan_job_hand_in(
    job: &Job,
    partner: Option<&JobHandInPartner>,
    hold: &ShipInventory,
    own_cr: u32,
) -> Result<JobHandInPlan, JobHandInRefusalType> {
    let partner = partner.ok_or(JobHandInRefusalType::NotDocked)?;
    if partner.id != job.target {
        return Err(JobHandInRefusalType::WrongStation {
            needed: job.target.clone(),
        });
    }
    let JobType::Procure { item, count } = &job.kind;
    let have = hold.count(item);
    if have < *count {
        return Err(JobHandInRefusalType::ShortItems {
            item: item.clone(),
            need: *count,
            have,
        });
    }
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
        item: item.clone(),
        count: *count,
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

    #[test]
    fn a_board_is_the_same_for_its_seed_and_station_and_asks_the_cell_s_categories() {
        let items = catalog();
        let rocks = [ClusterType::AsteroidRich];
        let board = station_job_board(10, &station(1), &rocks, &items);
        assert_eq!(board.len(), JOBS_PER_BOARD as usize);
        assert_eq!(
            board,
            station_job_board(10, &station(1), &rocks, &items),
            "the same seed and station post the same board"
        );
        assert_ne!(
            board,
            station_job_board(11, &station(1), &rocks, &items),
            "another seed posts another board"
        );
        assert_ne!(
            board,
            station_job_board(10, &station(2), &rocks, &items),
            "another station posts another board"
        );
        let mut seen = Vec::new();
        for (slot, job) in board.iter().enumerate() {
            job.validate().unwrap();
            assert_eq!(job.id.slot, slot as u32);
            assert_eq!(job.posted_by(), &station(1));
            let JobType::Procure { item, count } = &job.kind;
            assert_eq!(
                items.design(item).category,
                ItemCategoryType::Raw,
                "an asteroid cell asks for raw material"
            );
            assert!((PROCURE_COUNT.0..=PROCURE_COUNT.1).contains(count));
            let goods = count * items.design(item).ask_cr * 3 / 2;
            assert!(
                job.reward_cr > goods && job.reward_cr <= goods + PROCURE_FEE_STEPS * 50,
                "the reward pays the goods and a fee: {job:?}"
            );
            assert!(!seen.contains(item), "three raw items, each once");
            seen.push(item.clone());
        }
        let derelicts = station_job_board(10, &station(1), &[ClusterType::DerelictField], &items);
        for job in &derelicts {
            let JobType::Procure { item, .. } = &job.kind;
            assert!(
                matches!(
                    items.design(item).category,
                    ItemCategoryType::Parts | ItemCategoryType::Repair
                ),
                "a derelict field asks for parts and plates: {job:?}"
            );
        }
    }

    #[test]
    fn a_small_pool_repeats_an_item_and_an_empty_catalog_posts_nothing() {
        let only_food = GameItems::new([item("Rations", ItemCategoryType::Food, 8)]);
        let board = station_job_board(3, &station(1), &[ClusterType::PlanetHeavy], &only_food);
        assert_eq!(board.len(), JOBS_PER_BOARD as usize);
        assert!(board.iter().all(|job| {
            let JobType::Procure { item, .. } = &job.kind;
            item.as_str() == "Rations"
        }));
        let ammo_only = GameItems::new([item("PdcRound", ItemCategoryType::Ammo, 4)]);
        let board = station_job_board(3, &station(1), &[ClusterType::RockOnly], &ammo_only);
        assert_eq!(
            board.len(),
            JOBS_PER_BOARD as usize,
            "no wanted item draws from every item"
        );
        assert!(station_job_board(3, &station(1), &[], &GameItems::default()).is_empty());
    }

    #[test]
    fn accept_and_hand_in_move_a_job_through_held_to_completed_and_refuse_in_order() {
        let items = catalog();
        let poster = station(1);
        let board = station_job_board(10, &poster, &[ClusterType::AsteroidRich], &items);
        let job = board[0].clone();
        let JobType::Procure { item, count } = job.kind.clone();
        let mut jobs = Jobs::default();

        assert_eq!(
            plan_job_accept(&job, None, &jobs),
            Err(JobAcceptRefusalType::NotDocked)
        );
        assert_eq!(
            plan_job_accept(&job, Some(&station(2)), &jobs),
            Err(JobAcceptRefusalType::NotPostedHere)
        );
        plan_job_accept(&job, Some(&poster), &jobs).unwrap();
        jobs.accept(job.clone());
        assert_eq!(jobs.held(), &[job.clone()]);
        assert_eq!(
            plan_job_accept(&job, Some(&poster), &jobs),
            Err(JobAcceptRefusalType::AlreadyHeld)
        );

        let mut hold = ShipInventory::new(&items, 1_000_000, []);
        let here = JobHandInPartner {
            id: poster.clone(),
            credits_cr: 10_000,
        };
        assert_eq!(
            plan_job_hand_in(&job, None, &hold, 100),
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
                100
            ),
            Err(JobHandInRefusalType::WrongStation {
                needed: poster.clone()
            })
        );
        assert_eq!(
            plan_job_hand_in(&job, Some(&here), &hold, 100),
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
                100
            ),
            Err(JobHandInRefusalType::StationCannotPay {
                need: job.reward_cr,
                have: job.reward_cr - 1
            })
        );
        assert_eq!(
            plan_job_hand_in(&job, Some(&here), &hold, u32::MAX),
            Err(JobHandInRefusalType::CreditOverflow)
        );
        let plan = plan_job_hand_in(&job, Some(&here), &hold, 100).unwrap();
        assert_eq!(
            plan,
            JobHandInPlan {
                item: item.clone(),
                count,
                reward_cr: job.reward_cr,
                own_cr: 100 + job.reward_cr,
                station_cr: 10_000 - job.reward_cr,
            }
        );
        let done = jobs.complete(&job.id);
        assert_eq!(done, job);
        assert!(jobs.held().is_empty());
        assert_eq!(jobs.completed(), &[job.id.clone()]);
        assert_eq!(
            plan_job_accept(&job, Some(&poster), &jobs),
            Err(JobAcceptRefusalType::AlreadyDone)
        );
        jobs.validate().unwrap();
    }

    #[test]
    fn validate_refuses_a_job_held_twice_or_held_and_completed() {
        let items = catalog();
        let poster = station(1);
        let board = station_job_board(10, &poster, &[ClusterType::AsteroidRich], &items);
        let mut twice = Jobs::default();
        twice.held.push(board[0].clone());
        twice.held.push(board[0].clone());
        assert!(twice.validate().unwrap_err().contains("twice"));
        let mut both = Jobs::default();
        both.held.push(board[1].clone());
        both.completed.push(board[1].id.clone());
        assert!(both.validate().unwrap_err().contains("completed"));
        let mut zero = board[2].clone();
        zero.reward_cr = 0;
        assert!(zero.validate().unwrap_err().contains("pays nothing"));
        let mut elsewhere = board[2].clone();
        elsewhere.target = station(2);
        assert!(elsewhere.validate().unwrap_err().contains("poster"));
    }
}
