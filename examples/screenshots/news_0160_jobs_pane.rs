//! news_0160_jobs_pane: the v0.16.0 news still of the Jobs pane docked at a
//! second station with a procurement held from the first
//! (`news-0160-jobs-pane.png`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `transport`, with no save folder, and the start docks
//! the freighter at its starting station. The walk opens the interface with
//! the real key, selects the Jobs tab and clicks Accept on the starter
//! board's first procurement. Then the freighter lets go, jumps to the
//! origin, and docks at the nearest other living station that is not on the
//! Enemy side and whose board posts three offers, an Escort among them.
//!
//! On that board the unarmed freighter cannot take the Escort, so its
//! Accept is greyed; the starting station's board leaves such a job out.
//! Your jobs holds the procurement with its `have / need` meter under the
//! role's milestone row. The walk clicks the held row, so the inspector
//! reads it in full with Show on Map, asserts what the frame must read and
//! shoots it.
//!
//! The figure holds no finished escort: a board lists only the offers the
//! player neither holds nor finished, so a finished escort at its own
//! station and a full board cannot share one frame.
//!
//! Staging: the jump to the origin and the berth are the shared
//! `open_world_stage` moves; the walk does not fly the approach. The seed is
//! pinned for its boards; a seed that gives either up stops the run, naming
//! it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the PNG, staged under
//!   `NOVA_CAPTURE_DIR`.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_jobs_pane --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::{collections::BTreeSet, sync::Arc};

use bevy::{prelude::*, ui::InteractionDisabled};
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    nova_interface::{
        jobs::prelude::{JOBS_BOARD, JOBS_MILESTONE, JOB_ACCEPT_PREFIX, JOB_SHOW_ON_MAP},
        pane::InterfacePaneType,
    },
    prelude::*,
};
use open_world_stage::{
    advance_station_dock, the_player, window_is, window_settled, StationDock, StationDockPhaseType,
};

#[derive(Parser)]
#[command(name = "news_0160_jobs_pane")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 Jobs pane: a held procurement and a second station's board",
    long_about = None
)]
struct Cli {
    /// The world seed. Another seed must still post a procurement on the
    /// starter board and an Escort on a second station's board.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the figure plays.
const WORLD_SEED: u32 = 10;

/// The start role the walk flies: its boards lean to procurements and
/// escorts, and its hull carries no weapon.
const ROLE: &str = "transport";

/// The offers a board posts.
const BOARD_OFFERS: usize = 3;

/// The still this producer writes.
const SHOT_FILE: &str = "news-0160-jobs-pane.png";

/// Real seconds the world gets to start, stream and dock on a software
/// rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 60.0;

/// Frames the pane gets to draw before the shot.
const PANE_FRAMES: u32 = 10;

/// Real seconds a frame capture gets on a software rasterizer.
const SHOT_SECS: f32 = 120.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(open_world_stage::OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script(cli.seed));
    app.run()
}

/// The procurement the walk accepts off the starter board.
#[derive(Resource, Clone, Debug)]
struct HeldProcure {
    job: Job,
    title: String,
}

/// The second station's dock, driven once the procurement is held.
#[derive(Resource, Default)]
struct SecondDock {
    dock: StationDock,
    docked: Option<Entity>,
}

/// The window is in, the start docked the freighter and its board posted.
#[derive(Resource)]
struct StartReady;

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The one player ship, if exactly one stands.
fn player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// The entity wearing generated id `id`, if one does.
fn body(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
    world
        .try_query::<(Entity, &GeneratedBodyId)>()?
        .iter(world)
        .find(|(_, worn)| *worn == id)
        .map(|(entity, _)| entity)
}

/// The starting station, once it stands.
fn start_station(world: &World) -> Option<Entity> {
    let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
    body(world, &id)
}

