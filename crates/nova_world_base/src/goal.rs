//! What derives the start role's goal progress while the open world streams,
//! and stamps the goal met.
//!
//! Every role's goal opens with the same first line, one owned station, read
//! live ([`OwnedStationMarker`]) and off the frozen ledger
//! (`FrozenStation::owned`), so a bought station still counts while its cell
//! is off the window. The flavour line after it (`RoleGoal`, content on the
//! `StartRole`) is a predicate over facts the world save already carries:
//! the jobs the player handed in ([`Jobs::completed`]), read by kind, by the
//! item they brought ([`GameItems`] category), by their poster and by the
//! hull they named; and the player's [`CivilizationStances`], for the
//! civilizations the player's events made Enemy and the hulls taken while
//! they stand so. [`track_role_goal`] derives [`GoalProgress`] from them
//! every frame for the Map, the Jobs pane and the save header, and stamps
//! [`GoalRecord`] with the world clock the first frame the progress is met.
//! The record is the one saved fact; the progress is never saved. Both
//! clear with the chart (`chart::clear_chart`) and the record comes back on
//! a Load.
//!
//! A met goal is an optional milestone, not an ending: the record is saved
//! and shown only, and the world does not change at thaw.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    CivilizationStances, GameItems, GoalLine, GoalProgress, GoalRecord, ItemCategoryType, Job,
    JobType, Jobs, OwnedStationMarker,
};
use nova_scenario::prelude::{GameStartRoles, OpenWorldSession, RoleGoal, RoleGoalType};
use nova_world::prelude::{FrozenBodyType, FrozenSectors, WorldClock};

/// The label of the shared first line every role's goal opens with.
pub(crate) const STATIONS_OWNED_LABEL: &str = "Stations owned";

/// How many stations the shared first line asks for.
pub(crate) const STATIONS_OWNED_NEED: u64 = 1;

/// The saved facts one evaluation of a goal reads.
pub(crate) struct GoalFacts<'a> {
    /// The jobs the player handed in, in hand-in order.
    pub completed: &'a [Job],
    /// How many stations the player owns, live or frozen.
    pub owned_stations: u64,
    /// The item catalog, for the category of a Procure job's item.
    pub items: &'a GameItems,
    /// How many civilizations the player's events made Enemy, and the most
    /// intact hulls taken from one of them while it stands so.
    pub outlaw: (u64, u64),
}

/// The progress of `goal` over `facts`: the shared `Stations owned` line
/// first, then one `have / need` line per count the goal asks for, labelled
/// as the Map prints them.
///
/// An ore delivery is a Procure whose item is in the `Raw` category; a
/// recovery is a Recover; a station served is a distinct poster of a Procure
/// or an Escort; a bounty is a distinct hull named by a Neutralize; a patrol
/// is a Distress or an Escort. A Procure for an item the catalog lacks
/// counts no delivery: the save check refuses such a job before a Load.
pub(crate) fn goal_progress(goal: &RoleGoal, facts: &GoalFacts<'_>) -> GoalProgress {
    let completed =
        |kept: fn(&JobType) -> bool| facts.completed.iter().filter(move |job| kept(&job.kind));
    let mut lines = vec![GoalLine::new(
        STATIONS_OWNED_LABEL,
        facts.owned_stations,
        STATIONS_OWNED_NEED,
    )];
    match &goal.kind {
        RoleGoalType::OreDeliveries { jobs } => {
            let have = facts
                .completed
                .iter()
                .filter(|job| match &job.kind {
                    JobType::Procure { item, .. } => facts
                        .items
                        .get(item)
                        .is_some_and(|design| design.category == ItemCategoryType::Raw),
                    _ => false,
                })
                .count();
            lines.push(GoalLine::new(
                "Ore deliveries",
                have as u64,
                u64::from(*jobs),
            ));
        }
        RoleGoalType::Recoveries { jobs } => {
            let have = completed(|kind| matches!(kind, JobType::Recover { .. })).count();
            lines.push(GoalLine::new(
                "Wrecks recovered",
                have as u64,
                u64::from(*jobs),
            ));
        }
        RoleGoalType::StationNetwork { stations } => {
            let served: BTreeSet<_> =
                completed(|kind| matches!(kind, JobType::Procure { .. } | JobType::Escort { .. }))
                    .map(Job::posted_by)
                    .collect();
            lines.push(GoalLine::new(
                "Stations served",
                served.len() as u64,
                u64::from(*stations),
            ));
        }
        RoleGoalType::KillList { hulls } => {
            let named: BTreeSet<_> = facts
                .completed
                .iter()
                .filter_map(|job| match &job.kind {
                    JobType::Neutralize { hull, .. } => Some(hull),
                    _ => None,
                })
                .collect();
            lines.push(GoalLine::new(
                "Bounties handed in",
                named.len() as u64,
                u64::from(*hulls),
            ));
        }
        RoleGoalType::Patrols { jobs } => {
            let have =
                completed(|kind| matches!(kind, JobType::Distress { .. } | JobType::Escort { .. }))
                    .count();
            lines.push(GoalLine::new(
                "Patrols flown",
                have as u64,
                u64::from(*jobs),
            ));
        }
        RoleGoalType::Outlaw { hulls } => {
            lines.push(GoalLine::new(
                "Civilizations turned Enemy",
                facts.outlaw.0,
                1,
            ));
            lines.push(GoalLine::new(
                "Hulls taken while Enemy",
                facts.outlaw.1,
                u64::from(*hulls),
            ));
        }
    }
    GoalProgress {
        title: goal.title.clone(),
        lines,
    }
}

