//! encounter_sandbox: start in open space and call each open-world encounter by key, to try it by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed, as the Create
//! World modal does, but without a save folder: nothing this run does is
//! saved. When the window has streamed in and the start has docked the
//! player, the sandbox undocks and puts the hull in open space, 14 km out, at
//! rest. From there the open world runs as shipped: fly faster than 50 m/s
//! and the travel clock counts toward the next drawn encounter (300 to 900 s).
//! The keys below ask the director for one now, as the `encounter <id>`
//! command does; it stands 6 km ahead of the nose. An ask is refused while
//! an encounter is live or a hail job is held.
//!
//! | key | what it does |
//! | - | - |
//! | `1` | distress hail: a comms line, the `HAIL <NAME>` notice and a `HAIL` chip on the caller |
//! | `2` | supply hail: the same, and the job asks for goods in the hold |
//! | `3` | ambush: raiders ahead (the hull must carry a weapon) |
//! | `4` | derelict find: a wreck under a `SIGNAL` chip |
//! | `5` | jump 1.5 km off the live encounter's first body |
//! | `6` | jump 14 km on into open space |
//!
//! Try: `1`, then TAB > Jobs: the offer is the one row under Available, with
//! its range, threat, reward and time left; Accept moves it to Your jobs and
//! `X` declines it. `5`, fly to the caller and dock (or dock without Accept:
//! the dock takes the offer): the job pays its reward at once and the notice
//! reads `JOB-1 PAID <reward> CR`. `2`, `5`, dock without the goods: the job
//! is held, and its row under Your jobs shows Hand in while the dock holds. `4`, `5`, fly in: within 1 km the `SIGNAL` chip
//! goes and the wreck stays to dock with. `3`, `5`: fight the raiders, or
//! press `6` three times to leave them 30 km behind, which ends the ambush.
//! The diagnostics panel (`F3`) reads the record: draws spent, travel
//! seconds against the next gap, the live encounter and the offer's time
//! left.
//!
//! `NOVA_SANDBOX_ROLE=<start role id>` picks the start role; the default
//! `bounty_hunter` carries weapons, so the ambush can fire.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk presses `4` and reads the `SIGNAL`
//! chip, jumps off the wreck and closes inside 1 km until the find ends;
//! presses `3`, reads the raiders standing and hostile from where they wait
//! and leaves them past 30 km until the ambush ends. Before any hail it reads
//! the Jobs pane with Available empty over Your jobs. It presses `1`, reads
//! the hail on the HUD and the offer row under Available with its Accept and
//! `X` inside the pane, accepts it, reads it under Your jobs, docks with the
//! caller and reads the pay (the reward and no more, with no Station Control
//! line); presses `1` again and docks with no Accept for the same pay;
//! presses `2`, docks with the asked goods out of the hold, reads the held
//! job's Hand in disabled, loads the goods and hands in for the pay; presses
//! `2` again and declines the offer with `X`. `NOVA_CAPTURE=1` shoots
//! `sandbox-encounter-signal.png`, `sandbox-encounter-ambush.png`,
//! `sandbox-encounter-empty.png`, `sandbox-encounter-hail.png`,
//! `sandbox-encounter-pane.png`, `sandbox-encounter-held.png`,
//! `sandbox-encounter-paid.png` and `sandbox-encounter-short.png`. Unset, no
//! walk and no deadline run.

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};
use nova_world::prelude::WorldConfig;

#[path = "../shared/encounter_range.rs"]
mod encounter_range;

use encounter_range::{
    berth_and_dock, body_wearing, open_space, park_off, park_player, the_player, window_streamed,
};

#[derive(Parser)]
#[command(name = "encounter_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start in open space on a pinned seed and call each open-world encounter by key: \
             the two hails, the ambush and the derelict find",
    long_about = None
)]
struct Cli;

