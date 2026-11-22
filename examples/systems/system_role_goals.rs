//! system_role_goals: a bounty hunter's charter starts standing, readies
//! step 1 at three hand-ins, hands it in from the Jobs pane in flight for
//! 3,000 cr, readies step 2 once a station is bought, hands it in for
//! 10,000 cr to meet the charter, shows it in the Map, and survives the
//! leave save and a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `bounty_hunter`, and waits for the armed world and its first save.
//! The range checks the authored charter, then injects three completed
//! Neutralize jobs as DISCLOSED FIXTURE STATE: each valid job goes through
//! `Jobs::accept`, `mark_done` and `complete`, with posters in cells
//! (1, 0, 0), (2, 0, 0) and (9, 0, 0), each naming a distinct hull; this
//! fixture replaces flying and fighting, not the goal tracker. It checks
//! step 1 reads ready, undocks, opens the Jobs pane and presses the step's
//! Hand in by name. It then jumps to the origin, picks the nearest living
//! non-Enemy station outside the origin's cell, warms its civilization to
//! Ally by merits the way a player does, berths and docks, sets the
//! player's balance to 100,000,000 cr as a second piece of DISCLOSED
//! FIXTURE STATE so the station is affordable, and buys it through the
//! Station pane's Buy station button. Step 2 reads ready; the range hands
//! it in from the Jobs pane, checks the met stamp, reads the met charter in
//! the Map's chart view, leaves to the main menu, reads the save through
//! [`open_world`], then loads it again and checks the live progress.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the charter opens standing with its steps at zero` | the bounty hunter's `A kill list` has step 1 `Bounties handed in 0 / 3` for 3,000 cr and step 2 `Stations owned 0 / 1` for 10,000 cr, with no step paid |
//! | 2 | `outcome: three neutralize hand-ins ready goal step 1` | three completed Neutralize jobs on three distinct hulls yield `Bounties handed in 3 / 3`, step 1 current and ready, one `OnGoalStepDone` with step 1, nothing paid |
//! | 3 | `outcome: the goal step 1 hand-in pays 3,000 cr in flight` | undocked, the Jobs pane's `GoalHandIn-0` raises the balance by 3,000 cr, fires `OnCreditsChanged` with cause `GoalReward` and `OnGoalStepHandedIn` with step 1, and stamps step 1 inside the play clock |
//! | 4 | `outcome: buying a station readies goal step 2` | after the buy, step 2 reads `Stations owned 1 / 1`, current and ready, with one more `OnGoalStepDone` with step 2 |
//! | 5 | `outcome: the goal step 2 hand-in meets the charter` | the Jobs pane's `GoalHandIn-1` raises the balance by 10,000 cr with cause `GoalReward`; the goal is met at the second stamp, which does not move across two more frames |
//! | 6 | `outcome: the map's chart view shows the charter met` | its OBJECTIVES block shows one row with the stamp as a clock face and no milestone row |
//! | 7 | `outcome: the leave save keeps both paid steps` | `open_world` reads the same two stamps from the leave save |
//! | 8 | `outcome: a load restores both paid steps` | after Load and a frame, the same stamps and met progress return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_role_goals --features debug
//! # look for: `role goals: PASS goal step 2 meets the charter` and
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `role-goals-charter.png` of the chart view
//! with the met goal's OBJECTIVES block.

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_role_goals")]
#[command(version = "1.0.0")]
#[command(
    about = "Start the bounty hunter, ready its kill list at three hand-ins and hand step 1 in \
             from the Jobs pane, buy a station and hand step 2 in to meet the charter, read it \
             in the Map, and check it through a save and a Load. Autopilot-only correctness \
             range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Update, walk::hold_the_walk_clock);
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<walk::GoalWatch>();
        app.add_observer(walk::watch_goal_events);
        app.add_plugins(walk::script());
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{
            jobs::prelude::GOAL_HAND_IN_PREFIX,
            pane::InterfacePaneType,
            station::{STATION_BUY_ASK, STATION_BUY_BUTTON},
        },
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, SectorRoot, WorldClock, WorldConfig};

    use super::*;

    const WORLD_NAME: &str = "Role Goals World";
    const STATION_SEED: u32 = 10;
    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const SEED_FIELD: &str = "World Seed Field";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const START_ROLE_BUTTON: &str = "Start Role Button: bounty_hunter";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const MAP_TAB: &str = "InterfaceTabMap";
    const JOBS_TAB: &str = "InterfaceTabJobs";
    /// What each step's hand-in pays on every base role.
    const STEP_REWARDS_CR: [u32; 2] = [3_000, 10_000];
    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";
    /// The disclosed fixture balance the player holds once the station is
    /// bought.
    const FIXTURE_CREDITS: u32 = 100_000_000;
    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    const SESSION_SECS: f32 = 240.0;
    const BEAT_SECS: f32 = 60.0;
    /// Frames the Station pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The frame the capture path writes of the chart view's OBJECTIVES block.
    const CHARTER_SHOT: &str = "role-goals-charter.png";

    /// What the chosen station was, the goal and credit events the walk
    /// heard, the balance before a hand-in and the met stamp.
    #[derive(Resource, Default)]
    pub(super) struct GoalWatch {
        /// The chosen station's typed id.
        station: Option<GeneratedBodyId>,
        /// The cell it stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        position: Option<Meters3>,
        /// The first stamp observed once the charter is met.
        met_at: Option<f64>,
        /// Every step's stamp the frame the charter was met: the leave to
        /// the main menu clears the live record before the save is read.
        paid_at: Vec<f64>,
        /// The player's balance the frame a hand-in was pressed.
        before_cr: Option<u32>,
        /// `ongoalstepdone`, `ongoalstephandedin` and `oncreditschanged`
        /// payloads, in firing order.
        events: Vec<(String, serde_json::Value)>,
    }

    impl GoalWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(found, _)| found == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    /// Keep each goal step and credit change scenario event the world fires.
    pub(super) fn watch_goal_events(event: On<GameEvent>, mut watch: ResMut<GoalWatch>) {
        let name = event.name();
        if !matches!(
            name,
            "ongoalstepdone" | "ongoalstephandedin" | "oncreditschanged"
        ) {
            return;
        }
        let data = event.info().data.clone().unwrap_or_default();
        info!("role goals: {name} {data}");
        watch.events.push((name.to_string(), data));
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("role goals: exactly one player ship")
    }

    fn world_armed(world: &World) -> bool {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && the_player(world).is_some()
    }

    fn saved_and_idle(world: &World) -> bool {
        world
            .get_resource::<WorldSaveSession>()
            .is_some_and(|session| {
                session.is_idle() && matches!(session.status(), WorldSaveStatus::Saved { .. })
            })
    }

    fn armed_and_saved() -> Arc<Predicate> {
        when(|world| world_armed(world) && saved_and_idle(world))
    }

    fn set_field(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("role goals: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// The record's stamps, one per paid step.
    fn paid(world: &World) -> Vec<f64> {
        world
            .get_resource::<GoalRecord>()
            .map(|record| record.paid_at.clone())
            .unwrap_or_default()
    }

    /// Whether step `step` is the current one and its line is met.
    fn step_ready(world: &World, step: usize) -> bool {
        paid(world).len() == step
            && world
                .get_resource::<GoalProgress>()
                .and_then(|progress| progress.steps.get(step))
                .is_some_and(|goal_step| goal_step.line.is_met())
    }

    fn progress_is_met(world: &World) -> bool {
        world
            .get_resource::<GoalProgress>()
            .zip(world.get_resource::<GoalRecord>())
            .is_some_and(|(progress, record)| record.met_at(progress.steps.len()).is_some())
    }

    fn balance(world: &World) -> u32 {
        world
            .get::<ShipCredits>(player(world))
            .expect("role goals: the player has a balance")
            .0
    }

    fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && pane.is_none_or(|pane| {
                    world
                        .get_resource::<InterfacePaneType>()
                        .is_some_and(|shown| *shown == pane)
                })
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goals: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goals: release {label}"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    fn charter_jobs() -> Vec<Job> {
        [1, 2, 9]
            .into_iter()
            .enumerate()
            .map(|(index, x)| {
                let cell = SectorCoord::new(x, 0, 0);
                let station = GeneratedBodyId::new(cell, "cluster_station", 0);
                Job {
                    id: JobId::slot(station.clone(), 0, 0),
                    kind: JobType::Neutralize {
                        hull: GeneratedBodyId::new(cell, "cluster_hull", index as u32),
                        hull_name: format!("Fixture target {index}"),
                    },
                    target: station,
                    leg_m: 0,
                    threat: 0,
                    reward: JobReward::flat(400),
                }
            })
            .collect()
    }

    /// Disclosed fixture state: complete three valid neutralize jobs through
    /// the player's production Jobs API, each naming a distinct hull.
    fn inject_completed_jobs(world: &mut World) {
        let fixtures = charter_jobs();
        let mut jobs = world.resource_mut::<Jobs>();
        assert!(
            jobs.is_empty(),
            "role goals: the new world starts with no jobs"
        );
        for job in fixtures {
            job.validate()
                .unwrap_or_else(|error| panic!("role goals: invalid fixture job: {error}"));
            let id = job.id.clone();
            jobs.accept(job);
            jobs.mark_done(&id);
            jobs.complete(&id);
        }
        jobs.validate()
            .unwrap_or_else(|error| panic!("role goals: invalid completed jobs: {error}"));
    }

    /// The charter's title and each step's line with its reward.
    fn read_progress(world: &World) -> (String, Vec<(GoalLine, u32)>) {
        let progress = world.resource::<GoalProgress>();
        (
            progress.title.clone(),
            progress
                .steps
                .iter()
                .map(|step| (step.line.clone(), step.reward_cr))
                .collect(),
        )
    }

    /// The kill list's two steps at `bounties` handed in and `stations`
    /// owned, each with its reward.
    fn expected_steps(bounties: u64, stations: u64) -> Vec<(GoalLine, u32)> {
        vec![
            (
                GoalLine::new("Bounties handed in", bounties, 3),
                STEP_REWARDS_CR[0],
            ),
            (
                GoalLine::new("Stations owned", stations, 1),
                STEP_REWARDS_CR[1],
            ),
        ]
    }

    fn step_lines(steps: &[(GoalLine, u32)]) -> Vec<String> {
        steps
            .iter()
            .map(|(line, reward)| {
                format!(
                    "{} {} / {} for {reward} cr",
                    line.label, line.have, line.need
                )
            })
            .collect()
    }

    /// Claim 1: the charter opens standing with both steps at zero.
    fn assert_standing_charter(world: &mut World) {
        let (title, steps) = read_progress(world);
        assert_eq!(title, "A kill list", "role goals: charter title");
        assert_eq!(
            steps,
            expected_steps(0, 0),
            "role goals: the charter opens with zero progress"
        );
        assert!(paid(world).is_empty(), "role goals: no step is paid");
        nova_probe::probe_marker(
            world,
            "outcome: the charter opens standing with its steps at zero",
            serde_json::json!({ "title": title, "steps": step_lines(&steps), "paid_at": paid(world) }),
        );
        info!("role goals: the kill list opens standing at zero");
    }

    /// Claim 2: three hand-ins ready step 1, and the tracker raises its done
    /// event once.
    fn assert_step_one_ready(world: &mut World) {
        let (_, steps) = read_progress(world);
        assert_eq!(
            steps,
            expected_steps(3, 0),
            "role goals: three hand-ins meet step 1's line"
        );
        assert!(
            step_ready(world, 0),
            "role goals: step 1 is current and ready"
        );
        assert!(
            paid(world).is_empty(),
            "role goals: nothing paid before the hand-in"
        );
        let done: Vec<serde_json::Value> = world
            .resource::<GoalWatch>()
            .named("ongoalstepdone")
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(
            done,
            vec![serde_json::json!({ "step": 1, "reward_cr": STEP_REWARDS_CR[0] })],
            "role goals: one OnGoalStepDone for step 1"
        );
        nova_probe::probe_marker(
            world,
            "outcome: three neutralize hand-ins ready goal step 1",
            serde_json::json!({ "steps": step_lines(&steps), "done": done }),
        );
        info!("role goals: PASS three hand-ins ready goal step 1");
    }

    /// Note the balance before a hand-in press.
    fn note_balance(world: &mut World) {
        let before = balance(world);
        world.resource_mut::<GoalWatch>().before_cr = Some(before);
        info!("role goals: the balance before the hand-in is {before} cr");
    }

    /// The hand-in of step `step` paid its reward to the player with cause
    /// `GoalReward`, fired `OnGoalStepHandedIn` and stamped the step: the
    /// stamp, after its checks.
    fn assert_paid_step(world: &mut World, step: usize) -> f64 {
        let reward = STEP_REWARDS_CR[step];
        let before = world
            .resource::<GoalWatch>()
            .before_cr
            .expect("role goals: the balance was noted before the press");
        assert_eq!(
            balance(world),
            before + reward,
            "role goals: step {} pays {reward} cr",
            step + 1
        );
        let watch = world.resource::<GoalWatch>();
        let rewarded: Vec<&serde_json::Value> = watch
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "GoalReward")
            .collect();
        assert_eq!(
            rewarded.len(),
            step + 1,
            "role goals: one OnCreditsChanged with cause GoalReward per paid step: {rewarded:?}"
        );
        assert_eq!(
            rewarded[step]["delta_cr"],
            i64::from(reward),
            "{rewarded:?}"
        );
        assert_eq!(
            rewarded[step]["current_cr"],
            before + reward,
            "{rewarded:?}"
        );
        let handed: Vec<&serde_json::Value> = watch.named("ongoalstephandedin");
        assert_eq!(
            handed.last().copied(),
            Some(&serde_json::json!({ "step": step + 1, "reward_cr": reward })),
            "role goals: OnGoalStepHandedIn names step {}: {handed:?}",
            step + 1
        );
        assert_eq!(
            handed.len(),
            step + 1,
            "role goals: one handed-in event per step"
        );
        let stamps = paid(world);
        assert_eq!(
            stamps.len(),
            step + 1,
            "role goals: step {} is stamped",
            step + 1
        );
        let clock = world.resource::<WorldClock>().play_seconds;
        let at = stamps[step];
        assert!(
            at.is_finite() && at > 0.0 && at <= clock,
            "role goals: stamp {at} is within the positive play clock {clock}"
        );
        at
    }

    /// Claim 3: step 1's hand-in in flight pays 3,000 cr.
    fn assert_step_one_paid(world: &mut World) {
        assert!(
            world.get::<DockedShip>(player(world)).is_none(),
            "role goals: the step 1 hand-in happens in flight"
        );
        let at = assert_paid_step(world, 0);
        assert!(
            !progress_is_met(world),
            "role goals: one paid step does not meet a two-step charter"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the goal step 1 hand-in pays 3,000 cr in flight",
            serde_json::json!({ "reward_cr": STEP_REWARDS_CR[0], "cause": "GoalReward", "paid_at": at }),
        );
        info!(
            "role goals: PASS goal step 1 paid {} cr in flight at {at} s",
            STEP_REWARDS_CR[0]
        );
    }

    /// Claim 4: the bought station readies step 2.
    fn assert_step_two_ready(world: &mut World) {
        let (_, steps) = read_progress(world);
        assert_eq!(
            steps,
            expected_steps(3, 1),
            "role goals: the bought station meets step 2's line"
        );
        assert!(
            step_ready(world, 1),
            "role goals: step 2 is current and ready"
        );
        let done: Vec<serde_json::Value> = world
            .resource::<GoalWatch>()
            .named("ongoalstepdone")
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(
            done.last(),
            Some(&serde_json::json!({ "step": 2, "reward_cr": STEP_REWARDS_CR[1] })),
            "role goals: OnGoalStepDone for step 2: {done:?}"
        );
        assert_eq!(
            done.len(),
            2,
            "role goals: one done event per step: {done:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: buying a station readies goal step 2",
            serde_json::json!({ "steps": step_lines(&steps) }),
        );
        info!("role goals: PASS buying a station readies goal step 2");
    }

    /// The entity wearing `id`, if one does.
    fn wearing(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn station_root(world: &World) -> Option<Entity> {
        wearing(world, world.resource::<GoalWatch>().station.as_ref()?)
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("role goals: the station stands")
    }

    fn owned(world: &World, station: Entity) -> bool {
        world.get::<OwnedStationMarker>(station).is_some()
    }

    /// The chosen station reads the player's own side: the world layer's
    /// `sync_civilization_sides` wrote its civilization's stance there.
    fn station_is_ally() -> Arc<Predicate> {
        when(|world| {
            station_root(world)
                .and_then(|station| world.get::<Allegiance>(station))
                .is_some_and(|allegiance| *allegiance == Allegiance::Player)
        })
    }

    /// Warm the chosen station's civilization to Ally the way a player
    /// does: one merit per handed-in job, [`MERIT_STEP`] merits per step.
    /// Nothing when it is Ally already.
    ///
    /// # Panics
    ///
    /// The civilization never reaches Ally within twice [`MERIT_STEP`]
    /// merits.
    fn warm_to_ally(world: &mut World) {
        let station = station(world);
        let civ = world
            .get::<TraderMarket>(station)
            .expect("role goals: the chosen station prices goods")
            .civilization;
        let now = world.resource::<WorldClock>().play_seconds;
        let seeded = seeded_stance(&civ);
        let mut stances = world.resource_mut::<CivilizationStances>();
        let mut merits = 0;
        while stances.effective(&civ, seeded, now) != StanceType::Ally {
            record_merit(&mut stances, civ, now);
            merits += 1;
            assert!(
                merits <= 2 * MERIT_STEP,
                "role goals: '{}' never warmed to Ally",
                civ.name()
            );
        }
        info!(
            "role goals: '{}' warmed to Ally after {merits} merit(s)",
            civ.name()
        );
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
    }

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window around `centre`.
    fn window_is(
        centre: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        when(move |world| {
            if !world_armed(world) {
                return false;
            }
            let Some(centre) = centre(world) else {
                return false;
            };
            let (_, radius) = world_shape(world);
            live_cells(world) == desired_sectors(centre, radius)
        })
    }

    fn the_station_stands() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| {
                world.get::<Position>(station).is_some()
                    && world.get::<StationServices>(station).is_some()
            })
        })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    /// Pick the nearest station that is living, generated, not Enemy, and
    /// stands outside the origin's cell, where the walk stands after its
    /// jump.
    fn choose_the_station(world: &mut World) {
        let (edge, _) = world_shape(world);
        let seen: Vec<(GeneratedBodyId, Option<Allegiance>, bool, Vec3)> = world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                Has<DerelictShipMarker>,
                Option<&Allegiance>,
            ), With<StationMarker>>()
            .iter(world)
            .map(|(id, position, derelict, side)| (id.clone(), side.copied(), derelict, position.0))
            .collect();
        let mut stations: Vec<(f32, GeneratedBodyId, Vec3)> = seen
            .into_iter()
            .filter(|(_, side, derelict, at)| {
                !derelict
                    && side.is_some_and(|side| side != Allegiance::Enemy)
                    && SectorCoord::containing(Meters3::from_engine(*at), edge)
                        != SectorCoord::ORIGIN
            })
            .map(|(id, _, _, at)| {
                (
                    Meters3::from_engine(at).distance(Meters3::ZERO).get(),
                    id,
                    at,
                )
            })
            .collect();
        stations.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((distance, id, at)) = stations.into_iter().next() else {
            panic!(
                "role goals: the window on seed {STATION_SEED} holds no living non-Enemy \
                 station outside the origin's cell"
            );
        };
        let position = Meters3::from_engine(at);
        let cell = SectorCoord::containing(position, edge);
        info!("role goals: chose '{id}' in cell {cell}, {distance:.0} m out");
        let mut watch = world.resource_mut::<GoalWatch>();
        watch.station = Some(id);
        watch.cell = Some(cell);
        watch.position = Some(position);
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("role goals: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the station toward the
    /// origin, in the station's own cell.
    fn park_beside(world: &mut World) {
        let watch = world.resource::<GoalWatch>();
        let position = watch.position.expect("the station is chosen");
        let toward_origin = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_origin * PARK_DISTANCE.get());
        let (edge, _) = world_shape(world);
        assert_eq!(
            SectorCoord::containing(beside, edge),
            watch.cell.expect("the station is chosen"),
            "role goals: the park point stands in the station's cell"
        );
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goals: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goals: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (
                With<DockingSectionMarker>,
                Without<DockedPort>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .collect();
        let mut ports: Vec<Entity> = ports
            .into_iter()
            .filter(|port| {
                let mut at = *port;
                while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                    if parent == root {
                        return true;
                    }
                    at = parent;
                }
                false
            })
            .collect();
        ports.sort();
        ports
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("role goals: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull.
    fn outward_port(world: &mut World, target: Entity) -> Entity {
        free_ports(world, target)
            .into_iter()
            .map(|port| {
                let (at, rotation) = local_pose(world, port, target);
                let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
                (port, facing)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(port, _)| port)
            .unwrap_or_else(|| panic!("role goals: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, moving and
    /// turning with the station, then ask to dock. Every frame until the
    /// dock takes.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let position = world.get::<Position>(station).expect("a station stands").0;
        let rotation = world.get::<Rotation>(station).expect("a station stands").0;
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let (at, turn) =
            parked_hull_pose((position, rotation), (port_at, port_rot), (own_at, own_rot));
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        // Point the mouse rig where the berth points the hull, as a pilot who
        // flew in would. Left on the spawn heading, it turns the hull back
        // the moment it undocks and sweeps it through the station.
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`].
    fn set_fixture_credits(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("role goals: the player has a balance")
            .0 = FIXTURE_CREDITS;
        info!("role goals: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
    }

    /// Whether the named node lies inside the Station panel's box, where a
    /// click lands on it.
    fn in_panel(world: &World, name: &str) -> bool {
        match (
            ui_node_rect(world, name),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
        }
    }

    /// Open the Station pane and scroll its Buy station button into view.
    fn open_the_station_pane(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goals: open the Station pane",
            "InterfaceTabStation",
            the_interface_shows(Some(InterfacePaneType::Station)),
            SESSION_SECS,
        )
        .step("role goals: settle the station lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goals: scroll the Buy button into view")
        .each(move |world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, STATION_BUY_BUTTON) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(|world| in_panel(world, STATION_BUY_BUTTON)))
        .deadline(SESSION_SECS)
        .add()
        .step("role goals: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
    }

    fn named_node(world: &mut World, name: &str) -> bool {
        world
            .query::<&Name>()
            .iter(world)
            .any(|named| named.as_str() == name)
    }

    /// Step 2's hand-in pays 10,000 cr and meets the charter at its stamp.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let at = assert_paid_step(world, 1);
        let met_at = world
            .resource::<GoalRecord>()
            .met_at(2)
            .expect("role goals: two paid steps meet the charter");
        assert_eq!(
            met_at, at,
            "role goals: the charter is met at the second stamp"
        );
        let stamps = paid(world);
        let mut watch = world.resource_mut::<GoalWatch>();
        watch.met_at = Some(met_at);
        watch.paid_at = stamps;
    }

    /// Claim 5: the met stamp is unchanged after two more frames.
    fn assert_stamp_stable(world: &mut World) {
        assert!(
            progress_is_met(world),
            "role goals: the kill-list charter stays met"
        );
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the first stamp was captured");
        let stamps = paid(world);
        assert_eq!(stamps.len(), 2, "role goals: no step is paid twice");
        assert_eq!(
            stamps.last().copied(),
            Some(expected),
            "role goals: the met stamp does not move"
        );
        let completed = world.resource::<Jobs>().completed().to_vec();
        assert_eq!(completed.len(), 3, "role goals: three jobs completed");
        assert!(
            completed
                .iter()
                .all(|job| matches!(&job.kind, JobType::Neutralize { .. })),
            "role goals: every fixture hand-in is Neutralize"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the goal step 2 hand-in meets the charter",
            serde_json::json!({
                "reward_cr": STEP_REWARDS_CR[1],
                "cause": "GoalReward",
                "paid_at": stamps,
            }),
        );
        info!("role goals: PASS goal step 2 meets the charter at {expected} s");
    }

    /// Claim 6: the chart view's OBJECTIVES block holds the met goal's stamp
    /// row in place of the current step.
    fn assert_map_charter(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let row = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, named)| named.as_str() == "MapChartObjectiveMet")
            .map(|(entity, _)| entity)
            .expect("role goals: the OBJECTIVES block holds the met row");
        let mut q_children = world.query::<&Children>();
        let mut q_text = world.query::<&Text>();
        let mut open = vec![row];
        let mut texts = Vec::new();
        while let Some(entity) = open.pop() {
            if let Ok(text) = q_text.get(world, entity) {
                texts.push(text.0.clone());
            }
            if let Ok(children) = q_children.get(world, entity) {
                open.extend(children.iter());
            }
        }
        let met_row = play_time_text(expected);
        assert!(
            texts.contains(&met_row),
            "role goals: the met row reads the stamp {met_row}: {texts:?}"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goals: a met role lists no milestone row"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the charter met",
            serde_json::json!({ "stamp": met_row }),
        );
        info!("role goals: the Map chart view shows the met charter: {met_row}");
    }

    /// Read the leave save from disk and release its lock before the Load step.
    fn assert_leave_save(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let root = nova_assets::storage::worlds_root()
            .expect("role goals: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("role goals: world name has a slug");
        let (_folder, lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
            world.resource::<GameShipDesigns>(),
            world.resource::<GameTradeProfiles>(),
            world.resource::<GameRecipes>(),
            world.resource::<GameConsumables>(),
            world.resource::<GameEncounters>(),
        )
        .unwrap_or_else(|refusal| panic!("role goals: cannot open the leave save: {refusal}"));
        let stamps = world.resource::<GoalWatch>().paid_at.clone();
        assert_eq!(
            state.goal.paid_at, stamps,
            "role goals: the leave save keeps both stamps"
        );
        assert_eq!(
            state.goal.met_at(2),
            Some(expected),
            "role goals: the leave save records the charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps both paid steps",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "paid_at": state.goal.paid_at,
            }),
        );
        info!("role goals: leave save keeps the charter met at {expected} s");
        drop(lock);
    }

    /// Claim 8: a Load restores both stamps and the met charter.
    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let (title, steps) = read_progress(world);
        assert_eq!(title, "A kill list", "role goals: loaded title");
        assert_eq!(steps, expected_steps(3, 1), "role goals: loaded steps");
        assert!(progress_is_met(world), "role goals: loaded progress is met");
        let stamps = paid(world);
        assert_eq!(
            stamps,
            world.resource::<GoalWatch>().paid_at,
            "role goals: Load restores both stamps"
        );
        assert_eq!(
            stamps.last().copied(),
            Some(expected),
            "role goals: Load restores the original met stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a load restores both paid steps",
            serde_json::json!({ "title": title, "steps": step_lines(&steps), "paid_at": stamps }),
        );
        info!("role goals: PASS Load restored the met charter at {expected} s");
    }

    /// Open the interface on the Jobs pane and press step `step`'s Hand in,
    /// noting the balance first.
    fn hand_in_step(
        script: AutopilotPlugin<GameStates>,
        step: usize,
    ) -> AutopilotPlugin<GameStates> {
        let button = format!("{GOAL_HAND_IN_PREFIX}{step}");
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            &format!("role goals: open the Jobs pane for step {}", step + 1),
            JOBS_TAB,
            the_interface_shows(Some(InterfacePaneType::Jobs)),
            BEAT_SECS,
        )
        .step(format!(
            "role goals: settle the Jobs pane for step {}",
            step + 1
        ))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!(
            "role goals: note the balance before step {}",
            step + 1
        ))
        .on_enter(note_balance)
        .add()
        .click_named(
            &format!("role goals: hand goal step {} in", step + 1),
            &button,
            when(move |world| paid(world).len() > step),
            BEAT_SECS,
        );
        // The scenario drains the pane's credit change on the next fixed
        // tick, and the open interface holds the clock: the `GoalReward`
        // event comes once the pane shuts.
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step(format!("role goals: hear the step {} reward", step + 1))
            .until(when(move |world| {
                world
                    .resource::<GoalWatch>()
                    .named("oncreditschanged")
                    .into_iter()
                    .filter(|data| data["cause"] == "GoalReward")
                    .count()
                    > step
            }))
            .deadline(BEAT_SECS)
            .add()
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goals: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "role goals: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("role goals: name the world and enter seed 10")
            .on_enter(|world: &mut World| {
                set_field(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "role goals: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "role goals: pick bounty hunter",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "role goals: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goals: the world arms and saves")
            .until(armed_and_saved())
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: assert the standing charter")
            .on_enter(assert_standing_charter)
            .add()
            .step("role goals: inject disclosed fixture hand-ins")
            .on_enter(inject_completed_jobs)
            .until(frames(1))
            .add()
            .step("role goals: wait for step 1 to read ready")
            .until(when(|world| step_ready(world, 0)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: assert three hand-ins ready step 1")
            .on_enter(assert_step_one_ready)
            .add()
            .step("role goals: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(player_free(), frames(SETTLE_FRAMES)))
            .deadline(SESSION_SECS)
            .add();
        let script = hand_in_step(script, 0)
            .step("role goals: assert the step 1 hand-in in flight")
            .on_enter(assert_step_one_paid)
            .add()
            .step("role goals: jump to the origin, the spawn before the docked start")
            .on_enter(|world: &mut World| park_player(world, Meters3::ZERO))
            .until(and(
                window_is(|_| Some(SectorCoord::ORIGIN)),
                when(saved_and_idle),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: choose the station")
            .on_enter(choose_the_station)
            .add()
            .step("role goals: jump to the station's side")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<GoalWatch>().cell),
                the_station_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: berth and dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: set the fixture balance")
            .on_enter(set_fixture_credits)
            .until(frames(1))
            .add()
            .step("role goals: warm the station's civilization to Ally")
            .on_enter(warm_to_ally)
            .until(station_is_ally())
            .deadline(SESSION_SECS)
            .add();
        let script = open_the_station_pane(script)
            .click_named(
                "role goals: ask to buy the station",
                STATION_BUY_BUTTON,
                ui_node_present(STATION_BUY_ASK),
                SESSION_SECS,
            )
            .click_named(
                "role goals: confirm the buy",
                STATION_BUY_BUTTON,
                when(|world| station_root(world).is_some_and(|station| owned(world, station))),
                SESSION_SECS,
            );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goals: wait for step 2 to read ready")
            .until(when(|world| step_ready(world, 1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: assert the bought station readies step 2")
            .on_enter(assert_step_two_ready)
            .add();
        let script = hand_in_step(script, 1)
            .step("role goals: check the met stamp")
            .on_enter(assert_met_and_capture_stamp)
            .add()
            .step("role goals: hold the stamp for two frames")
            .until(frames(2))
            .add()
            .step("role goals: assert the stamp is stable")
            .on_enter(assert_stamp_stable)
            .add();
        let script = tap(
            script,
            "interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goals: open Map pane",
            MAP_TAB,
            the_interface_shows(Some(InterfacePaneType::Map)),
            BEAT_SECS,
        );
        let script = tap(
            script,
            "chart key",
            "map_chart",
            ui_node_present("MapChartObjectiveMet"),
        )
        .step("role goals: settle the objectives block")
        .until(frames(6))
        .add()
        .step("role goals: assert the Map charter")
        .on_enter(assert_map_charter)
        .add()
        .step("role goals: shoot the objectives block")
        .on_enter(|world: &mut World| shoot(world, CHARTER_SHOT))
        .until(shot_written(CHARTER_SHOT))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(script, "interface key", "interface_toggle", unpaused())
            .step("role goals: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: release Escape")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "role goals: leave to main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("role goals: assert the leave save")
            .on_enter(assert_leave_save)
            .add()
            .click_named(
                "role goals: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goals: world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "role goals: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("role goals: world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "role goals: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("role goals: wait for the loaded world")
            .until(when(world_armed))
            .deadline(SESSION_SECS)
            .add()
            .step("role goals: let the goal tracker run after Load")
            .until(and(when(progress_is_met), frames(1)))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: assert the loaded charter")
            .on_enter(assert_loaded_charter)
            .add();
        script
    }
}
