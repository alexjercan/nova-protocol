//! ui_gallery: a clickable MOCK of the interface proposals in the owner's
//! 2026-10-17 UI feedback (tasks 20261012-100022, -100025 and -100030), drawn
//! in the game's own chrome so each screen reads as part of the game. It is
//! built from `nova_ui::widget` and bevy alone: every job, sector, price,
//! save and role is hard-coded data under `ui_gallery/data.rs`, and every
//! click moves that data so the owner can judge how the proposed screens
//! would look AND react.
//!
//! The top bar switches between six pages: `Jobs` (the JOBS pane: board and
//! own jobs, docked or undocked), `Chart` (the MAP pane's chart view with a
//! 3D sector chart), `Market` (the INVENTORY pane with prices), `Load` (the
//! menu's LOAD panel with a 3D save preview), `New Game` (world name, seed,
//! seed preview and role cards with spinning hulls) and `Icons` (every icon
//! mask at three sizes). `S` flips the theme; a text field keeps the keys
//! while it is focused.
//!
//! Interactive run:  `cargo run --example ui_gallery`.
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

#[path = "ui_gallery/chart.rs"]
mod chart;
#[path = "ui_gallery/chart3d.rs"]
mod chart3d;
#[path = "ui_gallery/chrome.rs"]
mod chrome;
#[path = "ui_gallery/data.rs"]
mod data;
#[path = "ui_gallery/icon_sheet.rs"]
mod icon_sheet;
#[path = "ui_gallery/icons.rs"]
mod icons;
#[path = "ui_gallery/jobs.rs"]
mod jobs;
#[path = "ui_gallery/load.rs"]
mod load;
#[path = "ui_gallery/market.rs"]
mod market;
#[path = "ui_gallery/new_game.rs"]
mod new_game;

use bevy::{prelude::*, ui::IsDefaultUiCamera, ui_widgets::Activate};
use chart3d::{ChartOrbits, ChartPointer, SceneAssets, SceneKind};
use chrome::{fade_toasts, spawn_toast, text};
use clap::Parser;
use data::{job, next_seed, Category, INITIAL_SEED, WARES};
use icons::GalleryIcons;
use nova_protocol::prelude::GameStates;
use nova_ui::{
    prelude::*,
    theme::{base::base_ui_themes, ActiveUiTheme, UiColor, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
    widget::ThemedFill,
    NovaUiPlugin,
};

#[derive(Parser)]
#[command(name = "ui_gallery")]
#[command(version = "2.0.0")]
#[command(
    about = "A clickable mock of the proposed job board, chart, market, load and new game screens in the game's chrome",
    long_about = None
)]
struct Cli;