/// The world seed: `system_encounters`'s.
const SANDBOX_SEED: u32 = 17;
/// The start role the player spawns as when `NOVA_SANDBOX_ROLE` is unset: an
/// armed hull, which the ambush asks for.
const SANDBOX_ROLE: &str = "bounty_hunter";
/// How far off a body key `5` puts the hull.
const JUMP_OFF: Meters = Meters(1_500.0);
/// The designs keys `1` to `4` ask for.
const CALLS: [(KeyCode, &str); 4] = [
    (KeyCode::Digit1, "base/distress_call"),
    (KeyCode::Digit2, "base/supply_call"),
    (KeyCode::Digit3, "base/ambush"),
    (KeyCode::Digit4, "base/derelict_find"),
];
/// Key `5`: jump off the live encounter's first body.
const JUMP_KEY: KeyCode = KeyCode::Digit5;
/// Key `6`: jump on into open space.
const LEAVE_KEY: KeyCode = KeyCode::Digit6;

/// The start role this run plays: `NOVA_SANDBOX_ROLE`, else [`SANDBOX_ROLE`].
fn sandbox_role() -> String {
    std::env::var("NOVA_SANDBOX_ROLE").unwrap_or_else(|_| SANDBOX_ROLE.to_string())
}

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
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(OnEnter(GameStates::MainMenu), start_the_world);
    app.add_systems(
        Update,
        (drive_the_sandbox, read_the_keys)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        read_the_record.in_set(diagnostics::DiagnosticsSystems),
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
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str =
    "Open space. 1 distress hail, 2 supply hail, 3 ambush, 4 derelict find, 5 jump to it, 6 leave.";

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The world has not started yet.
    #[default]
    Menu,
    /// New Game is up; the window around the start is streaming in and the
    /// start is docking the player at its starting station.
    Streaming,
    /// The player was asked to let go of the starting station.
    Undocking,
    /// The hull stands in open space. The player has the run.
    Ready,
}

/// The sandbox's setup state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
}

/// Start New Game on [`SANDBOX_SEED`] the first time the main menu comes up,
/// with no save folder. A later Back to Main Menu stays on the menu.
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
        sandbox_role().as_str().into(),
        &loaded,
        &styles,
        &items,
        &consumables,
        &profiles,
    )
    .unwrap_or_else(|refusal| panic!("encounter sandbox: seed {SANDBOX_SEED} refused: {refusal}"));
    commands.insert_resource(session);
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    sandbox.phase = SandboxPhaseType::Streaming;
    info!("encounter sandbox: New Game on seed {SANDBOX_SEED}, no save folder");
}

/// Advance the setup one phase at a time, each on a world value.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    let Some(player) = the_player(world) else {
        return;
    };
    match phase {
        SandboxPhaseType::Menu | SandboxPhaseType::Ready => {}
        SandboxPhaseType::Streaming => {
            if window_streamed(world) && world.get::<DockedShip>(player).is_some() {
                world.trigger(DockingReleaseRequest { entity: player });
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Undocking;
            }
        }
        SandboxPhaseType::Undocking => {
            if world.get::<DockedShip>(player).is_none() {
                leave(world);
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
                info!("encounter sandbox: the hull stands in open space; keys 1 to 6 are live");
            }
        }
    }
}

/// Key `6`: put the hull at rest in open space, 14 km on. With no clear
/// axis the hull stays where it is.
fn leave(world: &mut World) {
    match open_space(world) {
        Some((at, heading)) => park_player(world, at, heading, MetersPerSecond(0.0)),
        None => warn!("encounter sandbox: no axis out of here is clear of stations and bodies"),
    }
}

/// Key `5`: put the hull [`JUMP_OFF`] from the live encounter's first body.
fn jump_to_the_encounter(world: &mut World) {
    let body = world
        .resource::<Encounters>()
        .live
        .as_ref()
        .and_then(|live| live.bodies.first().cloned())
        .and_then(|id| body_wearing(world, &id));
    match body {
        Some(body) => park_off(world, body, JUMP_OFF),
        None => info!("encounter sandbox: no live encounter body stands to jump to"),
    }
}

