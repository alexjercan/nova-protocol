//! ui_gallery: a clickable MOCK of the interface proposals in the owner's
//! 2026-10-17 UI feedback (tasks 20261012-100022, -100025 and -100030), built
//! from the `nova_ui::widget` factories alone. Nothing here touches the game:
//! every job, price, save and role is hard-coded data in this file, and every
//! click moves that data so the owner can judge how the proposed screens would
//! look AND react.
//!
//! The top bar switches between five pages: `Jobs` (the station job board),
//! `Chart` (the sector chart, the CHARTER block and the discovery log),
//! `Market` (the Inventory-style trade view with prices), `Load` (the saves
//! page) and `New Game` (the one-screen world setup). Each page is a list of
//! rows on the left and an inspector on the right, the cross-cutting pattern
//! the feedback asks for.
//!
//! Interactive run:  `cargo run --example ui_gallery`  (`S` flips the theme).
//! Capture every page: `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=target/gallery cargo
//! run --example ui_gallery --features debug` -> ui_gallery-<page>.png.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example ui_gallery --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `gallery: <beat>` verdict lines,
//! #           `autopilot: cycle complete, no panic`
//! ```
//!
//! Like `widget_zoo`, the gallery carries `GameStates` and steps into
//! `Playing` in `Startup` purely so the shared harness can drive it.

use bevy::{
    ecs::relationship::RelatedSpawner,
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use clap::Parser;
use nova_protocol::prelude::GameStates;
use nova_ui::{
    prelude::*,
    theme::{base::base_ui_themes, UiColor, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
    widget::{ButtonSpec, ThemedBorder, ThemedFill, ThemedText, UiText},
    NovaUiPlugin,
};

#[derive(Parser)]
#[command(name = "ui_gallery")]
#[command(version = "1.0.0")]
#[command(
    about = "A clickable mock of the proposed job board, chart, market, load and new game screens",
    long_about = None
)]
struct Cli;

// The `Name`s the harness clicks by. Rows and buttons that exist per record
// take the record's id as a suffix (`Gallery Accept 4`), so a name resolves to
// exactly one entity on every page.
const PAGE_JOBS: &str = "Gallery Page Jobs";
const PAGE_CHART: &str = "Gallery Page Chart";
const PAGE_MARKET: &str = "Gallery Page Market";
const PAGE_LOAD: &str = "Gallery Page Load";
const PAGE_NEW_GAME: &str = "Gallery Page NewGame";
const THEME_HARDWARE: &str = "Gallery Theme Hardware";
const THEME_PHOSPHOR: &str = "Gallery Theme Phosphor";
const FILTER_PREFIX: &str = "Gallery Filter";
const ACCEPT_PREFIX: &str = "Gallery Accept";
const HAND_IN_PREFIX: &str = "Gallery Hand In";
const TRACK_PREFIX: &str = "Gallery Track";
const POSTED_ROW_PREFIX: &str = "Gallery Posted Row";
const HELD_ROW_PREFIX: &str = "Gallery Held Row";
const SHOW_ON_MAP: &str = "Gallery Show On Map";
const EMPTY_STATE: &str = "Gallery Empty State";
const TOAST: &str = "Gallery Toast";
const TILE_PREFIX: &str = "Gallery Tile";
const MARKET_REFUSED: &str = "Gallery Market Refused";
const TRADE_ROW_PREFIX: &str = "Gallery Trade Row";
const SAVE_ROW_PREFIX: &str = "Gallery Save Row";
const CONTINUE: &str = "Gallery Continue";
const RANDOMIZE: &str = "Gallery Randomize";
const DISCLOSURE: &str = "Gallery Disclosure";
const ROLE_PREFIX: &str = "Gallery Role";
const CREATE: &str = "Gallery Create";

fn main() -> AppExit {
    let _ = Cli::parse();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1600, 900).into(),
            title: "nova_ui gallery".into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(NovaUiPlugin);
    app.insert_resource(GameUiThemes(base_ui_themes()));
    app.add_observer(button_on_setting::<SelectedUiTheme>);
    app.add_observer(button_on_setting::<GalleryPage>);
    app.add_observer(button_on_setting::<JobFilter>);
    app.init_resource::<GalleryPage>();
    app.init_resource::<JobFilter>();
    app.init_resource::<JobsState>();
    app.init_resource::<ChartState>();
    app.init_resource::<MarketState>();
    app.init_resource::<LoadState>();
    app.init_resource::<NewGameState>();
    #[cfg(feature = "debug")]
    app.init_resource::<Capture>();
    app.init_state::<GameStates>();
    app.add_systems(Startup, (setup, reach_playing));
    app.add_systems(
        Update,
        (
            rebuild_body.run_if(body_dirty),
            toggle_theme_key,
            sync_marks::<SelectedUiTheme>.run_if(resource_changed::<SelectedUiTheme>),
            sync_marks::<GalleryPage>.run_if(resource_changed::<GalleryPage>),
            fade_toasts,
            pulse_newest,
        ),
    );
    #[cfg(feature = "debug")]
    app.add_systems(Update, drive_capture);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("{}", nova_protocol::nova_debug::harness::REACHED_PLAYING)
            });
        }
        app.add_plugins(gallery_script());
    }

    app.run()
}

/// One unconditional step into `Playing`, so the harness can drive the app.
fn reach_playing(mut next: ResMut<NextState<GameStates>>) {
    next.set(GameStates::Playing);
}

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

/// The five job kinds of the proposed board, each with a one-character glyph
/// so a board scans by shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum JobKind {
    Bring,
    Recover,
    Escort,
    Answer,
    Neutralize,
}

impl JobKind {
    const ALL: [JobKind; 5] = [
        JobKind::Bring,
        JobKind::Recover,
        JobKind::Escort,
        JobKind::Answer,
        JobKind::Neutralize,
    ];

    fn glyph(self) -> &'static str {
        match self {
            JobKind::Bring => ">",
            JobKind::Recover => "^",
            JobKind::Escort => "=",
            JobKind::Answer => "?",
            JobKind::Neutralize => "x",
        }
    }

    fn label(self) -> &'static str {
        match self {
            JobKind::Bring => "Bring",
            JobKind::Recover => "Recover",
            JobKind::Escort => "Escort",
            JobKind::Answer => "Answer",
            JobKind::Neutralize => "Neutralize",
        }
    }
}

/// One job of the mock board. `have`/`need` are the held progress, `done` is
/// the world's own mark, and `hand_in_here` says the docked station is the
/// target.
#[derive(Clone, Copy)]
struct JobCard {
    id: u32,
    kind: JobKind,
    title: &'static str,
    reward: u32,
    target: (i32, i32),
    target_name: &'static str,
    minutes_left: u32,
    window: u32,
    fits_role: bool,
    poster: &'static str,
    hand_in: &'static str,
    task: &'static str,
    have: u32,
    need: u32,
    done: bool,
    hand_in_here: bool,
}

const JOBS: [JobCard; 10] = [
    JobCard {
        id: 1,
        kind: JobKind::Bring,
        title: "Bring 12 t iron ore",
        reward: 1_800,
        target: (3, 3),
        target_name: "Dust Gate (here)",
        minutes_left: 14,
        window: 30,
        fits_role: true,
        poster: "Dust Gate yard office",
        hand_in: "Dust Gate",
        task: "Bring 12 t of iron ore to Dust Gate. The yard pays on the dock; ore from any belt counts.",
        have: 0,
        need: 12,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 2,
        kind: JobKind::Recover,
        title: "Recover a drive stack",
        reward: 2_400,
        target: (5, 2),
        target_name: "Reef Yard belt",
        minutes_left: 6,
        window: 30,
        fits_role: false,
        poster: "Reef Yard salvage desk",
        hand_in: "Dust Gate",
        task: "A drive stack drifts in the Reef Yard belt. Cut it free, take it aboard and hand it in here.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 3,
        kind: JobKind::Escort,
        title: "Escort hauler to Reef Yard",
        reward: 3_100,
        target: (1, 5),
        target_name: "Reef Yard",
        minutes_left: 22,
        window: 30,
        fits_role: true,
        poster: "Driftco freight",
        hand_in: "Reef Yard",
        task: "Fly beside the Driftco hauler to Reef Yard. Raiders wait at the halfway mark.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: false,
    },
    JobCard {
        id: 4,
        kind: JobKind::Answer,
        title: "Answer a distress call",
        reward: 900,
        target: (2, 1),
        target_name: "Ash orbit",
        minutes_left: 3,
        window: 20,
        fits_role: false,
        poster: "Unknown caller",
        hand_in: "Dust Gate",
        task: "A caller in Ash orbit asks for a tow. The board cannot say whether it is a trap.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 5,
        kind: JobKind::Neutralize,
        title: "Neutralize a raider hull",
        reward: 4_600,
        target: (6, 6),
        target_name: "Far belt",
        minutes_left: 18,
        window: 30,
        fits_role: false,
        poster: "Dust Gate watch",
        hand_in: "Dust Gate",
        task: "A raider corvette holds the far belt. Neutralize it and report back on the dock.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 6,
        kind: JobKind::Bring,
        title: "Bring 4 t water ice",
        reward: 700,
        target: (3, 3),
        target_name: "Dust Gate (here)",
        minutes_left: 0,
        window: 30,
        fits_role: true,
        poster: "Dust Gate galley",
        hand_in: "Dust Gate",
        task: "Bring 4 t of water ice to the galley. This offer has expired.",
        have: 0,
        need: 4,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 7,
        kind: JobKind::Neutralize,
        title: "Neutralize the trap caller",
        reward: 2_200,
        target: (0, 0),
        target_name: "Corner drift",
        minutes_left: 9,
        window: 30,
        fits_role: false,
        poster: "Driftco freight",
        hand_in: "Dust Gate",
        task: "A fake distress caller baits haulers in the corner drift. Put it down.",
        have: 0,
        need: 1,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 8,
        kind: JobKind::Bring,
        title: "Bring 8 t copper",
        reward: 1_200,
        target: (3, 3),
        target_name: "Dust Gate (here)",
        minutes_left: 11,
        window: 30,
        fits_role: true,
        poster: "Dust Gate yard office",
        hand_in: "Dust Gate",
        task: "Bring 8 t of copper to Dust Gate.",
        have: 8,
        need: 8,
        done: true,
        hand_in_here: true,
    },
    JobCard {
        id: 9,
        kind: JobKind::Recover,
        title: "Recover 3 crates of parts",
        reward: 2_000,
        target: (2, 4),
        target_name: "Wreck WRK-2",
        minutes_left: 16,
        window: 30,
        fits_role: false,
        poster: "Reef Yard salvage desk",
        hand_in: "Dust Gate",
        task: "Three crates of parts sit in wreck WRK-2. Recover every crate and hand them in here.",
        have: 1,
        need: 3,
        done: false,
        hand_in_here: true,
    },
    JobCard {
        id: 10,
        kind: JobKind::Escort,
        title: "Escort hauler to Dust Gate",
        reward: 2_800,
        target: (1, 5),
        target_name: "Reef Yard",
        minutes_left: 5,
        window: 30,
        fits_role: true,
        poster: "Driftco freight",
        hand_in: "Reef Yard",
        task: "The hauler reached Dust Gate. Driftco pays at Reef Yard, 2 sectors west and 2 south.",
        have: 1,
        need: 1,
        done: true,
        hand_in_here: false,
    },
];

fn job(id: u32) -> &'static JobCard {
    JOBS.iter()
        .find(|j| j.id == id)
        .unwrap_or_else(|| panic!("no mock job {id}"))
}