// The `Name`s the harness clicks by. Rows and buttons that exist per record
// take the record's id as a suffix (`Gallery Accept 4`), so a name resolves
// to exactly one entity on every page.
pub const PAGE_JOBS: &str = "Gallery Page Jobs";
pub const PAGE_CHART: &str = "Gallery Page Chart";
pub const PAGE_MARKET: &str = "Gallery Page Market";
pub const PAGE_LOAD: &str = "Gallery Page Load";
pub const PAGE_NEW_GAME: &str = "Gallery Page NewGame";
pub const PAGE_ICONS: &str = "Gallery Page Icons";
pub const THEME_HARDWARE: &str = "Gallery Theme Hardware";
pub const THEME_PHOSPHOR: &str = "Gallery Theme Phosphor";
pub const DOCK_PREFIX: &str = "Gallery Dock";
pub const TIER_PREFIX: &str = "Gallery Tier";
pub const ACCEPT_PREFIX: &str = "Gallery Accept";
pub const HAND_IN_PREFIX: &str = "Gallery Hand In";
pub const POSTED_ROW_PREFIX: &str = "Gallery Posted Row";
pub const HELD_ROW_PREFIX: &str = "Gallery Held Row";
pub const SHOW_ON_MAP: &str = "Gallery Show On Map";
pub const EMPTY_STATE: &str = "Gallery Empty State";
pub const TOAST: &str = "Gallery Toast";
pub const TILE_PREFIX: &str = "Gallery Tile";
pub const CHART_VIEWPORT: &str = "Gallery Chart Viewport";
pub const MAP_REFRAME: &str = "Gallery Map Reframe";
pub const MAP_CHART: &str = "Gallery Map Chart";
pub const FILTER_PREFIX: &str = "Gallery Filter";
pub const MARKET_REFUSED: &str = "Gallery Market Refused";
pub const TRADE_ROW_PREFIX: &str = "Gallery Trade Row";
pub const MARKET_BUY: &str = "Gallery Buy";
pub const MARKET_SELL: &str = "Gallery Sell";
pub const MARKET_BUY_ALL: &str = "Gallery Buy All";
pub const MARKET_SELL_ALL: &str = "Gallery Sell All";
pub const SAVE_ROW_PREFIX: &str = "Gallery Save Row";
pub const CONTINUE: &str = "Gallery Continue";
pub const LOAD_PREVIEW: &str = "Gallery Load Preview";
pub const LOAD_LOAD: &str = "Gallery Load";
pub const LOAD_DELETE: &str = "Gallery Delete";
pub const RANDOMIZE: &str = "Gallery Randomize";
pub const SEED_FIELD: &str = "Gallery Seed";
pub const WORLD_NAME_FIELD: &str = "Gallery World Name";
pub const SEED_PREVIEW: &str = "Gallery Seed Preview";
pub const ROLE_PREFIX: &str = "Gallery Role";
pub const CREATE: &str = "Gallery Create";
pub const ICON_SHEET: &str = "Gallery Icon Sheet";

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
    app.add_observer(button_on_setting::<DockState>);
    app.add_observer(button_on_setting::<BoardTier>);
    app.add_observer(chart3d::on_scene_target_removed);
    app.init_resource::<GalleryPage>();
    app.init_resource::<DockState>();
    app.init_resource::<BoardTier>();
    app.init_resource::<JobsState>();
    app.init_resource::<ChartState>();
    app.init_resource::<MarketState>();
    app.init_resource::<MarketDraft>();
    app.init_resource::<LoadState>();
    app.init_resource::<NewGameState>();
    app.init_resource::<ChartOrbits>();
    app.init_resource::<ChartPointer>();
    #[cfg(feature = "debug")]
    app.init_resource::<Capture>();
    #[cfg(feature = "debug")]
    app.init_resource::<OrbitProbe>();
    app.init_state::<GameStates>();
    app.add_systems(
        Startup,
        (
            icons::init_gallery_icons,
            chart3d::init_tile_textures,
            setup,
            reach_playing,
        ),
    );
    app.add_systems(
        Update,
        (
            rebuild_body.run_if(body_dirty),
            toggle_theme_key,
            sync_marks::<SelectedUiTheme>.run_if(resource_changed::<SelectedUiTheme>),
            sync_marks::<GalleryPage>.run_if(resource_changed::<GalleryPage>),
            sync_marks::<DockState>.run_if(resource_changed::<DockState>),
            fade_toasts,
            submit_fields,
            market::read_draft_field,
            market::sync_draft,
        ),
    );
    app.add_systems(
        Update,
        (
            chart3d::chart_input,
            chart3d::drive_chart_cameras,
            chart3d::spin_hulls,
        )
            .chain(),
    );
    // The targets and the labels follow the layout, after this frame's UI
    // pass has sized the nodes and placed the cameras.
    app.add_systems(
        PostUpdate,
        (
            chart3d::reconcile_scene_targets,
            chart3d::project_chart_labels,
        )
            .chain()
            .after(bevy::ui::UiSystems::Layout)
            .after(bevy::transform::TransformSystems::Propagate),
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
// State
// ---------------------------------------------------------------------------

/// The page the top bar shows.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum GalleryPage {
    #[default]
    Jobs,
    Chart,
    Market,
    Load,
    NewGame,
    Icons,
}

