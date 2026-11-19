//! What derives the start role's goal progress while the open world streams,
//! and stamps the goal met.
//!
//! Every role's goal opens with the role's own line (`RoleGoal`, content on
//! the `StartRole`), a predicate over facts the world save carries: the jobs
//! the player handed in ([`Jobs::completed`]), read by kind, by the item
//! they brought ([`GameItems`] category) and by the hull they named; and
//! the trading hulls the player took out ([`GoalRecord::hulls_taken`],
//! counted by [`count_hull_taken`]). It closes with the same last line, one
//! owned station, read live ([`OwnedStationMarker`]) and off the frozen
//! ledger (`FrozenStation::owned`), so a bought station still counts while
//! its cell is off the window. [`track_role_goal`] derives [`GoalProgress`]
//! from them every frame for the Map, the Jobs pane and the save header,
//! and stamps [`GoalRecord`] with the world clock the first frame the
//! progress is met. The record is the saved part; the progress is never
//! saved. Both clear with the chart (`chart::clear_chart`) and the record
//! comes back on a Load.
//!
//! A met goal is an optional milestone, not an ending: the record is saved
//! and shown only, and the world does not change at thaw.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    GameItems, GoalLine, GoalProgress, GoalRecord, ItemCategoryType, Job, JobType, Jobs,
    NeutralizedMarker, OwnedStationMarker, StationMarker, TraderMarket,
};
use nova_scenario::prelude::{GameStartRoles, OpenWorldSession, RoleGoal, RoleGoalType};
use nova_world::prelude::{FrozenBodyType, FrozenSectors, WorldClock};

use crate::stance::PlayerTakedown;

/// The label of the shared last line every role's goal closes with.
pub(crate) const STATIONS_OWNED_LABEL: &str = "Stations owned";

/// How many stations the shared last line asks for.
pub(crate) const STATIONS_OWNED_NEED: u64 = 1;

/// The saved facts one evaluation of a goal reads. Public so a start-role
/// census counts a hand-in toward a goal by this rule, not by a copy of it.
pub struct GoalFacts<'a> {
    /// The jobs the player handed in, in hand-in order.
    pub completed: &'a [Job],
    /// How many stations the player owns, live or frozen.
    pub owned_stations: u64,
    /// The item catalog, for the category of a Procure job's item.
    pub items: &'a GameItems,
    /// How many trading hulls the player took out
    /// ([`GoalRecord::hulls_taken`]).
    pub hulls_taken: u64,
}

