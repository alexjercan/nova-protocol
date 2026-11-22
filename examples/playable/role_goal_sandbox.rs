//! role_goal_sandbox: start as the bounty hunter with the role goal's
//! step 1 ready, and hand the steps in from the Jobs pane by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the
//! `bounty_hunter`, as the Create World modal does, but without a save
//! folder: nothing this run does is saved. Once the window has streamed in
//! around the docked start, the fixture lands, once, as DISCLOSED FIXTURE
//! STATE: three valid Neutralize jobs are completed through the production
//! `Jobs` API (`accept`, `mark_done`, `complete`), each naming a distinct
//! hull. That stands the goal's step 1, `Bounties handed in 3 / 3`, ready,
//! and the Jobs tab opens on it. `NOVA_SANDBOX_HUNT=1` leaves the fixture
//! out: the starter board posts the bounty hunter's neutralizations, and
//! three of them handed in ready the step.
//!
//! Try: TAB > Jobs: Your jobs opens with the goal block, `A kill list`
//! over `GOAL-1 Bounties handed in` and `GOAL-2 Stations owned`, step 1 lit
//! with its 3,000 cr reward and an enabled Hand in, step 2 dim. TAB, `D` to
//! undock, TAB > Jobs > Hand in: the payout sound plays, the balance counts
//! up by 3,000 and step 1 reads dim with its play time. TAB: the HUD notice
//! reads `GOAL-1 PAID 3,000 CR`. Step 2 needs a station: dock again, `;` for
//! the shell, `cheats enable`, `credits give player 100000`, TAB > Station >
//! Buy station twice; the HUD notice reads `GOAL-2 done: hand in` once the
//! interface shuts, and the Map's chart view (`M`, then `C`) lists the step
//! as `done`. TAB > Jobs > Hand in pays 10,000 cr: the title reads the play
//! time the goal was met. The diagnostics panel (`F3`) lists each step,
//! the record's stamps and the balance.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk reads step 1 ready on the Jobs tab
//! (shot on the phosphor and the hardware theme), undocks, presses
//! `GoalHandIn-0` by name and checks the balance rose by 3,000 cr with one
//! `OnGoalStepHandedIn`, and that step 2 is now the current step with its
//! Hand in disabled. It shuts the interface, reads `GOAL-1 PAID 3,000 CR`
//! in the HUD notice and hears one `OnCreditsChanged` of cause
//! `GoalReward`: the scenario drains a pane's credit change on the next
//! fixed tick, which the open interface holds. The clock is held while both
//! themes are shot. It docks at the starting
//! station again, sets the balance to 100,000,000 cr (a second piece of
//! DISCLOSED FIXTURE STATE), buys the station through the Station pane's Buy
//! station button, waits for step 2 to read ready, shuts the interface and
//! reads `GOAL-2 done: hand in` in the HUD notice, hands it in by name and
//! checks the 10,000 cr and the met stamp; the done chain is shot on both
//! themes, and the interface shuts for step 2's `GoalReward` change. Each claim is a `nova_probe` marker on the probe timeline
//! (`NOVA_PROBE_TIMELINE`) and a `role goal sandbox: PASS` line.
//! `NOVA_CAPTURE=1` writes `role-goal-ready-<theme>.png`,
//! `role-goal-paid-<theme>.png` and `role-goal-done-<theme>.png` for
//! `phosphor` and `hardware`. Unset, no walk and no deadline run.

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, nova_interface::pane::InterfacePaneType, prelude::*};
use nova_world::prelude::{desired_sectors, SectorRoot, WorldConfig};

#[derive(Parser)]
#[command(name = "role_goal_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start as the bounty hunter with the role goal's step 1 ready, and hand its steps in \
             from the Jobs pane by hand",
    long_about = None
)]
struct Cli;