/// A chart mark: what a known cell holds.
#[derive(Clone, Copy)]
enum Mark {
    Asteroids(u8),
    Rocks(u8),
    Planet(&'static str),
    Derelicts(u8),
    Station(StationKind, &'static str),
    Wreck(&'static str),
    Job(u32),
}

/// The station mark variants the chart tells apart by shape, not colour alone.
#[derive(Clone, Copy)]
enum StationKind {
    Living,
    Dead,
    Owned,
    Refusing,
    Posting(u8),
}

impl Mark {
    /// The glyph the chart tile and the legend draw.
    fn glyph(self) -> String {
        match self {
            Mark::Asteroids(_) => "*".into(),
            Mark::Rocks(_) => "o".into(),
            Mark::Planet(_) => "O".into(),
            Mark::Derelicts(_) => "x".into(),
            Mark::Station(kind, _) => match kind {
                StationKind::Living => "#".into(),
                StationKind::Dead => "#.".into(),
                StationKind::Owned => "#+".into(),
                StationKind::Refusing => "#!".into(),
                StationKind::Posting(n) => format!("#{n}"),
            },
            Mark::Wreck(_) => "%".into(),
            Mark::Job(id) => format!("!{id}"),
        }
    }

    /// The tone a mark reads in: dead and refusing stations fall back, owned
    /// ones light up, everything else is primary ink.
    fn color(self) -> UiColor {
        match self {
            Mark::Station(StationKind::Dead, _) => UiColor::Label,
            Mark::Station(StationKind::Owned, _) => UiColor::Nominal,
            Mark::Station(StationKind::Refusing, _) => UiColor::Accent,
            Mark::Job(_) => UiColor::Accent,
            Mark::Wreck(_) | Mark::Derelicts(_) => UiColor::Secondary,
            _ => UiColor::Primary,
        }
    }

    /// The side-panel row: a name and a count.
    fn row(self) -> (String, String) {
        match self {
            Mark::Asteroids(n) => ("Asteroids".into(), format!("{n}")),
            Mark::Rocks(n) => ("Rocks".into(), format!("{n}")),
            Mark::Planet(name) => (format!("Planet {name}"), "1".into()),
            Mark::Derelicts(n) => ("Derelicts".into(), format!("{n}")),
            Mark::Station(kind, name) => {
                let state = match kind {
                    StationKind::Living => "living",
                    StationKind::Dead => "dead",
                    StationKind::Owned => "owned",
                    StationKind::Refusing => "refusing, 6 min",
                    StationKind::Posting(_) => "posting jobs",
                };
                (format!("Station {name}"), state.into())
            }
            Mark::Wreck(code) => (format!("Wreck {code}"), "1".into()),
            Mark::Job(id) => (
                format!("JOB-{id} {}", job(id).title),
                job(id).kind.label().into(),
            ),
        }
    }
}

/// A charted cell: a visited cell carries the play time it was entered and
/// its finds; a cell known only from a job mark carries neither.
struct ChartCell {
    at: (i32, i32),
    entered: Option<&'static str>,
    marks: &'static [Mark],
    finds: &'static [&'static str],
}

const CHART_SIDE: i32 = 7;
const PLAYER_CELL: (i32, i32) = (3, 3);

const CHART: [ChartCell; 12] = [
    ChartCell {
        at: (3, 3),
        entered: Some("00:00"),
        marks: &[
            Mark::Station(StationKind::Posting(5), "Dust Gate"),
            Mark::Asteroids(12),
            Mark::Job(1),
        ],
        finds: &["STN-1 Dust Gate"],
    },
    ChartCell {
        at: (4, 3),
        entered: Some("00:12"),
        marks: &[Mark::Asteroids(30), Mark::Rocks(4)],
        finds: &[],
    },
    ChartCell {
        at: (2, 3),
        entered: Some("00:21"),
        marks: &[Mark::Rocks(9), Mark::Derelicts(1)],
        finds: &["DRL-1"],
    },
    ChartCell {
        at: (3, 2),
        entered: Some("00:33"),
        marks: &[
            Mark::Station(StationKind::Owned, "Vane Dock"),
            Mark::Asteroids(8),
        ],
        finds: &["STN-2 Vane Dock"],
    },
    ChartCell {
        at: (4, 2),
        entered: Some("00:40"),
        marks: &[Mark::Derelicts(3), Mark::Rocks(2)],
        finds: &["DRL-3"],
    },
    ChartCell {
        at: (5, 2),
        entered: Some("00:52"),
        marks: &[
            Mark::Station(StationKind::Refusing, "Reef Yard"),
            Mark::Asteroids(16),
            Mark::Job(2),
        ],
        finds: &["STN-4 Reef Yard"],
    },
    ChartCell {
        at: (2, 4),
        entered: Some("01:05"),
        marks: &[Mark::Wreck("WRK-2"), Mark::Rocks(5), Mark::Job(9)],
        finds: &["WRK-2"],
    },
    ChartCell {
        at: (1, 2),
        entered: Some("01:30"),
        marks: &[
            Mark::Planet("Ash"),
            Mark::Station(StationKind::Dead, "Ash Relay"),
        ],
        finds: &["PLN-1 Ash", "STN-3 Ash Relay"],
    },
    ChartCell {
        at: (3, 4),
        entered: Some("01:48"),
        marks: &[Mark::Asteroids(22)],
        finds: &[],
    },
    ChartCell {
        at: (1, 5),
        entered: None,
        marks: &[
            Mark::Station(StationKind::Living, "Reef Yard"),
            Mark::Job(3),
        ],
        finds: &[],
    },
    ChartCell {
        at: (2, 1),
        entered: None,
        marks: &[Mark::Job(4)],
        finds: &[],
    },
    ChartCell {
        at: (6, 6),
        entered: None,
        marks: &[Mark::Job(5)],
        finds: &[],
    },
];

fn chart_cell(at: (i32, i32)) -> Option<&'static ChartCell> {
    CHART.iter().find(|c| c.at == at)
}

/// The discovery log, newest first: glyph, code, sector, relative play time,
/// and whether it was found this session.
const DISCOVERY_LOG: [(&str, &str, &str, &str, bool); 6] = [
    ("#", "STN-4 Reef Yard", "(5,2)", "4 min ago", true),
    ("%", "WRK-2", "(2,4)", "11 min ago", true),
    ("x", "DRL-3", "(4,2)", "26 min ago", true),
    ("O", "PLN-1 Ash", "(1,2)", "1 h ago", false),
    ("#", "STN-2 Vane Dock", "(3,2)", "1 h ago", false),
    ("#", "STN-1 Dust Gate", "(3,3)", "2 h ago", false),
];

/// The last-seen prices and stock the chart keeps for a docked station.
const LAST_SEEN: [(&str, u32, u32); 3] = [
    ("Iron ore", 38, 14),
    ("Copper", 126, 2),
    ("Water ice", 21, 40),
];

/// One ware of the trade view: what the ship holds, what the station holds,
/// the reference price, what the station pays and asks, and the reason for
/// the gap.
struct Ware {
    name: &'static str,
    ship_qty: u32,
    station_qty: u32,
    reference: u32,
    bid: u32,
    ask: u32,
    reason: &'static str,
}

const WARES: [Ware; 6] = [
    Ware {
        name: "Iron ore",
        ship_qty: 12,
        station_qty: 60,
        reference: 40,
        bid: 33,
        ask: 38,
        reason: "Cluster type: belt station, ore is plentiful (60 t against a 40 t target)",
    },
    Ware {
        name: "Copper",
        ship_qty: 8,
        station_qty: 2,
        reference: 90,
        bid: 126,
        ask: 140,
        reason: "Scarcity: 2 t in stock against a 20 t target",
    },
    Ware {
        name: "Water ice",
        ship_qty: 0,
        station_qty: 40,
        reference: 25,
        bid: 19,
        ask: 21,
        reason: "Demand: the galley is full, water sells under reference",
    },
    Ware {
        name: "Drive parts",
        ship_qty: 3,
        station_qty: 1,
        reference: 400,
        bid: 520,
        ask: 560,
        reason: "Civilization: Driftco yards pay over reference for drive parts",
    },
    Ware {
        name: "Hull plate",
        ship_qty: 0,
        station_qty: 25,
        reference: 60,
        bid: 58,
        ask: 61,
        reason: "At reference: stock is on target",
    },
    Ware {
        name: "Fuel cells",
        ship_qty: 4,
        station_qty: 6,
        reference: 120,
        bid: 108,
        ask: 150,
        reason: "Scarcity: 6 against a 10 target; demand: the watch buys fuel",
    },
];

/// Signed percent of `price` against `reference`.
fn delta_percent(price: u32, reference: u32) -> i32 {
    let delta = price as i64 - reference as i64;
    (delta * 100 / reference as i64) as i32
}

/// A start role of the mock New Game, with its charter, credits and kit.
struct RoleCard {
    id: &'static str,
    name: &'static str,
    code: &'static str,
    hull: &'static str,
    charter: &'static str,
    counts: &'static str,
    credits: u32,
    kit: &'static [&'static str],
    lore: &'static str,
}

const ROLES: [RoleCard; 6] = [
    RoleCard {
        id: "industrial",
        name: "Industrial",
        code: "IND",
        hull: "cargoa",
        charter: "Mining empire",
        counts: "2 stations, 2 escorts",
        credits: 6_000,
        kit: &["mining laser", "20 t hold"],
        lore: "You start with a laser, a hold and a yard that owes you nothing. Own two stations and keep two escorts on the books, and the belt works for you.",
    },
    RoleCard {
        id: "scavenger",
        name: "Scavenger",
        code: "SCV",
        hull: "cargoa",
        charter: "Own yard",
        counts: "7 part families",
        credits: 3_500,
        kit: &["cutter", "salvage drone"],
        lore: "Wrecks are the only honest market. Cut, haul and sort until your own yard stocks every part family there is.",
    },
    RoleCard {
        id: "transport",
        name: "Transport",
        code: "TRN",
        hull: "cargoa",
        charter: "Station network",
        counts: "8 jobs, 4 stations, 2 rings",
        credits: 5_000,
        kit: &["40 t hold"],
        lore: "Haul for whoever posts. Eight jobs handed in across four stations and two rings makes you the name every board knows.",
    },
    RoleCard {
        id: "bounty_hunter",
        name: "Bounty hunter",
        code: "BNT",
        hull: "cargoa",
        charter: "Kill list",
        counts: "6 hulls, 2 rings",
        credits: 4_000,
        kit: &["PDC x2", "railgun"],
        lore: "The watch pays per hull. Six raiders put down across two rings and the list is yours to keep.",
    },
    RoleCard {
        id: "military",
        name: "Military",
        code: "MIL",
        hull: "cargoa",
        charter: "Line Warship",
        counts: "64,300 cr",
        credits: 8_000,
        kit: &["PDC x2", "torpedoes"],
        lore: "A corvette is a start. Earn the 64,300 cr a line warship costs and the yard builds it to your spec.",
    },
    RoleCard {
        id: "pirate",
        name: "Pirate",
        code: "PRT",
        hull: "racer",
        charter: "Outlive a civilization",
        counts: "1 civilization",
        credits: 2_500,
        kit: &["PDC", "boarding kit"],
        lore: "Nobody posts for you. Take what moves, keep moving, and be there when a civilization is not.",
    },
];

/// One saved world on the mock Load page.
struct SaveCard {
    world: &'static str,
    role: usize,
    charter_label: &'static str,
    have: u32,
    need: u32,
    met: bool,
    play_time: &'static str,
    saved_rel: &'static str,
    saved_at: &'static str,
    seed: u64,
    sector: &'static str,
    credits: u32,
    version: &'static str,
}

const RUNNING_VERSION: &str = "v0.14.0";