/// Derive the session role's goal progress from the player's owned stations,
/// the completed jobs and the player's stances every frame, and stamp the
/// record once when it is met.
///
/// No session or a role the catalog does not hold: the progress is empty and
/// the record is left as it is. Owned stations are read live and frozen.
pub(crate) fn track_role_goal(
    session: Option<Res<OpenWorldSession>>,
    roles: Res<GameStartRoles>,
    jobs: Res<Jobs>,
    items: Res<GameItems>,
    clock: Res<WorldClock>,
    owned: Query<(), With<OwnedStationMarker>>,
    frozen: Res<FrozenSectors>,
    stances: Res<CivilizationStances>,
    mut progress: ResMut<GoalProgress>,
    mut record: ResMut<GoalRecord>,
) {
    let goal = session
        .as_deref()
        .and_then(|session| roles.get_role(&session.role))
        .map(|role| &role.goal);
    let Some(goal) = goal else {
        if !progress.is_empty() {
            *progress = GoalProgress::default();
        }
        return;
    };
    let frozen_owned = frozen
        .iter()
        .flat_map(|(_, sector)| sector.bodies())
        .filter(|body| matches!(body.body(), FrozenBodyType::Station(station) if station.owned()))
        .count();
    let next = goal_progress(
        goal,
        &GoalFacts {
            completed: jobs.completed(),
            owned_stations: (owned.iter().count() + frozen_owned) as u64,
            items: &items,
            outlaw: stances.outlaw_progress(),
        },
    );
    if next.is_met() && !record.is_met() {
        record.met_at = Some(clock.play_seconds);
        info!(
            "nova_world_base: the charter '{}' is met at {} s of play",
            next.title, clock.play_seconds
        );
    }
    if *progress != next {
        *progress = next;
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{GeneratedBodyId, SectorCoord};
    use nova_gameplay::prelude::{ItemDesign, JobId};

    use super::*;

    /// A job of `kind` posted by the station in cell `(x, 0, 0)`, slot
    /// `slot`.
    fn handed_in(x: i32, slot: u32, kind: JobType) -> Job {
        let station = GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_station", 0);
        Job {
            id: JobId {
                station: station.clone(),
                slot,
                posting: 0,
            },
            kind,
            target: station,
            reward_cr: 100,
        }
    }

    fn procure(item: &str) -> JobType {
        JobType::Procure {
            item: item.into(),
            count: 4,
        }
    }

    fn recover(x: i32) -> JobType {
        JobType::Recover {
            wreck: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_wreck", 0),
            wreck_name: "Vell derelict".to_string(),
            item: "FrameSpar".into(),
            count: 2,
        }
    }

    fn escort(x: i32) -> JobType {
        JobType::Escort {
            hauler: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_hauler", 0),
            cargo_cr: 500,
        }
    }

    fn distress(x: i32) -> JobType {
        JobType::Distress {
            caller: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_caller", 0),
        }
    }

    fn kill(x: i32, index: u32) -> JobType {
        JobType::Neutralize {
            hull: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_hull", index),
            hull_name: format!("Harrow {index}"),
        }
    }

    /// Two items: iron ore in `Raw`, rations in `Food`.
    fn items() -> GameItems {
        let item = |id: &str, category: ItemCategoryType| ItemDesign {
            id: id.into(),
            name: id.to_string(),
            about: String::new(),
            category,
            mass_g: 10_000,
            price_lot: 1,
            ask_cr: 16,
            bid_cr: 12,
        };
        GameItems::new([
            item("IronOre", ItemCategoryType::Raw),
            item("Rations", ItemCategoryType::Food),
        ])
    }

    fn facts<'a>(completed: &'a [Job], items: &'a GameItems, owned_stations: u64) -> GoalFacts<'a> {
        GoalFacts {
            completed,
            owned_stations,
            items,
            outlaw: (0, 0),
        }
    }

    fn goal(title: &str, kind: RoleGoalType) -> RoleGoal {
        RoleGoal {
            title: title.to_string(),
            kind,
        }
    }

    fn haves(progress: &GoalProgress) -> Vec<u64> {
        progress.lines.iter().map(|line| line.have).collect()
    }

    fn needs(progress: &GoalProgress) -> Vec<u64> {
        progress.lines.iter().map(|line| line.need).collect()
    }

    /// Every kind opens with `Stations owned have / 1`, reads its title, and
    /// stands with no owned station even when its own count is met; the
    /// pirate's two lines follow the shared one.
    #[test]
    fn every_goal_opens_with_the_shared_station_line_and_stands_without_a_station() {
        let items = items();
        let six_kills: Vec<Job> = (0..6)
            .map(|index| handed_in(1, index, kill(1, index)))
            .collect();
        let kinds = [
            RoleGoalType::OreDeliveries { jobs: 6 },
            RoleGoalType::Recoveries { jobs: 6 },
            RoleGoalType::StationNetwork { stations: 4 },
            RoleGoalType::KillList { hulls: 6 },
            RoleGoalType::Patrols { jobs: 6 },
            RoleGoalType::Outlaw { hulls: 6 },
        ];
        for kind in kinds {
            let goal = goal("A charter", kind.clone());
            let progress = goal_progress(&goal, &facts(&[], &items, 0));
            assert_eq!(progress.title, "A charter", "{kind:?}");
            assert_eq!(
                progress.lines[0],
                GoalLine::new(STATIONS_OWNED_LABEL, 0, 1),
                "{kind:?}"
            );
            let flavour = if matches!(kind, RoleGoalType::Outlaw { .. }) {
                3
            } else {
                2
            };
            assert_eq!(progress.lines.len(), flavour, "{kind:?}");
            assert!(!progress.is_met(), "{kind:?}");
        }
        let kill_list = goal("A kill list", RoleGoalType::KillList { hulls: 6 });
        let no_station = goal_progress(&kill_list, &facts(&six_kills, &items, 0));
        assert_eq!(haves(&no_station), [0, 6]);
        assert!(!no_station.is_met(), "six bounties with no station stand");
        let outlaw = goal("An outlaw's name", RoleGoalType::Outlaw { hulls: 6 });
        let progress = goal_progress(
            &outlaw,
            &GoalFacts {
                outlaw: (1, 6),
                ..facts(&[], &items, 1)
            },
        );
        assert_eq!(haves(&progress), [1, 1, 6]);
        assert_eq!(needs(&progress), [1, 1, 6]);
        assert!(progress.is_met());
    }

    /// Six Procure hand-ins of a `Raw` item meet the deliveries with one
    /// owned station; a Procure of rations, a Recover and an Escort count
    /// none.
    #[test]
    fn ore_deliveries_count_raw_procure_hand_ins_only() {
        let items = items();
        let goal = goal("Ore to market", RoleGoalType::OreDeliveries { jobs: 6 });
        let mixed: Vec<Job> = (0..5)
            .map(|slot| handed_in(1, slot, procure("IronOre")))
            .chain([
                handed_in(2, 0, procure("Rations")),
                handed_in(2, 1, recover(2)),
                handed_in(2, 2, escort(2)),
            ])
            .collect();
        let progress = goal_progress(&goal, &facts(&mixed, &items, 1));
        assert_eq!(haves(&progress), [1, 5]);
        assert_eq!(needs(&progress), [1, 6]);
        assert!(!progress.is_met());
        let six: Vec<Job> = (0..6)
            .map(|slot| handed_in(1, slot, procure("IronOre")))
            .collect();
        assert!(goal_progress(&goal, &facts(&six, &items, 1)).is_met());
    }

    /// Six Recover hand-ins meet the run with one owned station; a Procure
    /// of the same item counts none.
    #[test]
    fn recoveries_count_recover_hand_ins_only() {
        let items = items();
        let goal = goal("A salvage run", RoleGoalType::Recoveries { jobs: 6 });
        let five: Vec<Job> = (0..5)
            .map(|slot| handed_in(1, slot, recover(1)))
            .chain([handed_in(2, 0, procure("IronOre"))])
            .collect();
        let progress = goal_progress(&goal, &facts(&five, &items, 1));
        assert_eq!(haves(&progress), [1, 5]);
        assert!(!progress.is_met());
        let six: Vec<Job> = (0..6).map(|slot| handed_in(1, slot, recover(1))).collect();
        assert!(goal_progress(&goal, &facts(&six, &items, 1)).is_met());
    }

    /// Eight procurements at one station serve one station; Procure and
    /// Escort hand-ins at four distinct posters meet the network with one
    /// owned station; a Neutralize serves none.
    #[test]
    fn a_station_network_counts_distinct_posters_of_trade_hand_ins() {
        let items = items();
        let goal = goal(
            "A station network",
            RoleGoalType::StationNetwork { stations: 4 },
        );
        let one_station: Vec<Job> = (0..8)
            .map(|slot| handed_in(1, slot, procure("Rations")))
            .collect();
        let progress = goal_progress(&goal, &facts(&one_station, &items, 1));
        assert_eq!(haves(&progress), [1, 1]);
        assert_eq!(needs(&progress), [1, 4]);
        assert!(!progress.is_met());
        let four: Vec<Job> = vec![
            handed_in(1, 0, procure("Rations")),
            handed_in(2, 0, escort(2)),
            handed_in(3, 0, procure("IronOre")),
            handed_in(9, 0, kill(9, 0)),
        ];
        assert_eq!(
            haves(&goal_progress(&goal, &facts(&four, &items, 1))),
            [1, 3]
        );
        let served: Vec<Job> = four
            .into_iter()
            .chain([handed_in(4, 0, escort(4))])
            .collect();
        assert!(goal_progress(&goal, &facts(&served, &items, 1)).is_met());
    }

    /// Six Neutralize hand-ins naming six hulls meet the list with one owned
    /// station; two completed jobs naming one hull count one bounty, a save
    /// production never writes but the count still reads as hulls; a
    /// Distress counts none.
    #[test]
    fn a_kill_list_counts_distinct_hulls_of_neutralize_hand_ins() {
        let items = items();
        let goal = goal("A kill list", RoleGoalType::KillList { hulls: 6 });
        let duplicated: Vec<Job> = (0..5)
            .map(|index| handed_in(1, index, kill(1, index)))
            .chain([handed_in(2, 0, kill(1, 0)), handed_in(9, 0, distress(9))])
            .collect();
        let progress = goal_progress(&goal, &facts(&duplicated, &items, 1));
        assert_eq!(
            haves(&progress),
            [1, 5],
            "one hull named twice is one bounty"
        );
        assert_eq!(needs(&progress), [1, 6]);
        assert!(!progress.is_met());
        let six: Vec<Job> = (0..6)
            .map(|index| handed_in(if index < 3 { 1 } else { 9 }, index, kill(1, index)))
            .collect();
        assert!(goal_progress(&goal, &facts(&six, &items, 1)).is_met());
    }

    /// Distress and Escort hand-ins both count a patrol; six meet the record
    /// with one owned station; a Neutralize counts none.
    #[test]
    fn patrols_count_distress_and_escort_hand_ins() {
        let items = items();
        let goal = goal("Patrol record", RoleGoalType::Patrols { jobs: 6 });
        let five: Vec<Job> = vec![
            handed_in(1, 0, distress(1)),
            handed_in(1, 1, escort(1)),
            handed_in(2, 0, distress(2)),
            handed_in(2, 1, escort(2)),
            handed_in(3, 0, distress(3)),
            handed_in(3, 1, kill(3, 0)),
        ];
        let progress = goal_progress(&goal, &facts(&five, &items, 1));
        assert_eq!(haves(&progress), [1, 5]);
        assert!(!progress.is_met());
        let six: Vec<Job> = five
            .into_iter()
            .chain([handed_in(4, 0, escort(4))])
            .collect();
        assert!(goal_progress(&goal, &facts(&six, &items, 1)).is_met());
    }

    /// An outlaw's charter reads the civilizations the player's events made
    /// Enemy against one and the hulls taken while Enemy against its count:
    /// a seeded Enemy and the flipping kill count for nothing, six hulls
    /// while Enemy with an owned station meet it.
    #[test]
    fn an_outlaw_counts_civilizations_turned_enemy_and_hulls_taken_while_enemy() {
        let items = items();
        let goal = goal("An outlaw's name", RoleGoalType::Outlaw { hulls: 6 });
        let clean = goal_progress(&goal, &facts(&[], &items, 1));
        assert_eq!(haves(&clean), [1, 0, 0]);
        assert!(!clean.is_met());
        let outlawed = GoalFacts {
            outlaw: (1, 5),
            ..facts(&[], &items, 1)
        };
        let progress = goal_progress(&goal, &outlawed);
        assert_eq!(haves(&progress), [1, 1, 5]);
        assert!(!progress.is_met());
        let met = GoalFacts {
            outlaw: (1, 6),
            ..facts(&[], &items, 1)
        };
        assert!(goal_progress(&goal, &met).is_met());
    }
}
