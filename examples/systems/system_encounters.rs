//! system_encounters: fly in open space until the open world hails the player, take the hail job in space, dock with its caller for the pay, then carry a standing offer through a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed whose first encounter draw is the distress hail, undocks and flies
//! the player's hull through open space at cruise speed. The travel clock
//! counts on its own; the walk then writes the record's travel seconds to
//! two seconds short of the drawn gap, the one test-only write of the range,
//! so the wait is seconds and not minutes. Everything after it is the
//! director's: the draw, the place, the caller hull, the offer and the pay.
//! The second offer is asked for with the `encounter` command at the shell.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: eligible travel fires the drawn encounter` | the travel clock counts while the hull cruises in open space; at the drawn gap the record holds the drawn design live with its caller standing under a cell root ahead of the player, a standing offer, the `HAIL` chip on the caller, the HUD notice and the comms line |
//! | 2 | `outcome: the hail job is accepted in space` | undocked, the Jobs pane lists the offer; the Accept click holds the job, ends the offer and re-labels the caller's chip with the job's code |
//! | 3 | `outcome: the dock with the caller pays the hail job` | the dock marks the job done and pays the reward to the player's credits at once, and the played scenario grants its first-delivery bonus on the hand-in event; the job is listed completed, the live record ends and the caller stays |
//! | 4 | `outcome: the encounter command fires a named encounter` | with cheats armed, `encounter base/distress_call` at the shell puts a second caller ahead with a standing offer |
//! | 5 | `outcome: a standing offer and its caller come back on Load` | after Back to Main Menu and Load the record holds the same offer and live encounter, and the caller stands again wearing the `HAIL` chip |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_encounters --features debug
//! # look for: `encounters: PASS the drawn encounter fired`,
//! #           `encounters: PASS the dock paid`,
//! #           `encounters: PASS the offer came back on Load`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `encounter-hail-hud.png` of the HUD with the
//! chip, the notice and the comms line, `encounter-hail-pane.png` of the Jobs
//! pane undocked with the offer row selected, and `encounter-hail-paid.png`
//! of the HUD after the dock.

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "../shared/encounter_range.rs"]
mod encounter_range;