const SAVES: [SaveCard; 4] = [
    SaveCard {
        world: "Dust Gate run",
        role: 2,
        charter_label: "jobs",
        have: 3,
        need: 8,
        met: false,
        play_time: "02:14:07",
        saved_rel: "2 h ago",
        saved_at: "2026-10-17 09:41",
        seed: 8_841_201_337,
        sector: "(3,3) Dust Gate",
        credits: 12_450,
        version: "v0.14.0",
    },
    SaveCard {
        world: "Second yard",
        role: 1,
        charter_label: "families",
        have: 4,
        need: 7,
        met: false,
        play_time: "00:48:22",
        saved_rel: "yesterday",
        saved_at: "2026-10-16 22:05",
        seed: 1_923_775,
        sector: "(2,4) Wreck field",
        credits: 2_980,
        version: "v0.14.0",
    },
    SaveCard {
        world: "Line",
        role: 4,
        charter_label: "cr",
        have: 64_300,
        need: 64_300,
        met: true,
        play_time: "05:30:51",
        saved_rel: "3 d ago",
        saved_at: "2026-10-14 18:12",
        seed: 55_017,
        sector: "(1,2) Ash orbit",
        credits: 71_020,
        version: "v0.13.2",
    },
    SaveCard {
        world: "Empire",
        role: 0,
        charter_label: "stations",
        have: 1,
        need: 2,
        met: false,
        play_time: "01:05:00",
        saved_rel: "1 w ago",
        saved_at: "2026-10-10 11:00",
        seed: 7_777,
        sector: "(3,2) Vane Dock",
        credits: 23_400,
        version: "v0.14.0",
    },
];

const INITIAL_SEED: u64 = 8_841_201_337;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Which gallery page the top bar shows.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
enum GalleryPage {
    #[default]
    Jobs,
    Chart,
    Market,
    Load,
    NewGame,
}

impl GalleryPage {
    const ALL: [(GalleryPage, &'static str, &'static str); 5] = [
        (GalleryPage::Jobs, "Jobs", PAGE_JOBS),
        (GalleryPage::Chart, "Chart", PAGE_CHART),
        (GalleryPage::Market, "Market", PAGE_MARKET),
        (GalleryPage::Load, "Load", PAGE_LOAD),
        (GalleryPage::NewGame, "New Game", PAGE_NEW_GAME),
    ];
}

/// The job board's filter chip.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
enum JobFilter {
    #[default]
    All,
    Kind(JobKind),
}

impl JobFilter {
    fn admits(self, kind: JobKind) -> bool {
        match self {
            JobFilter::All => true,
            JobFilter::Kind(k) => k == kind,
        }
    }

    fn label(self) -> &'static str {
        match self {
            JobFilter::All => "All",
            JobFilter::Kind(k) => k.label(),
        }
    }
}

/// The board: which mock jobs are posted, which are held, the selection, the
/// tracked job and the balance a hand-in moves.
#[derive(Resource)]
struct JobsState {
    posted: Vec<u32>,
    held: Vec<u32>,
    selected: Option<u32>,
    tracked: Option<u32>,
    balance: u32,
}

impl Default for JobsState {
    fn default() -> Self {
        Self {
            posted: vec![1, 2, 3, 4, 5, 6, 7],
            held: vec![8, 9, 10],
            selected: None,
            tracked: Some(9),
            balance: 12_450,
        }
    }
}

#[derive(Resource, Default)]
struct ChartState {
    selected: Option<(i32, i32)>,
}

/// Which trade table a selected row is in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TradeSide {
    Tender,
    Trader,
}

#[derive(Resource, Default)]
struct MarketState {
    selected: Option<(TradeSide, usize)>,
}

#[derive(Resource, Default)]
struct LoadState {
    selected: usize,
}

#[derive(Resource)]
struct NewGameState {
    seed: u64,
    role: usize,
    disclosure_open: bool,
}

impl Default for NewGameState {
    fn default() -> Self {
        Self {
            seed: INITIAL_SEED,
            role: 2,
            disclosure_open: false,
        }
    }
}

#[derive(Component)]
struct GalleryRoot;
#[derive(Component)]
struct GalleryBody;

/// A clickable record carrying the id its click acts on.
#[derive(Component, Clone, Copy)]
enum Click {
    Accept(u32),
    HandIn(u32),
    Track(u32),
    SelectJob(u32),
    ShowOnMap(u32),
    Tile(i32, i32),
    Trade(TradeSide, usize),
    Save(usize),
    Randomize,
    Disclosure,
    Role(usize),
}

/// A hand-in toast: fades over `life` seconds, then despawns.
#[derive(Component)]
struct Toast {
    ttl: f32,
    life: f32,
}

/// The newest discovery, pulsing once after the chart opens.
#[derive(Component)]
struct Pulse(f32);

const TOAST_SECS: f32 = 4.0;
const PULSE_SECS: f32 = 2.0;

/// The body rebuilds when the page or any page's state moves.
#[expect(
    clippy::too_many_arguments,
    reason = "one Res per gallery state; the condition is the OR of their change flags"
)]
fn body_dirty(
    page: Res<GalleryPage>,
    filter: Res<JobFilter>,
    jobs: Res<JobsState>,
    chart: Res<ChartState>,
    market: Res<MarketState>,
    load: Res<LoadState>,
    new_game: Res<NewGameState>,
) -> bool {
    page.is_changed()
        || filter.is_changed()
        || jobs.is_changed()
        || chart.is_changed()
        || market.is_changed()
        || load.is_changed()
        || new_game.is_changed()
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(UiFont(
        asset_server.load("fonts/SGr-IosevkaTerm-Medium.ttf"),
    ));
    commands
        .spawn((
            GalleryRoot,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(20), px(14)),
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
        ))
        .with_children(top_bar);
}

/// The persistent header: title, the page control and the Theme control.
fn top_bar(root: &mut ChildSpawnerCommands) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(16),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|bar| {
        bar.spawn(text("NOVA UI // GALLERY", 18.0, UiColor::Primary));
        bar.spawn(segmented_container()).with_children(|seg| {
            for (page, label, name) in GalleryPage::ALL {
                let mut b = seg.spawn((
                    segmented_option_fit(label),
                    ButtonValue(page),
                    Name::new(name),
                ));
                if page == GalleryPage::default() {
                    b.insert(Selected);
                }
            }
        });
        bar.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        bar.spawn(text("Theme", 12.0, UiColor::Label));
        bar.spawn(segmented_container()).with_children(|seg| {
            for (label, id, name) in [
                ("Phosphor", PHOSPHOR_THEME_ID, THEME_PHOSPHOR),
                ("Hardware", HARDWARE_THEME_ID, THEME_HARDWARE),
            ] {
                let mut b = seg.spawn((
                    segmented_option(label),
                    ButtonValue(SelectedUiTheme(id.to_string())),
                    Name::new(name),
                ));
                if id == PHOSPHOR_THEME_ID {
                    b.insert(Selected);
                }
            }
        });
    });
}

/// Despawn the body and respawn the current page from its state.
#[expect(
    clippy::too_many_arguments,
    reason = "one Res per gallery state; each page reads its own"
)]
fn rebuild_body(
    mut commands: Commands,
    page: Res<GalleryPage>,
    filter: Res<JobFilter>,
    jobs: Res<JobsState>,
    chart: Res<ChartState>,
    market: Res<MarketState>,
    load: Res<LoadState>,
    new_game: Res<NewGameState>,
    roots: Query<Entity, With<GalleryRoot>>,
    bodies: Query<Entity, With<GalleryBody>>,
) {
    for body in &bodies {
        commands.entity(body).despawn();
    }
    let Ok(root) = roots.single() else {
        return;
    };
    commands.entity(root).with_children(|root| {
        root.spawn((
            GalleryBody,
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
        ))
        .with_children(|body| match *page {
            GalleryPage::Jobs => jobs_page(body, *filter, &jobs),
            GalleryPage::Chart => chart_page(body, &chart, &jobs),
            GalleryPage::Market => market_page(body, &market),
            GalleryPage::Load => load_page(body, &load),
            GalleryPage::NewGame => new_game_page(body, &new_game),
        });
    });
}

/// Move the `Selected` mark of a `ButtonValue<T>` group onto the live value,
/// for the two groups that are written from outside their own buttons (the
/// `S` key, the capture pass, Show on Map).
fn sync_marks<T: Resource + PartialEq + Clone>(
    mut commands: Commands,
    value: Res<T>,
    buttons: Query<(Entity, &ButtonValue<T>, Has<Selected>)>,
) {
    for (entity, button, marked) in &buttons {
        let wanted = button.0 == *value;
        if wanted == marked {
            continue;
        }
        let mut entity = commands.entity(entity);
        if wanted {
            entity.try_insert(Selected);
        } else {
            entity.try_remove::<Selected>();
        }
    }
}

fn toggle_theme_key(keys: Res<ButtonInput<KeyCode>>, mut selected: ResMut<SelectedUiTheme>) {
    if keys.just_pressed(KeyCode::KeyS) {
        let next = if selected.0 == PHOSPHOR_THEME_ID {
            HARDWARE_THEME_ID
        } else {
            PHOSPHOR_THEME_ID
        };
        *selected = SelectedUiTheme(next.to_string());
    }
}

/// Fade every toast's fill and text with its remaining life, then drop it.
fn fade_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<(
        Entity,
        &mut Toast,
        &mut ThemedFill,
        &mut ThemedBorder,
        &Children,
    )>,
    mut texts: Query<&mut ThemedText>,
) {
    for (entity, mut toast, mut fill, mut border, children) in &mut toasts {
        toast.ttl -= time.delta_secs();
        if toast.ttl <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (toast.ttl / toast.life).clamp(0.0, 1.0);
        fill.alpha = alpha;
        border.alpha = alpha;
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.alpha = alpha;
            }
        }
    }
}

/// Pulse the newest discovery's code once, then settle it.
fn pulse_newest(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Pulse, &mut ThemedText)>,
) {
    for (entity, mut pulse, mut text) in &mut q {
        pulse.0 += time.delta_secs();
        if pulse.0 >= PULSE_SECS {
            text.alpha = 1.0;
            commands.entity(entity).remove::<Pulse>();
        } else {
            text.alpha = 0.3 + 0.7 * (pulse.0 * 6.0).sin().abs();
        }
    }
}

// ---------------------------------------------------------------------------
// Click handling
// ---------------------------------------------------------------------------

/// Wrap a visual node so it receives clicks and carries what the click does.
fn clickable(widget: impl Bundle, click: Click) -> impl Bundle {
    (
        widget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        click,
        observe(on_click),
    )
}

/// Give a `button(...)`, which already carries `Button` + `Hovered`, its click.
fn acts(widget: impl Bundle, click: Click) -> impl Bundle {
    (widget, click, observe(on_click))
}

/// A clickable, named table row: a `list_row` at the table's tighter vertical
/// padding, so a board of ten rows fits a 900 px window beside its inspector.
fn spawn_row<'a>(
    p: &'a mut ChildSpawnerCommands,
    click: Click,
    name: String,
    selected: bool,
) -> EntityCommands<'a> {
    let mut row = p.spawn((clickable(list_row(), click), Name::new(name)));
    row.insert(Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(12),
        padding: UiRect::axes(px(13), px(3)),
        border: UiRect::all(px(1)),
        ..default()
    });
    if selected {
        row.insert(Selected);
    }
    row
}

/// Apply a click to the gallery state. Every branch writes one resource and
/// `rebuild_body` re-renders the page.
#[expect(
    clippy::too_many_arguments,
    reason = "one ResMut per gallery state; a click may land on any page"
)]
fn on_click(
    activate: On<Activate>,
    mut commands: Commands,
    q: Query<&Click>,
    mut page: ResMut<GalleryPage>,
    mut jobs: ResMut<JobsState>,
    mut chart: ResMut<ChartState>,
    mut market: ResMut<MarketState>,
    mut load: ResMut<LoadState>,
    mut new_game: ResMut<NewGameState>,
) {
    let Ok(click) = q.get(activate.entity) else {
        return;
    };
    match *click {
        Click::Accept(id) => {
            jobs.posted.retain(|j| *j != id);
            jobs.held.push(id);
            jobs.selected = Some(id);
        }
        Click::HandIn(id) => {
            jobs.held.retain(|j| *j != id);
            if jobs.tracked == Some(id) {
                jobs.tracked = None;
            }
            if jobs.selected == Some(id) {
                jobs.selected = None;
            }
            let reward = job(id).reward;
            jobs.balance += reward;
            spawn_toast(&mut commands, reward, jobs.balance);
        }
        Click::Track(id) => {
            jobs.tracked = if jobs.tracked == Some(id) {
                None
            } else {
                Some(id)
            };
        }
        Click::SelectJob(id) => {
            jobs.selected = Some(id);
        }
        Click::ShowOnMap(id) => {
            chart.selected = Some(job(id).target);
            *page = GalleryPage::Chart;
        }
        Click::Tile(x, y) => {
            chart.selected = Some((x, y));
        }
        Click::Trade(side, i) => {
            market.selected = Some((side, i));
        }
        Click::Save(i) => {
            load.selected = i;
        }
        Click::Randomize => {
            new_game.seed = next_seed(new_game.seed);
        }
        Click::Disclosure => {
            new_game.disclosure_open = !new_game.disclosure_open;
        }
        Click::Role(i) => {
            new_game.role = i;
        }
    }
}