/// The UI entity named `name`, if one stands.
fn named(world: &World, name: &str) -> Option<Entity> {
    world
        .try_query::<(Entity, &Name)>()?
        .iter(world)
        .find(|(_, worn)| worn.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Every visible text with a laid-out width and height under `root`,
/// itself included. A text that exists at zero size draws nothing.
fn texts_under(world: &World, root: Entity) -> Vec<String> {
    let mut texts = Vec::new();
    let mut stack = vec![root];
    while let Some(at) = stack.pop() {
        let drawn = world
            .get::<InheritedVisibility>(at)
            .is_some_and(|visible| visible.get())
            && world
                .get::<ComputedNode>(at)
                .is_some_and(|node| node.size().min_element() > 0.0);
        if let (Some(text), true) = (world.get::<Text>(at), drawn) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(at) {
            stack.extend(children.iter());
        }
    }
    texts
}

/// The index of the row named `<prefix><index>` that draws `title`.
fn row_drawing(world: &World, prefix: &str, title: &str) -> Option<usize> {
    (0..BOARD_OFFERS * 2).find(|index| {
        named(world, &format!("{prefix}{index}"))
            .is_some_and(|row| texts_under(world, row).iter().any(|text| text == title))
    })
}

/// The Accept button of the starter board row that draws the held
/// procurement's title.
fn procure_accept(world: &World) -> Option<String> {
    let title = &world.get_resource::<HeldProcure>()?.title;
    row_drawing(world, "JobRowPosted-", title).map(|index| format!("{JOB_ACCEPT_PREFIX}{index}"))
}

/// Pick the starter board's first procurement.
///
/// # Panics
///
/// When the starter board posts none: the seed no longer gives the figure
/// its held job.
fn pick_the_procure(world: &mut World, seed: u32) {
    let station = start_station(world).expect("jobs pane: the starting station stands");
    let board = world
        .get::<JobBoard>(station)
        .expect("jobs pane: the starting station posted its board");
    let Some(job) = board
        .0
        .iter()
        .find(|job| matches!(job.kind, JobType::Procure { .. }))
        .cloned()
    else {
        let kinds: Vec<&str> = board.0.iter().map(|job| job.kind.label()).collect();
        panic!("jobs pane: seed {seed}'s starter board posts no procurement: {kinds:?}");
    };
    let title = job.title(world.resource::<GameItems>());
    info!("jobs pane: accepting '{title}' ({})", job.id);
    world.insert_resource(HeldProcure { job, title });
}

/// Click the UI node `find` names when it runs, through the real pointer
/// path: aim and hold the aim until the pointer is over it, press, release.
/// `click_named` with the name read off the world.
fn click_found(
    script: AutopilotPlugin<GameStates>,
    label: &str,
    find: fn(&World) -> Option<String>,
    landed: Arc<Predicate>,
) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("{label}: the widget is up"))
        .until(when(move |world| {
            find(world).is_some_and(|name| ui_node_present(name)(world))
        }))
        .deadline(BEAT_SECS)
        .add()
        .step(format!("{label}: aim"))
        .each(move |world: &mut World, _, _| {
            if let Some(name) = find(world) {
                keep_hovering_named(name)(world);
            }
        })
        .until(when(move |world| {
            find(world).is_some_and(|name| pointer_over_node(name)(world))
        }))
        .deadline(BEAT_SECS)
        .add()
        .step(format!("{label}: press"))
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_SECS)
        .add()
        .step(format!("{label}: release"))
        .on_enter(release_mouse(MouseButton::Left))
        .until(landed)
        .deadline(BEAT_SECS)
        .add()
}

/// Every station in the window around the player with its board posted,
/// once all of them have one: the ids whose board posts [`BOARD_OFFERS`]
/// offers, an Escort among them, none held or finished.
fn second_boards(world: &mut World) -> Option<BTreeSet<GeneratedBodyId>> {
    let jobs = world.resource::<Jobs>().clone();
    let mut q_stations = world.query_filtered::<
        (&GeneratedBodyId, Option<&JobBoard>, Has<DerelictShipMarker>),
        With<StationMarker>,
    >();
    let mut eligible = BTreeSet::new();
    for (id, board, derelict) in q_stations.iter(world) {
        if derelict {
            continue;
        }
        let board = board?;
        let open: Vec<&Job> = board
            .0
            .iter()
            .filter(|job| !jobs.holds(&job.id) && !jobs.is_completed(&job.id))
            .collect();
        if open.len() == BOARD_OFFERS
            && open
                .iter()
                .any(|job| matches!(job.kind, JobType::Escort { .. }))
        {
            eligible.insert(id.clone());
        }
    }
    Some(eligible)
}

