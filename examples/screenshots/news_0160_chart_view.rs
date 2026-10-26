//! news_0160_chart_view: the v0.16.0 news chart loop - the chart view on the
//! starting station's layer, the layer stepping down to the second station's
//! layer, that station's mark selected and cleared
//! (`news-0160-chart-view.webm`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `transport`, with no save folder, and the start docks
//! the freighter at its starting station on Y [`START_LAYER`], which the
//! sensors find first: STN-1. Then the freighter lets go, jumps to the origin
//! and docks at [`SECOND_STATION`] on Y [`SECOND_LAYER`], the station
//! `news_0160_jobs_pane` docks at on this seed, in a derelict field: STN-2.
//! The berth holds [`SENSOR_FRAMES`] frames, then the walk reads what the
//! chart found. A derelict hull the sensors found in that cell is a WRK-n
//! mark the loop waits on; with none found the loop shows none. A seed that
//! moves either station stops the run, naming what the chart holds; nothing
//! marks a find the sensors did not make.
//!
//! Staging: the jump to the origin and the berth are the shared
//! `open_world_stage` moves; the walk does not fly the approach. The cells
//! entered on the way are the chart's own record of that route, so the
//! layers between the two stations hold only unvisited outlines.
//!
//! The walk opens the interface with the real key, the Map tab with a click,
//! the chart with the real `map_chart` key (C) and steps up to STN-1's layer
//! with the real `map_layer_up` key (X). Then the loop records, one beat
//! after another: the chart on STN-1's layer; the real `map_layer_down` key
//! (Z) once per layer, the OBJECTIVES heading stepping down to the player's
//! layer; that layer with STN-2 counting its posted jobs and every found
//! wreck's mark; a click on STN-2's mark, the SECTOR block of its cell; and a
//! second click on the same mark, the OBJECTIVES block and the discovery log
//! back. Each beat waits on the drawn heading, block or label, and the log
//! must list STN-2 and every found wreck before the loop closes.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-chart-view.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_chart_view --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, LoopCapturePlugin, LoopProfile, Predicate},
    nova_interface::pane::InterfacePaneType,
    prelude::*,
};
use open_world_stage::{
    advance_station_dock, the_player, window_settled, OpenWorldStartPlugin, StationDock,
};

#[derive(Parser)]
#[command(name = "news_0160_chart_view")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 chart loop: STN-1's layer, Z down to STN-2's layer, its mark selected and cleared",
    long_about = None
)]
struct Cli;

/// The seed the loop plays.
const WORLD_SEED: u32 = 10;

/// The start role the walk flies.
const ROLE: &str = "transport";

/// The station the walk docks at second: in a derelict field, in the
/// origin's window on [`WORLD_SEED`].
const SECOND_STATION: &str = "sector_1_0_0_cluster_1_0_0_station_0";

/// The starting station's chart code: the sensors find it first.
const START_CODE: &str = "STN-1";

/// The second station's chart code: the starting station is found first.
const SECOND_CODE: &str = "STN-2";

/// The Y layer of the starting station's cell on [`WORLD_SEED`].
const START_LAYER: i32 = 6;

/// The Y layer of [`SECOND_STATION`]'s cell.
const SECOND_LAYER: i32 = 0;

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0160-chart-view";

/// Real seconds the world gets to start, stream and dock on a software
/// rasterizer, and again for the second dock.
const SETUP_SECS: f32 = 600.0;

/// Frames the berth at the second station holds before the walk reads the
/// chart. The sensors pass once per frame and nothing at the berth moves, so
/// a hull they have not found by then is out of their reach.
const SENSOR_FRAMES: u32 = 60;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 60.0;

/// Frames the Map pane gets to build its scene before the loop.
const PANE_FRAMES: u32 = 10;

/// Frames each end layer holds on screen, so each reads apart: STN-1's
/// layer and STN-2's. One capture frame is 1/30 s.
const HOLD_FRAMES: u32 = 30;

/// Frames each layer between the two stations holds on screen, so each
/// layer's heading reads apart.
const LAYER_FRAMES: u32 = 36;

/// Frames the SECTOR block and the returned overview hold: each has a panel
/// to read.
const READ_FRAMES: u32 = 45;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