impl GalleryPage {
    const ALL: [(GalleryPage, &'static str, &'static str); 6] = [
        (GalleryPage::Jobs, "Jobs", PAGE_JOBS),
        (GalleryPage::Chart, "Chart", PAGE_CHART),
        (GalleryPage::Market, "Market", PAGE_MARKET),
        (GalleryPage::Load, "Load", PAGE_LOAD),
        (GalleryPage::NewGame, "New Game", PAGE_NEW_GAME),
        (GalleryPage::Icons, "Icons", PAGE_ICONS),
    ];
}

/// Gallery-only: whether the Jobs page is drawn docked at a station.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DockState {
    #[default]
    Docked,
    Undocked,
}

/// Gallery-only: whether the docked station is a starter or a core board.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum BoardTier {
    #[default]
    Starter,
    Core,
}

/// The mock job board: posted ids, held ids, the inspected id, the balance.
#[derive(Resource)]
pub struct JobsState {
    pub posted: Vec<u32>,
    pub held: Vec<u32>,
    pub selected: Option<u32>,
    pub balance: u32,
}

impl Default for JobsState {
    fn default() -> Self {
        Self {
            posted: vec![1, 2, 3, 4, 5, 6, 7],
            held: vec![8, 9, 10],
            selected: None,
            balance: 12_450,
        }
    }
}

/// The chart's selected cell.
#[derive(Resource, Default)]
pub struct ChartState {
    pub selected: Option<(i32, i32)>,
}

/// Which trade column a row is in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TradeSide {
    Tender,
    Trader,
}

/// The mock trade: live copies of the stocks, the selection, the filter,
/// the credits, the last trade's note and the refusal timer.
#[derive(Resource)]
pub struct MarketState {
    pub ship: Vec<u32>,
    pub station: Vec<u32>,
    pub selected: Option<(TradeSide, usize)>,
    pub filter: Option<Category>,
    pub credits: u32,
    pub note: Option<String>,
    pub refused_min: u32,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            ship: WARES.iter().map(|w| w.ship_qty).collect(),
            station: WARES.iter().map(|w| w.station_qty).collect(),
            selected: None,
            filter: None,
            credits: 12_450,
            note: None,
            refused_min: 6,
        }
    }
}

/// The open trade draft's quantity. Kept out of the page rebuild so the
/// slider and the field update in place while they are used.
#[derive(Resource)]
pub struct MarketDraft {
    pub qty: u32,
}

impl Default for MarketDraft {
    fn default() -> Self {
        Self { qty: 1 }
    }
}

/// The Load page's selected save.
#[derive(Resource, Default)]
pub struct LoadState {
    pub selected: usize,
}

/// The New Game page's seed, role and world name.
#[derive(Resource)]
pub struct NewGameState {
    pub seed: u64,
    pub role: usize,
    pub world_name: String,
}

impl Default for NewGameState {
    fn default() -> Self {
        Self {
            seed: INITIAL_SEED,
            role: 0,
            world_name: "Dust Gate run".into(),
        }
    }
}

#[derive(Component)]
struct GalleryRoot;
#[derive(Component)]
struct GalleryBody;

/// What a click does. Every variant writes one resource and `rebuild_body`
/// re-renders the page.
#[derive(Component, Clone, Copy)]
pub enum Click {
    Accept(u32),
    HandIn(u32),
    SelectJob(u32),
    ShowOnMap(u32),
    Tile((i32, i32)),
    Trade(TradeSide, usize),
    Filter(Option<Category>),
    Buy,
    Sell,
    BuyAll,
    SellAll,
    DraftAll,
    Save(usize),
    Randomize,
    Role(usize),
    Reframe(SceneKind),
}