/// Drive the dock at the second station one phase per frame. The choice
/// waits for every station round the origin to post its board.
fn drive_the_second_dock(world: &mut World) {
    let mut second = world
        .remove_resource::<SecondDock>()
        .expect("jobs pane: the second dock is staged");
    let choosing = second.dock.phase == StationDockPhaseType::Returning;
    let eligible = if choosing && window_is(world, SectorCoord::ORIGIN) {
        second_boards(world)
    } else {
        Some(BTreeSet::new())
    };
    if let Some(eligible) = eligible {
        if let Some((_, station)) = advance_station_dock(
            world,
            &mut second.dock,
            "living, non-Enemy station whose board posts three offers with an Escort",
            |candidate| {
                eligible.contains(candidate.id)
                    && candidate.offers_service(StationServiceType::Jobs)
                    && candidate.not_enemy()
            },
        ) {
            second.docked = Some(station);
        }
    }
    world.insert_resource(second);
}

/// Docked at the second station.
fn docked_at_the_second() -> Arc<Predicate> {
    when(|world| {
        world
            .get_resource::<SecondDock>()
            .and_then(|second| second.docked)
            .is_some_and(|station| {
                player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
                    && world.get::<JobBoard>(station).is_some()
            })
    })
}

/// The interface is open on `pane`.
fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world.get_resource::<InterfacePaneType>() == Some(&pane)
    })
}

/// The interface is closed.
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
        .step(format!("jobs pane: press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("jobs pane: let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .deadline(BEAT_SECS)
        .add()
}

/// Open the interface on the Jobs tab.
fn open_the_jobs_tab(script: AutopilotPlugin<GameStates>, at: &str) -> AutopilotPlugin<GameStates> {
    tap(
        script,
        &format!("the interface key at the {at}"),
        "interface_toggle",
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }),
    )
    .click_named(
        &format!("jobs pane: open the Jobs tab at the {at}"),
        "InterfaceTabJobs",
        and(
            the_interface_shows(InterfacePaneType::Jobs),
            ui_node_present(JOBS_BOARD),
        ),
        BEAT_SECS,
    )
}

/// What the frame must show, read off the world and the drawn pane: the
/// board's three offers with the Escort's Accept greyed, the milestone row,
/// the held procurement with its meter, and the inspector on it.
fn the_pane_reads(world: &World) -> Result<(), String> {
    let held = world
        .get_resource::<HeldProcure>()
        .ok_or("no held procurement")?;
    let station = world
        .get_resource::<SecondDock>()
        .and_then(|second| second.docked)
        .ok_or("not docked at the second station")?;
    let board = world
        .get::<JobBoard>(station)
        .ok_or("the second station has no board")?;
    let items = world.resource::<GameItems>();
    for index in 0..BOARD_OFFERS {
        if named(world, &format!("JobRowPosted-{index}")).is_none() {
            return Err(format!("no posted row {index}"));
        }
    }
    if named(world, &format!("JobRowPosted-{BOARD_OFFERS}")).is_some() {
        return Err("more posted rows than the board's offers".to_string());
    }
    let escorts: Vec<&Job> = board
        .0
        .iter()
        .filter(|job| matches!(job.kind, JobType::Escort { .. }))
        .collect();
    let greyed = escorts.iter().any(|escort| {
        row_drawing(world, "JobRowPosted-", &escort.title(items)).is_some_and(|index| {
            named(world, &format!("{JOB_ACCEPT_PREFIX}{index}"))
                .is_some_and(|accept| world.get::<InteractionDisabled>(accept).is_some())
        })
    });
    if !greyed {
        return Err("no Escort row with a greyed Accept".to_string());
    }
    if named(world, JOBS_MILESTONE).is_none() {
        return Err("no milestone row".to_string());
    }
    let hold = player(world).and_then(|player| world.get::<ShipInventory>(player));
    let (have, need) = held.job.progress(hold, false);
    let row = named(world, "JobRowHeld-0").ok_or("no held row")?;
    let row_texts = texts_under(world, row);
    if !row_texts.contains(&held.title) || !row_texts.contains(&format!("{have} / {need}")) {
        return Err(format!(
            "the held row reads {row_texts:?}, not '{}' at {have} / {need}",
            held.title
        ));
    }
    if named(world, "JobRowHeld-1").is_some() {
        return Err("more than one held row".to_string());
    }
    if named(world, JOB_SHOW_ON_MAP).is_none() {
        return Err("the inspector shows no Show on Map".to_string());
    }
    Ok(())
}