/// The progress of `goal` over `facts`: the role's own `have / need` line
/// first, then the shared `Stations owned` line, labelled as the Map prints
/// them.
///
/// An ore delivery is a Procure whose item is in the `Raw` category; a
/// recovery is a Recover; a cargo run is a Procure, a Supply or an Escort; a
/// bounty is a distinct hull named by a Neutralize; a patrol is a Distress,
/// a Supply or an Escort; a hull taken is one the record counted. A Procure
/// for an item the catalog lacks counts no delivery: the save check refuses
/// such a job before a Load.
pub fn goal_progress(goal: &RoleGoal, facts: &GoalFacts<'_>) -> GoalProgress {
    let completed = |kept: fn(&JobType) -> bool| {
        facts
            .completed
            .iter()
            .filter(move |job| kept(&job.kind))
            .count() as u64
    };
    let own = match &goal.kind {
        RoleGoalType::OreDeliveries { jobs } => {
            let have = facts
                .completed
                .iter()
                .filter(|job| match &job.kind {
                    JobType::Procure { item, .. } => facts
                        .items
                        .get(item)
                        .is_some_and(|design| design.category == ItemCategoryType::Raw),
                    JobType::Recover { .. }
                    | JobType::Escort { .. }
                    | JobType::Distress { .. }
                    | JobType::Neutralize { .. }
                    | JobType::Supply { .. } => false,
                })
                .count();
            GoalLine::new("Ore deliveries", have as u64, u64::from(*jobs))
        }
        RoleGoalType::Recoveries { jobs } => {
            let have = completed(|kind| match kind {
                JobType::Recover { .. } => true,
                JobType::Procure { .. }
                | JobType::Escort { .. }
                | JobType::Distress { .. }
                | JobType::Neutralize { .. }
                | JobType::Supply { .. } => false,
            });
            GoalLine::new("Wrecks recovered", have, u64::from(*jobs))
        }
        RoleGoalType::CargoRuns { jobs } => {
            let have = completed(|kind| match kind {
                JobType::Procure { .. } | JobType::Supply { .. } | JobType::Escort { .. } => true,
                JobType::Recover { .. } | JobType::Distress { .. } | JobType::Neutralize { .. } => {
                    false
                }
            });
            GoalLine::new("Cargo runs", have, u64::from(*jobs))
        }
        RoleGoalType::KillList { hulls } => {
            let named: BTreeSet<_> = facts
                .completed
                .iter()
                .filter_map(|job| match &job.kind {
                    JobType::Neutralize { hull, .. } => Some(hull),
                    JobType::Procure { .. }
                    | JobType::Recover { .. }
                    | JobType::Escort { .. }
                    | JobType::Distress { .. }
                    | JobType::Supply { .. } => None,
                })
                .collect();
            GoalLine::new("Bounties handed in", named.len() as u64, u64::from(*hulls))
        }
        RoleGoalType::Patrols { jobs } => {
            let have = completed(|kind| match kind {
                JobType::Distress { .. } | JobType::Supply { .. } | JobType::Escort { .. } => true,
                JobType::Procure { .. } | JobType::Recover { .. } | JobType::Neutralize { .. } => {
                    false
                }
            });
            GoalLine::new("Patrols flown", have, u64::from(*jobs))
        }
        RoleGoalType::Outlaw { hulls } => {
            GoalLine::new("Hulls taken", facts.hulls_taken, u64::from(*hulls))
        }
    };
    GoalProgress {
        title: goal.title.clone(),
        lines: vec![
            own,
            GoalLine::new(
                STATIONS_OWNED_LABEL,
                facts.owned_stations,
                STATIONS_OWNED_NEED,
            ),
        ],
    }
}

/// Count a trading hull the player just neutralized: a ship hull, never a
/// station, that carries a [`TraderMarket`] (every intact generated ship
/// does) and that the player's ship or a fleet escort took out
/// ([`PlayerTakedown::took_out`], the rule an offence is recorded by,
/// `stance::record_offence`), whatever the civilization's stance: a hostile
/// patrol with a market is a hull taken too. A hull the player hit once and
/// left, a hull thawed already neutralized and a hull a job put in the
/// world, which carries no market, count nothing.
pub(crate) fn count_hull_taken(
    add: On<Add<NeutralizedMarker>>,
    hulls: Query<(), (With<TraderMarket>, Without<StationMarker>)>,
    takedown: PlayerTakedown,
    mut record: ResMut<GoalRecord>,
) {
    if hulls.contains(add.entity) && takedown.took_out(add.entity) {
        record.hulls_taken = record.hulls_taken.saturating_add(1);
    }
}