/// The page rebuilds when any page state or the theme moves. The theme is
/// in the set because the 3D scenes bake their tints at build time.
#[expect(
    clippy::too_many_arguments,
    reason = "one Res per gallery state; a change on any of them rebuilds"
)]
fn body_dirty(
    page: Res<GalleryPage>,
    dock: Res<DockState>,
    tier: Res<BoardTier>,
    jobs: Res<JobsState>,
    chart: Res<ChartState>,
    market: Res<MarketState>,
    load: Res<LoadState>,
    new_game: Res<NewGameState>,
    theme: Res<ActiveUiTheme>,
) -> bool {
    page.is_changed()
        || dock.is_changed()
        || tier.is_changed()
        || jobs.is_changed()
        || chart.is_changed()
        || market.is_changed()
        || load.is_changed()
        || new_game.is_changed()
        || theme.is_changed()
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
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
                padding: UiRect::axes(px(20), px(12)),
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
        ))
        .with_children(top_bar);
}

/// The persistent header: the title, the page control, the gallery-only
/// dock toggle and the Theme control.
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
        bar.spawn(text("Jobs page", 12.0, UiColor::Label));
        bar.spawn(segmented_container()).with_children(|seg| {
            for (value, label) in [
                (DockState::Docked, "Docked"),
                (DockState::Undocked, "Undocked"),
            ] {
                let mut b = seg.spawn((
                    segmented_option(label),
                    ButtonValue(value),
                    Name::new(format!("{DOCK_PREFIX} {label}")),
                ));
                if value == DockState::default() {
                    b.insert(Selected);
                }
            }
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
    dock: Res<DockState>,
    tier: Res<BoardTier>,
    jobs: Res<JobsState>,
    chart: Res<ChartState>,
    market: Res<MarketState>,
    load: Res<LoadState>,
    new_game: Res<NewGameState>,
    icons: Res<GalleryIcons>,
    mut assets: SceneAssets,
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
            GalleryPage::Jobs => jobs::jobs_page(body, &icons, &jobs, *dock, *tier),
            GalleryPage::Chart => chart::chart_page(body, &icons, &mut assets, &chart, &jobs),
            GalleryPage::Market => market::market_page(body, &icons, &market),
            GalleryPage::Load => load::load_page(body, &icons, &mut assets, &load),
            GalleryPage::NewGame => new_game::new_game_page(body, &icons, &mut assets, &new_game),
            GalleryPage::Icons => icon_sheet::icon_sheet_page(body, &icons),
        });
    });
}

/// Move the `Selected` mark of a `ButtonValue<T>` group onto the live value,
/// for the groups that are written from outside their own buttons (the `S`
/// key, the capture pass, Show on Map).
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

/// `S` flips the theme, unless a text field has the keys.
fn toggle_theme_key(
    keys: Res<ButtonInput<KeyCode>>,
    typing: Query<(), With<TextFieldFocused>>,
    mut selected: ResMut<SelectedUiTheme>,
) {
    if keys.just_pressed(KeyCode::KeyS) && typing.is_empty() {
        let next = if selected.0 == PHOSPHOR_THEME_ID {
            HARDWARE_THEME_ID
        } else {
            PHOSPHOR_THEME_ID
        };
        *selected = SelectedUiTheme(next.to_string());
    }
}