/// The world seed: the one the start-role sandboxes pin.
const SANDBOX_SEED: u32 = 10;
/// The start role whose goal the run plays.
const SANDBOX_ROLE: &str = "bounty_hunter";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
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

    let autopilot = std::env::var_os("NOVA_AUTOPILOT").is_some();
    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.insert_resource(Sandbox {
        fixture: autopilot || std::env::var_os("NOVA_SANDBOX_HUNT").is_none(),
        ..default()
    });
    app.add_plugins((
        diagnostics::DiagnosticsHintPlugin::new(INTRO),
        goal_overlay::GoalOverlayPlugin,
    ));
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );

    if autopilot {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        // The software rasterizer's frames are seconds long: the clamp
        // keeps the berth from integrating a whole frame of drift.
        app.add_systems(Update, smoke::hold_the_walk_clock);
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<smoke::GoalWatch>();
        app.add_observer(smoke::watch_goal_events);
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The world has not started yet.
    #[default]
    Menu,
    /// New Game is up; the window around the start is streaming in and the
    /// start is docking the player at its starting station.
    Streaming,
    /// Docked, the fixture landed and the Jobs tab open. The player has the
    /// run.
    Ready,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// Land the three completed bounties once the start is docked.
    fixture: bool,
}

/// Start New Game on [`SANDBOX_SEED`] as [`SANDBOX_ROLE`] the first time the
/// main menu comes up, with no save folder. A later Back to Main Menu stays
/// on the menu.
fn start_the_world(
    mut commands: Commands,
    mut sandbox: ResMut<Sandbox>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    items: Res<GameItems>,
    consumables: Res<GameConsumables>,
    profiles: Res<GameTradeProfiles>,
    mut mode: ResMut<GameMode>,
    mut pick: ResMut<NewGameScenario>,
    mut state: ResMut<NextState<GameStates>>,
) {
    if sandbox.phase != SandboxPhaseType::Menu {
        return;
    }
    let session = start_session(
        SANDBOX_SEED,
        SANDBOX_ROLE.into(),
        &loaded,
        &styles,
        &items,
        &consumables,
        &profiles,
    )
    .unwrap_or_else(|refusal| panic!("role goal sandbox: seed {SANDBOX_SEED} refused: {refusal}"));
    info!(
        "role goal sandbox: the start docks at '{}'",
        session.start.id
    );
    commands.insert_resource(session);
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    sandbox.phase = SandboxPhaseType::Streaming;
    info!("role goal sandbox: New Game on seed {SANDBOX_SEED} as {SANDBOX_ROLE}, no save folder");
}

/// Land the fixture and open the Jobs tab once the start stands docked in
/// its streamed window.
fn drive_the_sandbox(world: &mut World) {
    if world.resource::<Sandbox>().phase != SandboxPhaseType::Streaming {
        return;
    }
    let Some(player) = the_player(world) else {
        return;
    };
    if !(start_window_is_live(world) && world.get::<DockedShip>(player).is_some()) {
        return;
    }
    if world.resource::<Sandbox>().fixture {
        complete_three_bounties(world);
    }
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Jobs;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
    info!("role goal sandbox: the Jobs tab is open; TAB closes and reopens the interface");
}

/// Disclosed fixture state: three valid Neutralize jobs completed through
/// the player's production `Jobs` API, each naming a distinct hull, with
/// posters in cells (1, 0, 0), (2, 0, 0) and (9, 0, 0). It replaces flying
/// and fighting, not the goal tracker.
fn complete_three_bounties(world: &mut World) {
    let mut jobs = world.resource_mut::<Jobs>();
    for (index, x) in [1, 2, 9].into_iter().enumerate() {
        let cell = SectorCoord::new(x, 0, 0);
        let station = GeneratedBodyId::new(cell, "cluster_station", 0);
        let job = Job {
            id: JobId::slot(station.clone(), 0, 0),
            kind: JobType::Neutralize {
                hull: GeneratedBodyId::new(cell, "cluster_hull", index as u32),
                hull_name: format!("Fixture target {index}"),
            },
            target: station,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(400),
        };
        job.validate()
            .unwrap_or_else(|error| panic!("role goal sandbox: invalid fixture job: {error}"));
        let id = job.id.clone();
        jobs.accept(job);
        jobs.mark_done(&id);
        jobs.complete(&id);
    }
    jobs.validate()
        .unwrap_or_else(|error| panic!("role goal sandbox: invalid completed jobs: {error}"));
    info!("role goal sandbox: fixture completes three bounties");
}