/// A small LCG step, so Randomize always moves the seed.
fn next_seed(seed: u64) -> u64 {
    seed.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
        % 10_000_000_000
}

/// The hand-in toast: the credit delta and the new balance, bottom right,
/// fading over [`TOAST_SECS`].
fn spawn_toast(commands: &mut Commands, reward: u32, balance: u32) {
    commands.spawn((
        Name::new(TOAST),
        Toast {
            ttl: TOAST_SECS,
            life: TOAST_SECS,
        },
        Node {
            position_type: PositionType::Absolute,
            right: px(32),
            bottom: px(72),
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::axes(px(14), px(10)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Surface, 1.0),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Nominal, 1.0),
        GlobalZIndex(50),
        Pickable::IGNORE,
        children![
            text(
                &format!("+{} cr", thousands(reward)),
                16.0,
                UiColor::Nominal
            ),
            text(
                &format!("balance {} cr", thousands(balance)),
                12.0,
                UiColor::Body
            ),
        ],
    ));
}

// ---------------------------------------------------------------------------
// Shared layout helpers
// ---------------------------------------------------------------------------

fn text(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(value.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    )
}

/// A fixed-width column cell, so numbers line up down a table.
fn cell(width: f32, value: &str, size: f32, color: UiColor, right: bool) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            justify_content: if right {
                JustifyContent::FlexEnd
            } else {
                JustifyContent::FlexStart
            },
            ..default()
        },
        children![text(value, size, color)],
    )
}

/// A cell that takes the rest of the row.
fn grow_cell(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            min_width: px(0),
            overflow: Overflow::clip(),
            ..default()
        },
        children![text(value, size, color)],
    )
}

/// A `have / need` meter: a display-only slider track at a fixed width with
/// the counts beside it.
fn meter(width: f32, have: u32, need: u32) -> impl Bundle {
    let fraction = if need == 0 {
        0.0
    } else {
        have as f32 / need as f32
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        },
        children![
            (
                Node {
                    width: px(width),
                    flex_shrink: 0.0,
                    ..default()
                },
                children![slider_track(fraction)],
            ),
            text(
                &format!("{} / {}", thousands(have), thousands(need)),
                12.0,
                UiColor::Body
            ),
        ],
    )
}

/// A labelled meter row: label | track | have / need.
fn labelled_meter(c: &mut ChildSpawnerCommands, label: &str, have: u32, need: u32) {
    c.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(10),
        ..default()
    })
    .with_children(|r| {
        r.spawn(cell(90.0, label, 12.0, UiColor::Label, false));
        r.spawn(meter(150.0, have, need));
    });
}

fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn row_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(10),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A page's split: the list side and the inspector side.
fn split() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    }
}

/// The list side: a column that takes what the inspector leaves.
fn main_column() -> Node {
    Node {
        flex_grow: 1.0,
        min_width: px(0),
        min_height: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(12),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The inspector: a fixed-width panel on the right.
fn inspector(width: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(14)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    )
}

/// A titled panel with a padded column body.
fn titled_panel(
    c: &mut ChildSpawnerCommands,
    title: &str,
    tag: Option<&str>,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    c.spawn((
        Node {
            flex_shrink: 0.0,
            ..panel_node()
        },
        panel(),
    ))
    .with_children(|p| {
        p.spawn(panel_head(title, tag));
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::all(px(10)),
            ..default()
        })
        .with_children(build);
    });
}

/// An uppercase section label inside a panel.
fn section(c: &mut ChildSpawnerCommands, label: &str) {
    c.spawn((
        text(&label.to_uppercase(), 10.0, UiColor::Label),
        Node {
            margin: UiRect::top(px(6)),
            ..default()
        },
    ));
}

/// A `label: value` line of an inspector.
fn kv(c: &mut ChildSpawnerCommands, label: &str, value: &str) {
    c.spawn(row_node()).with_children(|r| {
        r.spawn(cell(110.0, label, 12.0, UiColor::Label, false));
        r.spawn(grow_cell(value, 12.0, UiColor::Body));
    });
}

/// A bordered placeholder box where a 3D view would render.
fn preview_box(width: f32, height: f32, lines: &[&str]) -> impl Bundle {
    let lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    (
        Node {
            width: px(width),
            height: px(height),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(4),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Surface, 0.6),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.6),
        Children::spawn(SpawnWith(move |p: &mut RelatedSpawner<ChildOf>| {
            for (i, line) in lines.iter().enumerate() {
                let (size, color) = if i == 0 {
                    (18.0, UiColor::Primary)
                } else {
                    (10.0, UiColor::Label)
                };
                p.spawn(text(line, size, color));
            }
        })),
    )
}

/// A compact button for a table cell.
fn small_button(spec: ButtonSpec) -> ButtonSpec {
    ButtonSpec {
        min_height: 24.0,
        font_size: 12.0,
        ..spec.fit()
    }
}

/// The mock HUD strip under a page: the objective chips the feedback asks for.
fn hud_strip(c: &mut ChildSpawnerCommands, chips: &[(&str, &str, &str)]) {
    c.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            flex_shrink: 0.0,
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::top(px(1)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.4),
    ))
    .with_children(|strip| {
        strip.spawn(text("HUD OBJECTIVES", 10.0, UiColor::Label));
        for (glyph, label, tip) in chips {
            strip
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        padding: UiRect::axes(px(10), px(4)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::alpha(UiColor::Surface, 0.8),
                    BorderColor::all(Color::NONE),
                    ThemedBorder::alpha(UiColor::Primary, 0.5),
                    Tooltip(tip.to_string()),
                ))
                .with_children(|chip| {
                    chip.spawn(text(glyph, 12.0, UiColor::Accent));
                    chip.spawn(text(label, 12.0, UiColor::Body));
                });
        }
        strip.spawn(text("hover a chip to expand it", 10.0, UiColor::Secondary));
    });
}

// ---------------------------------------------------------------------------
// Jobs page (task 20261012-100022, items 1-9)
// ---------------------------------------------------------------------------

fn jobs_page(body: &mut ChildSpawnerCommands, filter: JobFilter, jobs: &JobsState) {
    body.spawn(split()).with_children(|s| {
        s.spawn(main_column()).with_children(|col| {
            // 6: filter chips over the board.
            col.spawn(row_node()).with_children(|r| {
                r.spawn(text("Filter", 12.0, UiColor::Label));
                r.spawn(segmented_container()).with_children(|seg| {
                    let chips = std::iter::once(JobFilter::All)
                        .chain(JobKind::ALL.into_iter().map(JobFilter::Kind));
                    for f in chips {
                        let mut b = seg.spawn((
                            segmented_option(f.label()),
                            ButtonValue(f),
                            Name::new(format!("{FILTER_PREFIX} {}", f.label())),
                        ));
                        if f == filter {
                            b.insert(Selected);
                        }
                    }
                });
                r.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                r.spawn(text(
                    &format!("balance {} cr", thousands(jobs.balance)),
                    12.0,
                    UiColor::Body,
                ));
            });
            posted_panel(col, filter, jobs);
            held_panel(col, jobs);
        });
        job_inspector(s, jobs);
    });
    let tracked = jobs.tracked.map(job);
    let tracked_chip = tracked.map(|j| {
        (
            j.kind.glyph(),
            format!("{}  -  {}", j.title, offset_hint(j.target)),
            format!("Tracked: {}. {}", j.title, j.task),
        )
    });
    let mut chips: Vec<(&str, &str, &str)> = Vec::new();
    if let Some((g, l, t)) = &tracked_chip {
        chips.push((g, l, t));
    }
    chips.push((
        "+",
        "Station network 3/8 jobs",
        "CHARTER Station network: 3 / 8 jobs, 2 / 4 stations, 1 / 2 rings",
    ));
    hud_strip(body, &chips);
}

/// 1, 2, 3, 6: the posted table with aligned columns, a time-left meter or an
/// Amber badge, an expired row collapsed to one line, and a FITS YOUR ROLE
/// badge.
fn posted_panel(col: &mut ChildSpawnerCommands, filter: JobFilter, jobs: &JobsState) {
    let visible: Vec<&JobCard> = jobs
        .posted
        .iter()
        .map(|id| job(*id))
        .filter(|j| filter.admits(j.kind))
        .collect();
    titled_panel(col, "Jobs posted here", Some("DUST GATE"), |p| {
        table_head(
            p,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (90.0, "Reward"),
                (170.0, "Target"),
                (150.0, "Time left"),
                (130.0, ""),
                (90.0, ""),
            ],
        );
        if visible.is_empty() {
            // 9: the empty state names the next posting instead of hiding.
            p.spawn((
                Name::new(EMPTY_STATE),
                Node {
                    padding: UiRect::axes(px(13), px(12)),
                    ..default()
                },
                children![text(
                    "No jobs posted, next posting in 4 min",
                    13.0,
                    UiColor::Secondary
                )],
            ));
        }
        for j in visible {
            let selected = jobs.selected == Some(j.id);
            let expired = j.minutes_left == 0;
            let mut row = spawn_row(
                p,
                Click::SelectJob(j.id),
                format!("{POSTED_ROW_PREFIX} {}", j.id),
                selected,
            );
            row.with_children(|r| {
                let ink = if expired {
                    UiColor::Label
                } else {
                    UiColor::Body
                };
                r.spawn(cell(28.0, j.kind.glyph(), 14.0, UiColor::Accent, false));
                if expired {
                    // 2: an expired offer greys and collapses to one line.
                    r.spawn(grow_cell(j.title, 13.0, ink));
                    r.spawn(badge(BadgeKind::Mute, "expired"));
                    return;
                }
                r.spawn(grow_cell(j.title, 13.0, ink));
                r.spawn(cell(
                    90.0,
                    &format!("{} cr", thousands(j.reward)),
                    13.0,
                    ink,
                    true,
                ));
                r.spawn(cell(170.0, j.target_name, 12.0, UiColor::Secondary, false));
                time_left_cell(r, j);
                r.spawn(Node {
                    width: px(130),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    if j.fits_role {
                        c.spawn(badge(BadgeKind::Blue, "fits your role"));
                    }
                });
                r.spawn(Node {
                    width: px(90),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn((
                        acts(
                            button(small_button(ButtonSpec::new("Accept").primary())),
                            Click::Accept(j.id),
                        ),
                        Name::new(format!("{ACCEPT_PREFIX} {}", j.id)),
                    ));
                });
            });
        }
    });
}

/// 2: time left as a shrinking track, or an Amber badge under the threshold.
fn time_left_cell(r: &mut ChildSpawnerCommands, j: &JobCard) {
    r.spawn(Node {
        width: px(150),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(8),
        ..default()
    })
    .with_children(|c| {
        if j.minutes_left <= 6 {
            c.spawn(badge(BadgeKind::Amber, &format!("{} min", j.minutes_left)));
        } else {
            c.spawn((
                Node {
                    width: px(80),
                    flex_shrink: 0.0,
                    ..default()
                },
                children![slider_track(j.minutes_left as f32 / j.window as f32)],
            ));
            c.spawn(text(
                &format!("{} min", j.minutes_left),
                12.0,
                UiColor::Secondary,
            ));
        }
    });
}

