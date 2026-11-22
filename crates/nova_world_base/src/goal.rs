//! What derives the start role's goal progress while the open world streams,
//! and says when its current step is met.
//!
//! A role's goal (`RoleGoal`, content on the `StartRole`) is a chain of
//! steps, each a predicate over facts the world save carries: the jobs the
//! player handed in ([`Jobs::completed`]), read by kind, by the item they
//! brought ([`GameItems`] category) and by the hull they named; the trading
//! hulls the player took out ([`GoalRecord::hulls_taken`], counted by
//! [`count_hull_taken`]); and the stations the player owns, read live
//! ([`OwnedStationMarker`]) and off the frozen ledger
//! (`FrozenStation::owned`), so a bought station still counts while its
//! cell is off the window. [`track_role_goal`] derives [`GoalProgress`]
//! from them every frame for the Map, the Jobs pane and the save header,
//! and raises [`GoalStepDone`] the frame the current step reads met. The
//! Jobs pane hands the step in and stamps [`GoalRecord`]. The record is the
//! saved part; the progress is never saved. Both clear with the chart
//! (`chart::clear_chart`) and the record comes back on a Load.
//!
//! A met goal is an optional milestone, not an ending: the record is saved
//! and shown only, and the world does not change at thaw.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    GameItems, GoalLine, GoalProgress, GoalRecord, GoalStep, GoalStepDone, ItemCategoryType, Job,
    JobType, Jobs, NeutralizedMarker, OwnedStationMarker, StationMarker, TraderMarket,
};
use nova_scenario::prelude::{GameStartRoles, OpenWorldSession, RoleGoal, RoleGoalType};
use nova_world::prelude::{FrozenBodyType, FrozenSectors};

use crate::stance::PlayerTakedown;

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

/// The progress of `goal` over `facts`: one `have / need` line per step,
/// labelled as the Map prints it, with the step's reward.
///
/// An ore delivery is a Procure whose item is in the `Raw` category; a
/// recovery is a Recover; a cargo run is a Procure, a Supply or an Escort; a
/// bounty is a distinct hull named by a Neutralize; a patrol is a Distress,
/// a Supply or an Escort; a hull taken is one the record counted; a station
/// owned is one the player owns now. A Procure for an item the catalog
/// lacks counts no delivery: the save check refuses such a job before a
/// Load.
pub fn goal_progress(goal: &RoleGoal, facts: &GoalFacts<'_>) -> GoalProgress {
    GoalProgress {
        title: goal.title.clone(),
        steps: goal
            .steps
            .iter()
            .map(|step| GoalStep {
                line: step_line(&step.kind, facts),
                reward_cr: step.reward_cr,
            })
            .collect(),
    }
}

