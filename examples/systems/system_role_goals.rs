//! system_role_goals: a bounty hunter's charter starts standing, reaches its
//! kill-list counts from completed Neutralize jobs, shows in the Map, and
//! survives the leave save and a Load.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on seed 10,
//! picks `bounty_hunter`, and waits for the armed world and its first save.
//! The range checks the authored charter, then injects six completed
//! Neutralize jobs as DISCLOSED FIXTURE STATE: each valid job goes through
//! `Jobs::accept`, `mark_done` and `complete`, with posters in cells (1, 0, 0),
//! (2, 0, 0), (3, 0, 0), (9, 0, 0), (10, 0, 0) and (11, 0, 0). Those cells
//! span ring 0 and ring 1; this fixture replaces flying and fighting, not the
//! goal tracker. It checks the one-time play-clock stamp, reads the met
//! charter in the Map's chart view, leaves to the main menu, reads the save
//! through [`open_world`], then loads it again and checks the live progress.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the charter opens standing with its lines at zero` | the bounty hunter's `A kill list` has 0 / 6 hulls and 0 / 2 rings, with no stamp |
//! | 2 | `outcome: six neutralize hand-ins across two rings meet the kill list and stamp the record once` | six completed Neutralize jobs from two rings yield 6 / 6 and 2 / 2, and the record keeps its first stamp across two more frames |
//! | 3 | `outcome: the map's chart view shows the charter met` | its OBJECTIVES block shows one `Goal reached at` row in place of the milestones and no milestone row |
//! | 4 | `outcome: the leave save keeps the met charter` | `open_world` reads the same met stamp from the leave save |
//! | 5 | `outcome: a load restores the met charter` | after Load and a frame, the same stamp and met progress return |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_role_goals --features debug
//! # look for: `role goals: PASS six hand-ins meet the kill list` and
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
    about = "Start the bounty hunter, meet its kill-list charter, read it in the Map, and check it through a save and a Load. Autopilot-only correctness range",
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
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.init_resource::<walk::GoalWatch>();
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

    use nova_protocol::{
        nova_debug::harness::{press_action, release_action, shoot, AutopilotPlugin, Predicate},
        nova_interface::pane::InterfacePaneType,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{WorldClock, WorldConfig};

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
    const SESSION_SECS: f32 = 240.0;
    const BEAT_SECS: f32 = 60.0;
    /// The frame the capture path writes of the chart view's OBJECTIVES block.
    const CHARTER_SHOT: &str = "role-goals-charter.png";

    /// The first stamp observed after the fixture meets the charter.
    #[derive(Resource, Default)]
    pub(super) struct GoalWatch {
        met_at: Option<f64>,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player_stands(world: &World) -> bool {
        let Some(mut players) = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        let mut players = players.iter(world);
        players.next().is_some() && players.next().is_none()
    }

    fn world_armed(world: &World) -> bool {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>() && player_stands(world)
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

    fn progress_is_met(world: &World) -> bool {
        world
            .get_resource::<GoalProgress>()
            .is_some_and(GoalProgress::is_met)
            && world
                .get_resource::<GoalRecord>()
                .is_some_and(GoalRecord::is_met)
    }

    fn shown_texts(world: &mut World) -> Vec<String> {
        world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.clone())
            .collect()
    }

    fn a_text_reads(check: impl Fn(&str) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        when(move |world| {
            world
                .try_query::<&Text>()
                .is_some_and(|mut query| query.iter(world).any(|text| check(&text.0)))
        })
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
        [1, 2, 3, 9, 10, 11]
            .into_iter()
            .enumerate()
            .map(|(index, x)| {
                let cell = SectorCoord::new(x, 0, 0);
                let station = GeneratedBodyId::new(cell, "cluster_station", 0);
                Job {
                    id: JobId {
                        station: station.clone(),
                        slot: 0,
                        posting: 0,
                    },
                    kind: JobType::Neutralize {
                        hull: GeneratedBodyId::new(cell, "cluster_hull", index as u32),
                        hull_name: format!("Fixture target {index}"),
                    },
                    target: station,
                    reward_cr: 400,
                }
            })
            .collect()
    }

    /// Disclosed fixture state: complete six valid neutralize jobs through
    /// the player's production Jobs API, posted in ring 0 and ring 1.
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

    fn expected_lines() -> Vec<GoalLine> {
        vec![
            GoalLine::new("Bounties handed in", 6, 6),
            GoalLine::new("Rings hunted", 2, 2),
        ]
    }

    /// Check the tracker after it has observed the fixture's hand-ins.
    fn assert_met_and_capture_stamp(world: &mut World) {
        let progress = world.resource::<GoalProgress>();
        assert_eq!(progress.title, "A kill list", "role goals: charter title");
        assert_eq!(
            progress.lines,
            expected_lines(),
            "role goals: charter lines"
        );
        assert!(progress.is_met(), "role goals: all charter lines are met");
        let clock = world.resource::<WorldClock>().play_seconds;
        let met_at = world
            .resource::<GoalRecord>()
            .met_at
            .expect("role goals: a met charter has a stamp");
        assert!(
            met_at.is_finite() && met_at > 0.0 && met_at <= clock,
            "role goals: stamp {met_at} is within the positive play clock {clock}"
        );
        world.resource_mut::<GoalWatch>().met_at = Some(met_at);
    }

    /// The met stamp is unchanged after two more frames; emit the named claim.
    fn assert_stamp_stable(world: &mut World) {
        let (title, lines, met) = {
            let progress = world.resource::<GoalProgress>();
            (
                progress.title.clone(),
                progress.lines.clone(),
                progress.is_met(),
            )
        };
        assert_eq!(title, "A kill list", "role goals: charter title");
        assert_eq!(lines, expected_lines(), "role goals: charter lines");
        assert!(met, "role goals: all charter lines stay met");
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the first stamp was captured");
        let found = world.resource::<GoalRecord>().met_at;
        assert_eq!(
            found,
            Some(expected),
            "role goals: the met record is stamped only once"
        );
        let completed = world.resource::<Jobs>().completed().to_vec();
        assert_eq!(completed.len(), 6, "role goals: six jobs completed");
        assert!(
            completed
                .iter()
                .all(|job| matches!(&job.kind, JobType::Neutralize { .. })),
            "role goals: every fixture hand-in is Neutralize"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        let job_ids: Vec<String> = completed.iter().map(|job| job.id.to_string()).collect();
        nova_probe::probe_marker(
            world,
            "outcome: six neutralize hand-ins across two rings meet the kill list and stamp the record once",
            serde_json::json!({
                "jobs": job_ids,
                "progress": progress_lines,
                "met_at": expected,
            }),
        );
        info!("role goals: PASS six hand-ins meet the kill list at {expected} s");
    }

    /// The chart view's OBJECTIVES block holds the reached goal's stamp row
    /// in place of the milestones.
    fn assert_map_charter(world: &mut World) {
        let shown = shown_texts(world);
        let met_row = shown
            .iter()
            .find(|text| text.starts_with("Goal reached at "))
            .cloned()
            .unwrap_or_else(|| panic!("role goals: the chart view shows its met stamp: {shown:?}"));
        assert!(
            named_node(world, "MapChartObjectiveMet"),
            "role goals: the OBJECTIVES block holds the met row"
        );
        assert!(
            !named_node(world, "MapChartObjectiveMilestone"),
            "role goals: a met role lists no milestone row: {shown:?}"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the map's chart view shows the charter met",
            serde_json::json!({ "stamp": met_row }),
        );
        info!("role goals: the Map chart view shows the met charter: {met_row}");
    }

    fn named_node(world: &mut World, name: &str) -> bool {
        world
            .query::<&Name>()
            .iter(world)
            .any(|named| named.as_str() == name)
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
        )
        .unwrap_or_else(|refusal| panic!("role goals: cannot open the leave save: {refusal}"));
        assert_eq!(
            state.goal.met_at,
            Some(expected),
            "role goals: leave save keeps the met stamp"
        );
        assert!(
            state.goal.is_met(),
            "role goals: leave save records the charter met"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the leave save keeps the met charter",
            serde_json::json!({
                "slug": slug,
                "generation": header.generation,
                "format": header.format,
                "met_at": state.goal.met_at,
            }),
        );
        info!("role goals: leave save keeps the charter met at {expected} s");
        drop(lock);
    }

    fn assert_loaded_charter(world: &mut World) {
        let expected = world
            .resource::<GoalWatch>()
            .met_at
            .expect("role goals: the met stamp was captured");
        let (title, lines, met) = {
            let progress = world.resource::<GoalProgress>();
            (
                progress.title.clone(),
                progress.lines.clone(),
                progress.is_met(),
            )
        };
        assert_eq!(title, "A kill list", "role goals: loaded title");
        assert_eq!(lines, expected_lines(), "role goals: loaded lines");
        assert!(met, "role goals: loaded progress is met");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            Some(expected),
            "role goals: Load restores the original stamp"
        );
        let progress_lines: Vec<String> = lines
            .iter()
            .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
            .collect();
        nova_probe::probe_marker(
            world,
            "outcome: a load restores the met charter",
            serde_json::json!({ "title": title, "lines": progress_lines, "met_at": expected }),
        );
        info!("role goals: PASS Load restored the met charter at {expected} s");
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
            .step("role goals: wait for the goal tracker")
            .until(when(progress_is_met))
            .deadline(BEAT_SECS)
            .add()
            .step("role goals: check the first met stamp")
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
            a_text_reads(|text| text == "OBJECTIVES"),
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

    fn assert_standing_charter(world: &mut World) {
        let (title, lines, met) = {
            let progress = world.resource::<GoalProgress>();
            (
                progress.title.clone(),
                progress.lines.clone(),
                progress.is_met(),
            )
        };
        assert_eq!(title, "A kill list", "role goals: charter title");
        assert_eq!(
            lines,
            vec![
                GoalLine::new("Bounties handed in", 0, 6),
                GoalLine::new("Rings hunted", 0, 2),
            ],
            "role goals: the charter opens with zero progress"
        );
        assert!(!met, "role goals: the charter is standing");
        assert_eq!(
            world.resource::<GoalRecord>().met_at,
            None,
            "role goals: the standing charter has no stamp"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the charter opens standing with its lines at zero",
            serde_json::json!({ "title": title, "lines": ["Bounties handed in 0 / 6", "Rings hunted 0 / 2"], "met_at": None::<f64> }),
        );
        info!("role goals: the kill list opens standing at zero");
    }
}