/// 4, 3, 7: held rows with a `have / need` meter, DONE / HAND IN HERE badges, a
/// greyed Hand in with the refusal in a tooltip, and a Track toggle.
fn held_panel(col: &mut ChildSpawnerCommands, jobs: &JobsState) {
    titled_panel(col, "Your jobs", None, |p| {
        table_head(
            p,
            &[
                (28.0, ""),
                (0.0, "Job"),
                (230.0, "Progress"),
                (220.0, ""),
                (70.0, "Track"),
                (90.0, ""),
            ],
        );
        if jobs.held.is_empty() {
            p.spawn((
                Node {
                    padding: UiRect::axes(px(13), px(12)),
                    ..default()
                },
                children![text("No jobs held", 13.0, UiColor::Secondary)],
            ));
        }
        for id in &jobs.held {
            let j = job(*id);
            let selected = jobs.selected == Some(j.id);
            let mut row = spawn_row(
                p,
                Click::SelectJob(j.id),
                format!("{HELD_ROW_PREFIX} {}", j.id),
                selected,
            );
            row.with_children(|r| {
                r.spawn(cell(28.0, j.kind.glyph(), 14.0, UiColor::Accent, false));
                r.spawn(grow_cell(j.title, 13.0, UiColor::Body));
                r.spawn(Node {
                    width: px(230),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn(meter(120.0, j.have, j.need));
                });
                r.spawn(Node {
                    width: px(220),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: px(6),
                    ..default()
                })
                .with_children(|c| {
                    if j.done {
                        c.spawn(badge(BadgeKind::Green, "done"));
                    }
                    if j.hand_in_here {
                        c.spawn(badge(BadgeKind::Blue, "hand in here"));
                    }
                    if j.fits_role {
                        c.spawn(badge(BadgeKind::Mute, "fits your role"));
                    }
                });
                r.spawn(Node {
                    width: px(70),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn((
                        clickable(toggle(jobs.tracked == Some(j.id)), Click::Track(j.id)),
                        Name::new(format!("{TRACK_PREFIX} {}", j.id)),
                    ));
                });
                r.spawn(Node {
                    width: px(90),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                })
                .with_children(|c| {
                    let refusal = hand_in_refusal(j);
                    let mut b = c.spawn((
                        acts(
                            button(small_button(ButtonSpec::new("Hand in").primary())),
                            Click::HandIn(j.id),
                        ),
                        Name::new(format!("{HAND_IN_PREFIX} {}", j.id)),
                    ));
                    if let Some(why) = refusal {
                        b.insert((InteractionDisabled, Tooltip(why)));
                    }
                });
            });
        }
    });
}

/// Why a held job cannot be handed in here, or `None` when it can.
fn hand_in_refusal(j: &JobCard) -> Option<String> {
    if !j.done {
        return Some(format!(
            "Not done: {} of {} {}",
            j.have,
            j.need,
            match j.kind {
                JobKind::Bring => "t delivered",
                JobKind::Recover => "recovered",
                _ => "complete",
            }
        ));
    }
    if !j.hand_in_here {
        return Some(format!(
            "Hand in at {}, {}",
            j.hand_in,
            offset_hint(j.target)
        ));
    }
    None
}

/// A table's header line: `(width, label)` per column, width 0 for the column
/// that grows.
fn table_head(p: &mut ChildSpawnerCommands, columns: &[(f32, &str)]) {
    p.spawn(Node {
        padding: UiRect::axes(px(13), px(4)),
        ..row_node()
    })
    .with_children(|r| {
        for (width, label) in columns {
            if *width == 0.0 {
                r.spawn(grow_cell(&label.to_uppercase(), 10.0, UiColor::Label));
            } else {
                r.spawn(cell(
                    *width,
                    &label.to_uppercase(),
                    10.0,
                    UiColor::Label,
                    false,
                ));
            }
        }
    });
}

/// 5: the right-hand inspector for the selected job, with Show on Map.
fn job_inspector(s: &mut ChildSpawnerCommands, jobs: &JobsState) {
    s.spawn(inspector(380.0)).with_children(|i| {
        i.spawn(panel_header("Inspector"));
        let Some(j) = jobs.selected.map(job) else {
            i.spawn(text(
                "Select a job to read its task here.",
                12.0,
                UiColor::Secondary,
            ));
            return;
        };
        let held = jobs.held.contains(&j.id);
        i.spawn(row_node()).with_children(|r| {
            r.spawn(text(j.kind.glyph(), 18.0, UiColor::Accent));
            r.spawn(text(j.title, 15.0, UiColor::Primary));
        });
        i.spawn(row_node()).with_children(|r| {
            r.spawn(badge(BadgeKind::Mute, j.kind.label()));
            if held {
                r.spawn(badge(BadgeKind::Blue, "held"));
            }
            if j.fits_role {
                r.spawn(badge(BadgeKind::Blue, "fits your role"));
            }
        });
        i.spawn(separator());
        section(i, "Task");
        i.spawn(text(j.task, 12.0, UiColor::Body));
        section(i, "Details");
        kv(i, "Poster", j.poster);
        kv(i, "Target", &format!("{} {:?}", j.target_name, j.target));
        kv(i, "Hand in", j.hand_in);
        kv(i, "Reward", &format!("{} cr", thousands(j.reward)));
        kv(
            i,
            "Time left",
            &if j.minutes_left == 0 {
                "expired".to_string()
            } else {
                format!("{} min", j.minutes_left)
            },
        );
        if held {
            kv(i, "Progress", &format!("{} / {}", j.have, j.need));
        }
        i.spawn(separator());
        i.spawn((
            acts(
                button(ButtonSpec::new("Show on Map").key("M")),
                Click::ShowOnMap(j.id),
            ),
            Name::new(SHOW_ON_MAP),
        ));
    });
}

/// 20261012-100030 item 4: the sector offset from the player as words.
fn offset_hint(target: (i32, i32)) -> String {
    let dx = target.0 - PLAYER_CELL.0;
    let dy = target.1 - PLAYER_CELL.1;
    if dx == 0 && dy == 0 {
        return "this sector".into();
    }
    let mut parts = Vec::new();
    if dx != 0 {
        let s = if dx.abs() == 1 { "sector" } else { "sectors" };
        parts.push(format!(
            "{} {s} {}",
            dx.abs(),
            if dx > 0 { "east" } else { "west" }
        ));
    }
    if dy != 0 {
        let word = if dy > 0 { "south" } else { "north" };
        if dx == 0 {
            let s = if dy.abs() == 1 { "sector" } else { "sectors" };
            parts.push(format!("{} {s} {word}", dy.abs()));
        } else {
            parts.push(format!("{} {word}", dy.abs()));
        }
    }
    parts.join(", ")
}

// ---------------------------------------------------------------------------
// Chart page (task 20261012-100030, items 1-7; task 20261012-100025, item 6)
// ---------------------------------------------------------------------------

const TILE_PX: f32 = 72.0;

fn chart_page(body: &mut ChildSpawnerCommands, chart: &ChartState, jobs: &JobsState) {
    body.spawn(split()).with_children(|s| {
        s.spawn(main_column()).with_children(|col| {
            col.spawn(row_node()).with_children(|r| {
                r.spawn(text("SECTOR CHART", 13.0, UiColor::Primary));
                r.spawn(text(
                    "click a tile; the panel lists what it holds",
                    11.0,
                    UiColor::Secondary,
                ));
                r.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                r.spawn(text("PLAY 02:14:07", 12.0, UiColor::Label));
            });
            col.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(16),
                ..default()
            })
            .with_children(|r| {
                tile_grid(r, chart);
                r.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                })
                .with_children(|side| {
                    charter_block(side);
                    discovery_log(side);
                });
            });
            legend(col);
        });
        chart_inspector(s, chart, jobs);
    });
    hud_strip(
        body,
        &[(
            "+",
            "Station network 3/8 jobs",
            "CHARTER Station network: 3 / 8 jobs, 2 / 4 stations, 1 / 2 rings",
        )],
    );
}

/// 1, 2: the grid of tiles, each with up to four glyphs and the station mark
/// variants.
fn tile_grid(r: &mut ChildSpawnerCommands, chart: &ChartState) {
    r.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(3),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|grid| {
        for y in 0..CHART_SIDE {
            grid.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(3),
                ..default()
            })
            .with_children(|row| {
                for x in 0..CHART_SIDE {
                    tile(row, x, y, chart);
                }
            });
        }
    });
}

fn tile(row: &mut ChildSpawnerCommands, x: i32, y: i32, chart: &ChartState) {
    let cell = chart_cell((x, y));
    let selected = chart.selected == Some((x, y));
    let player = (x, y) == PLAYER_CELL;
    let visited = cell.is_some_and(|c| c.entered.is_some());
    let fill = if player {
        ThemedFill::alpha(UiColor::Primary, 0.18)
    } else if visited {
        ThemedFill::alpha(UiColor::Surface, 0.9)
    } else {
        ThemedFill::alpha(UiColor::Surface, 0.35)
    };
    let border = if selected {
        ThemedBorder::new(UiColor::Accent)
    } else if visited {
        ThemedBorder::alpha(UiColor::Secondary, 0.7)
    } else {
        ThemedBorder::alpha(UiColor::Secondary, 0.25)
    };
    row.spawn((
        clickable(
            (
                Node {
                    width: px(TILE_PX),
                    height: px(TILE_PX),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(px(4)),
                    border: UiRect::all(px(if selected { 2 } else { 1 })),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                fill,
                BorderColor::all(Color::NONE),
                border,
            ),
            Click::Tile(x, y),
        ),
        Name::new(format!("{TILE_PREFIX} {x} {y}")),
    ))
    .with_children(|t| {
        t.spawn(row_node()).with_children(|head| {
            head.spawn(text(&format!("{x},{y}"), 9.0, UiColor::Label));
            if player {
                head.spawn(text("@", 11.0, UiColor::Primary));
            }
        });
        t.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6),
            row_gap: px(0),
            ..default()
        })
        .with_children(|marks| {
            if let Some(cell) = cell {
                for mark in cell.marks.iter().take(4) {
                    marks.spawn(text(&mark.glyph(), 13.0, mark.color()));
                }
            }
        });
    });
}

/// 1: the legend as one icon row.
fn legend(col: &mut ChildSpawnerCommands) {
    col.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(14),
        row_gap: px(4),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|l| {
        l.spawn(text("LEGEND", 10.0, UiColor::Label));
        let entries: [(String, &str, UiColor); 12] = [
            ("@".into(), "you", UiColor::Primary),
            ("*".into(), "asteroids", UiColor::Primary),
            ("o".into(), "rocks", UiColor::Primary),
            ("O".into(), "planet", UiColor::Primary),
            ("x".into(), "derelicts", UiColor::Secondary),
            ("%".into(), "wreck", UiColor::Secondary),
            ("#".into(), "station", UiColor::Primary),
            ("#.".into(), "dead", UiColor::Label),
            ("#+".into(), "owned", UiColor::Nominal),
            ("#!".into(), "refusing", UiColor::Accent),
            ("#n".into(), "posting n jobs", UiColor::Primary),
            ("!n".into(), "JOB-n", UiColor::Accent),
        ];
        for (glyph, label, color) in entries {
            l.spawn(row_node()).with_children(|e| {
                e.spawn(text(&glyph, 13.0, color));
                e.spawn(text(label, 11.0, UiColor::Secondary));
            });
        }
    });
}

/// 5: the CHARTER block with labelled meters and a STANDING / MET badge.
fn charter_block(side: &mut ChildSpawnerCommands) {
    titled_panel(side, "Charter", Some("TRANSPORT"), |p| {
        p.spawn(row_node()).with_children(|r| {
            r.spawn(text("Station network", 14.0, UiColor::Primary));
            r.spawn(badge(BadgeKind::Amber, "standing"));
        });
        labelled_meter(p, "Jobs", 3, 8);
        labelled_meter(p, "Stations", 2, 4);
        labelled_meter(p, "Rings", 1, 2);
        p.spawn(text("Met at: not yet", 11.0, UiColor::Secondary));
    });
}