/// Derive the session role's goal progress from the completed jobs, the
/// hulls the record counted and the player's owned stations every frame,
/// and stamp the record once when it is met.
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
        .filter(
            |body| matches!(body.body(), FrozenBodyType::Station(station, _) if station.owned()),
        )
        .count();
    let next = goal_progress(
        goal,
        &GoalFacts {
            completed: jobs.completed(),
            owned_stations: (owned.iter().count() + frozen_owned) as u64,
            items: &items,
            hulls_taken: u64::from(record.hulls_taken),
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
    use std::collections::BTreeMap;

    use nova_events::prelude::{GeneratedBodyId, SectorCoord};
    use nova_gameplay::prelude::{
        CivilizationId, ClusterType, ItemDesign, JobId, JobReward, PlayerFleetMarker,
        PlayerSpaceshipMarker,
    };

    use super::*;

    /// A job of `kind` posted by the station in cell `(x, 0, 0)`, slot
    /// `slot`.
    fn handed_in(x: i32, slot: u32, kind: JobType) -> Job {
        let station = GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_station", 0);
        Job {
            id: JobId::slot(station.clone(), slot, 0),
            kind,
            target: station,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(100),
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
            raiders: 1,
        }
    }

    fn distress(x: i32) -> JobType {
        JobType::Distress {
            caller: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_caller", 0),
        }
    }

    fn supply(x: i32) -> JobType {
        JobType::Supply {
            caller: GeneratedBodyId::new(SectorCoord::new(x, 0, 0), "cluster_caller", 0),
            item: "Rations".into(),
            count: 4,
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
            hulls_taken: 0,
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

    /// Every kind reads its title, opens with its own line against its
    /// count and closes with `Stations owned have / 1`; a goal whose own
    /// count is met stands with no owned station.
    #[test]
    fn every_goal_opens_with_its_own_line_and_closes_with_the_shared_station_line() {
        let items = items();
        let kinds = [
            (RoleGoalType::OreDeliveries { jobs: 3 }, "Ore deliveries"),
            (RoleGoalType::Recoveries { jobs: 3 }, "Wrecks recovered"),
            (RoleGoalType::CargoRuns { jobs: 3 }, "Cargo runs"),
            (RoleGoalType::KillList { hulls: 3 }, "Bounties handed in"),
            (RoleGoalType::Patrols { jobs: 3 }, "Patrols flown"),
            (RoleGoalType::Outlaw { hulls: 3 }, "Hulls taken"),
        ];
        for (kind, label) in kinds {
            let goal = goal("A charter", kind.clone());
            let progress = goal_progress(&goal, &facts(&[], &items, 0));
            assert_eq!(progress.title, "A charter", "{kind:?}");
            assert_eq!(
                progress.lines,
                [
                    GoalLine::new(label, 0, 3),
                    GoalLine::new(STATIONS_OWNED_LABEL, 0, 1)
                ],
                "{kind:?}"
            );
            assert!(!progress.is_met(), "{kind:?}");
        }
        let three_kills: Vec<Job> = (0..3)
            .map(|index| handed_in(1, index, kill(1, index)))
            .collect();
        let kill_list = goal("A kill list", RoleGoalType::KillList { hulls: 3 });
        let no_station = goal_progress(&kill_list, &facts(&three_kills, &items, 0));
        assert_eq!(haves(&no_station), [3, 0]);
        assert!(!no_station.is_met(), "three bounties with no station stand");
    }

    /// Three Procure hand-ins of a `Raw` item meet the deliveries with one
    /// owned station; a Procure of rations, a Recover and an Escort count
    /// none.
    #[test]
    fn ore_deliveries_count_raw_procure_hand_ins_only() {
        let items = items();
        let goal = goal("Ore to market", RoleGoalType::OreDeliveries { jobs: 3 });
        let mixed: Vec<Job> = (0..2)
            .map(|slot| handed_in(1, slot, procure("IronOre")))
            .chain([
                handed_in(2, 0, procure("Rations")),
                handed_in(2, 1, recover(2)),
                handed_in(2, 2, escort(2)),
            ])
            .collect();
        let progress = goal_progress(&goal, &facts(&mixed, &items, 1));
        assert_eq!(haves(&progress), [2, 1]);
        assert_eq!(needs(&progress), [3, 1]);
        assert!(!progress.is_met());
        let three: Vec<Job> = (0..3)
            .map(|slot| handed_in(1, slot, procure("IronOre")))
            .collect();
        assert!(goal_progress(&goal, &facts(&three, &items, 1)).is_met());
    }

    /// Three Recover hand-ins meet the run with one owned station; a
    /// Procure of the same item counts none.
    #[test]
    fn recoveries_count_recover_hand_ins_only() {
        let items = items();
        let goal = goal("A salvage run", RoleGoalType::Recoveries { jobs: 3 });
        let two: Vec<Job> = (0..2)
            .map(|slot| handed_in(1, slot, recover(1)))
            .chain([handed_in(2, 0, procure("IronOre"))])
            .collect();
        let progress = goal_progress(&goal, &facts(&two, &items, 1));
        assert_eq!(haves(&progress), [2, 1]);
        assert!(!progress.is_met());
        let three: Vec<Job> = (0..3).map(|slot| handed_in(1, slot, recover(1))).collect();
        assert!(goal_progress(&goal, &facts(&three, &items, 1)).is_met());
    }

    /// A Procure, a Supply and an Escort hand-in each count a cargo run,
    /// three at one station as well as at three; a Recover, a Distress and a
    /// Neutralize count none.
    #[test]
    fn cargo_runs_count_procure_supply_and_escort_hand_ins() {
        let items = items();
        let goal = goal("Cargo runs", RoleGoalType::CargoRuns { jobs: 3 });
        let two: Vec<Job> = vec![
            handed_in(1, 0, procure("Rations")),
            handed_in(1, 1, supply(1)),
            handed_in(1, 2, recover(1)),
            handed_in(2, 0, distress(2)),
            handed_in(2, 1, kill(2, 0)),
        ];
        let progress = goal_progress(&goal, &facts(&two, &items, 1));
        assert_eq!(haves(&progress), [2, 1]);
        assert_eq!(needs(&progress), [3, 1]);
        assert!(!progress.is_met());
        let three: Vec<Job> = two
            .into_iter()
            .chain([handed_in(1, 3, escort(1))])
            .collect();
        assert!(goal_progress(&goal, &facts(&three, &items, 1)).is_met());
    }

    /// Three Neutralize hand-ins naming three hulls meet the list with one
    /// owned station; two completed jobs naming one hull count one bounty, a
    /// save production never writes but the count still reads as hulls; a
    /// Distress counts none.
    #[test]
    fn a_kill_list_counts_distinct_hulls_of_neutralize_hand_ins() {
        let items = items();
        let goal = goal("A kill list", RoleGoalType::KillList { hulls: 3 });
        let duplicated: Vec<Job> = (0..2)
            .map(|index| handed_in(1, index, kill(1, index)))
            .chain([handed_in(2, 0, kill(1, 0)), handed_in(9, 0, distress(9))])
            .collect();
        let progress = goal_progress(&goal, &facts(&duplicated, &items, 1));
        assert_eq!(
            haves(&progress),
            [2, 1],
            "one hull named twice is one bounty"
        );
        assert_eq!(needs(&progress), [3, 1]);
        assert!(!progress.is_met());
        let three: Vec<Job> = (0..3)
            .map(|index| handed_in(if index < 2 { 1 } else { 9 }, index, kill(1, index)))
            .collect();
        assert!(goal_progress(&goal, &facts(&three, &items, 1)).is_met());
    }

    /// Distress, Supply and Escort hand-ins each count a patrol; three meet
    /// the record with one owned station; a Neutralize counts none.
    #[test]
    fn patrols_count_distress_supply_and_escort_hand_ins() {
        let items = items();
        let goal = goal("Patrol record", RoleGoalType::Patrols { jobs: 3 });
        let two: Vec<Job> = vec![
            handed_in(1, 0, distress(1)),
            handed_in(2, 0, supply(2)),
            handed_in(3, 1, kill(3, 0)),
        ];
        let progress = goal_progress(&goal, &facts(&two, &items, 1));
        assert_eq!(haves(&progress), [2, 1]);
        assert!(!progress.is_met());
        let three: Vec<Job> = two
            .into_iter()
            .chain([handed_in(4, 0, escort(4))])
            .collect();
        assert!(goal_progress(&goal, &facts(&three, &items, 1)).is_met());
    }

    /// An outlaw's charter reads the hulls the record counted against its
    /// count and nothing of a stance: three hulls with an owned station
    /// meet it.
    #[test]
    fn an_outlaw_counts_the_hulls_taken() {
        let items = items();
        let goal = goal("An outlaw's name", RoleGoalType::Outlaw { hulls: 3 });
        let taken = |hulls_taken| GoalFacts {
            hulls_taken,
            ..facts(&[], &items, 1)
        };
        let progress = goal_progress(&goal, &taken(2));
        assert_eq!(haves(&progress), [2, 1]);
        assert_eq!(needs(&progress), [3, 1]);
        assert!(!progress.is_met());
        assert!(goal_progress(&goal, &taken(3)).is_met());
    }

    /// A trading hull the player's ship or a fleet escort neutralizes counts
    /// one hull taken with no stance asked of its civilization. A station,
    /// a hull another ship took out, a hull with no market, a hull thawed
    /// already neutralized and a hull the player hit and left, neutralized
    /// after the attribution window, count none.
    #[test]
    fn a_trading_hull_the_player_neutralizes_counts_one_hull_taken() {
        use std::time::Duration;

        use nova_ship::prelude::{LastAttacker, LAST_ATTACKER_WINDOW};

        let mut app = App::new();
        app.init_resource::<GoalRecord>();
        app.init_resource::<Time<Virtual>>();
        app.add_observer(count_hull_taken);
        let market = || TraderMarket {
            cluster: ClusterType::RockOnly,
            civilization: CivilizationId {
                world_seed: 7,
                node: [0, 0, 0],
            },
            role_preference: [1.0; 4],
            target: BTreeMap::new(),
            profile: None,
        };
        let world = app.world_mut();
        let player = world.spawn(PlayerSpaceshipMarker).id();
        let escort = world.spawn(PlayerFleetMarker).id();
        let stranger = world.spawn_empty().id();
        let taken = |world: &World| world.resource::<GoalRecord>().hulls_taken;
        let hit_by = |world: &World, attacker: Entity| LastAttacker {
            attacker,
            at: world.resource::<Time<Virtual>>().elapsed(),
        };

        world.spawn((market(), hit_by(world, player), NeutralizedMarker));
        assert_eq!(taken(world), 1, "the player's own kill");
        world.spawn((market(), hit_by(world, escort), NeutralizedMarker));
        assert_eq!(taken(world), 2, "a fleet escort's kill");
        world.spawn((
            StationMarker,
            market(),
            hit_by(world, player),
            NeutralizedMarker,
        ));
        assert_eq!(taken(world), 2, "a station is no hull taken");
        world.spawn((market(), hit_by(world, stranger), NeutralizedMarker));
        world.spawn((hit_by(world, player), NeutralizedMarker));
        world.spawn((market(), NeutralizedMarker));
        assert_eq!(
            taken(world),
            2,
            "another ship's kill, a hull with no market and a thawed wreck count none"
        );

        // The player hits a trader once and leaves; something nobody owns
        // takes it out after the window.
        let left = world.spawn((market(), hit_by(world, player))).id();
        world
            .resource_mut::<Time<Virtual>>()
            .advance_by(LAST_ATTACKER_WINDOW + Duration::from_secs(1));
        world.entity_mut(left).insert(NeutralizedMarker);
        assert_eq!(
            taken(world),
            2,
            "a hull neutralized after the window is not the player's"
        );
        let fresh = world.spawn((market(), hit_by(world, player))).id();
        world
            .resource_mut::<Time<Virtual>>()
            .advance_by(LAST_ATTACKER_WINDOW);
        world.entity_mut(fresh).insert(NeutralizedMarker);
        assert_eq!(taken(world), 3, "a hull neutralized inside the window is");
    }
}