const MAP_TAB: &str = "InterfaceTabMap";
const CHART_OBJECTIVES: &str = "MapChartObjectives";
const CHART_LOG: &str = "MapChartLog";
const CHART_SECTOR: &str = "MapChartSector";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(OpenWorldStartPlugin {
        seed: WORLD_SEED,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    // The OBJECTIVES heading's layer digit is 8 px of dim ink at 720p. At the
    // fleet's CRF 34 VP9 keeps the previous digit through Y5, Y4, Y2 and Y1;
    // CRF 10 encodes every layer's digit.
    app.add_plugins(LoopCapturePlugin::new(LoopProfile {
        crf: 10,
        ..LoopProfile::default()
    }));
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script());
    app.run()
}

/// The window is in and the start docked the freighter at its starting
/// station.
#[derive(Resource)]
struct StartReady;

/// The second station's dock, driven once the start is in.
#[derive(Resource, Default)]
struct SecondDock {
    dock: StationDock,
    docked: Option<Entity>,
}

/// What the chart found, read once before the Map opens: the cell and the
/// wreck codes the loop's beats wait on. No wreck code is a valid find.
#[derive(Resource, Clone, Debug)]
struct Finds {
    /// The second station.
    station: Entity,
    /// The cell the player stands in, the second station's.
    cell: SectorCoord,
    /// The codes of the wrecks found in that cell.
    wreck_codes: Vec<String>,
}

/// The name of the chart mark labelled `code`.
fn mark_name(code: &str) -> String {
    format!("MapChartMark{code}")
}

/// The OBJECTIVES heading on Y layer `layer`.
fn heading(layer: i32) -> String {
    format!("OBJECTIVES  Y{layer}")
}

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

fn finds(world: &World) -> &Finds {
    world
        .get_resource::<Finds>()
        .expect("chart view: the finds are read before the Map opens")
}

/// The second station's id.
fn second_station(world: &World) -> GeneratedBodyId {
    let second = world
        .get_resource::<SecondDock>()
        .and_then(|second| second.dock.station.clone())
        .expect("chart view: the second station is chosen");
    second.id
}

/// Every text under the node `name` that draws: visible, a node and a glyph
/// layout of positive size, and a themed colour. `themed_label` spawns its
/// text transparent until the theme tints it.
fn texts_under(world: &World, name: &str) -> Vec<String> {
    let Some(mut named) = world.try_query::<(Entity, &Name)>() else {
        return Vec::new();
    };
    let Some(root) = named
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    let mut open = vec![root];
    while let Some(at) = open.pop() {
        let shown = world
            .get::<InheritedVisibility>(at)
            .is_some_and(|visible| visible.get())
            && world
                .get::<ComputedNode>(at)
                .is_some_and(|node| node.size().min_element() > 0.0)
            && world
                .get::<bevy::text::TextLayoutInfo>(at)
                .is_some_and(|layout| !layout.glyphs.is_empty() && layout.size.min_element() > 0.0)
            && world
                .get::<TextColor>(at)
                .is_some_and(|color| color.0.alpha() > 0.0);
        if let (Some(text), true) = (world.get::<Text>(at), shown) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(at) {
            open.extend(children.iter());
        }
    }
    texts
}