/// 7: discovery log rows grouped by session, newest pulsing once.
fn discovery_log(side: &mut ChildSpawnerCommands) {
    titled_panel(side, "Discovery log", None, |p| {
        let mut last_session = None;
        for (i, (glyph, code, sector, when, this_session)) in DISCOVERY_LOG.iter().enumerate() {
            if last_session != Some(*this_session) {
                section(
                    p,
                    if *this_session {
                        "This session"
                    } else {
                        "Earlier"
                    },
                );
                last_session = Some(*this_session);
            }
            p.spawn(Node {
                padding: UiRect::axes(px(4), px(2)),
                ..row_node()
            })
            .with_children(|r| {
                r.spawn(cell(22.0, glyph, 13.0, UiColor::Primary, false));
                let mut code_cell = r.spawn(Node {
                    width: px(150),
                    flex_shrink: 0.0,
                    ..default()
                });
                code_cell.with_children(|c| {
                    let mut t = c.spawn(text(code, 12.0, UiColor::Body));
                    if i == 0 {
                        t.insert(Pulse(0.0));
                    }
                });
                r.spawn(cell(50.0, sector, 12.0, UiColor::Secondary, false));
                r.spawn(cell(80.0, when, 11.0, UiColor::Label, true));
            });
        }
    });
}

/// 3, 4, and economy 6: the selected cell as glyph rows with counts, the
/// entered time and the finds, the route hint, and a station's last-seen
/// prices and stock.
fn chart_inspector(s: &mut ChildSpawnerCommands, chart: &ChartState, jobs: &JobsState) {
    s.spawn(inspector(380.0)).with_children(|i| {
        i.spawn(panel_header("Sector"));
        let Some(at) = chart.selected else {
            i.spawn(text(
                "Click a tile to list what it holds.",
                12.0,
                UiColor::Secondary,
            ));
            return;
        };
        i.spawn(row_node()).with_children(|r| {
            r.spawn(text(&format!("Sector {:?}", at), 15.0, UiColor::Primary));
            if at == PLAYER_CELL {
                r.spawn(badge(BadgeKind::Blue, "you are here"));
            }
        });
        i.spawn(text(&offset_hint(at), 12.0, UiColor::Accent));
        let Some(known) = chart_cell(at) else {
            i.spawn(text("Uncharted.", 12.0, UiColor::Secondary));
            return;
        };
        i.spawn(separator());
        section(i, "Contents");
        for mark in known.marks {
            let (name, count) = mark.row();
            i.spawn(row_node()).with_children(|r| {
                r.spawn(cell(22.0, &mark.glyph(), 13.0, mark.color(), false));
                r.spawn(grow_cell(&name, 12.0, UiColor::Body));
                r.spawn(cell(90.0, &count, 12.0, UiColor::Secondary, true));
            });
            if let Mark::Job(id) = mark {
                let held = jobs.held.contains(id);
                i.spawn(Node {
                    padding: UiRect::left(px(32)),
                    ..row_node()
                })
                .with_children(|r| {
                    r.spawn(badge(
                        if held {
                            BadgeKind::Blue
                        } else {
                            BadgeKind::Mute
                        },
                        if held { "held" } else { "posted" },
                    ));
                    r.spawn(text(
                        &format!("{} cr", thousands(job(*id).reward)),
                        12.0,
                        UiColor::Secondary,
                    ));
                });
            }
        }
        section(i, "Visit");
        kv(
            i,
            "Entered",
            known.entered.unwrap_or("not yet, known from a job mark"),
        );
        if !known.finds.is_empty() {
            kv(i, "Finds", &known.finds.join(", "));
        }
        let station = known.marks.iter().find_map(|m| match m {
            Mark::Station(kind, name) => Some((*kind, *name)),
            _ => None,
        });
        if let Some((kind, name)) = station {
            section(i, &format!("{name}: last seen"));
            if let StationKind::Refusing = kind {
                i.spawn(row_node()).with_children(|r| {
                    r.spawn(badge(BadgeKind::Amber, "refused 6 min"));
                    r.spawn(text("economy window", 11.0, UiColor::Secondary));
                });
            }
            kv(i, "Docked at", "01:52:10");
            i.spawn(row_node()).with_children(|r| {
                r.spawn(cell(110.0, "ITEM", 10.0, UiColor::Label, false));
                r.spawn(cell(70.0, "PRICE", 10.0, UiColor::Label, true));
                r.spawn(cell(70.0, "STOCK", 10.0, UiColor::Label, true));
            });
            for (item, price, stock) in LAST_SEEN {
                i.spawn(row_node()).with_children(|r| {
                    r.spawn(cell(110.0, item, 12.0, UiColor::Body, false));
                    r.spawn(cell(
                        70.0,
                        &format!("{price} cr"),
                        12.0,
                        UiColor::Body,
                        true,
                    ));
                    r.spawn(cell(
                        70.0,
                        &format!("{stock} t"),
                        12.0,
                        UiColor::Secondary,
                        true,
                    ));
                });
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Market page (task 20261012-100025, items 1-5)
// ---------------------------------------------------------------------------

fn market_page(body: &mut ChildSpawnerCommands, market: &MarketState) {
    body.spawn(split()).with_children(|s| {
        s.spawn(main_column()).with_children(|col| {
            // 5: the play-time clock right of the pane tabs.
            col.spawn((
                Node {
                    flex_shrink: 0.0,
                    ..panel_node()
                },
                panel(),
            ))
            .with_children(|card| {
                card.spawn(panel_head("Inventory", None)).with_children(|head| {
                    head.spawn(segmented(&["Map", "Ship", "Inventory", "Station"], 2));
                    head.spawn(Node {
                        flex_grow: 1.0,
                        ..default()
                    });
                    head.spawn(text("PLAY 02:14:07", 12.0, UiColor::Label));
                });
                card.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    padding: UiRect::all(px(12)),
                    ..default()
                })
                .with_children(|b| {
                    // 4: the refusal window as a badge with its time left.
                    b.spawn(row_node()).with_children(|r| {
                        r.spawn(text("Dust Gate", 14.0, UiColor::Primary));
                        r.spawn((
                            Name::new(MARKET_REFUSED),
                            Node::default(),
                            Tooltip(
                                "The economy window closed on your last trade. Docking is refused for 6 min."
                                    .into(),
                            ),
                            children![badge(BadgeKind::Amber, "refused 6 min")],
                        ));
                        r.spawn(Node {
                            flex_grow: 1.0,
                            ..default()
                        });
                        r.spawn(text("balance 12,450 cr", 12.0, UiColor::Body));
                    });
                    b.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(12),
                        ..default()
                    })
                    .with_children(|tables| {
                        trade_table(tables, TradeSide::Tender, market);
                        trade_table(tables, TradeSide::Trader, market);
                    });
                    market_strip(b);
                    restock_line(b);
                });
            });
        });
        market_inspector(s, market);
    });
}

/// 1: one trade table with a price column and a delta badge whose tooltip
/// carries the reason.
fn trade_table(tables: &mut ChildSpawnerCommands, side: TradeSide, market: &MarketState) {
    let (title, tag, qty_label, price_label) = match side {
        TradeSide::Tender => ("Tender", "YOUR HOLD", "Qty", "Pays"),
        TradeSide::Trader => ("Trader", "DUST GATE", "Stock", "Asks"),
    };
    tables
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..panel_node()
            },
            panel(),
        ))
        .with_children(|p| {
            p.spawn(panel_head(title, Some(tag)));
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::all(px(8)),
                ..default()
            })
            .with_children(|b| {
                table_head(
                    b,
                    &[
                        (0.0, "Item"),
                        (60.0, qty_label),
                        (80.0, price_label),
                        (90.0, "vs ref"),
                    ],
                );
                for (i, w) in WARES.iter().enumerate() {
                    let qty = match side {
                        TradeSide::Tender => w.ship_qty,
                        TradeSide::Trader => w.station_qty,
                    };
                    if qty == 0 {
                        continue;
                    }
                    let price = match side {
                        TradeSide::Tender => w.bid,
                        TradeSide::Trader => w.ask,
                    };
                    let delta = delta_percent(price, w.reference);
                    // Green is the good side of each table: the station pays
                    // over reference, or asks under it.
                    let good = match side {
                        TradeSide::Tender => delta > 0,
                        TradeSide::Trader => delta < 0,
                    };
                    let kind = if delta.abs() < 5 {
                        BadgeKind::Mute
                    } else if good {
                        BadgeKind::Green
                    } else {
                        BadgeKind::Amber
                    };
                    let mut row = spawn_row(
                        b,
                        Click::Trade(side, i),
                        format!("{TRADE_ROW_PREFIX} {title} {i}"),
                        market.selected == Some((side, i)),
                    );
                    row.with_children(|r| {
                        r.spawn(grow_cell(w.name, 13.0, UiColor::Body));
                        r.spawn(cell(60.0, &format!("{qty} t"), 13.0, UiColor::Body, true));
                        r.spawn(cell(
                            80.0,
                            &format!("{price} cr"),
                            13.0,
                            UiColor::Body,
                            true,
                        ));
                        r.spawn((
                            Node {
                                width: px(90),
                                flex_shrink: 0.0,
                                justify_content: JustifyContent::FlexEnd,
                                ..default()
                            },
                            children![(
                                Node::default(),
                                Tooltip(w.reason.to_string()),
                                children![badge(kind, &format!("{delta:+}%"))],
                            )],
                        ));
                    });
                }
            });
        });
}

/// 2: the Market strip: what the station pays most over reference and sells
/// most under, as chips.
fn market_strip(b: &mut ChildSpawnerCommands) {
    let mut pays: Vec<(&Ware, i32)> = WARES
        .iter()
        .map(|w| (w, delta_percent(w.bid, w.reference)))
        .collect();
    pays.sort_by_key(|(_, d)| -d);
    let mut sells: Vec<(&Ware, i32)> = WARES
        .iter()
        .map(|w| (w, delta_percent(w.ask, w.reference)))
        .collect();
    sells.sort_by_key(|(_, d)| *d);
    b.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(6),
        ..default()
    })
    .with_children(|m| {
        section(m, "Market");
        for (label, list, kind) in [
            ("Pays most", &pays, BadgeKind::Green),
            ("Sells cheapest", &sells, BadgeKind::Blue),
        ] {
            m.spawn(row_node()).with_children(|r| {
                r.spawn(cell(120.0, label, 12.0, UiColor::Label, false));
                for (w, d) in list.iter().take(3) {
                    r.spawn((
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(6),
                            padding: UiRect::axes(px(8), px(3)),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        ThemedFill::alpha(UiColor::Surface, 0.8),
                        BorderColor::all(Color::NONE),
                        ThemedBorder::alpha(UiColor::Secondary, 0.6),
                        Tooltip(w.reason.to_string()),
                        children![
                            text(w.name, 12.0, UiColor::Body),
                            badge(kind, &format!("{d:+}%")),
                        ],
                    ));
                }
            });
        }
    });
}

/// 3: restock as a relative time and a stock-against-target meter.
fn restock_line(b: &mut ChildSpawnerCommands) {
    b.spawn(row_node()).with_children(|r| {
        r.spawn(cell(120.0, "Restock", 12.0, UiColor::Label, false));
        r.spawn(text("restocked 12 min ago", 12.0, UiColor::Body));
        r.spawn(text("stock at 70% of target", 12.0, UiColor::Secondary));
        r.spawn(meter(140.0, 7, 10));
        r.spawn(text("t", 12.0, UiColor::Secondary));
    });
}