/// A committed seed or world name moves the New Game state; the page then
/// rebuilds with the preview of the new seed. A seed that is not a number
/// is refused in the field.
fn submit_fields(
    mut submitted: MessageReader<TextFieldSubmitted>,
    mut commands: Commands,
    seeds: Query<(), With<new_game::SeedField>>,
    names: Query<(), With<new_game::WorldNameField>>,
    mut new_game: ResMut<NewGameState>,
) {
    for submit in submitted.read() {
        if seeds.contains(submit.entity) {
            match submit.value.trim().parse::<u64>() {
                Ok(seed) if seed != new_game.seed => new_game.seed = seed,
                Ok(_) => {}
                Err(_) => {
                    commands
                        .entity(submit.entity)
                        .insert(TextFieldError("digits only".into()));
                }
            }
        } else if names.contains(submit.entity) && new_game.world_name != submit.value {
            new_game.world_name = submit.value.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Click handling
// ---------------------------------------------------------------------------

/// Apply a click to the gallery state.
#[expect(
    clippy::too_many_arguments,
    reason = "one ResMut per gallery state; a click may land on any page"
)]
pub fn on_click(
    activate: On<Activate>,
    mut commands: Commands,
    q: Query<&Click>,
    mut page: ResMut<GalleryPage>,
    mut jobs: ResMut<JobsState>,
    mut chart: ResMut<ChartState>,
    mut market: ResMut<MarketState>,
    mut draft: ResMut<MarketDraft>,
    mut load: ResMut<LoadState>,
    mut new_game: ResMut<NewGameState>,
    mut orbits: ResMut<ChartOrbits>,
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
            if jobs.selected == Some(id) {
                jobs.selected = None;
            }
            let reward = job(id).reward;
            jobs.balance += reward;
            spawn_toast(
                &mut commands,
                &format!("+{} cr", data::thousands(reward)),
                &format!("balance {} cr", data::thousands(jobs.balance)),
            );
        }
        Click::SelectJob(id) => {
            jobs.selected = Some(id);
        }
        Click::ShowOnMap(id) => {
            chart.selected = Some(job(id).target);
            *page = GalleryPage::Chart;
        }
        Click::Tile(at) => {
            chart.selected = Some(at);
        }
        Click::Trade(side, i) => {
            market.selected = Some((side, i));
            market.note = None;
            draft.qty = 1;
        }
        Click::Filter(category) => {
            market.filter = category;
        }
        Click::Buy => {
            let note = market::buy(&mut market, draft.qty);
            trade_note(&mut commands, &mut market, note);
        }
        Click::Sell => {
            let note = market::sell(&mut market, draft.qty);
            trade_note(&mut commands, &mut market, note);
        }
        Click::BuyAll => {
            let note = market::buy(&mut market, u32::MAX);
            trade_note(&mut commands, &mut market, note);
        }
        Click::SellAll => {
            let note = market::sell(&mut market, u32::MAX);
            trade_note(&mut commands, &mut market, note);
        }
        Click::DraftAll => {
            draft.qty = market.draft_max();
        }
        Click::Save(i) => {
            load.selected = i;
        }
        Click::Randomize => {
            new_game.seed = next_seed(new_game.seed);
        }
        Click::Role(i) => {
            new_game.role = i;
        }
        Click::Reframe(kind) => {
            orbits.reframe(kind);
        }
    }
}

fn trade_note(commands: &mut Commands, market: &mut MarketState, note: String) {
    spawn_toast(
        commands,
        &note,
        &format!("credits {} cr", data::thousands(market.credits)),
    );
    market.note = Some(note);
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
const SHOTS: [(GalleryPage, &str, &str); 7] = [
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
        GalleryPage::Icons,
        PHOSPHOR_THEME_ID,
        "ui_gallery-icons.png",
    ),
    (
        GalleryPage::Jobs,
        HARDWARE_THEME_ID,
        "ui_gallery-jobs-hardware.png",
    ),
];

/// Frames a page gets to lay out, size its render targets and project its
/// labels before its shot.
#[cfg(feature = "debug")]
const SHOT_SETTLE_FRAMES: u32 = 60;

#[cfg(feature = "debug")]
#[expect(
    clippy::too_many_arguments,
    reason = "the pass seeds every page's selection so no inspector is empty"
)]
fn drive_capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut page: ResMut<GalleryPage>,
    mut selected: ResMut<SelectedUiTheme>,
    mut chart: ResMut<ChartState>,
    mut jobs: ResMut<JobsState>,
    mut market: ResMut<MarketState>,
    mut dock: ResMut<DockState>,
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
        // Show the page with something selected, so the inspector is not
        // empty.
        *page = wanted_page;
        *selected = SelectedUiTheme(theme.to_string());
        chrome::clear_toasts(&mut commands);
        if *dock != DockState::Docked {
            *dock = DockState::Docked;
        }
        if chart.selected.is_none() {
            chart.selected = Some((5, 2));
        }
        if jobs.selected.is_none() {
            jobs.selected = Some(2);
        }
        if market.selected.is_none() {
            market.selected = Some((TradeSide::Trader, 1));
        }
        cap.wait = SHOT_SETTLE_FRAMES;
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