/// Whether `section` is mounted under `root`, by its mount chain.
fn mounted_on(world: &World, section: Entity, root: Entity) -> bool {
    let mut at = section;
    while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
        if parent == root {
            return true;
        }
        at = parent;
    }
    false
}

/// Assert the frame's subject: the pane reads, and the freighter carries no
/// weapon, which is why the Escort's Accept is greyed.
fn assert_the_pane(world: &mut World) {
    if let Err(why) = the_pane_reads(world) {
        panic!("jobs pane: {why}");
    }
    let player = player(world).expect("jobs pane: one player ship");
    let weapons: Vec<Entity> = world
        .query_filtered::<Entity, Or<(
            With<TurretSectionMarker>,
            With<TorpedoSectionMarker>,
            With<RailgunSectionMarker>,
        )>>()
        .iter(world)
        .collect();
    let armed = weapons
        .into_iter()
        .filter(|section| mounted_on(world, *section, player))
        .count();
    assert_eq!(armed, 0, "jobs pane: the transport carries a weapon");
    let held = world.resource::<HeldProcure>().clone();
    let hold = world.get::<ShipInventory>(player);
    info!(
        "jobs pane: holding '{}' at {:?} on an unarmed hull",
        held.title,
        held.job.progress(hold, false)
    );
}

/// The walk: accept at the start, dock at the second station, select the
/// held row and shoot the pane.
fn script(seed: u32) -> AutopilotPlugin<GameStates> {
    let script = AutopilotPlugin::<GameStates>::new()
        .step("jobs pane: the window streams in, the start docks and the board posts")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| {
            let docked =
                the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
            let posted = start_station(world).is_some_and(|station| {
                world.get::<JobBoard>(station).is_some()
                    && world.get::<StarterBoardMarker>(station).is_some()
            });
            if docked && posted && window_settled(world) {
                world.insert_resource(StartReady);
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("jobs pane: pick the starter procurement")
        .on_enter(move |world: &mut World| pick_the_procure(world, seed))
        .add()
        .step("jobs pane: drop the status bar")
        .on_enter(hide_status_bar)
        .until(frames(1))
        .add();
    let script = open_the_jobs_tab(script, "starting station");
    let script = click_found(
        script,
        "jobs pane: accept the procurement",
        procure_accept,
        when(|world| {
            world
                .get_resource::<HeldProcure>()
                .is_some_and(|held| world.resource::<Jobs>().holds(&held.job.id))
        }),
    );
    let script = tap(script, "the interface key", "interface_toggle", unpaused())
        .step("jobs pane: dock at the second station")
        .on_enter(|world: &mut World| {
            world.init_resource::<SecondDock>();
        })
        .each(|world: &mut World, _, _| drive_the_second_dock(world))
        .until(docked_at_the_second())
        .diagnose(|world| {
            world
                .get_resource::<SecondDock>()
                .map(|second| format!("{:?}", second.dock))
                .unwrap_or_default()
        })
        .deadline(SETUP_SECS)
        .add();
    open_the_jobs_tab(script, "second station")
        .click_named(
            "jobs pane: select the held procurement",
            "JobRowHeld-0",
            ui_node_present(JOB_SHOW_ON_MAP),
            BEAT_SECS,
        )
        .step("jobs pane: the pane reads the board, the held job and the inspector")
        .until(when(|world| the_pane_reads(world).is_ok()))
        .diagnose(|world| format!("{:?}", the_pane_reads(world)))
        .deadline(BEAT_SECS)
        .add()
        .step("jobs pane: settle the pane")
        .until(frames(PANE_FRAMES))
        .add()
        .step("jobs pane: check the pane")
        .on_enter(assert_the_pane)
        .add()
        .step("jobs pane: shoot the pane")
        .on_enter(|world: &mut World| shoot(world, SHOT_FILE))
        .until(shot_written(SHOT_FILE))
        .deadline(SHOT_SECS)
        .add()
}