fn market_inspector(s: &mut ChildSpawnerCommands, market: &MarketState) {
    s.spawn(inspector(360.0)).with_children(|i| {
        i.spawn(panel_header("Inspector"));
        let Some((side, idx)) = market.selected else {
            i.spawn(text(
                "Select a row. The delta badge's tooltip carries the price reason.",
                12.0,
                UiColor::Secondary,
            ));
            return;
        };
        let w = &WARES[idx];
        i.spawn(text(w.name, 15.0, UiColor::Primary));
        i.spawn(badge(
            BadgeKind::Mute,
            match side {
                TradeSide::Tender => "your hold",
                TradeSide::Trader => "station stock",
            },
        ));
        i.spawn(separator());
        kv(i, "Reference", &format!("{} cr", w.reference));
        kv(
            i,
            "Pays",
            &format!("{} cr ({:+}%)", w.bid, delta_percent(w.bid, w.reference)),
        );
        kv(
            i,
            "Asks",
            &format!("{} cr ({:+}%)", w.ask, delta_percent(w.ask, w.reference)),
        );
        kv(i, "In hold", &format!("{} t", w.ship_qty));
        kv(i, "In stock", &format!("{} t", w.station_qty));
        section(i, "Last seen elsewhere");
        kv(i, "Reef Yard", &format!("{} cr at 01:52", w.reference + 4));
        kv(i, "Vane Dock", &format!("{} cr at 00:33", w.reference - 2));
        i.spawn(separator());
        i.spawn(row_node()).with_children(|r| {
            r.spawn(button(small_button(ButtonSpec::new("Buy 1 t").primary())));
            r.spawn(button(small_button(ButtonSpec::new("Sell 1 t"))));
        });
    });
}

// ---------------------------------------------------------------------------
// Load page (task 20261012-100030, items 8-9; task 20261012-100025, item 5)
// ---------------------------------------------------------------------------

fn load_page(body: &mut ChildSpawnerCommands, load: &LoadState) {
    body.spawn(split()).with_children(|s| {
        s.spawn(main_column()).with_children(|col| {
            // 8: Continue on the most recent world.
            col.spawn(row_node()).with_children(|r| {
                r.spawn((
                    button(
                        ButtonSpec::new(format!("Continue  {}", SAVES[0].world))
                            .primary()
                            .fit()
                            .key("Enter"),
                    ),
                    Name::new(CONTINUE),
                ));
                r.spawn(text(
                    &format!("last played {}", SAVES[0].saved_rel),
                    12.0,
                    UiColor::Secondary,
                ));
                r.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                r.spawn(text(
                    &format!("running {RUNNING_VERSION}"),
                    11.0,
                    UiColor::Label,
                ));
            });
            titled_panel(col, "Worlds", Some("BY LAST PLAYED"), |p| {
                table_head(
                    p,
                    &[
                        (50.0, "Role"),
                        (0.0, "World"),
                        (240.0, "Charter"),
                        (100.0, "Play time"),
                        (110.0, "Saved"),
                        (90.0, ""),
                    ],
                );
                for (i, save) in SAVES.iter().enumerate() {
                    let role = &ROLES[save.role];
                    let mut row = spawn_row(
                        p,
                        Click::Save(i),
                        format!("{SAVE_ROW_PREFIX} {i}"),
                        load.selected == i,
                    );
                    row.with_children(|r| {
                        r.spawn(cell(50.0, role.code, 12.0, UiColor::Accent, false));
                        r.spawn(grow_cell(save.world, 13.0, UiColor::Body));
                        r.spawn(Node {
                            width: px(240),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|c| {
                            c.spawn(meter(100.0, save.have, save.need));
                        });
                        r.spawn(cell(100.0, save.play_time, 12.0, UiColor::Body, true));
                        r.spawn(cell(110.0, save.saved_rel, 12.0, UiColor::Secondary, true));
                        r.spawn(Node {
                            width: px(90),
                            flex_shrink: 0.0,
                            justify_content: JustifyContent::FlexEnd,
                            ..default()
                        })
                        .with_children(|c| {
                            if save.version != RUNNING_VERSION {
                                c.spawn(badge(BadgeKind::Amber, save.version));
                            } else if save.met {
                                c.spawn(badge(BadgeKind::Green, "met"));
                            }
                        });
                    });
                }
            });
        });
        load_details(s, load);
    });
}

/// 9: the details card: the hull preview, then the save's facts, with the
/// version only as a badge when it differs.
fn load_details(s: &mut ChildSpawnerCommands, load: &LoadState) {
    let save = &SAVES[load.selected];
    let role = &ROLES[save.role];
    s.spawn(inspector(400.0)).with_children(|i| {
        i.spawn(panel_header("World"));
        i.spawn(preview_box(
            370.0,
            150.0,
            &[
                &format!("<[{}]>", role.hull),
                "hull_preview 3D view",
                &format!("{} start hull", role.name),
            ],
        ));
        i.spawn(row_node()).with_children(|r| {
            r.spawn(text(save.world, 16.0, UiColor::Primary));
            if save.version != RUNNING_VERSION {
                r.spawn(badge(BadgeKind::Amber, save.version));
            }
        });
        i.spawn(separator());
        kv(i, "Role", role.name);
        kv(i, "Seed", &save.seed.to_string());
        kv(i, "Sector", save.sector);
        kv(i, "Credits", &format!("{} cr", thousands(save.credits)));
        kv(i, "Play time", save.play_time);
        i.spawn(row_node()).with_children(|r| {
            r.spawn(cell(110.0, "Charter", 12.0, UiColor::Label, false));
            r.spawn(text(role.charter, 12.0, UiColor::Body));
            r.spawn(badge(
                if save.met {
                    BadgeKind::Green
                } else {
                    BadgeKind::Amber
                },
                if save.met { "met" } else { "standing" },
            ));
        });
        i.spawn(Node {
            padding: UiRect::left(px(110)),
            ..row_node()
        })
        .with_children(|c| {
            c.spawn(meter(140.0, save.have, save.need));
            c.spawn(text(save.charter_label, 12.0, UiColor::Secondary));
        });
        kv(
            i,
            "Saved",
            &format!("{} ({})", save.saved_at, save.saved_rel),
        );
        i.spawn(separator());
        i.spawn(row_node()).with_children(|r| {
            r.spawn(button(ButtonSpec::new("Load").primary().fit().key("Enter")));
            r.spawn(button(ButtonSpec::new("Delete").danger().fit()));
        });
    });
}

// ---------------------------------------------------------------------------
// New Game page (task 20261012-100030, items 10-12)
// ---------------------------------------------------------------------------

fn new_game_page(body: &mut ChildSpawnerCommands, new_game: &NewGameState) {
    body.spawn(split()).with_children(|s| {
        s.spawn(main_column()).with_children(|col| {
            // 11: world name and seed on one line, Randomize as an icon button.
            col.spawn(row_node()).with_children(|r| {
                r.spawn(cell(60.0, "World", 12.0, UiColor::Label, false));
                r.spawn((
                    Node {
                        width: px(320),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    children![text_field(
                        TextFieldSpec::new("Dust Gate run")
                            .placeholder("World name")
                            .dense()
                    )],
                ));
                r.spawn(cell(50.0, "Seed", 12.0, UiColor::Label, false));
                r.spawn((
                    Node {
                        width: px(180),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    children![text_field(TextFieldSpec::new(new_game.seed.to_string()).dense())],
                ));
                r.spawn((
                    acts(
                        (
                            button(small_button(ButtonSpec::new("::"))),
                            Tooltip("Randomize the seed".into()),
                        ),
                        Click::Randomize,
                    ),
                    Name::new(RANDOMIZE),
                ));
                r.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                let mark = if new_game.disclosure_open { "[-]" } else { "[+]" };
                r.spawn((
                    acts(
                        button(small_button(ButtonSpec::new(format!(
                            "What is a world? {mark}"
                        ))
                        .ghost())),
                        Click::Disclosure,
                    ),
                    Name::new(DISCLOSURE),
                ));
            });
            if new_game.disclosure_open {
                col.spawn((
                    Node {
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::alpha(UiColor::Surface, 0.6),
                    BorderColor::all(Color::NONE),
                    ThemedBorder::alpha(UiColor::Secondary, 0.5),
                    children![text(
                        "A world is one seeded open sector map: the belts, planets, civilizations and stations the seed draws, plus everything you do in it. Two worlds with one seed start the same and drift apart from your first dock.",
                        12.0,
                        UiColor::Body
                    )],
                ));
            }
            // 12: the seed preview strip, the ring-0 cells around the start.
            col.spawn(row_node()).with_children(|r| {
                r.spawn(cell(60.0, "Start", 12.0, UiColor::Label, false));
                seed_preview(r, new_game.seed);
                r.spawn(text(
                    "the five cells around the start, drawn from the seed",
                    11.0,
                    UiColor::Secondary,
                ));
            });
            // 10: one card per role.
            section(col, "Start role");
            col.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(10),
                ..default()
            })
            .with_children(|cards| {
                for (i, role) in ROLES.iter().enumerate() {
                    role_card(cards, i, role, new_game.role == i);
                }
            });
            col.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            col.spawn(row_node()).with_children(|r| {
                r.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                r.spawn(button(ButtonSpec::new("Back").ghost().fit().key("Esc")));
                r.spawn((
                    button(ButtonSpec::new("Create").primary().fit().key("Enter")),
                    Name::new(CREATE),
                ));
            });
        });
        role_details(s, &ROLES[new_game.role]);
    });
}

fn role_card(cards: &mut ChildSpawnerCommands, i: usize, role: &RoleCard, selected: bool) {
    let mut card = cards.spawn((
        clickable(list_row(), Click::Role(i)),
        Name::new(format!("{ROLE_PREFIX} {i}")),
    ));
    // A card is a column, not the row `list_row` spawns; the paint and the
    // hover/selection stay the row's.
    card.insert(Node {
        width: px(182),
        min_height: px(250),
        flex_direction: FlexDirection::Column,
        row_gap: px(6),
        padding: UiRect::all(px(8)),
        border: UiRect::all(px(1)),
        ..default()
    });
    if selected {
        card.insert(Selected);
    }
    card.with_children(|c| {
        c.spawn(preview_box(
            164.0,
            80.0,
            &[&format!("<[{}]>", role.hull), "hull_preview"],
        ));
        c.spawn(text(role.name, 14.0, UiColor::Primary));
        c.spawn(text(role.charter, 12.0, UiColor::Body));
        c.spawn(text(role.counts, 11.0, UiColor::Secondary));
        c.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4),
            row_gap: px(4),
            ..default()
        })
        .with_children(|chips| {
            chips.spawn(badge(
                BadgeKind::Blue,
                &format!("{} cr", thousands(role.credits)),
            ));
            for kit in role.kit {
                chips.spawn(badge(BadgeKind::Mute, kit));
            }
        });
    });
}

/// 12: five chart tiles drawn from the seed.
fn seed_preview(r: &mut ChildSpawnerCommands, seed: u64) {
    r.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(3),
        ..default()
    })
    .with_children(|strip| {
        for i in 0..5u64 {
            let h = next_seed(seed ^ (i.wrapping_mul(0x9E37_79B9_7F4A_7C15)));
            let mut marks: Vec<(String, UiColor)> = Vec::new();
            if h % 3 != 0 {
                marks.push(("*".into(), UiColor::Primary));
            }
            if (h / 3) % 2 == 0 {
                marks.push(("o".into(), UiColor::Primary));
            }
            if (h / 7) % 4 == 0 {
                marks.push(("O".into(), UiColor::Primary));
            }
            if (h / 11) % 3 == 0 {
                marks.push(("x".into(), UiColor::Secondary));
            }
            if i == 2 {
                marks.insert(0, ("#".into(), UiColor::Primary));
            }
            strip
                .spawn((
                    Node {
                        width: px(56),
                        height: px(56),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(px(4)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::alpha(UiColor::Surface, if i == 2 { 0.9 } else { 0.5 }),
                    BorderColor::all(Color::NONE),
                    ThemedBorder::alpha(UiColor::Secondary, 0.6),
                ))
                .with_children(|t| {
                    t.spawn(text(if i == 2 { "@" } else { "" }, 10.0, UiColor::Primary));
                    t.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(5),
                        ..default()
                    })
                    .with_children(|m| {
                        for (glyph, color) in marks.iter().take(4) {
                            m.spawn(text(glyph, 12.0, *color));
                        }
                    });
                });
        }
    });
}