/// The chart camera and where it stood before the orbit beat dragged it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct OrbitProbe {
    camera: Option<Entity>,
    before: Option<Vec3>,
}

/// The chart scene's camera, if the chart is up.
#[cfg(feature = "debug")]
fn chart_camera(world: &mut World) -> Option<Entity> {
    let mut scenes = world.query::<(&chart3d::ChartScene, &chart3d::SceneTarget)>();
    scenes
        .iter(world)
        .find(|(scene, _)| scene.kind == SceneKind::Chart)
        .map(|(_, target)| target.camera())
}

/// The driven run: one real click per beat, by `Name`, and a verdict beat
/// after each that reads the state the click moved.
#[cfg(feature = "debug")]
fn gallery_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::prelude::{
        resource_where, ui_node_diagnosis, ui_node_present, BEAT_DEADLINE_SECS,
    };

    let accept = format!("{ACCEPT_PREFIX} 4");
    let undock = format!("{DOCK_PREFIX} Undocked");
    let dock = format!("{DOCK_PREFIX} Docked");
    let core = format!("{TIER_PREFIX} Core");
    let held_row = format!("{HELD_ROW_PREFIX} 4");
    let tile = format!("{TILE_PREFIX} 5 2");
    let trade_row = format!("{TRADE_ROW_PREFIX} Trader 1");
    let save_row = format!("{SAVE_ROW_PREFIX} 2");
    let role = format!("{ROLE_PREFIX} 3");
    let hand_in = format!("{HAND_IN_PREFIX} 8");

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("gallery: the board is up")
        .until(ui_node_present(PAGE_JOBS))
        .diagnose(ui_node_diagnosis(PAGE_JOBS))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Jobs: the starter board lists only the jobs the hull can do.
        .step("gallery: the starter board")
        .on_enter(|world: &mut World| {
            let rows = count_named_prefix(world, POSTED_ROW_PREFIX);
            assert_eq!(rows, 2, "the starter board lists the two fitting open jobs");
            info!("gallery: starter board lists {rows} rows");
        })
        .add()
        // Core: every open job.
        .click_named(
            "gallery: core station",
            &core,
            resource_where::<BoardTier>(|t| *t == BoardTier::Core),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the core board")
        .on_enter(|world: &mut World| {
            let rows = count_named_prefix(world, POSTED_ROW_PREFIX);
            assert_eq!(rows, 6, "the core board lists every open job");
            info!("gallery: core board lists {rows} rows");
        })
        .add()
        // Accept moves a row from posted to held and selects it.
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
        // Undocked: the board goes, the own jobs stay.
        .click_named(
            "gallery: undock",
            &undock,
            resource_where::<DockState>(|d| *d == DockState::Undocked),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: undocked hides the board")
        .on_enter(|world: &mut World| {
            let posted = count_named_prefix(world, POSTED_ROW_PREFIX);
            let held = count_named_prefix(world, HELD_ROW_PREFIX);
            assert_eq!(posted, 0, "no board row while undocked");
            assert_eq!(held, 4, "the held rows stay while undocked");
            let station_tab = count_named_prefix(world, "InterfaceTabStation");
            assert_eq!(station_tab, 0, "the Station tab hides while undocked");
            info!("gallery: undocked shows {posted} board rows and {held} held rows");
        })
        .add()
        .click_named(
            "gallery: dock",
            &dock,
            resource_where::<DockState>(|d| *d == DockState::Docked),
            BEAT_DEADLINE_SECS,
        )
        // Select the held row, then Show on Map lands on the Chart page
        // with the job's cell selected.
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
        // Orbit: a drag on the chart's orbit moves its camera.
        .step("gallery: orbit the chart")
        .on_enter(|world: &mut World| {
            let camera = chart_camera(world).expect("the chart scene has a camera");
            let before = world.get::<Transform>(camera).map(|t| t.translation);
            *world.resource_mut::<OrbitProbe>() = OrbitProbe {
                camera: Some(camera),
                before,
            };
            world.resource_mut::<ChartOrbits>().drag(
                SceneKind::Chart,
                data::CHART_REACH,
                Vec2::new(180.0, 40.0),
            );
        })
        .until(std::sync::Arc::new(|world: &World| {
            let probe = world.resource::<OrbitProbe>();
            probe
                .camera
                .and_then(|camera| world.get::<Transform>(camera))
                .is_some_and(|t| Some(t.translation) != probe.before)
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("gallery: the camera moved")
        .on_enter(|world: &mut World| {
            let probe = world.resource::<OrbitProbe>();
            let before = probe.before;
            let after = probe
                .camera
                .and_then(|camera| world.get::<Transform>(camera))
                .map(|t| t.translation);
            assert_ne!(before, after, "the orbit drag must move the chart camera");
            info!("gallery: chart camera moved from {before:?} to {after:?}");
        })
        .add()
        // Market: the page shows the REFUSED badge; a trade row selects; Buy
        // moves the credits.
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
        .click_named(
            "gallery: Buy",
            MARKET_BUY,
            resource_where::<MarketState>(|m| m.credits != MarketState::default().credits),
            BEAT_DEADLINE_SECS,
        )
        .step("gallery: the buy moved the balance")
        .on_enter(|world: &mut World| {
            let market = world.resource::<MarketState>();
            let expected = MarketState::default().credits - WARES[1].ask;
            assert_eq!(
                market.credits, expected,
                "one copper at the ask leaves the hold"
            );
            assert_eq!(
                market.ship[1],
                WARES[1].ship_qty + 1,
                "the copper is aboard"
            );
            assert_eq!(
                market.station[1],
                WARES[1].station_qty - 1,
                "the station's copper went down"
            );
            info!(
                "gallery: bought one copper, credits {} cr, note {:?}",
                market.credits, market.note
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
            assert_eq!(
                count_named(world, LOAD_PREVIEW),
                1,
                "the details show the save's chart preview"
            );
            info!(
                "gallery: save selected {}",
                data::SAVES[world.resource::<LoadState>().selected].world
            );
        })
        .add()
        // New Game: Randomize moves the seed and the preview, a role card
        // selects.
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
        .step("gallery: new game state moved")
        .on_enter(|world: &mut World| {
            let n = world.resource::<NewGameState>();
            assert_ne!(n.seed, INITIAL_SEED, "Randomize must change the seed");
            assert_eq!(n.role, 3, "the clicked role card is selected");
            let before = load::seeded_view(INITIAL_SEED, 1, 9, 2, "START").signature();
            let after = load::seeded_view(n.seed, 1, 9, 2, "START").signature();
            assert_ne!(before, after, "the seed preview must change with the seed");
            info!(
                "gallery: seed {} role {} preview {before:#x} -> {after:#x}",
                n.seed,
                data::ROLES[n.role].id
            );
        })
        .add()
        // Icons: the sheet is up.
        .click_named(
            "gallery: open Icons",
            PAGE_ICONS,
            ui_node_present(ICON_SHEET),
            BEAT_DEADLINE_SECS,
        )
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
        // Last: when capturing, arm the capture pass and wait it out.
        .step("gallery: capture pass")
        .on_enter(|world: &mut World| {
            world.resource_mut::<Capture>().armed = true;
        })
        .until(resource_where::<Capture>(|cap| {
            cap.finished || !nova_protocol::prelude::capturing()
        }))
        .deadline(240.0)
        .add()
}

/// How many entities carry exactly the `Name` `name`.
#[cfg(feature = "debug")]
fn count_named(world: &mut World, name: &str) -> usize {
    let mut q = world.query::<&Name>();
    q.iter(world).filter(|n| n.as_str() == name).count()
}

/// How many laid-out entities carry a `Name` starting with `prefix`.
#[cfg(feature = "debug")]
fn count_named_prefix(world: &mut World, prefix: &str) -> usize {
    let mut q = world.query::<&Name>();
    q.iter(world)
        .filter(|n| n.as_str().starts_with(prefix))
        .count()
}