/// The sandbox keys, read in flight only: the shell and the panes take
/// digits as text.
fn read_the_keys(world: &mut World) {
    let flying = world.resource::<Sandbox>().phase == SandboxPhaseType::Ready
        && *world.resource::<State<PauseStates>>().get() == PauseStates::Unpaused;
    if !flying {
        return;
    }
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let call = CALLS
        .iter()
        .find(|(key, _)| keys.just_pressed(*key))
        .map(|(_, id)| *id);
    let (jump, away) = (keys.just_pressed(JUMP_KEY), keys.just_pressed(LEAVE_KEY));
    if let Some(id) = call {
        world
            .resource_mut::<Messages<EncounterFireCommand>>()
            .write(EncounterFireCommand { design: id.into() });
        info!("encounter sandbox: asked the director for '{id}'");
    }
    if jump {
        jump_to_the_encounter(world);
    }
    if away {
        leave(world);
    }
}

/// The diagnostics panel: the encounter record and the distance to the
/// live encounter's first body.
fn read_the_record(world: &mut World) {
    let Some(config) = world.get_resource::<WorldConfig<NovaLayeredWorld>>() else {
        return;
    };
    let Some(designs) = world.get_resource::<GameEncounters>() else {
        return;
    };
    let record = world.resource::<Encounters>().clone();
    // The gap does not read the hull, so either answer gives it.
    let gap = plan_encounter_draw(config.seed, record.fired, designs, true).gap_s;
    let clock = world.resource::<WorldClock>().play_seconds;
    let at = |entity: Entity| world.get::<Position>(entity).map(|position| position.0);
    let range = record
        .live
        .as_ref()
        .and_then(|live| live.bodies.first())
        .and_then(|id| body_wearing(world, id))
        .and_then(at)
        .zip(the_player(world).and_then(at))
        .map(|(body, player)| Meters::from_engine(body.distance(player)));
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!(
        "Encounters: {} fired, travel {:.0} s of {gap} s",
        record.fired, record.travel_seconds
    ));
    match (&record.live, range) {
        (Some(live), Some(range)) => panel.line(format!(
            "Live: {} - {} - {} bodies - {:.0} m",
            live.design.as_str(),
            live.name,
            live.bodies.len(),
            range.get()
        )),
        (Some(live), None) => panel.line(format!("Live: {} - out of view", live.design.as_str())),
        (None, _) => panel.line("Live: none"),
    }
    if let Some(offer) = &record.offer {
        panel.detail(format!(
            "offer: {} cr, {:.0} s left",
            offer.job.reward.total_cr(),
            offer.expires_at - clock
        ));
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::{
            jobs::prelude::{
                JOBS_AVAILABLE, JOBS_HELD, JOBS_PANE, JOB_ACCEPT_PREFIX, JOB_HAIL_DECLINE,
                JOB_HAND_IN_PREFIX,
            },
            pane::InterfacePaneType,
        },
    };
    use nova_ui::prelude::VerbGate;

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds a click or a short world beat gets.
    const BEAT_SECS: f32 = 120.0;
    /// Frames a pane or the HUD gets to draw.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The most a job row of one line stands high, in logical px: a list
    /// row, its border and layout rounding. A row that wrapped is two.
    const ROW_LINE_PX: f32 = 32.0;
    /// How far inside the derelict find's 1 km the walk closes.
    const FOUND_RANGE: Meters = Meters(800.0);
    /// How far past the ambush's 30 km the walk leaves the raiders.
    const BREAK_OFF: Meters = Meters(31_000.0);
    const SIGNAL_SHOT: &str = "sandbox-encounter-signal.png";
    const AMBUSH_SHOT: &str = "sandbox-encounter-ambush.png";
    const HAIL_SHOT: &str = "sandbox-encounter-hail.png";
    const PANE_SHOT: &str = "sandbox-encounter-pane.png";
    const HELD_SHOT: &str = "sandbox-encounter-held.png";
    const PAID_SHOT: &str = "sandbox-encounter-paid.png";
    const EMPTY_SHOT: &str = "sandbox-encounter-empty.png";
    const SHORT_SHOT: &str = "sandbox-encounter-short.png";
    /// Who speaks the open world scenario's job lines. A hail job fires no
    /// job event, so the walk hears none.
    const STATION_CONTROL: &str = "Station Control";

    /// The player's balance when the hail job was accepted.
    #[derive(Resource)]
    struct BalanceAtAccept(u32);

    /// The goods the standing supply offer asks for, taken out of the hold
    /// so the dock finds it short.
    #[derive(Resource)]
    struct AskedGoods(ItemDesignId, u32);

    fn credits(world: &World) -> u32 {
        let player = the_player(world).expect("encounter sandbox: one player ship");
        world
            .get::<ShipCredits>(player)
            .expect("encounter sandbox: the player has credits")
            .0
    }

    /// The dock paid the hail job's reward and nothing over it, the HUD
    /// notice reads that sum, and no job event reached the scenario.
    fn assert_the_pay(world: &mut World) {
        let before = world.resource::<BalanceAtAccept>().0;
        let reward = world
            .resource::<Jobs>()
            .completed()
            .iter()
            .rfind(|job| job.id.is_hail())
            .expect("encounter sandbox: the hail job is completed")
            .reward
            .total_cr();
        assert!(reward > 0, "encounter sandbox: the hail job pays");
        assert_eq!(
            credits(world),
            before + reward,
            "encounter sandbox: the dock paid the reward and nothing over it"
        );
        let paid = notice(world);
        let sum = paid
            .split_once(" PAID ")
            .and_then(|(_, sum)| sum.strip_suffix(" CR"))
            .and_then(|sum| sum.replace(',', "").parse::<u32>().ok());
        assert_eq!(
            sum,
            Some(reward),
            "encounter sandbox: the notice '{paid}' reads the {reward} cr reward"
        );
        let spoken: Vec<String> = world
            .resource::<NovaEventWorld>()
            .story_messages()
            .iter()
            .filter(|cue| cue.speaker == STATION_CONTROL)
            .map(|cue| cue.text.clone())
            .collect();
        assert!(
            spoken.is_empty(),
            "encounter sandbox: a hail job fires no job event, but Station Control said \
             {spoken:?}"
        );
        info!(
            "encounter sandbox: PASS the hail job paid its {reward} cr reward and no more \
             ({before} -> {}), with no Station Control line: '{paid}'",
            before + reward
        );
    }

    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn record(world: &World) -> &Encounters {
        world.resource::<Encounters>()
    }

    /// The live encounter is of `design`.
    fn live_is(design: &'static str) -> Arc<Predicate> {
        when(move |world| {
            record(world)
                .live
                .as_ref()
                .is_some_and(|live| live.design.as_str() == design)
        })
    }

    fn nothing_is_live() -> Arc<Predicate> {
        when(|world| record(world).live.is_none() && record(world).offer.is_none())
    }

    /// The live encounter's bodies that stand.
    fn bodies(world: &World) -> Vec<Entity> {
        record(world)
            .live
            .iter()
            .flat_map(|live| &live.bodies)
            .filter_map(|id| body_wearing(world, id))
            .collect()
    }

    /// The first standing body wears the chip `label` and the HUD drew one
    /// chip for it.
    fn the_chip_shows(label: &'static str) -> Arc<Predicate> {
        when(move |world| {
            bodies(world).first().is_some_and(|body| {
                world
                    .get::<ObjectiveMarkerTarget>(*body)
                    .is_some_and(|marker| marker.label == label)
                    && world
                        .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
                        .is_some_and(|mut chips| {
                            chips.iter(world).filter(|target| ***target == *body).count() == 1
                        })
            })
        })
    }

    fn notice(world: &World) -> String {
        world
            .try_query_filtered::<&Text, With<HudNoticeText>>()
            .and_then(|mut texts| texts.iter(world).next().map(|text| text.0.clone()))
            .unwrap_or_default()
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

    fn docked(world: &World) -> bool {
        the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
    }

    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("encounter sandbox: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    fn describe(world: &World) -> String {
        let record = record(world);
        format!(
            "fired {}, live {:?}, {} bodies standing, offer {}, docked {}, notice '{}'",
            record.fired,
            record.live.as_ref().map(|live| live.design.as_str()),
            bodies(world).len(),
            record.offer.is_some(),
            docked(world),
            notice(world),
        )
    }

    /// Press and let go of the sandbox key `key`; the beat lands on `landed`.
    fn key(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        key: KeyCode,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("encounter sandbox: press {label}"))
            .on_enter(press_key(key))
            .until(frames(2))
            .add()
            .step(format!("encounter sandbox: let {label} up"))
            .on_enter(release_key(key))
            .until(landed)
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
    }

    fn tap(
        script: AutopilotPlugin<GameStates>,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step("encounter sandbox: press the interface key")
            .on_enter(press_action("interface_toggle"))
            .until(frames(1))
            .add()
            .step("encounter sandbox: let the interface key up")
            .on_enter(release_action("interface_toggle"))
            .until(landed)
            .deadline(BEAT_SECS)
            .add()
    }

    fn open_the_jobs_pane(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        tap(script, the_interface_shows(None)).click_named(
            "encounter sandbox: open the Jobs tab",
            "InterfaceTabJobs",
            the_interface_shows(Some(InterfacePaneType::Jobs)),
            BEAT_SECS,
        )
    }

    /// How many job rows whose `Name` starts with `prefix` the pane draws.
    fn rows(world: &World, prefix: &str) -> usize {
        world.try_query::<&Name>().map_or(0, |mut names| {
            names
                .iter(world)
                .filter(|name| name.starts_with(prefix))
                .count()
        })
    }

    /// The pane draws Available over Your jobs with `available` rows in the
    /// first and `held` in the second.
    fn the_split_lists(available: usize, held: usize) -> Arc<Predicate> {
        when(move |world| {
            let (Some(top), Some(under)) = (
                ui_node_rect(world, JOBS_AVAILABLE),
                ui_node_rect(world, JOBS_HELD),
            ) else {
                return false;
            };
            top.max.y <= under.min.y + 0.5
                && rows(world, "JobRowPosted-") == available
                && rows(world, "JobRowHeld-") == held
        })
    }

    /// How the held row's Hand in answers a press, while the pane draws one.
    fn hand_in_gate(world: &World) -> Option<VerbGate> {
        let hand_in = format!("{JOB_HAND_IN_PREFIX}0");
        world
            .try_query::<(&Name, &VerbGate)>()?
            .iter(world)
            .find(|(name, _)| name.as_str() == hand_in)
            .map(|(_, gate)| gate.clone())
    }

    fn open_the_jobs_tab(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        open_the_jobs_pane(script)
            .step("encounter sandbox: the pane lists the offer")
            .until(and(
                ui_node_present(format!("{JOB_ACCEPT_PREFIX}0")),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: select the offer row")
            .on_enter(|world: &mut World| activate_named(world, "JobRowPosted-0"))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_SECS)
            .add()
    }

    fn shot(
        script: AutopilotPlugin<GameStates>,
        name: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("encounter sandbox: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, name))
            .until(shot_written(name))
            .deadline(SHOT_SECS)
            .add()
    }

    /// The offer is the one row of Available and no held row, it is one
    /// line and its two buttons stand inside the pane.
    fn assert_the_offer_row(world: &mut World) {
        assert!(
            !docked(world),
            "encounter sandbox: the pane is read undocked"
        );
        assert!(
            the_split_lists(1, 0)(world),
            "encounter sandbox: the offer is under Available and not under Your jobs: {} \
             available, {} held",
            rows(world, "JobRowPosted-"),
            rows(world, "JobRowHeld-")
        );
        let rect = |name: &str| {
            ui_node_rect(world, name)
                .unwrap_or_else(|| panic!("encounter sandbox: '{name}' is not laid out"))
        };
        let pane = rect(JOBS_PANE);
        let row = rect("JobRowPosted-0");
        assert!(
            row.height() <= ROW_LINE_PX,
            "encounter sandbox: the offer row is one line, not {} px high",
            row.height()
        );
        for name in [
            format!("{JOB_ACCEPT_PREFIX}0"),
            JOB_HAIL_DECLINE.to_string(),
        ] {
            let button = rect(&name);
            assert!(
                pane.contains(button.min) && pane.contains(button.max),
                "encounter sandbox: '{name}' {button:?} stands inside the pane {pane:?}"
            );
        }
        info!(
            "encounter sandbox: PASS the offer row is one {} px line with Accept and X in the pane",
            row.height()
        );
    }

    /// Jump off the caller and dock with it.
    fn dock_with_the_caller(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        key(script, "5, the jump", JUMP_KEY, frames(SETTLE_FRAMES))
            .step("encounter sandbox: berth and dock with the caller")
            .each(|world, _, _| {
                // The dock pays the job and ends the record on the frame it
                // takes, before this step reads the dock.
                if let Some(caller) = bodies(world).first().copied() {
                    berth_and_dock(world, caller);
                }
            })
            .until(when(docked))
            .diagnose(describe)
            .deadline(SETUP_SECS)
            .add()
    }

    /// Hail job `paid` is paid, counted from the first the walk was paid
    /// for: read the pay.
    fn be_paid(script: AutopilotPlugin<GameStates>, paid: usize) -> AutopilotPlugin<GameStates> {
        script
            .step("encounter sandbox: the job pays")
            .until(and(
                nothing_is_live(),
                when(move |world| {
                    world
                        .resource::<Jobs>()
                        .completed()
                        .iter()
                        .filter(|job| job.id.is_hail())
                        .count()
                        == paid
                        && notice(world).contains("PAID")
                }),
            ))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            // Before any shot: the notice fades four seconds after it posts.
            .step("encounter sandbox: assert the pay")
            .on_enter(assert_the_pay)
            .add()
    }

    /// Note the balance with the offer standing and no job held: what comes
    /// next answers the hail by the dock alone.
    fn note_the_standing_offer(world: &mut World) {
        assert!(
            record(world).offer.is_some() && world.resource::<Jobs>().held().is_empty(),
            "encounter sandbox: the offer stands and no job is held"
        );
        let balance = credits(world);
        world.insert_resource(BalanceAtAccept(balance));
    }

    /// Take every unit the standing supply offer asks for out of the hold.
    fn unload_the_asked_goods(world: &mut World) {
        let offer = record(world).offer.clone();
        let Some(JobType::Supply { item, count, .. }) = offer.map(|offer| offer.job.kind) else {
            panic!("encounter sandbox: the supply hail offers a supply job");
        };
        let player = the_player(world).expect("encounter sandbox: one player ship");
        let mut hold = world
            .get_mut::<ShipInventory>(player)
            .expect("encounter sandbox: the player has a hold");
        let aboard = hold.count(&item);
        if aboard > 0 {
            hold.remove(&item, aboard);
        }
        world.insert_resource(AskedGoods(item, count));
    }

    /// Put the asked goods in the hold, as a trade while docked would.
    fn load_the_asked_goods(world: &mut World) {
        let AskedGoods(item, count) = world
            .remove_resource::<AskedGoods>()
            .expect("encounter sandbox: the asked goods were noted");
        let items = world.resource::<GameItems>().clone();
        let player = the_player(world).expect("encounter sandbox: one player ship");
        world
            .get_mut::<ShipInventory>(player)
            .expect("encounter sandbox: the player has a hold")
            .add(&items, &item, count);
    }

    /// Let go of the caller and stand in open space again.
    fn stand_off(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("encounter sandbox: undock")
            .on_enter(|world: &mut World| {
                let player = the_player(world).expect("encounter sandbox: one player ship");
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(when(|world| !docked(world)))
            .deadline(SETUP_SECS)
            .add()
            .step("encounter sandbox: stand in open space again")
            .on_enter(leave)
            .until(when(window_streamed))
            .deadline(SETUP_SECS)
            .add()
    }

    /// The walk. Unarmed (no `NOVA_AUTOPILOT`) it adds nothing.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new();
        if std::env::var_os("NOVA_AUTOPILOT").is_none() {
            return script;
        }
        let script = script
            .step("encounter sandbox: the hull stands in open space")
            .until(and(
                when(|world| world.resource::<Sandbox>().phase == SandboxPhaseType::Ready),
                and(when(window_streamed), unpaused()),
            ))
            .deadline(SETUP_SECS)
            .add();

        // No hail yet: the pane still splits, with nothing under Available.
        let script = open_the_jobs_pane(script)
            .step("encounter sandbox: Available is empty over Your jobs")
            .until(and(the_split_lists(0, 0), frames(SETTLE_FRAMES)))
            .diagnose(|world| {
                format!(
                    "Available {:?}, Your jobs {:?}, {} available, {} held",
                    ui_node_rect(world, JOBS_AVAILABLE),
                    ui_node_rect(world, JOBS_HELD),
                    rows(world, "JobRowPosted-"),
                    rows(world, "JobRowHeld-")
                )
            })
            .deadline(BEAT_SECS)
            .add();
        let script = tap(shot(script, EMPTY_SHOT), unpaused());

        // The derelict find.
        let script = key(
            script,
            "4, the derelict find",
            KeyCode::Digit4,
            and(live_is("base/derelict_find"), the_chip_shows(SIGNAL_CHIP)),
        );
        let script = key(script, "5, the jump", JUMP_KEY, frames(SETTLE_FRAMES))
            .step("encounter sandbox: the find stays live 1.5 km off")
            .on_enter(|world: &mut World| {
                assert!(
                    record(world).live.is_some(),
                    "encounter sandbox: 1.5 km off, the find is live"
                );
            })
            .until(the_chip_shows(SIGNAL_CHIP))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, SIGNAL_SHOT)
            .step("encounter sandbox: close inside 1 km and the find ends")
            .on_enter(|world: &mut World| {
                let wreck = bodies(world)[0];
                park_off(world, wreck, FOUND_RANGE);
            })
            .until(nothing_is_live())
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: report the find")
            .on_enter(|world: &mut World| {
                info!(
                    "encounter sandbox: PASS the derelict find wore SIGNAL and ended inside 1 km"
                );
                leave(world);
            })
            .until(when(window_streamed))
            .deadline(SETUP_SECS)
            .add();

        // The ambush.
        let script = key(
            script,
            "3, the ambush",
            KeyCode::Digit3,
            and(
                live_is("base/ambush"),
                when(|world| {
                    let standing = bodies(world);
                    !standing.is_empty()
                        && standing.len()
                            == record(world)
                                .live
                                .as_ref()
                                .map_or(0, |live| live.bodies.len())
                }),
            ),
        )
        .step("encounter sandbox: the raiders are hostile")
        .on_enter(|world: &mut World| {
            let raiders = bodies(world);
            for raider in &raiders {
                assert_eq!(
                    world.get::<Allegiance>(*raider),
                    Some(&Allegiance::Enemy),
                    "encounter sandbox: a raider stands on the Enemy side"
                );
                assert!(
                    world.get::<ObjectiveMarkerTarget>(*raider).is_none(),
                    "encounter sandbox: an ambush wears no chip"
                );
            }
            info!("encounter sandbox: {} raiders stand hostile", raiders.len());
        })
        .add();
        // The walk reads the raiders from where the director put them: a
        // jump to 1.5 km puts an idle hull under their guns.
        let script = shot(script, AMBUSH_SHOT)
            .step("encounter sandbox: leave the raiders past 30 km and the ambush ends")
            .on_enter(|world: &mut World| {
                let raider = bodies(world)[0];
                park_off(world, raider, BREAK_OFF);
            })
            .until(nothing_is_live())
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: report the ambush")
            .on_enter(|world: &mut World| {
                info!("encounter sandbox: PASS the ambush ended with the raiders past 30 km");
                leave(world);
            })
            .until(when(window_streamed))
            .deadline(SETUP_SECS)
            .add();

        // The distress hail, taken and paid.
        let script = key(
            script,
            "1, the distress hail",
            KeyCode::Digit1,
            and(
                and(live_is("base/distress_call"), the_chip_shows(HAIL_CHIP)),
                when(|world| notice(world).starts_with("HAIL ")),
            ),
        );
        let script = shot(script, HAIL_SHOT);
        let script = open_the_jobs_tab(script)
            .step("encounter sandbox: assert the offer row")
            .on_enter(assert_the_offer_row)
            .add();
        let script = shot(script, PANE_SHOT)
            .step("encounter sandbox: click Accept")
            .on_enter(|world: &mut World| activate_named(world, &format!("{JOB_ACCEPT_PREFIX}0")))
            .until(and(
                when(|world| {
                    record(world).offer.is_none()
                        && world
                            .resource::<Jobs>()
                            .held()
                            .iter()
                            .any(|job| job.id.is_hail())
                }),
                frames(SETTLE_FRAMES),
            ))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: the accepted job is under Your jobs")
            .until(and(
                the_split_lists(0, 1),
                // In space the caller is not docked with: no Hand in.
                when(|world| hand_in_gate(world) == Some(VerbGate::Hidden)),
            ))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, HELD_SHOT)
            .step("encounter sandbox: note the balance")
            .on_enter(|world: &mut World| {
                let balance = credits(world);
                world.insert_resource(BalanceAtAccept(balance));
            })
            .add();
        let script = tap(script, unpaused());
        let script = shot(be_paid(dock_with_the_caller(script), 1), PAID_SHOT);
        let script = stand_off(script);

        // The distress hail again, answered by the dock alone: no Accept.
        let script = key(
            script,
            "1, the distress hail",
            KeyCode::Digit1,
            and(live_is("base/distress_call"), the_chip_shows(HAIL_CHIP)),
        )
        .step("encounter sandbox: note the balance with the offer standing")
        .on_enter(note_the_standing_offer)
        .add();
        let script = stand_off(be_paid(dock_with_the_caller(script), 2));

        // The supply hail, taken by a dock with the hold short and handed
        // in from the pane once the goods are aboard, on that same dock.
        let script = key(
            script,
            "2, the supply hail",
            KeyCode::Digit2,
            and(live_is("base/supply_call"), the_chip_shows(HAIL_CHIP)),
        )
        .step("encounter sandbox: take the asked goods out of the hold")
        .on_enter(|world: &mut World| {
            note_the_standing_offer(world);
            unload_the_asked_goods(world);
        })
        .add();
        let script = dock_with_the_caller(script)
            .step("encounter sandbox: the short dock holds the job and pays nothing")
            .until(when(|world| {
                let jobs = world.resource::<Jobs>();
                record(world).offer.is_none()
                    && jobs
                        .held()
                        .iter()
                        .any(|job| job.id.is_hail() && !jobs.is_done(&job.id))
                    && notice(world).contains("SHORT")
                    && credits(world) == world.resource::<BalanceAtAccept>().0
            }))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add();
        let script = open_the_jobs_pane(script)
            .step("encounter sandbox: the held job's Hand in is disabled")
            .until(and(
                and(
                    the_split_lists(0, 1),
                    when(|world| hand_in_gate(world) == Some(VerbGate::Disabled)),
                ),
                frames(SETTLE_FRAMES),
            ))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add();
        let script = shot(script, SHORT_SHOT)
            .step("encounter sandbox: load the asked goods")
            .on_enter(load_the_asked_goods)
            .until(and(
                when(|world| hand_in_gate(world) == Some(VerbGate::Ready)),
                frames(SETTLE_FRAMES),
            ))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: click Hand in")
            .on_enter(|world: &mut World| {
                assert!(docked(world), "encounter sandbox: the dock still holds");
                activate_named(world, &format!("{JOB_HAND_IN_PREFIX}0"));
            })
            .until(the_split_lists(0, 0))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add();
        let script = stand_off(tap(be_paid(script, 3), unpaused()));

        // The supply hail, declined.
        let script = key(
            script,
            "2, the supply hail",
            KeyCode::Digit2,
            and(live_is("base/supply_call"), the_chip_shows(HAIL_CHIP)),
        );
        open_the_jobs_tab(script)
            .step("encounter sandbox: click X")
            .on_enter(|world: &mut World| activate_named(world, JOB_HAIL_DECLINE))
            .until(and(nothing_is_live(), frames(SETTLE_FRAMES)))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("encounter sandbox: report the decline")
            .on_enter(|world: &mut World| {
                assert!(
                    world.resource::<Jobs>().held().is_empty(),
                    "encounter sandbox: a declined offer is no held job"
                );
                info!("encounter sandbox: PASS the X declined the supply offer");
            })
            .add()
    }
}