/// 10: the lore paragraph behind the card, in the details pane.
fn role_details(s: &mut ChildSpawnerCommands, role: &RoleCard) {
    s.spawn(inspector(380.0)).with_children(|i| {
        i.spawn(panel_header("Role"));
        i.spawn(text(role.name, 16.0, UiColor::Primary));
        i.spawn(text(role.lore, 12.0, UiColor::Body));
        i.spawn(separator());
        section(i, "Charter");
        i.spawn(text(role.charter, 13.0, UiColor::Body));
        i.spawn(text(role.counts, 12.0, UiColor::Secondary));
        section(i, "Start");
        kv(i, "Role id", role.id);
        kv(i, "Hull", role.hull);
        kv(i, "Credits", &format!("{} cr", thousands(role.credits)));
        kv(i, "Kit", &role.kit.join(", "));
    });
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Capture {
    /// Under the autopilot the pass waits for the script's last beat to arm
    /// it, so the two do not fight over the page.
    armed: bool,
    stage: usize,
    wait: u32,
    pending: Option<String>,
    finished: bool,
}

/// The shots: a page, the theme to take it in, and the file.
#[cfg(feature = "debug")]
const SHOTS: [(GalleryPage, &str, &str); 6] = [
    (GalleryPage::Jobs, PHOSPHOR_THEME_ID, "ui_gallery-jobs.png"),
    (
        GalleryPage::Chart,
        PHOSPHOR_THEME_ID,
        "ui_gallery-chart.png",
    ),
    (
        GalleryPage::Market,
        PHOSPHOR_THEME_ID,
        "ui_gallery-market.png",
    ),
    (GalleryPage::Load, PHOSPHOR_THEME_ID, "ui_gallery-load.png"),
    (
        GalleryPage::NewGame,
        PHOSPHOR_THEME_ID,
        "ui_gallery-new-game.png",
    ),
    (
        GalleryPage::Jobs,
        HARDWARE_THEME_ID,
        "ui_gallery-jobs-hardware.png",
    ),
];

#[cfg(feature = "debug")]
fn drive_capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut page: ResMut<GalleryPage>,
    mut selected: ResMut<SelectedUiTheme>,
    mut chart: ResMut<ChartState>,
    mut jobs: ResMut<JobsState>,
    mut market: ResMut<MarketState>,
    mut exit: MessageWriter<AppExit>,
    log: Option<Res<nova_protocol::prelude::CaptureLog>>,
) {
    if !nova_protocol::prelude::capturing() || cap.finished {
        return;
    }
    let driven = std::env::var_os("NOVA_AUTOPILOT").is_some();
    if driven && !cap.armed {
        return;
    }
    if cap.wait > 0 {
        cap.wait -= 1;
        return;
    }
    if let Some(shot) = cap.pending.clone() {
        if !log.is_some_and(|log| log.wrote(&shot)) {
            return;
        }
        cap.pending = None;
    }
    let stage = cap.stage;
    if stage >= SHOTS.len() * 2 {
        cap.finished = true;
        if !driven {
            exit.write(AppExit::Success);
        }
        return;
    }
    let (wanted_page, theme, file) = SHOTS[stage / 2];
    if stage % 2 == 0 {
        // Show the page with something selected, so the inspector is not empty.
        *page = wanted_page;
        *selected = SelectedUiTheme(theme.to_string());
        if chart.selected.is_none() {
            chart.selected = Some((5, 2));
        }
        if jobs.selected.is_none() {
            jobs.selected = Some(2);
        }
        if market.selected.is_none() {
            market.selected = Some((TradeSide::Trader, 1));
        }
        cap.wait = 45;
    } else {
        shoot(file, &mut commands);
        cap.pending = Some(file.to_string());
    }
    cap.stage += 1;
}

#[cfg(feature = "debug")]
fn shoot(name: &str, commands: &mut Commands) {
    let name = name.to_string();
    commands.queue(move |world: &mut World| {
        nova_protocol::prelude::capture_window(world, &name);
        info!("ui_gallery capture: {name}");
    });
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The driven run: one real click per beat, by `Name`, and a verdict beat
/// after each that reads the state the click moved.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::prelude::{
        resource_where, ui_node_diagnosis, ui_node_present, BEAT_DEADLINE_SECS,
    };

    let accept = format!("{ACCEPT_PREFIX} 4");
    let filter_answer = format!("{FILTER_PREFIX} Answer");
    let filter_all = format!("{FILTER_PREFIX} All");
    let held_row = format!("{HELD_ROW_PREFIX} 4");
    let tile = format!("{TILE_PREFIX} 5 2");
    let trade_row = format!("{TRADE_ROW_PREFIX} Trader 1");
    let save_row = format!("{SAVE_ROW_PREFIX} 2");
    let role = format!("{ROLE_PREFIX} 3");
    let hand_in = format!("{HAND_IN_PREFIX} 8");
    let track = format!("{TRACK_PREFIX} 4");

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("gallery: the board is up")
        .until(ui_node_present(PAGE_JOBS))
        .diagnose(ui_node_diagnosis(PAGE_JOBS))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Jobs: Accept moves a row from posted to held and selects it.
        .click_named(
            "gallery: accept JOB-4",
            &accept,
            resource_where::<JobsState>(|jobs| jobs.held.contains(&4)),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: accept moved the job")
        .on_enter(|world: &mut World| {
            let jobs = world.resource::<JobsState>();
            assert!(
                !jobs.posted.contains(&4) && jobs.held.contains(&4),
                "accepting JOB-4 must move it from posted to held"
            );
            assert_eq!(jobs.selected, Some(4), "the accepted job is selected");
            info!(
                "gallery: accept moved JOB-4 to held ({} held)",
                jobs.held.len()
            );
        })
        .add()
        // The Answer filter now hides every posted row and shows the empty
        // state, since the only Answer job was just accepted.
        .click_named(
            "gallery: filter Answer",
            &filter_answer,
            ui_node_present(EMPTY_STATE),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the filter hid the rows")
        .on_enter(|world: &mut World| {
            assert_eq!(
                *world.resource::<JobFilter>(),
                JobFilter::Kind(JobKind::Answer),
                "clicking the Answer chip must drive the filter"
            );
            let rows = count_named_prefix(world, POSTED_ROW_PREFIX);
            assert_eq!(rows, 0, "no posted row survives the Answer filter");
            info!("gallery: filter Answer shows {rows} rows and the empty state");
        })
        .add()
        .click_named(
            "gallery: filter All",
            &filter_all,
            resource_where::<JobFilter>(|f| *f == JobFilter::All),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the rows are back")
        .on_enter(|world: &mut World| {
            let rows = count_named_prefix(world, POSTED_ROW_PREFIX);
            assert_eq!(rows, 6, "six posted rows under All after one accept");
            info!("gallery: filter All shows {rows} rows");
        })
        .add()
        // Select the held row, then Show on Map lands on the Chart page with
        // the job's cell selected.
        .click_named(
            "gallery: select the held row",
            &held_row,
            resource_where::<JobsState>(|jobs| jobs.selected == Some(4)),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: Show on Map",
            SHOW_ON_MAP,
            resource_where::<GalleryPage>(|p| *p == GalleryPage::Chart),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the chart opened on the job's cell")
        .on_enter(|world: &mut World| {
            let chart = world.resource::<ChartState>();
            assert_eq!(
                chart.selected,
                Some(job(4).target),
                "Show on Map must select the job's target cell"
            );
            info!("gallery: chart opened on {:?}", chart.selected);
        })
        .add()
        .click_named(
            "gallery: click a tile",
            &tile,
            resource_where::<ChartState>(|c| c.selected == Some((5, 2))),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the tile is selected")
        .on_enter(|world: &mut World| {
            info!(
                "gallery: tile selected {:?}",
                world.resource::<ChartState>().selected
            );
        })
        .add()
        // Market: the page shows the REFUSED badge; a trade row selects.
        .click_named(
            "gallery: open Market",
            PAGE_MARKET,
            ui_node_present(MARKET_REFUSED),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: select a trade row",
            &trade_row,
            resource_where::<MarketState>(|m| m.selected == Some((TradeSide::Trader, 1))),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the trade row is selected")
        .on_enter(|world: &mut World| {
            info!(
                "gallery: trade row selected {:?}",
                world.resource::<MarketState>().selected
            );
        })
        .add()
        // Load: a row selects and fills the details card.
        .click_named(
            "gallery: open Load",
            PAGE_LOAD,
            ui_node_present(CONTINUE),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: select a save",
            &save_row,
            resource_where::<LoadState>(|l| l.selected == 2),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the save is selected")
        .on_enter(|world: &mut World| {
            info!(
                "gallery: save selected {}",
                SAVES[world.resource::<LoadState>().selected].world
            );
        })
        .add()
        // New Game: Randomize moves the seed, a role card selects, the
        // disclosure opens.
        .click_named(
            "gallery: open New Game",
            PAGE_NEW_GAME,
            ui_node_present(CREATE),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: Randomize",
            RANDOMIZE,
            resource_where::<NewGameState>(|n| n.seed != INITIAL_SEED),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: pick a role",
            &role,
            resource_where::<NewGameState>(|n| n.role == 3),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: open the disclosure",
            DISCLOSURE,
            resource_where::<NewGameState>(|n| n.disclosure_open),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: new game state moved")
        .on_enter(|world: &mut World| {
            let n = world.resource::<NewGameState>();
            assert_ne!(n.seed, INITIAL_SEED, "Randomize must change the seed");
            assert_eq!(n.role, 3, "the clicked role card is selected");
            assert!(n.disclosure_open, "the disclosure is open");
            info!(
                "gallery: seed {} role {} disclosure open",
                n.seed, ROLES[n.role].id
            );
        })
        .add()
        // Back to Jobs: Hand in removes the done job, pays, and toasts.
        .click_named(
            "gallery: back to Jobs",
            PAGE_JOBS,
            resource_where::<GalleryPage>(|p| *p == GalleryPage::Jobs),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "gallery: hand in JOB-8",
            &hand_in,
            resource_where::<JobsState>(|jobs| !jobs.held.contains(&8)),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the hand-in paid and toasted")
        .until(ui_node_present(TOAST))
        .diagnose(ui_node_diagnosis(TOAST))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("gallery: the balance moved")
        .on_enter(|world: &mut World| {
            let jobs = world.resource::<JobsState>();
            let expected = JobsState::default().balance + job(8).reward;
            assert_eq!(
                jobs.balance, expected,
                "hand-in adds the reward to the balance"
            );
            info!("gallery: balance {} cr after the hand-in", jobs.balance);
        })
        .add()
        .click_named(
            "gallery: track JOB-4",
            &track,
            resource_where::<JobsState>(|jobs| jobs.tracked == Some(4)),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the track toggle moved the HUD chip")
        .on_enter(|world: &mut World| {
            info!(
                "gallery: tracked {:?}",
                world.resource::<JobsState>().tracked
            );
        })
        .add()
        // Last: when capturing, arm the capture pass and wait it out.
        .step("gallery: capture pass")
        .on_enter(|world: &mut World| {
            world.resource_mut::<Capture>().armed = true;
        })
        .until(resource_where::<Capture>(|cap| {
            cap.finished || !nova_protocol::prelude::capturing()
        }))
        .deadline(180.0)
        .add()
}

/// How many laid-out entities carry a `Name` starting with `prefix`.
#[cfg(feature = "debug")]
fn count_named_prefix(world: &mut World, prefix: &str) -> usize {
    let mut q = world.query::<&Name>();
    q.iter(world)
        .filter(|n| n.as_str().starts_with(prefix))
        .count()
}