/// Whether the node `name` draws `text` among its texts.
fn draws(world: &World, name: &str, text: &str) -> bool {
    texts_under(world, name).iter().any(|drawn| drawn == text)
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

/// Drive the dock at [`SECOND_STATION`] one phase per frame.
fn drive_the_second_dock(world: &mut World) {
    let mut second = world
        .remove_resource::<SecondDock>()
        .expect("chart view: the second dock is staged");
    if let Some((_, station)) = advance_station_dock(
        world,
        &mut second.dock,
        &format!("living, non-Enemy station '{SECOND_STATION}' that posts jobs"),
        |candidate| {
            candidate.id.to_string() == SECOND_STATION
                && candidate.offers_service(StationServiceType::Jobs)
                && candidate.not_enemy()
        },
    ) {
        second.docked = Some(station);
    }
    world.insert_resource(second);
}

/// Docked at the second station with its board posted.
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

/// What the chart holds, for a stall or a refusal.
fn chart_finds(world: &World) -> String {
    let chart = world.resource::<Chart>();
    let stations: Vec<String> = chart
        .stations()
        .iter()
        .map(|station| format!("{} (derelict {})", station.id, station.derelict))
        .collect();
    let wrecks: Vec<String> = chart
        .wrecks()
        .iter()
        .map(|wreck| wreck.id.to_string())
        .collect();
    let cells: Vec<String> = chart
        .visited()
        .map(|(coord, _)| coord.to_string())
        .collect();
    format!("stations {stations:?}, wrecks {wrecks:?}, cells {cells:?}")
}

/// How many jobs `station`'s board posts that the player neither holds, has
/// completed nor dropped: the count its chart mark adds.
fn posting(world: &World, station: Entity) -> usize {
    let jobs = world.resource::<Jobs>();
    world
        .get::<JobBoard>(station)
        .expect("chart view: the second station posted its board")
        .0
        .iter()
        .filter(|job| {
            !jobs.holds(&job.id) && !jobs.is_completed(&job.id) && !jobs.is_dropped(&job.id)
        })
        .count()
}

/// The text the second station's mark draws: its code and its open jobs.
fn station_label(world: &World) -> String {
    let finds = finds(world);
    format!("{SECOND_CODE} ({})", posting(world, finds.station))
}

/// Assert the chart holds the starting station first on [`START_LAYER`], the
/// second station second on [`SECOND_LAYER`], both living, and an open job
/// on the second station's board, then read what the loop waits on.
fn read_the_finds(world: &mut World) {
    let start = world.resource::<OpenWorldSession>().start.id.clone();
    let second = second_station(world);
    let station = world
        .get_resource::<SecondDock>()
        .and_then(|dock| dock.docked)
        .expect("chart view: docked at the second station");
    let seen = chart_finds(world);
    let chart = world.resource::<Chart>();
    assert_eq!(
        chart.station_code(&start).as_deref(),
        Some(START_CODE),
        "chart view: the starting station is the first find; {seen}"
    );
    assert_eq!(
        chart.station_code(&second).as_deref(),
        Some(SECOND_CODE),
        "chart view: the second station is the second find; {seen}"
    );
    for id in [&start, &second] {
        let found = chart.station(id).expect("found above");
        assert!(
            !found.derelict,
            "chart view: '{id}' is charted dead; {seen}"
        );
    }
    assert_eq!(
        (start.origin.y, second.origin.y),
        (START_LAYER, SECOND_LAYER),
        "chart view: the stations stand on other layers; {seen}"
    );
    let cell = second.origin;
    let wreck_codes: Vec<String> = chart
        .wrecks()
        .iter()
        .enumerate()
        .filter(|(_, wreck)| wreck.id.origin == cell)
        .map(|(index, _)| format!("WRK-{}", index + 1))
        .collect();
    if wreck_codes.is_empty() {
        info!("chart view: the sensors found no wreck in {cell}; the loop shows none");
    }
    assert!(
        posting(world, station) > 0,
        "chart view: '{second}' posts no open job, so its mark counts none"
    );
    let finds = Finds {
        station,
        cell,
        wreck_codes,
    };
    info!("chart view: {finds:?}; {seen}");
    world.insert_resource(finds);
}

/// The chart view draws the overview on the player's layer, the second
/// station's mark with its count and every wreck mark in its cell.
fn the_chart_shows() -> Arc<Predicate> {
    when(|world| {
        let finds = finds(world);
        draws(world, CHART_OBJECTIVES, &heading(finds.cell.y))
            && draws(world, &mark_name(SECOND_CODE), &station_label(world))
            && finds
                .wreck_codes
                .iter()
                .all(|code| draws(world, &mark_name(code), code))
    })
}

/// The overview's heading reads Y layer `layer`.
fn the_heading_reads(layer: i32) -> Arc<Predicate> {
    when(move |world| draws(world, CHART_OBJECTIVES, &heading(layer)))
}

/// The overview's heading reads [`START_LAYER`] and STN-1's mark stands in
/// the frame.
fn the_start_shows() -> Arc<Predicate> {
    when(|world| {
        draws(world, CHART_OBJECTIVES, &heading(START_LAYER))
            && texts_under(world, &mark_name(START_CODE))
                .iter()
                .any(|text| text == START_CODE || text.starts_with(&format!("{START_CODE} (")))
    })
}

/// The SECTOR block shows the second station's cell and the overview hides.
fn the_sector_shows() -> Arc<Predicate> {
    when(|world| {
        let finds = finds(world);
        draws(world, CHART_SECTOR, &format!("SECTOR {}", finds.cell))
            && texts_under(world, CHART_OBJECTIVES).is_empty()
            && texts_under(world, CHART_LOG).is_empty()
    })
}

/// The SECTOR block hides and the overview and the discovery log are back,
/// the log listing the second station and every wreck in its cell.
fn the_overview_returns() -> Arc<Predicate> {
    when(|world| {
        let finds = finds(world);
        let log = texts_under(world, CHART_LOG);
        texts_under(world, CHART_SECTOR).is_empty()
            && draws(world, CHART_OBJECTIVES, &heading(finds.cell.y))
            && log.iter().any(|text| text == "DISCOVERY LOG")
            && log
                .iter()
                .any(|text| text.starts_with(&format!("{SECOND_CODE} ")))
            && finds
                .wreck_codes
                .iter()
                .all(|code| log.iter().any(|text| text == code))
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
        .step(format!("chart view: press {label}"))
        .on_enter(press_action(action))
        .until(frames(1))
        .add()
        .step(format!("chart view: let {label} up"))
        .on_enter(release_action(action))
        .until(landed)
        .diagnose(|world| {
            format!(
                "pause {:?}, pane {:?}, overview {:?}",
                world.get_resource::<State<PauseStates>>().map(State::get),
                world.get_resource::<InterfacePaneType>(),
                texts_under(world, CHART_OBJECTIVES)
            )
        })
        .deadline(BEAT_SECS)
        .add()
}

/// Hold the frame `count` frames.
fn hold(
    script: AutopilotPlugin<GameStates>,
    what: &str,
    count: u32,
) -> AutopilotPlugin<GameStates> {
    script
        .step(format!("chart view: hold {what}"))
        .until(frames(count))
        .add()
}

/// The walk: dock at the start, dock at the second station, read the finds,
/// open the chart on STN-1's layer, then the chart loop.
fn script() -> AutopilotPlugin<GameStates> {
    let script = AutopilotPlugin::<GameStates>::new()
        .step("chart view: the window streams in and the start docks")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| {
            let docked =
                the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
            if docked && window_settled(world) {
                world.insert_resource(StartReady);
            }
        })
        .until(when(|world| world.contains_resource::<StartReady>()))
        .deadline(SETUP_SECS)
        .add()
        .step("chart view: dock at the second station")
        .on_enter(|world: &mut World| {
            world.init_resource::<SecondDock>();
        })
        .each(|world: &mut World, _, _| drive_the_second_dock(world))
        .until(docked_at_the_second())
        .diagnose(|world| {
            world
                .get_resource::<SecondDock>()
                .map(|second| format!("{:?}; {}", second.dock, chart_finds(world)))
                .unwrap_or_default()
        })
        .deadline(SETUP_SECS)
        .add()
        .step("chart view: the sensors sweep the second station's cell")
        .until(frames(SENSOR_FRAMES))
        .add()
        .step("chart view: read the finds")
        .on_enter(|world: &mut World| {
            read_the_finds(world);
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .add();
    let script = tap(
        script,
        "the interface key",
        "interface_toggle",
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }),
    )
    .click_named(
        "chart view: open the Map tab",
        MAP_TAB,
        the_interface_shows(InterfacePaneType::Map),
        BEAT_SECS,
    )
    .step("chart view: settle the Map")
    .until(frames(PANE_FRAMES))
    .add();
    let mut script = tap(script, "C", "map_chart", the_chart_shows());
    for layer in SECOND_LAYER + 1..START_LAYER {
        script = tap(
            script,
            &format!("X to Y{layer}"),
            "map_layer_up",
            the_heading_reads(layer),
        );
    }
    let script = tap(
        script,
        &format!("X to Y{START_LAYER}"),
        "map_layer_up",
        the_start_shows(),
    )
    .step("chart view: open the loop")
    .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
    .add();
    let mut script = hold(script, "STN-1's layer", HOLD_FRAMES);
    for layer in (SECOND_LAYER + 1..START_LAYER).rev() {
        script = tap(
            script,
            &format!("Z to Y{layer}"),
            "map_layer_down",
            the_heading_reads(layer),
        );
        script = hold(script, &format!("layer Y{layer}"), LAYER_FRAMES);
    }
    let script = tap(
        script,
        &format!("Z to Y{SECOND_LAYER}"),
        "map_layer_down",
        the_chart_shows(),
    );
    let script = hold(script, "STN-2's layer", HOLD_FRAMES);
    let script = script.click_named(
        &format!("chart view: select {SECOND_CODE}"),
        &mark_name(SECOND_CODE),
        the_sector_shows(),
        BEAT_SECS,
    );
    let script = hold(script, "the SECTOR block", READ_FRAMES);
    let script = script.click_named(
        &format!("chart view: clear {SECOND_CODE}"),
        &mark_name(SECOND_CODE),
        the_overview_returns(),
        BEAT_SECS,
    );
    hold(script, "the overview and the discovery log", READ_FRAMES)
        .step("chart view: close the loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}