#[derive(Parser)]
#[command(name = "system_encounters")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, cruise in open space until a hail fires, accept its job \
             in space, dock with the caller for the pay, and carry a second offer through a \
             Load. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

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

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds the encounter's path.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<walk::EncounterWatch>();
        app.add_systems(Update, walk::hold_the_walk_clock);
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

    use avian3d::prelude::Position;
    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::{
            jobs::prelude::JOB_ACCEPT_PREFIX, nova_command::prelude::CommandTerminal,
            pane::InterfacePaneType,
        },
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{SectorRoot, WorldConfig};

    use super::{
        encounter_range::{
            berth_and_dock, body_wearing, open_space, park_off, park_player, the_player,
            window_streamed,
        },
        *,
    };

    /// The world seed: its first two encounter draws are the distress hail,
    /// for an armed hull and for an unarmed one.
    const SEED: u32 = 17;
    /// The design those draws name.
    const DISTRESS: &str = "base/distress_call";
    const START_ROLE_BUTTON: &str = "Start Role Button: bounty_hunter";
    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Encounter World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";
    const JOBS_TAB: &str = "InterfaceTabJobs";
    const HUD_SHOT: &str = "encounter-hail-hud.png";
    const PANE_SHOT: &str = "encounter-hail-pane.png";
    const PAID_SHOT: &str = "encounter-hail-paid.png";

    /// The speed the hull cruises at: above the 50 m/s the travel clock asks.
    const CRUISE: MetersPerSecond = MetersPerSecond(100.0);
    /// How far off a caller the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(1_500.0);
    /// What the open world's scenario grants on the first job handed in. A
    /// paid hail job raises the same event, so the dock grants it too.
    const FIRST_DELIVERY_BONUS_CR: u32 = 50;
    /// How far short of the drawn gap the walk sets the travel seconds.
    const GAP_LEFT_SECS: f64 = 2.0;
    /// Real seconds a session-sized beat gets on a software GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Real seconds a click or a short world beat gets.
    const BEAT_SECS: f32 = 60.0;
    /// Frames a pane gets to draw.
    const SETTLE_FRAMES: u32 = 6;
    /// The virtual clock's clamp: a software frame is seconds long, and the
    /// berth must not integrate a whole frame of drift.
    const WALK_CLOCK_SECS: f32 = 1.0;

    /// What the walk carries between its beats.
    #[derive(Resource, Default)]
    pub(super) struct EncounterWatch {
        /// The way the hull cruises.
        heading: Vec3,
        /// The caller of the offer in hand and the job it offers.
        caller: Option<GeneratedBodyId>,
        job: Option<Job>,
        /// The player's credits before the dock.
        credits: u32,
        /// The record as the leave save took it.
        left: Option<Encounters>,
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

    fn player(world: &World) -> Entity {
        the_player(world).expect("encounters: exactly one player ship")
    }

    fn record(world: &World) -> &Encounters {
        world.resource::<Encounters>()
    }

    fn caller_id(world: &World) -> GeneratedBodyId {
        world
            .resource::<EncounterWatch>()
            .caller
            .clone()
            .expect("encounters: a hail fired")
    }

    fn caller(world: &World) -> Entity {
        let id = caller_id(world);
        body_wearing(world, &id).unwrap_or_else(|| panic!("encounters: no '{id}' stands"))
    }

    fn the_job(world: &World) -> Job {
        world
            .resource::<EncounterWatch>()
            .job
            .clone()
            .expect("encounters: a hail offered a job")
    }

    fn credits(world: &World) -> u32 {
        world
            .get::<ShipCredits>(player(world))
            .expect("encounters: the player has credits")
            .0
    }

    fn docked(world: &World) -> bool {
        the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
    }

    fn player_docked() -> Arc<Predicate> {
        when(docked)
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| the_player(world).is_some() && !docked(world))
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

    /// The line in the HUD notice slot, or an empty one.
    fn notice(world: &World) -> String {
        world
            .try_query_filtered::<&Text, With<HudNoticeText>>()
            .and_then(|mut texts| texts.iter(world).next().map(|text| text.0.clone()))
            .unwrap_or_default()
    }

    /// The label of the marker the body `id` wears, if it stands and wears
    /// one.
    fn marker(world: &World, id: &GeneratedBodyId) -> Option<String> {
        let body = body_wearing(world, id)?;
        world
            .get::<ObjectiveMarkerTarget>(body)
            .map(|marker| marker.label.clone())
    }

    /// How many HUD chips point at the body `id`.
    fn chips_on(world: &World, id: &GeneratedBodyId) -> usize {
        let Some(body) = body_wearing(world, id) else {
            return 0;
        };
        world
            .try_query_filtered::<&AnchoredChipTarget, With<ObjectiveMarkerChipHudMarker>>()
            .map_or(0, |mut chips| {
                chips.iter(world).filter(|target| ***target == body).count()
            })
    }

    /// The offer's caller stands wearing the `HAIL` chip, and the HUD drew
    /// one chip for it.
    fn the_hail_chip_shows() -> Arc<Predicate> {
        when(|world| {
            record(world).offer.as_ref().is_some_and(|offer| {
                let id = &offer.job.id.poster;
                marker(world, id).as_deref() == Some(HAIL_CHIP) && chips_on(world, id) == 1
            })
        })
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("encounters: no field named '{name}'"));
        value.0 = text.to_string();
    }

    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("encounters: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    /// Press and let go of the player action `action`; the beat lands on
    /// `landed`.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("encounters: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("encounters: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Type `line` at the Command shell and enter it, as the CRT prompt does.
    fn enter_command(world: &mut World, line: &str) {
        let mut terminal = world.resource_mut::<CommandTerminal>();
        terminal.reset_prompt();
        terminal.insert_text(line);
        terminal.submit();
        info!("encounters: entered `{line}` at the shell");
    }

    /// Jump the player to open space and set it cruising.
    fn cruise_in_open_space(world: &mut World) {
        let (at, heading) = open_space(world)
            .expect("encounters: an axis out of the start is clear of stations and bodies");
        world.resource_mut::<EncounterWatch>().heading = heading;
        park_player(world, at, heading, CRUISE);
        info!("encounters: the player cruises from {at:?} along {heading:?}");
    }

    /// Hold the cruise speed: the flight computer brakes a hull nobody
    /// flies.
    fn hold_the_cruise(world: &mut World) {
        let heading = world.resource::<EncounterWatch>().heading;
        let player = player(world);
        let at = Meters3::from_engine(
            world
                .get::<Position>(player)
                .expect("encounters: the player stands")
                .0,
        );
        park_player(world, at, heading, CRUISE);
    }

    /// Stop the hull where it is.
    fn stop(world: &mut World) {
        let heading = world.resource::<EncounterWatch>().heading;
        let player = player(world);
        let at = Meters3::from_engine(
            world
                .get::<Position>(player)
                .expect("encounters: the player stands")
                .0,
        );
        park_player(world, at, heading, MetersPerSecond(0.0));
    }

    /// The test-only write: leave [`GAP_LEFT_SECS`] of the drawn gap.
    fn bring_the_draw_near(world: &mut World) {
        let seed = world.resource::<WorldConfig<NovaLayeredWorld>>().seed;
        let designs = world.resource::<GameEncounters>();
        let gaps: Vec<u32> = [false, true]
            .into_iter()
            .map(|armed| {
                let draw = plan_encounter_draw(seed, 0, designs, armed);
                assert_eq!(
                    draw.design.map(|design| design.id.as_str()),
                    Some(DISTRESS),
                    "encounters: seed {seed}'s first draw is the distress hail (armed {armed})"
                );
                draw.gap_s
            })
            .collect();
        let gap = gaps[0];
        let counted = record(world).travel_seconds;
        assert!(
            counted > 0.0 && counted < f64::from(gap),
            "encounters: the clock counted {counted} s of travel, short of the {gap} s gap"
        );
        assert_eq!(record(world).fired, 0, "encounters: nothing fired yet");
        world.resource_mut::<Encounters>().travel_seconds = f64::from(gap) - GAP_LEFT_SECS;
        info!(
            "encounters: the clock counted {counted:.1} s on its own; set to {GAP_LEFT_SECS} s \
             short of the {gap} s gap"
        );
    }

    fn describe_the_travel(world: &World) -> String {
        let record = record(world);
        format!(
            "fired {}, travel {:.1} s, live {:?}, offer {}, docked {}, notice '{}'",
            record.fired,
            record.travel_seconds,
            record.live.as_ref().map(|live| live.design.as_str()),
            record.offer.is_some(),
            docked(world),
            notice(world),
        )
    }

    /// Claim 1: the drawn encounter fired with everything a hail shows.
    fn assert_the_fire(world: &mut World) {
        let record = record(world).clone();
        let live = record.live.clone().expect("encounters: one is live");
        let offer = record.offer.clone().expect("encounters: the hail offers");
        assert_eq!(live.design.as_str(), DISTRESS);
        assert_eq!(record.fired, 1, "encounters: one draw is spent");
        assert_eq!(
            record.travel_seconds, 0.0,
            "encounters: the clock starts over"
        );
        let id = offer.job.id.poster.clone();
        assert!(offer.job.id.is_hail(), "encounters: the job has a hail id");
        assert_eq!(live.bodies, vec![id.clone()]);
        let body =
            body_wearing(world, &id).unwrap_or_else(|| panic!("encounters: no '{id}' stands"));
        let root = world
            .get::<ChildOf>(body)
            .expect("encounters: the caller hangs under a root")
            .parent();
        assert!(
            world.get::<SectorRoot>(root).is_some(),
            "encounters: the caller hangs under a cell root"
        );
        let at = |entity: Entity| {
            Meters3::from_engine(world.get::<Position>(entity).expect("it stands").0)
        };
        let player = player(world);
        let heading = world.resource::<EncounterWatch>().heading;
        let to_caller = at(body).get() - at(player).get();
        let range = to_caller.length();
        assert!(
            (5_000.0..=8_000.0).contains(&range) && to_caller.dot(heading) > 0.0,
            "encounters: the caller stands about 6 km ahead, not {range:.0} m at {to_caller:?}"
        );
        assert_eq!(marker(world, &id).as_deref(), Some(HAIL_CHIP));
        assert_eq!(
            chips_on(world, &id),
            1,
            "encounters: one chip on the caller"
        );
        let name = world
            .get::<Name>(body)
            .expect("encounters: the caller has a name")
            .to_string();
        assert_eq!(live.name, name);
        let line = match &world
            .resource::<GameEncounters>()
            .get(&live.design)
            .expect("encounters: the design is loaded")
            .kind
        {
            EncounterType::Hail { line, .. } => line.clone(),
            other => panic!("encounters: {DISTRESS} is a hail, not {other:?}"),
        };
        assert!(
            world
                .resource::<NovaEventWorld>()
                .story_messages()
                .iter()
                .any(|cue| cue.speaker == name && cue.text == line),
            "encounters: the comms log holds the caller's line"
        );
        assert!(
            !docked(world),
            "encounters: the player is undocked at the hail"
        );
        let mut watch = world.resource_mut::<EncounterWatch>();
        watch.caller = Some(id.clone());
        watch.job = Some(offer.job.clone());
        nova_probe::probe_marker(
            world,
            "outcome: eligible travel fires the drawn encounter",
            serde_json::json!({
                "design": live.design.as_str(),
                "caller": id.to_string(),
                "name": name,
                "range_m": range,
                "reward_cr": offer.job.reward.total_cr(),
                "expires_at": offer.expires_at,
            }),
        );
        info!(
            "encounters: PASS the drawn encounter fired: '{name}' ({id}) hails from {range:.0} m \
             ahead for {} cr",
            offer.job.reward.total_cr()
        );
    }

    /// Claim 2: the accept in space holds the job.
    fn assert_the_accept(world: &mut World) {
        let job = the_job(world);
        let id = caller_id(world);
        let jobs = world.resource::<Jobs>().clone();
        assert!(jobs.holds(&job.id), "encounters: the hail job is held");
        assert!(!jobs.is_done(&job.id), "encounters: the job is not done");
        assert!(!docked(world), "encounters: the accept was in space");
        let record = record(world).clone();
        assert!(record.offer.is_none(), "encounters: the offer ended");
        assert!(
            record.live.is_some(),
            "encounters: the encounter stays live"
        );
        let code = jobs
            .mark_code(&job.id)
            .expect("encounters: a held job has a code");
        let label = format!("{code} {}", job.verb(world.resource::<GameItems>()));
        assert_eq!(
            marker(world, &id).as_deref(),
            Some(label.as_str()),
            "encounters: the caller's chip reads the held job"
        );
        world.resource_mut::<EncounterWatch>().credits = credits(world);
        nova_probe::probe_marker(
            world,
            "outcome: the hail job is accepted in space",
            serde_json::json!({ "job": format!("{:?}", job.id), "chip": label }),
        );
        info!("encounters: PASS the hail job is held in space under the chip '{label}'");
    }

    /// Claim 3: the dock paid.
    fn assert_the_pay(world: &mut World) {
        let job = the_job(world);
        let id = caller_id(world);
        let before = world.resource::<EncounterWatch>().credits;
        let reward = job.reward.total_cr();
        let jobs = world.resource::<Jobs>().clone();
        assert!(reward > 0, "encounters: the hail job pays");
        assert!(
            jobs.is_completed(&job.id) && !jobs.holds(&job.id),
            "encounters: the job is completed and no longer held"
        );
        let after = before + reward + FIRST_DELIVERY_BONUS_CR;
        assert_eq!(
            credits(world),
            after,
            "encounters: the dock paid the reward and the scenario its first-delivery bonus"
        );
        assert_eq!(
            *record(world),
            Encounters {
                fired: 1,
                ..default()
            },
            "encounters: the record ended"
        );
        assert!(
            body_wearing(world, &id).is_some(),
            "encounters: the caller stays in the world"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the dock with the caller pays the hail job",
            serde_json::json!({ "reward_cr": reward, "credits": after }),
        );
        info!(
            "encounters: PASS the dock paid {reward} cr and the scenario \
             {FIRST_DELIVERY_BONUS_CR} cr ({before} -> {after}); notice '{}'",
            notice(world)
        );
    }

    /// Claim 4: the command's encounter stands.
    fn assert_the_command(world: &mut World) {
        let record = record(world).clone();
        let live = record.live.clone().expect("encounters: one is live");
        let offer = record.offer.clone().expect("encounters: the hail offers");
        assert_eq!(live.design.as_str(), DISTRESS);
        assert_eq!(record.fired, 2, "encounters: the command spent a draw");
        let id = offer.job.id.poster.clone();
        assert_ne!(id, caller_id(world), "encounters: a second caller");
        assert!(body_wearing(world, &id).is_some());
        let mut watch = world.resource_mut::<EncounterWatch>();
        watch.caller = Some(id.clone());
        watch.job = Some(offer.job.clone());
        watch.left = Some(record);
        nova_probe::probe_marker(
            world,
            "outcome: the encounter command fires a named encounter",
            serde_json::json!({ "design": DISTRESS, "caller": id.to_string() }),
        );
        info!(
            "encounters: PASS the command fired {DISTRESS}: '{}' ({id})",
            live.name
        );
    }

    /// Claim 5: the Load gave the offer and its caller back.
    fn assert_the_load(world: &mut World) {
        let left = world
            .resource::<EncounterWatch>()
            .left
            .clone()
            .expect("encounters: the record was read before the leave");
        assert_eq!(
            *record(world),
            left,
            "encounters: the loaded record is the one left"
        );
        let id = caller_id(world);
        assert_eq!(marker(world, &id).as_deref(), Some(HAIL_CHIP));
        assert_eq!(
            chips_on(world, &id),
            1,
            "encounters: one chip on the caller"
        );
        assert!(
            !world.resource::<Jobs>().holds(&the_job(world).id),
            "encounters: an offer is not a held job"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a standing offer and its caller come back on Load",
            serde_json::json!({
                "caller": id.to_string(),
                "expires_at": left.offer.as_ref().map(|offer| offer.expires_at),
                "clock": world.resource::<WorldClock>().play_seconds,
            }),
        );
        info!("encounters: PASS the offer came back on Load with its caller '{id}'");
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("encounters: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "encounters: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                BEAT_SECS,
            )
            .step("encounters: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "encounters: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                BEAT_SECS,
            )
            .click_named(
                "encounters: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                BEAT_SECS,
            )
            .click_named(
                "encounters: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("encounters: the window streams in and the start docks the player")
            .until(and(when(window_streamed), player_docked()))
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: the docked start counts no travel")
            .on_enter(|world: &mut World| {
                assert_eq!(*record(world), Encounters::default());
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: cruise in open space")
            .on_enter(cruise_in_open_space)
            .each(|world, _, _| hold_the_cruise(world))
            .until(and(
                when(window_streamed),
                when(|world| record(world).travel_seconds > 1.0),
            ))
            .diagnose(describe_the_travel)
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: bring the drawn gap near and fly it out")
            .on_enter(bring_the_draw_near)
            .each(|world, _, _| hold_the_cruise(world))
            .until(when(|world| record(world).live.is_some()))
            .diagnose(describe_the_travel)
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: stop, and the HUD shows the hail")
            .on_enter(stop)
            .until(and(
                the_hail_chip_shows(),
                when(|world| notice(world).starts_with("HAIL ")),
            ))
            .diagnose(describe_the_travel)
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: assert the fire")
            .on_enter(assert_the_fire)
            .add()
            .step("encounters: shoot the HUD")
            .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
            .until(shot_written(HUD_SHOT))
            .deadline(BEAT_SECS)
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(None),
        )
        .click_named(
            "encounters: open the Jobs tab",
            JOBS_TAB,
            the_interface_shows(Some(InterfacePaneType::Jobs)),
            BEAT_SECS,
        )
        .step("encounters: the pane lists the offer undocked")
        .until(and(
            ui_node_present(format!("{JOB_ACCEPT_PREFIX}0")),
            frames(SETTLE_FRAMES),
        ))
        .deadline(BEAT_SECS)
        .add()
        .step("encounters: select the offer row")
        .on_enter(|world: &mut World| activate_named(world, "JobRowPosted-0"))
        .until(frames(SETTLE_FRAMES))
        .deadline(BEAT_SECS)
        .add()
        .step("encounters: shoot the pane")
        .on_enter(|world: &mut World| {
            assert!(!docked(world), "encounters: the pane is read undocked");
            shoot(world, PANE_SHOT);
        })
        .until(shot_written(PANE_SHOT))
        .deadline(BEAT_SECS)
        .add()
        .step("encounters: click Accept")
        .on_enter(|world: &mut World| activate_named(world, &format!("{JOB_ACCEPT_PREFIX}0")))
        .until(and(
            when(|world| world.resource::<Jobs>().holds(&the_job(world).id)),
            frames(SETTLE_FRAMES),
        ))
        .deadline(BEAT_SECS)
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("encounters: the caller's chip reads the job")
            .until(when(|world| {
                marker(world, &caller_id(world)).is_some_and(|label| label != HAIL_CHIP)
            }))
            .diagnose(|world: &World| format!("chip {:?}", marker(world, &caller_id(world))))
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: assert the accept")
            .on_enter(assert_the_accept)
            .add()
            .step("encounters: park off the caller")
            .on_enter(|world: &mut World| {
                let caller = caller(world);
                park_off(world, caller, PARK_DISTANCE);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("encounters: berth and dock with the caller")
            .each(|world, _, _| {
                let caller = caller(world);
                berth_and_dock(world, caller);
            })
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: the dock pays")
            .until(and(
                when(|world| world.resource::<Jobs>().is_completed(&the_job(world).id)),
                when(|world| notice(world).contains("PAID")),
            ))
            .diagnose(describe_the_travel)
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: assert the pay")
            .on_enter(assert_the_pay)
            .add()
            .step("encounters: shoot the paid HUD")
            .on_enter(|world: &mut World| shoot(world, PAID_SHOT))
            .until(shot_written(PAID_SHOT))
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: undock from the caller")
            .on_enter(|world: &mut World| {
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: stand in open space again")
            .on_enter(|world: &mut World| {
                let (at, heading) =
                    open_space(world).expect("encounters: an axis out of the caller is clear");
                world.resource_mut::<EncounterWatch>().heading = heading;
                park_player(world, at, heading, MetersPerSecond(0.0));
            })
            .until(when(window_streamed))
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: arm cheats at the shell")
            .on_enter(|world: &mut World| enter_command(world, "cheats enable"))
            .until(when(|world| world.resource::<RunCheats>().is_armed()))
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: ask for the distress hail at the shell")
            .on_enter(|world: &mut World| enter_command(world, &format!("encounter {DISTRESS}")))
            .until(the_hail_chip_shows())
            .diagnose(describe_the_travel)
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: assert the command")
            .on_enter(assert_the_command)
            .add()
            .step("encounters: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("encounters: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "encounters: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .click_named(
                "encounters: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                BEAT_SECS,
            )
            .click_named(
                "encounters: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                BEAT_SECS,
            )
            .click_named(
                "encounters: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("encounters: the loaded window streams the caller back")
            .until(and(when(window_streamed), the_hail_chip_shows()))
            .diagnose(describe_the_travel)
            .deadline(SESSION_SECS)
            .add()
            .step("encounters: assert the loaded world")
            .on_enter(assert_the_load)
            .add();
        script
    }
}