/// The `have / need` line of one step of `kind` over `facts`.
fn step_line(kind: &RoleGoalType, facts: &GoalFacts<'_>) -> GoalLine {
    let completed = |kept: fn(&JobType) -> bool| {
        facts
            .completed
            .iter()
            .filter(move |job| kept(&job.kind))
            .count() as u64
    };
    match kind {
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
        RoleGoalType::StationsOwned { stations } => {
            GoalLine::new("Stations owned", facts.owned_stations, u64::from(*stations))
        }
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
/// and raise [`GoalStepDone`] the frame the current step reads met.
///
/// `announced` is the current step and whether it read met the frame
/// before. A step is announced when it reads met as the current step and
/// did not the frame before: its line turned met, or a hand-in made it
/// current already met. The first frame a goal is derived (New Game, and a
/// Load, whose `Cleanup` cleared the progress) only records it. No session
/// or a role the catalog does not hold: the progress is empty and the
/// record is left as it is. Owned stations are read live and frozen.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the session, the roles, the jobs, the items, the owned stations live and frozen, the progress, the record and the announced step"
)]
pub(crate) fn track_role_goal(
    session: Option<Res<OpenWorldSession>>,
    roles: Res<GameStartRoles>,
    jobs: Res<Jobs>,
    items: Res<GameItems>,
    owned: Query<(), With<OwnedStationMarker>>,
    frozen: Res<FrozenSectors>,
    mut progress: ResMut<GoalProgress>,
    record: Res<GoalRecord>,
    mut announced: Local<Option<(usize, bool)>>,
    mut commands: Commands,
) {
    let goal = session
        .as_deref()
        .and_then(|session| roles.get_role(&session.role))
        .map(|role| &role.goal);
    let Some(goal) = goal else {
        if !progress.is_empty() {
            *progress = GoalProgress::default();
        }
        *announced = None;
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
    let current = record.current();
    let ready = next
        .steps
        .get(current)
        .is_some_and(|step| step.line.is_met());
    let first = progress.is_empty();
    if !first && ready && *announced != Some((current, true)) {
        let step = &next.steps[current];
        info!(
            "nova_world_base: goal step {} of '{}' is met: {} {} / {}",
            current + 1,
            next.title,
            step.line.label,
            step.line.have,
            step.line.need
        );
        commands.trigger(GoalStepDone {
            step: current,
            reward_cr: step.reward_cr,
        });
    }
    *announced = Some((current, ready));
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
    use nova_scenario::prelude::RoleGoalStep;

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

    /// A goal of the one step `kind`, paying 3,000 cr.
    fn goal(title: &str, kind: RoleGoalType) -> RoleGoal {
        RoleGoal {
            title: title.to_string(),
            steps: vec![RoleGoalStep {
                kind,
                reward_cr: 3_000,
            }],
        }
    }

    /// The one step's `have / need` of `goal` over `facts`.
    fn line(goal: &RoleGoal, facts: &GoalFacts<'_>) -> (u64, u64) {
        let progress = goal_progress(goal, facts);
        assert_eq!(progress.steps.len(), 1);
        let line = &progress.steps[0].line;
        (line.have, line.need)
    }

    /// Every kind reads its title and one line per step, labelled, against
    /// its count and with its reward, in the authored order.
    #[test]
    fn every_step_reads_its_own_line_and_reward_in_chain_order() {
        let items = items();
        let kinds = [
            (RoleGoalType::OreDeliveries { jobs: 3 }, "Ore deliveries"),
            (RoleGoalType::Recoveries { jobs: 3 }, "Wrecks recovered"),
            (RoleGoalType::CargoRuns { jobs: 3 }, "Cargo runs"),
            (RoleGoalType::KillList { hulls: 3 }, "Bounties handed in"),
            (RoleGoalType::Patrols { jobs: 3 }, "Patrols flown"),
            (RoleGoalType::Outlaw { hulls: 3 }, "Hulls taken"),
            (
                RoleGoalType::StationsOwned { stations: 3 },
                "Stations owned",
            ),
        ];
        for (kind, label) in kinds {
            let mut chain = goal("A charter", kind.clone());
            chain.steps.push(RoleGoalStep {
                kind: RoleGoalType::StationsOwned { stations: 1 },
                reward_cr: 10_000,
            });
            let progress = goal_progress(&chain, &facts(&[], &items, 0));
            assert_eq!(progress.title, "A charter", "{kind:?}");
            assert_eq!(
                progress.steps,
                [
                    GoalStep {
                        line: GoalLine::new(label, 0, 3),
                        reward_cr: 3_000,
                    },
                    GoalStep {
                        line: GoalLine::new("Stations owned", 0, 1),
                        reward_cr: 10_000,
                    },
                ],
                "{kind:?}"
            );
        }
    }

    /// A Stations owned step counts the stations the facts own, whatever
    /// the jobs: two of three, then three.
    #[test]
    fn stations_owned_counts_the_owned_stations() {
        let items = items();
        let goal = goal("A holding", RoleGoalType::StationsOwned { stations: 3 });
        let kills: Vec<Job> = (0..3)
            .map(|index| handed_in(1, index, kill(1, index)))
            .collect();
        assert_eq!(line(&goal, &facts(&kills, &items, 2)), (2, 3));
        assert_eq!(line(&goal, &facts(&[], &items, 3)), (3, 3));
    }

    /// Three Procure hand-ins of a `Raw` item meet the deliveries; a Procure
    /// of rations, a Recover and an Escort count none.
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
        assert_eq!(line(&goal, &facts(&mixed, &items, 1)), (2, 3));
        let three: Vec<Job> = (0..3)
            .map(|slot| handed_in(1, slot, procure("IronOre")))
            .collect();
        assert_eq!(line(&goal, &facts(&three, &items, 0)), (3, 3));
    }

    /// Three Recover hand-ins meet the run; a Procure of the same item
    /// counts none.
    #[test]
    fn recoveries_count_recover_hand_ins_only() {
        let items = items();
        let goal = goal("A salvage run", RoleGoalType::Recoveries { jobs: 3 });
        let two: Vec<Job> = (0..2)
            .map(|slot| handed_in(1, slot, recover(1)))
            .chain([handed_in(2, 0, procure("IronOre"))])
            .collect();
        assert_eq!(line(&goal, &facts(&two, &items, 1)), (2, 3));
        let three: Vec<Job> = (0..3).map(|slot| handed_in(1, slot, recover(1))).collect();
        assert_eq!(line(&goal, &facts(&three, &items, 0)), (3, 3));
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
        assert_eq!(line(&goal, &facts(&two, &items, 1)), (2, 3));
        let three: Vec<Job> = two
            .into_iter()
            .chain([handed_in(1, 3, escort(1))])
            .collect();
        assert_eq!(line(&goal, &facts(&three, &items, 0)), (3, 3));
    }

    /// Three Neutralize hand-ins naming three hulls meet the list; two
    /// completed jobs naming one hull count one bounty, a save production
    /// never writes but the count still reads as hulls; a Distress counts
    /// none.
    #[test]
    fn a_kill_list_counts_distinct_hulls_of_neutralize_hand_ins() {
        let items = items();
        let goal = goal("A kill list", RoleGoalType::KillList { hulls: 3 });
        let duplicated: Vec<Job> = (0..2)
            .map(|index| handed_in(1, index, kill(1, index)))
            .chain([handed_in(2, 0, kill(1, 0)), handed_in(9, 0, distress(9))])
            .collect();
        assert_eq!(
            line(&goal, &facts(&duplicated, &items, 1)),
            (2, 3),
            "one hull named twice is one bounty"
        );
        let three: Vec<Job> = (0..3)
            .map(|index| handed_in(if index < 2 { 1 } else { 9 }, index, kill(1, index)))
            .collect();
        assert_eq!(line(&goal, &facts(&three, &items, 0)), (3, 3));
    }

    /// Distress, Supply and Escort hand-ins each count a patrol; three meet
    /// the record; a Neutralize counts none.
    #[test]
    fn patrols_count_distress_supply_and_escort_hand_ins() {
        let items = items();
        let goal = goal("Patrol record", RoleGoalType::Patrols { jobs: 3 });
        let two: Vec<Job> = vec![
            handed_in(1, 0, distress(1)),
            handed_in(2, 0, supply(2)),
            handed_in(3, 1, kill(3, 0)),
        ];
        assert_eq!(line(&goal, &facts(&two, &items, 1)), (2, 3));
        let three: Vec<Job> = two
            .into_iter()
            .chain([handed_in(4, 0, escort(4))])
            .collect();
        assert_eq!(line(&goal, &facts(&three, &items, 0)), (3, 3));
    }

    /// An outlaw's step reads the hulls the record counted against its
    /// count and nothing of a stance.
    #[test]
    fn an_outlaw_counts_the_hulls_taken() {
        let items = items();
        let goal = goal("An outlaw's name", RoleGoalType::Outlaw { hulls: 3 });
        let taken = |hulls_taken| GoalFacts {
            hulls_taken,
            ..facts(&[], &items, 1)
        };
        assert_eq!(line(&goal, &taken(2)), (2, 3));
        assert_eq!(line(&goal, &taken(3)), (3, 3));
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

    /// Every [`GoalStepDone`] raised, in order.
    #[derive(Resource, Default)]
    struct Announced(Vec<GoalStepDone>);

    /// `track_role_goal` over the fixture role (six bounties for 3,000 cr,
    /// then one station for 10,000 cr) in an open session, hearing every
    /// [`GoalStepDone`].
    fn tracking_app() -> App {
        use nova_scenario::prelude::StartingStation;

        let mut app = App::new();
        app.insert_resource(crate::test_support::fixture_roles());
        app.insert_resource(OpenWorldSession {
            seed: 42,
            role: crate::test_support::FIXTURE_ROLE_ID.into(),
            start: StartingStation {
                id: GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_station", 0),
                position: nova_events::prelude::Meters3::ZERO,
                spawn: nova_events::prelude::Meters3::ZERO,
            },
        });
        app.init_resource::<Jobs>();
        app.insert_resource(items());
        app.init_resource::<FrozenSectors>();
        app.init_resource::<GoalProgress>();
        app.init_resource::<GoalRecord>();
        app.init_resource::<Announced>();
        app.add_observer(|done: On<GoalStepDone>, mut heard: ResMut<Announced>| {
            heard.0.push(*done);
        });
        app.add_systems(Update, track_role_goal);
        app
    }

    /// Complete the bounty `index` named by a held Neutralize.
    fn hand_in_bounty(app: &mut App, index: u32) {
        let job = handed_in(1, index, kill(1, index));
        let mut jobs = app.world_mut().resource_mut::<Jobs>();
        jobs.accept(job.clone());
        jobs.complete(&job.id);
    }

    fn heard(app: &App) -> Vec<usize> {
        app.world()
            .resource::<Announced>()
            .0
            .iter()
            .map(|done| done.step)
            .collect()
    }

    /// The step that turns met as the current one is announced once, with
    /// its reward; more work on a met step announces nothing more. A paid
    /// step hands over to the next, which is announced the frame it reads
    /// met: here at once, as the station was bought first. A Load, whose
    /// `Cleanup` empties the progress, and a world first derived with a
    /// step already met announce none.
    #[test]
    fn a_step_turning_met_raises_goal_step_done_once_and_a_load_raises_none() {
        let mut app = tracking_app();
        app.update();
        for index in 0..5 {
            hand_in_bounty(&mut app, index);
        }
        app.update();
        assert!(heard(&app).is_empty(), "5 of 6 bounties is short");
        let station = app.world_mut().spawn(OwnedStationMarker).id();
        hand_in_bounty(&mut app, 5);
        app.update();
        assert_eq!(
            app.world().resource::<Announced>().0,
            [GoalStepDone {
                step: 0,
                reward_cr: 3_000
            }]
        );
        hand_in_bounty(&mut app, 6);
        app.update();
        app.update();
        assert_eq!(heard(&app), [0], "a met step is announced once");

        // The pane hands step 1 in; step 2 reads met already.
        app.world_mut()
            .resource_mut::<GoalRecord>()
            .paid_at
            .push(0.0);
        app.update();
        assert_eq!(heard(&app), [0, 1]);
        assert_eq!(
            app.world().resource::<GoalProgress>().steps[1].line,
            GoalLine::new("Stations owned", 1, 1)
        );

        // A Load: Cleanup empties the progress and puts back a record with
        // nothing paid, then the first derived frame reads step 1 met.
        app.world_mut().resource_mut::<Announced>().0.clear();
        *app.world_mut().resource_mut::<GoalProgress>() = GoalProgress::default();
        app.world_mut().resource_mut::<GoalRecord>().paid_at.clear();
        app.update();
        app.update();
        assert!(heard(&app).is_empty(), "a Load announces nothing");
        assert!(
            app.world().resource::<GoalProgress>().steps[0]
                .line
                .is_met(),
            "the loaded step reads met"
        );

        // Step 1 paid with no station owned: step 2 stands short until one
        // is bought.
        app.world_mut().despawn(station);
        app.world_mut()
            .resource_mut::<GoalRecord>()
            .paid_at
            .push(1.0);
        app.update();
        assert!(heard(&app).is_empty(), "step 2 reads 0 / 1");
        app.world_mut().spawn(OwnedStationMarker);
        app.update();
        assert_eq!(heard(&app), [1], "a bought station meets step 2");
    }
}
