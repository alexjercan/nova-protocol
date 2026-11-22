//! SCRATCH reproduction for the role-goal hand-in design. Delete after the
//! design run. Reads the shipped base start roles, hands in each role's own
//! work one job at a time through the production `Jobs` API, and prints what
//! the production rules (`goal_progress`, `open_objectives`) show after each
//! hand-in, plus whether the record would stamp.

use nova_events::prelude::{GeneratedBodyId, SectorCoord};
use nova_gameplay::prelude::{
    open_objectives, Chart, GameItems, GoalRecord, ItemCategoryType, ItemDesign, Job, JobId,
    JobReward, JobType, Jobs, OpenObjectiveType,
};
use nova_scenario::prelude::{RoleGoalType, StartRole};
use nova_world_base::prelude::{goal_progress, GoalFacts};

#[derive(serde::Deserialize)]
enum Entry {
    StartRole(StartRole),
}

fn items() -> GameItems {
    GameItems::new([ItemDesign {
        id: "IronOre".into(),
        name: "Iron ore".to_string(),
        about: String::new(),
        category: ItemCategoryType::Raw,
        mass_g: 10_000,
        price_lot: 1,
        ask_cr: 16,
        bid_cr: 12,
    }])
}

fn job(index: u32, kind: JobType) -> Job {
    let station = GeneratedBodyId::new(SectorCoord::new(index as i32 + 1, 0, 0), "cluster_station", 0);
    Job {
        id: JobId::slot(station.clone(), 0, 0),
        kind,
        target: station,
        leg_m: 0,
        threat: 0,
        reward: JobReward::flat(1_136),
    }
}

/// The role's own work as one job, or `None` for the outlaw, whose line
/// counts hulls taken, not hand-ins.
fn own_work(kind: &RoleGoalType, index: u32) -> Option<JobType> {
    let cell = SectorCoord::new(index as i32 + 1, 0, 0);
    let body = |name: &str| GeneratedBodyId::new(cell, name, index);
    Some(match kind {
        RoleGoalType::OreDeliveries { .. } => JobType::Procure {
            item: "IronOre".into(),
            count: 8,
        },
        RoleGoalType::Recoveries { .. } => JobType::Recover {
            wreck: body("cluster_wreck"),
            wreck_name: format!("Wreck {index}"),
            item: "IronOre".into(),
            count: 2,
        },
        RoleGoalType::CargoRuns { .. } => JobType::Procure {
            item: "IronOre".into(),
            count: 8,
        },
        RoleGoalType::KillList { .. } => JobType::Neutralize {
            hull: body("cluster_hull"),
            hull_name: format!("Harrow {index}"),
        },
        RoleGoalType::Patrols { .. } => JobType::Distress {
            caller: body("cluster_caller"),
        },
        RoleGoalType::Outlaw { .. } => return None,
    })
}

fn shown(objectives: &[OpenObjectiveType]) -> String {
    objectives
        .iter()
        .map(|objective| match objective {
            OpenObjectiveType::Milestone(line) => {
                format!("Milestone({} {} / {})", line.label, line.have, line.need)
            }
            OpenObjectiveType::Job { code, title, .. } => format!("Job({code} {title})"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn scratch_a_met_role_line_hands_over_to_the_station_line_with_nothing_between() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/base/roles/base.content.ron"
    ))
    .expect("the base roles file reads");
    let roles: Vec<Entry> = ron::from_str(&text).expect("the base roles parse");
    let items = items();
    let chart = Chart::default();
    let record = GoalRecord::default();
    for Entry::StartRole(role) in roles {
        println!("== role {} goal '{}' {:?}", role.id, role.goal.title, role.goal.kind);
        let mut jobs = Jobs::default();
        let mut hulls_taken = 0;
        for index in 0..4u32 {
            let facts = |jobs: &Jobs, hulls_taken| {
                goal_progress(
                    &role.goal,
                    &GoalFacts {
                        completed: jobs.completed(),
                        owned_stations: 0,
                        items: &items,
                        hulls_taken,
                    },
                )
            };
            let before = facts(&jobs, hulls_taken);
            let before_shown = open_objectives(&chart, &jobs, Some(&items), None, &before, &record);
            match own_work(&role.goal.kind, index) {
                Some(kind) => {
                    let job = job(index, kind);
                    let id = job.id.clone();
                    jobs.accept(job);
                    jobs.mark_done(&id);
                    jobs.complete(&id);
                }
                None => hulls_taken += 1,
            }
            let after = facts(&jobs, hulls_taken);
            let after_shown = open_objectives(&chart, &jobs, Some(&items), None, &after, &record);
            // track_role_goal (goal.rs:214) stamps only when every line is met.
            let would_stamp = after.is_met() && !record.is_met();
            println!(
                "  work {}: lines {:?} -> {:?}; shown [{}] -> [{}]; stamp {}",
                index + 1,
                before.lines.iter().map(|l| (l.have, l.need)).collect::<Vec<_>>(),
                after.lines.iter().map(|l| (l.have, l.need)).collect::<Vec<_>>(),
                shown(&before_shown),
                shown(&after_shown),
                would_stamp
            );
            if index == 2 {
                let own = &after.lines[0];
                assert_eq!(own.have, own.need, "{}: the own line is met at 3", role.id);
                assert!(!would_stamp, "{}: no stamp while no station is owned", role.id);
                assert!(
                    matches!(
                        after_shown.as_slice(),
                        [OpenObjectiveType::Milestone(line)]
                            if line.label == "Stations owned" && line.have == 0
                    ),
                    "{}: the shown milestone jumps to the station line",
                    role.id
                );
            }
        }
    }
}
