//! The open objectives of a world: what the player has to do now. One rule
//! ([`open_objectives`]) feeds the Map's OBJECTIVES block, the HUD's
//! objective chips and the world save header, so the three never list
//! different things.
//!
//! It reads the session resources as they stand: the held [`Jobs`], the
//! [`Chart`] for the hand-in station codes, and the role goal's
//! [`GoalProgress`] and [`GoalRecord`]. It does not know the goal kinds: a
//! milestone is the first goal line short of its need.

use crate::{
    chart::prelude::Chart,
    goal::prelude::{GoalLine, GoalProgress, GoalRecord},
    inventory::prelude::{GameItems, ShipInventory},
    jobs::prelude::{job_title, JobId, Jobs},
};

/// Glob-import surface: `use nova_gameplay::open_objectives::prelude::*`.
pub mod prelude {
    pub use super::{open_objectives, OpenObjectiveType};
}

/// One thing the player has to do now.
#[derive(Clone, Debug, PartialEq)]
pub enum OpenObjectiveType {
    /// A held job.
    Job {
        /// The held job.
        id: JobId,
        /// The code the chart and the world marker call it by: `JOB-n`.
        code: String,
        /// The one line it is called by ([`job_title`]).
        title: String,
        /// Its `have / need` ([`crate::jobs::Job::progress`]).
        have: u32,
        /// See `have`.
        need: u32,
        /// Its work is done: only the hand-in is left.
        done: bool,
        /// The chart code of the station it hands in at: `STN-n`, empty
        /// while the chart has not found that station.
        hand_in: String,
    },
    /// The role goal's first line whose `have` is short of its `need`.
    Milestone(GoalLine),
}

/// Every open objective, in order: each held job in accept order with its
/// `have / need` against `hold`, then the role goal's next unmet line. A met
/// `record` lists no milestone, and neither does an empty `goal`.
pub fn open_objectives(
    chart: &Chart,
    jobs: &Jobs,
    items: Option<&GameItems>,
    hold: Option<&ShipInventory>,
    goal: &GoalProgress,
    record: &GoalRecord,
) -> Vec<OpenObjectiveType> {
    let held = jobs.held().iter().map(|job| {
        let done = jobs.is_done(&job.id);
        let (have, need) = job.progress(hold, done);
        OpenObjectiveType::Job {
            id: job.id.clone(),
            code: jobs.mark_code(&job.id).expect("a held job has a mark code"),
            title: job_title(job, items),
            have,
            need,
            done,
            hand_in: chart.station_code(&job.target).unwrap_or_default(),
        }
    });
    let milestone = (!record.is_met())
        .then(|| goal.lines.iter().find(|line| line.have < line.need))
        .flatten()
        .map(|line| OpenObjectiveType::Milestone(line.clone()));
    held.chain(milestone).collect()
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{GeneratedBodyId, SectorCoord};

    use super::*;
    use crate::{
        inventory::prelude::{ItemCategoryType, ItemDesign},
        jobs::prelude::{Job, JobType},
    };

    /// Each held job lists with its code, its `have / need` and its hand-in
    /// station's code, then only the first unmet milestone; a met record
    /// lists no milestone.
    #[test]
    fn the_open_objectives_list_held_jobs_and_only_the_next_unmet_milestone() {
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let mut chart = Chart::default();
        chart.find_station(&station, false, false, None, 0.0);
        let job = |slot: u32, kind: JobType| Job {
            id: JobId {
                station: station.clone(),
                slot,
                posting: 0,
            },
            kind,
            target: station.clone(),
            reward_cr: 100,
        };
        let procure = job(
            0,
            JobType::Procure {
                item: "IronOre".into(),
                count: 4,
            },
        );
        let distress = job(
            1,
            JobType::Distress {
                caller: GeneratedBodyId::new(
                    SectorCoord::ORIGIN,
                    "cluster_0_0_0_station_job_1_0_caller",
                    0,
                ),
                genuine: true,
            },
        );
        let mut jobs = Jobs::default();
        jobs.accept(procure.clone());
        jobs.accept(distress.clone());
        jobs.mark_done(&distress.id);
        let items = GameItems::new([ItemDesign {
            id: "IronOre".into(),
            name: "Iron ore".into(),
            about: String::new(),
            category: ItemCategoryType::Raw,
            mass_g: 1_000,
            ask_cr: 16,
            bid_cr: 12,
        }]);
        let hold = ShipInventory::new(&items, 10_000, [("IronOre".into(), 3)]);
        let progress = GoalProgress {
            title: "A station network".to_string(),
            lines: vec![
                GoalLine::new("Trade jobs handed in", 4, 4),
                GoalLine::new("Stations served", 1, 3),
                GoalLine::new("Rings worked", 0, 2),
            ],
        };
        let open = open_objectives(
            &chart,
            &jobs,
            Some(&items),
            Some(&hold),
            &progress,
            &GoalRecord::default(),
        );
        assert_eq!(
            open,
            vec![
                OpenObjectiveType::Job {
                    id: procure.id.clone(),
                    code: "JOB-1".to_string(),
                    title: procure.title(&items),
                    have: 3,
                    need: 4,
                    done: false,
                    hand_in: "STN-1".to_string(),
                },
                OpenObjectiveType::Job {
                    id: distress.id.clone(),
                    code: "JOB-2".to_string(),
                    title: distress.title(&items),
                    have: 1,
                    need: 1,
                    done: true,
                    hand_in: "STN-1".to_string(),
                },
                OpenObjectiveType::Milestone(GoalLine::new("Stations served", 1, 3)),
            ]
        );

        assert!(
            open_objectives(
                &chart,
                &Jobs::default(),
                Some(&items),
                None,
                &progress,
                &GoalRecord { met_at: Some(90.0) },
            )
            .is_empty(),
            "a met goal lists no milestone"
        );
        assert!(
            open_objectives(
                &chart,
                &Jobs::default(),
                None,
                None,
                &GoalProgress::default(),
                &GoalRecord::default(),
            )
            .is_empty(),
            "no job and no goal list nothing"
        );
    }
}