/// The one player ship, if exactly one stands.
fn the_player(world: &mut World) -> Option<Entity> {
    let mut players = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// Whether the world is armed and the live set is exactly the window around
/// the player's cell.
fn start_window_is_live(world: &mut World) -> bool {
    let Some((edge, radius)) = world
        .get_resource::<WorldConfig<NovaLayeredWorld>>()
        .map(|config| (config.sector_edge, config.active_radius))
    else {
        return false;
    };
    let Some(player) = the_player(world) else {
        return false;
    };
    let Some(at) = world.get::<Position>(player).map(|position| position.0) else {
        return false;
    };
    let centre = SectorCoord::containing(Meters3::from_engine(at), edge);
    let live: std::collections::BTreeSet<SectorCoord> = world
        .query::<&SectorRoot>()
        .iter(world)
        .map(|root| root.0)
        .collect();
    live == desired_sectors(centre, radius)
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str =
    "Bounty hunter with goal step 1 ready: TAB > Jobs > Hand in, undocked or docked.";

/// The diagnostics panel: each goal step, the record's stamps and the
/// balance.
mod goal_overlay {
    use super::*;

    pub(super) struct GoalOverlayPlugin;

    impl Plugin for GoalOverlayPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(
                PostUpdate,
                read_the_goal.in_set(diagnostics::DiagnosticsSystems),
            );
        }
    }

    fn read_the_goal(world: &mut World) {
        let (Some(progress), Some(record)) = (
            world.get_resource::<GoalProgress>().cloned(),
            world.get_resource::<GoalRecord>().cloned(),
        ) else {
            return;
        };
        let credits =
            the_player(world).and_then(|player| world.get::<ShipCredits>(player).map(|c| c.0));
        let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
        panel.line(format!(
            "Goal: {} ({} / {} paid)",
            progress.title,
            record.current().min(progress.steps.len()),
            progress.steps.len()
        ));
        if let Some(credits) = credits {
            panel.line(format!("Balance: {credits} cr"));
        }
        for (index, step) in progress.steps.iter().enumerate() {
            let state = match record.paid_at.get(index) {
                Some(at) => format!("paid at {}", play_time_text(*at)),
                None if index == record.current() && step.line.is_met() => "ready".to_string(),
                None if index == record.current() => "current".to_string(),
                None => "later".to_string(),
            };
            panel.detail(format!(
                "GOAL-{} {} {} / {} - {} cr - {state}",
                index + 1,
                step.line.label,
                step.line.have,
                step.line.need,
                step.reward_cr
            ));
        }
    }
}

/// The autopilot smoke walk, inert unless `NOVA_AUTOPILOT` is set.
mod smoke {
    use std::sync::Arc;

    use bevy::ui::InteractionDisabled;
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{
            jobs::prelude::{GOAL_HAND_IN_PREFIX, JOBS_GOAL_STEP_PREFIX},
            station::{STATION_BUY_ASK, STATION_BUY_BUTTON},
        },
    };
    use nova_ui::theme::{ActiveUiTheme, SelectedUiTheme};

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds a beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 10;
    /// Frames a theme switch gets to re-tint.
    const THEME_FRAMES: u32 = 4;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    const JOBS_TAB: &str = "InterfaceTabJobs";
    const STATION_TAB: &str = "InterfaceTabStation";
    /// The Station pane's panel, which scrolls under the wheel.
    const STATION_PANEL: &str = "StationPanel";
    /// What each step's hand-in pays on every base role.
    const STEP_REWARDS_CR: [u32; 2] = [3_000, 10_000];
    /// The disclosed fixture balance that makes the station affordable.
    const FIXTURE_CREDITS: u32 = 100_000_000;
    /// The themes every frame is shot on, by id and file tag.
    const THEMES: [(&str, &str); 2] = [
        (nova_ui::theme::PHOSPHOR_THEME_ID, "phosphor"),
        (nova_ui::theme::HARDWARE_THEME_ID, "hardware"),
    ];

    /// The goal and credit events the walk heard and the balance before a
    /// hand-in.
    #[derive(Resource, Default)]
    pub(super) struct GoalWatch {
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
        info!("role goal sandbox: {name} {data}");
        watch.events.push((name.to_string(), data));
    }

    /// Hold the virtual clock's clamp every frame: a scenario load hands
    /// `Time<Virtual>` back at its default.
    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The one player ship, from a shared world.
    fn player(world: &World) -> Entity {
        let mut players = world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .expect("role goal sandbox: the player query");
        let mut players = players.iter(world);
        let player = players.next().expect("role goal sandbox: a player ship");
        assert!(
            players.next().is_none(),
            "role goal sandbox: one player ship"
        );
        player
    }

    fn balance(world: &World) -> u32 {
        world
            .get::<ShipCredits>(player(world))
            .expect("role goal sandbox: the player has a balance")
            .0
    }

    fn docked(world: &World) -> bool {
        world.get::<DockedShip>(player(world)).is_some()
    }

    fn paid(world: &World) -> Vec<f64> {
        world.resource::<GoalRecord>().paid_at.clone()
    }

    /// Whether step `step` is the current one and its line is met.
    fn step_ready(world: &World, step: usize) -> bool {
        paid(world).len() == step
            && world
                .resource::<GoalProgress>()
                .steps
                .get(step)
                .is_some_and(|goal_step| goal_step.line.is_met())
    }

    /// The entity named `name`, if one stands.
    fn named(world: &World, name: &str) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &Name)>()?;
        query
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
    }

    /// The texts under `entity`, depth first.
    fn texts_under(world: &World, entity: Entity) -> Vec<String> {
        let mut open = vec![entity];
        let mut texts = Vec::new();
        while let Some(at) = open.pop() {
            if let Some(text) = world.get::<Text>(at) {
                texts.push(text.0.clone());
            }
            if let Some(children) = world.get::<Children>(at) {
                open.extend(children.iter());
            }
        }
        texts
    }

    /// Whether the Hand in of `step` stands, and whether it is enabled.
    fn hand_in(world: &World, step: usize) -> Option<bool> {
        let button = named(world, &format!("{GOAL_HAND_IN_PREFIX}{step}"))?;
        Some(world.get::<InteractionDisabled>(button).is_none())
    }

    fn the_interface_shows(pane: Option<InterfacePaneType>) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && pane.is_none_or(|pane| world.get_resource::<InterfacePaneType>() == Some(&pane))
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("role goal sandbox: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("role goal sandbox: release {label}"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(BEAT_SECS)
            .add()
    }

    /// Shoot `what` on each theme, then go back to the default theme.
    fn shoot_both_themes(
        mut script: AutopilotPlugin<GameStates>,
        what: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        for (theme, tag) in THEMES {
            let shot = format!("role-goal-{what}-{tag}.png");
            let written = shot.clone();
            script = script
                .step(format!("role goal sandbox: switch to {tag} for {what}"))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<SelectedUiTheme>().0 = theme.to_string();
                })
                .until(when(move |world| {
                    world.resource::<ActiveUiTheme>().id() == theme
                }))
                .deadline(BEAT_SECS)
                .add()
                .step(format!("role goal sandbox: settle {tag} for {what}"))
                .until(frames(THEME_FRAMES))
                .add()
                .step(format!("role goal sandbox: shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &shot))
                .until(shot_written(written))
                .deadline(SHOT_SECS)
                .add();
        }
        script
            .step(format!(
                "role goal sandbox: back to the default theme after {what}"
            ))
            .on_enter(|world: &mut World| {
                *world.resource_mut::<SelectedUiTheme>() = SelectedUiTheme::default();
            })
            .until(frames(THEME_FRAMES))
            .add()
    }

    /// Claim 1: docked on the Jobs tab, step 1 reads ready with an enabled
    /// Hand in and step 2 later with none.
    fn assert_step_one_ready(world: &mut World) {
        assert!(
            step_ready(world, 0),
            "role goal sandbox: step 1 is current and ready"
        );
        assert_eq!(
            hand_in(world, 0),
            Some(true),
            "role goal sandbox: GoalHandIn-0 is enabled"
        );
        assert_eq!(
            hand_in(world, 1),
            None,
            "role goal sandbox: a later step has no Hand in"
        );
        let row = named(world, &format!("{JOBS_GOAL_STEP_PREFIX}0"))
            .map(|row| texts_under(world, row))
            .expect("role goal sandbox: the step 1 row");
        assert!(
            row.contains(&"GOAL-1 Bounties handed in".to_string())
                && row.contains(&"3 / 3".to_string()),
            "role goal sandbox: step 1 reads its line: {row:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the jobs pane reads goal step 1 ready",
            serde_json::json!({ "row": row }),
        );
        info!("role goal sandbox: PASS the Jobs pane reads goal step 1 ready: {row:?}");
    }

    fn note_balance(world: &mut World) {
        let before = balance(world);
        world.resource_mut::<GoalWatch>().before_cr = Some(before);
        info!("role goal sandbox: the balance before the hand-in is {before} cr");
    }

    /// The `OnCreditsChanged` events of cause `GoalReward` heard so far.
    fn rewarded(world: &World) -> Vec<serde_json::Value> {
        world
            .resource::<GoalWatch>()
            .named("oncreditschanged")
            .into_iter()
            .filter(|data| data["cause"] == "GoalReward")
            .cloned()
            .collect()
    }

    /// Whether the `GoalReward` credit change of step `step` was heard. The
    /// scenario drains a pane's credit reports on the next fixed tick, and
    /// the open interface holds the clock: it fires once the pane shuts.
    fn reward_heard(step: usize) -> Arc<Predicate> {
        when(move |world| rewarded(world).len() > step)
    }

    /// Step `step`'s reward fired one `OnCreditsChanged` of cause
    /// `GoalReward` on the player's ship, its delta the reward.
    fn assert_reward_event(world: &mut World, step: usize) {
        let reward = STEP_REWARDS_CR[step];
        let rewarded = rewarded(world);
        assert_eq!(rewarded.len(), step + 1, "role goal sandbox: {rewarded:?}");
        assert_eq!(
            rewarded[step]["delta_cr"],
            i64::from(reward),
            "{rewarded:?}"
        );
        nova_probe::probe_marker(
            world,
            &format!(
                "outcome: goal step {} fires one OnCreditsChanged with cause GoalReward",
                step + 1
            ),
            rewarded[step].clone(),
        );
        info!(
            "role goal sandbox: PASS goal step {} fires OnCreditsChanged GoalReward: {}",
            step + 1,
            rewarded[step]
        );
    }

    /// Step `step`'s hand-in paid its reward, fired one
    /// `OnGoalStepHandedIn` and stamped the step.
    fn assert_paid_step(world: &mut World, step: usize) -> serde_json::Value {
        let reward = STEP_REWARDS_CR[step];
        let watch = world.resource::<GoalWatch>();
        let before = watch
            .before_cr
            .expect("role goal sandbox: the balance was noted");
        let handed: Vec<serde_json::Value> = watch
            .named("ongoalstephandedin")
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(
            balance(world),
            before + reward,
            "role goal sandbox: step {} pays {reward} cr",
            step + 1
        );
        assert_eq!(handed.len(), step + 1, "role goal sandbox: {handed:?}");
        assert_eq!(
            handed[step],
            serde_json::json!({ "step": step + 1, "reward_cr": reward }),
            "role goal sandbox: OnGoalStepHandedIn names the step"
        );
        let stamps = paid(world);
        assert_eq!(
            stamps.len(),
            step + 1,
            "role goal sandbox: the step is stamped"
        );
        serde_json::json!({
            "before_cr": before,
            "after_cr": balance(world),
            "handed_in": handed[step],
            "paid_at": stamps,
        })
    }

    /// Claim 2: in flight, step 1's Hand in pays 3,000 cr, and step 2 is
    /// the current step with its Hand in disabled.
    fn assert_step_one_paid(world: &mut World) {
        assert!(
            !docked(world),
            "role goal sandbox: the step 1 hand-in is in flight"
        );
        let data = assert_paid_step(world, 0);
        assert_eq!(
            hand_in(world, 1),
            Some(false),
            "role goal sandbox: GoalHandIn-1 is disabled"
        );
        assert_eq!(
            hand_in(world, 0),
            None,
            "role goal sandbox: a paid step has no Hand in"
        );
        nova_probe::probe_marker(
            world,
            "outcome: goal step 1 pays 3,000 cr in flight",
            data.clone(),
        );
        info!("role goal sandbox: PASS goal step 1 paid in flight: {data}");
        let row = named(world, &format!("{JOBS_GOAL_STEP_PREFIX}1"))
            .map(|row| texts_under(world, row))
            .expect("role goal sandbox: the step 2 row");
        assert!(
            row.contains(&"GOAL-2 Stations owned".to_string())
                && row.contains(&"0 / 1".to_string()),
            "role goal sandbox: step 2 reads its line: {row:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: goal step 2 is current after step 1 is paid",
            serde_json::json!({ "row": row }),
        );
        info!("role goal sandbox: PASS goal step 2 is current: {row:?}");
    }

    /// The HUD notice slot reads `line`.
    fn the_notice_reads(line: String) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query_filtered::<&Text, With<HudNoticeText>>()
                .is_some_and(|mut texts| texts.iter(world).any(|text| text.0 == line))
        })
    }

    fn set_virtual_paused(paused: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let mut time = world.resource_mut::<Time<Virtual>>();
            if paused {
                time.pause();
            } else {
                time.unpause();
            }
        }
    }

    /// The starting station's root.
    fn start_station(world: &World) -> Option<Entity> {
        let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| **worn == id)
            .map(|(entity, _)| entity)
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount
    /// chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("role goal sandbox: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("role goal sandbox: a section's chain reaches its root")
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

    /// Put the player on the berth at the starting station's most outward
    /// free port, moving and turning with the station, then ask to dock.
    /// Every frame until the dock takes.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = start_station(world).expect("role goal sandbox: the starting station");
        let station_port = free_ports(world, station)
            .into_iter()
            .map(|port| {
                let (at, rotation) = local_pose(world, port, station);
                (port, (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero()))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(port, _)| port)
            .expect("role goal sandbox: the starting station has a free port");
        let player_port = free_ports(world, player)
            .into_iter()
            .next()
            .expect("role goal sandbox: the player has a free port");
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
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    /// Disclosed fixture state: the player's balance set to
    /// [`FIXTURE_CREDITS`], so the starting station is affordable.
    fn set_fixture_credits(world: &mut World) {
        let player = player(world);
        world
            .get_mut::<ShipCredits>(player)
            .expect("role goal sandbox: the player has a balance")
            .0 = FIXTURE_CREDITS;
        info!("role goal sandbox: fixture sets the player's balance to {FIXTURE_CREDITS} cr");
    }

    /// Whether the named node lies inside the Station panel's box.
    fn in_panel(world: &World, name: &str) -> bool {
        match (
            ui_node_rect(world, name),
            ui_node_rect(world, STATION_PANEL),
        ) {
            (Some(node), Some(panel)) => node.min.y >= panel.min.y && node.max.y <= panel.max.y,
            _ => false,
        }
    }

    fn start_owned(world: &World) -> bool {
        start_station(world)
            .is_some_and(|station| world.get::<OwnedStationMarker>(station).is_some())
    }

    /// Open the interface on the Jobs pane and press step `step`'s Hand in,
    /// noting the balance first. The interface stays open.
    fn hand_in_step(
        script: AutopilotPlugin<GameStates>,
        step: usize,
    ) -> AutopilotPlugin<GameStates> {
        let script = if step == 0 {
            script
        } else {
            tap(
                script,
                "the interface key",
                "interface_toggle",
                the_interface_shows(None),
            )
        };
        script
            .click_named(
                &format!(
                    "role goal sandbox: open the Jobs pane for step {}",
                    step + 1
                ),
                JOBS_TAB,
                the_interface_shows(Some(InterfacePaneType::Jobs)),
                BEAT_SECS,
            )
            .step(format!(
                "role goal sandbox: settle the Jobs pane for step {}",
                step + 1
            ))
            .until(and(
                when(move |world| hand_in(world, step) == Some(true)),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step(format!(
                "role goal sandbox: note the balance before step {}",
                step + 1
            ))
            .on_enter(note_balance)
            .add()
            .click_named(
                &format!("role goal sandbox: hand goal step {} in", step + 1),
                &format!("{GOAL_HAND_IN_PREFIX}{step}"),
                when(move |world| paid(world).len() > step),
                BEAT_SECS,
            )
            .step(format!("role goal sandbox: settle after step {}", step + 1))
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("role goal sandbox: docked at the start on the Jobs tab")
            .until(and(
                when(|world| {
                    world
                        .get_resource::<Sandbox>()
                        .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
                }),
                the_interface_shows(Some(InterfacePaneType::Jobs)),
            ))
            .deadline(SETUP_SECS)
            .add()
            .step("role goal sandbox: step 1 reads ready")
            .until(and(
                when(|world| step_ready(world, 0) && hand_in(world, 0) == Some(true)),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: assert step 1 ready")
            .on_enter(assert_step_one_ready)
            .add();
        let script = shoot_both_themes(script, "ready");
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal sandbox: undock from the starting station")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(and(when(|world| !docked(world)), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        );
        let script = hand_in_step(script, 0)
            .step("role goal sandbox: assert the step 1 hand-in in flight")
            .on_enter(assert_step_one_paid)
            .add();
        // The PAID line was posted under the pane, where the HUD is hidden
        // and the clock held: it shows once the interface shuts. The clock
        // is held again while both themes are shot, so the dwell outlasts
        // the software rasterizer's frames.
        let paid_line = format!("GOAL-1 PAID {}", cr_text(STEP_REWARDS_CR[0]).to_uppercase());
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal sandbox: the HUD notice reads what step 1 paid")
            .until(the_notice_reads(paid_line.clone()))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: hear the step 1 reward")
            .until(reward_heard(0))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: assert the step 1 reward event")
            .on_enter(|world: &mut World| assert_reward_event(world, 0))
            .add()
            .step("role goal sandbox: hold the clock on the paid notice")
            .on_enter(set_virtual_paused(true))
            .until(frames(1))
            .add()
            .step("role goal sandbox: mark the paid notice")
            .on_enter(move |world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: the hud notice reads goal step 1 paid after the pane shuts",
                    serde_json::json!({ "line": paid_line }),
                );
                info!("role goal sandbox: PASS the HUD notice reads '{paid_line}'");
            })
            .add();
        let script = shoot_both_themes(script, "paid")
            .step("role goal sandbox: release the clock")
            .on_enter(set_virtual_paused(false))
            .until(frames(1))
            .add()
            .step("role goal sandbox: dock at the starting station again")
            .each(|world, _, _| berth_and_dock(world))
            .until(when(docked))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: set the fixture balance")
            .on_enter(set_fixture_credits)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "role goal sandbox: open the Station pane",
            STATION_TAB,
            the_interface_shows(Some(InterfacePaneType::Station)),
            BEAT_SECS,
        )
        .step("role goal sandbox: settle the station lists")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("role goal sandbox: scroll the Buy button into view")
        .each(move |world, _, frame| {
            keep_hovering_named(STATION_PANEL)(world);
            if frame % 2 == 1 && !in_panel(world, STATION_BUY_BUTTON) {
                scroll_lines(-1.0)(world);
            }
        })
        .until(when(|world| in_panel(world, STATION_BUY_BUTTON)))
        .deadline(BEAT_SECS)
        .add()
        .step("role goal sandbox: settle the scroll")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "role goal sandbox: ask to buy the station",
            STATION_BUY_BUTTON,
            ui_node_present(STATION_BUY_ASK),
            BEAT_SECS,
        )
        .click_named(
            "role goal sandbox: confirm the buy",
            STATION_BUY_BUTTON,
            when(start_owned),
            BEAT_SECS,
        )
        .step("role goal sandbox: step 2 reads ready")
        .until(when(|world| step_ready(world, 1)))
        .deadline(BEAT_SECS)
        .add()
        .step("role goal sandbox: assert step 2 ready")
        .on_enter(|world: &mut World| {
            let done: Vec<serde_json::Value> = world
                .resource::<GoalWatch>()
                .named("ongoalstepdone")
                .into_iter()
                .cloned()
                .collect();
            assert_eq!(
                done.last(),
                Some(&serde_json::json!({ "step": 2, "reward_cr": STEP_REWARDS_CR[1] })),
                "role goal sandbox: OnGoalStepDone for step 2: {done:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: buying the station readies goal step 2",
                serde_json::json!({ "done": done }),
            );
            info!("role goal sandbox: PASS buying the station readies goal step 2");
        })
        .add();
        // The done line was posted under the pane: it shows once the
        // interface shuts.
        let done_line = "GOAL-2 done: hand in".to_string();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal sandbox: the HUD notice reads step 2 done")
            .until(the_notice_reads(done_line.clone()))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: mark the done notice")
            .on_enter(move |world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: the hud notice reads goal step 2 done after the pane shuts",
                    serde_json::json!({ "line": done_line }),
                );
                info!("role goal sandbox: PASS the HUD notice reads '{done_line}'");
            })
            .add();
        let script = hand_in_step(script, 1)
            .step("role goal sandbox: assert the step 2 hand-in meets the goal")
            .on_enter(|world: &mut World| {
                let data = assert_paid_step(world, 1);
                let stamps = paid(world);
                assert_eq!(
                    world.resource::<GoalRecord>().met_at(2),
                    stamps.last().copied(),
                    "role goal sandbox: the goal is met at the second stamp"
                );
                assert_eq!(
                    hand_in(world, 1),
                    None,
                    "role goal sandbox: no Hand in is left"
                );
                let block = named(
                    world,
                    nova_protocol::nova_interface::jobs::prelude::JOBS_GOAL,
                )
                .map(|block| texts_under(world, block))
                .expect("role goal sandbox: the goal block");
                nova_probe::probe_marker(
                    world,
                    "outcome: goal step 2 pays 10,000 cr and meets the goal",
                    serde_json::json!({ "paid": data, "block": block }),
                );
                info!("role goal sandbox: PASS goal step 2 meets the goal: {block:?}");
            })
            .add();
        let script = shoot_both_themes(script, "done");
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("role goal sandbox: hear the step 2 reward")
            .until(reward_heard(1))
            .deadline(BEAT_SECS)
            .add()
            .step("role goal sandbox: assert the step 2 reward event")
            .on_enter(|world: &mut World| assert_reward_event(world, 1))
            .add()
    }
}
