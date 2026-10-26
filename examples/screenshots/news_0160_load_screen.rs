//! news_0160_load_screen: the v0.16.0 news Load still - the Load screen
//! listing three saved worlds with their role icons, play times and save
//! ages, the selected world's details drawing its chart with STN, WRK and
//! YOU, a FLEET block naming the hull and one escort, and two open
//! objectives.
//!
//! The fresh profile is a first launch, so the menu's bottom-left corner
//! holds the New pilot offer and a field note, and the Load modal does not
//! cover that corner. The walk answers both the way a player does - `Not now`
//! on the offer, `Don't show again` on the note - before the first world, and
//! the answers hold for every later menu entry of the run.
//!
//! Every world is made the way a player makes one, in this run's own fresh
//! profile: New Game, the name and the seed typed, the start role clicked,
//! Create, a stay docked at the starting station, then Back to Main Menu
//! through the pause menu, which writes the leave save. Each world of
//! [`WORLDS`] is played in that order, then Load is opened and shot.
//!
//! The world that sorts first is the one Load selects, so it carries the
//! figure's details. Before its leave the walk, still docked:
//! - warms the starting station's civilization to Ally by merits, the way a
//!   player who hands in its jobs would, and grants [`HIRE_GRANT_CR`], since
//!   the hire gate needs an Ally and the start kit cannot pay;
//! - hires the station's first escort through [`EscortHireCommand`], the
//!   message the Station tab's Hire button writes;
//! - accepts the board's first Procure through [`JobAcceptCommand`], the
//!   message the Jobs pane's Accept button writes, so a held job stands
//!   open beside the role goal;
//! - waits at most [`WRECK_SECS`] for the player's own sensors to find a
//!   derelict hull, the chart's WRK. A seed whose start sees none stops the
//!   run, naming it; nothing marks a wreck the sensors did not find.
//!
//! The saved-ago column reads the wall clock, so it is shown, not asserted.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, capturing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-load-screen.png` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_load_screen --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

// The pointer gestures, shared with the other menu walks.
#[path = "shared/ui_walk.rs"]
mod ui_walk;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_interface::{
        jobs::prelude::JobAcceptCommand,
        station::{EscortHireCommand, StationRuntime},
    },
    prelude::*,
};
use nova_ui::widget::TextFieldValue;
use open_world_stage::{warm_to_ally, window_settled};
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "news_0160_load_screen")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 Load still: three saved worlds, the first with an escort, a held job and a found wreck",
    long_about = None
)]
struct Cli;

/// One world the walk makes and saves.
struct WorldSpec {
    /// The name typed into the world name field.
    name: &'static str,
    /// The seed typed into the seed field.
    seed: u32,
    /// The start role clicked on the role step.
    role: &'static str,
    /// The world Load selects: it hires an escort, holds a job and finds a
    /// wreck before its leave.
    showcase: bool,
}

/// The worlds, in the order the walk plays them. Load lists them by folder
/// name, so the showcase world's name must sort first.
const WORLDS: [WorldSpec; 3] = [
    WorldSpec {
        name: "Ashfall Salvage",
        seed: 34,
        role: "scavenger",
        showcase: true,
    },
    WorldSpec {
        name: "Kestrel Ore Run",
        seed: 2,
        role: "industrial",
        showcase: false,
    },
    WorldSpec {
        name: "Tollgate Hunt",
        seed: 115,
        role: "bounty_hunter",
        showcase: false,
    },
];

/// The backdrop behind the menu, pinned: menu entry draws one of the shipped
/// backdrops at random.
const BACKDROP: &str = "menu_gauntlet";

/// The still this producer writes.
const SHOT: &str = "news-0160-load-screen.png";

/// Credits granted before the hire. The start kits hold far less than an
/// escort's hire price.
const HIRE_GRANT_CR: u32 = 100_000;

/// Frames each world is held docked before its leave, so the worlds read
/// different play times. Not asserted: a frame's play time is the frame's
/// virtual delta.
const STAY_FRAMES: [u32; 3] = [240, 90, 160];

/// Real seconds a world gets to stream in, dock the start and post its board
/// on a software rasterizer.
const WORLD_SECS: f32 = 600.0;

/// Real seconds the showcase world's sensors get to find a derelict hull.
const WRECK_SECS: f32 = 120.0;

/// Real seconds the leave save gets to land and the menu to come back.
const LEAVE_SECS: f32 = 300.0;

/// Backspaces that clear a pre-filled field: a name holds at most 32
/// characters, a seed at most ten digits.
const NAME_CHARS_MAX: usize = 32;
const SEED_DIGITS_MAX: usize = 10;

const NEW_GAME_BUTTON: &str = "New Game Button";
const NAME_FIELD: &str = "World Name Field";
const SEED_FIELD: &str = "World Seed Field";
const WORLD_NEXT_BUTTON: &str = "World Next Button";
const CREATE_WORLD_BUTTON: &str = "Create World Button";
const ROLE_DETAILS_NAME: &str = "Start Role Details Name";
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
const LOAD_BUTTON: &str = "Load Button";
const LOAD_PREVIEW: &str = "Load World Details Preview";
const LOAD_ROW_PREFIX: &str = "Load World Row: ";
const LOAD_ROW_CHECKING: &str = "Load World Row Checking";
const LOAD_DETAILS_NAME: &str = "Load World Details Name";
const TRAINING_PROMPT: &str = "Training Prompt";
const TRAINING_PROMPT_DISMISS: &str = "Training Prompt Dismiss";
const FIELD_NOTE: &str = "Menu Field Note";
const FIELD_NOTE_DISMISS: &str = "Menu Field Note Dismiss";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // This run's own settings and saved worlds: the Load list must hold
    // exactly the worlds this walk makes, and none of a player's.
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));
    std::env::set_var(MENU_BACKDROP_ENV, BACKDROP);

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(load_script());
    app.run()
}

fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// The text of the text node named `name`, if one stands.
fn text_of(world: &World, name: &str) -> Option<String> {
    world
        .try_query::<(&Name, &Text)>()?
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, text)| text.0.clone())
}

/// The text of every text node named `name`, sorted.
fn texts_of(world: &mut World, name: &str) -> Vec<String> {
    let mut texts: Vec<String> = world
        .query::<(&Name, &Text)>()
        .iter(world)
        .filter(|(node, _)| node.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .collect();
    texts.sort();
    texts
}

/// How many nodes are named `name`.
fn count_named(world: &mut World, name: &str) -> usize {
    world
        .query::<&Name>()
        .iter(world)
        .filter(|node| node.as_str() == name)
        .count()
}

/// The value of the details row named `name`: its second text cell.
fn detail_value(world: &mut World, name: &str) -> Option<String> {
    let mut rows = world.query::<(&Name, &Children)>();
    let mut texts = world.query::<&Text>();
    rows.iter(world)
        .find(|(row, _)| row.as_str() == name)
        .and_then(|(_, children)| {
            children
                .iter()
                .filter_map(|cell| texts.get(world, cell).ok())
                .nth(1)
                .map(|text| text.0.clone())
        })
}

/// The value of the laid-out text field named `name`, if one stands.
fn field_value(world: &World, name: &str) -> Option<String> {
    world
        .try_query::<(&Name, &TextFieldValue)>()?
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, value)| value.0.clone())
}

/// The text of every chart preview label shown, sorted.
fn shown_chart_labels(world: &mut World) -> Vec<String> {
    let mut labels: Vec<String> = world
        .query::<(&Name, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, shown)| shown.get())
        .filter_map(|(name, _)| name.as_str().strip_prefix("Menu Chart Label "))
        .map(str::to_string)
        .collect();
    labels.sort();
    labels
}

/// The name a start role's card reads.
///
/// # Panics
///
/// When no loaded role is `role`.
fn role_name(world: &World, role: &str) -> String {
    world
        .resource::<GameStartRoles>()
        .get_role(&StartRoleDesignId::from(role))
        .unwrap_or_else(|| panic!("load screen: no start role '{role}' is loaded"))
        .name
        .clone()
}

/// The folder slug of a world named `name`.
fn slug_of(name: &str) -> String {
    world_slug(name)
        .unwrap_or_else(|refusal| panic!("load screen: '{name}' names no world: {refusal}"))
}

/// The one player ship, if exactly one stands.
fn the_player(world: &World) -> Option<Entity> {
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

/// The session's starting station, once it stands.
fn start_station(world: &World) -> Option<Entity> {
    let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
    body(world, &id)
}

/// The escorts in the player's fleet.
fn fleet_len(world: &World) -> usize {
    world
        .try_query_filtered::<Entity, (With<PlayerFleetMarker>, With<AIFollow>)>()
        .map_or(0, |mut fleet| fleet.iter(world).count())
}

/// The first escort for hire coasting beside `station`.
fn hire_offer(world: &World, station: Entity) -> Option<Entity> {
    world.get::<ParkedHulls>(station)?.iter().find(|hull| {
        matches!(
            world.get::<ParkedHullType>(*hull),
            Some(ParkedHullType::Hire { .. })
        )
    })
}

/// The window is in, the player is docked and the starting station has
/// posted its board.
fn start_is_ready(world: &mut World) -> bool {
    if !window_settled(world) {
        return false;
    }
    let docked = the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
    docked && start_station(world).is_some_and(|station| world.get::<JobBoard>(station).is_some())
}

/// The world at this index of [`WORLDS`] has streamed in docked with its
/// board posted.
#[derive(Resource)]
struct StartReady(usize);

/// Assert Create started the world `spec` asks for.
fn assert_started(world: &mut World, spec: &WorldSpec) {
    let session = world.resource::<OpenWorldSession>();
    assert_eq!(
        (session.seed, session.role.as_str()),
        (spec.seed, spec.role),
        "load screen: Create must start '{}' on seed {} as '{}'",
        spec.name,
        spec.seed,
        spec.role
    );
}

/// Warm the starting station's side to Ally and grant the hire credits.
///
/// # Panics
///
/// When the starting station hires out no escort: the seed no longer gives
/// the figure its FLEET line.
fn warm_the_start(world: &mut World, seed: u32) {
    let station = start_station(world).expect("load screen: the starting station stands");
    assert!(
        hire_offer(world, station).is_some(),
        "load screen: seed {seed}'s starting station hires out no escort"
    );
    warm_to_ally(world, station, "load screen");
    let player = the_player(world).expect("load screen: exactly one player ship");
    world
        .get_mut::<ShipCredits>(player)
        .expect("load screen: the player ship holds credits")
        .0 += HIRE_GRANT_CR;
    info!("load screen: granted {HIRE_GRANT_CR} cr");
}

/// Ask for the starting station's first escort, once its side is the
/// player's: a hire before that is refused as not allied.
fn hire_the_escort(world: &mut World) {
    let station = start_station(world).expect("load screen: the starting station stands");
    let hull = hire_offer(world, station).expect("load screen: the escort for hire stands");
    info!("load screen: hiring the escort {hull:?}");
    world.write_message(EscortHireCommand { hull });
}

/// Fail the run on a refused hire, naming the refusal.
fn refuse_a_refused_hire(world: &mut World) {
    if let Some(note) = world.resource::<StationRuntime>().notice.text() {
        assert!(
            !note.starts_with("hire:"),
            "load screen: the station refused the hire: '{note}'"
        );
    }
}

/// Accept the starting station's first Procure as the Accept button does.
///
/// # Panics
///
/// When the board posts no Procure.
fn accept_a_procure(world: &mut World, seed: u32) {
    let station = start_station(world).expect("load screen: the starting station stands");
    let board = world
        .get::<JobBoard>(station)
        .expect("load screen: the starting station posted its board");
    let job = board
        .0
        .iter()
        .find(|job| matches!(job.kind, JobType::Procure { .. }))
        .unwrap_or_else(|| {
            let kinds: Vec<&str> = board.0.iter().map(|job| job.kind.label()).collect();
            panic!("load screen: seed {seed}'s starter board posts no Procure: {kinds:?}")
        });
    let id = job.id.clone();
    info!("load screen: accepting the Procure '{id}'");
    world.write_message(JobAcceptCommand { id });
}

/// The steps that make, play and leave one world.
fn play_world(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    index: usize,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let spec = &WORLDS[index];
    let (name, seed, role) = (spec.name, spec.seed, spec.role);
    let mut script = script
        .click_named(
            &format!("{name}: click New Game"),
            NEW_GAME_BUTTON,
            ui_node_present(WORLD_NEXT_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .click(&format!("{name}: focus the name field"), NAME_FIELD)
        .step(format!("{name}: type the name"))
        .on_enter(move |world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..NAME_CHARS_MAX {
                press_edit_key(Key::Backspace)(world);
            }
            type_text(name.to_string())(world);
        })
        .until(when(move |world| {
            field_value(world, NAME_FIELD).as_deref() == Some(name)
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click(&format!("{name}: focus the seed field"), SEED_FIELD)
        .step(format!("{name}: type the seed"))
        .on_enter(move |world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..SEED_DIGITS_MAX {
                press_edit_key(Key::Backspace)(world);
            }
            type_text(seed.to_string())(world);
        })
        .until(when(move |world| {
            field_value(world, SEED_FIELD) == Some(seed.to_string())
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_named(
            &format!("{name}: click Next"),
            WORLD_NEXT_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            &format!("{name}: click the {role} role"),
            &format!("Start Role Button: {role}"),
            when(move |world| text_of(world, ROLE_DETAILS_NAME) == Some(role_name(world, role))),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            &format!("{name}: click Create"),
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            WORLD_SECS,
        )
        .step(format!("{name}: Create started the typed world"))
        .on_enter(move |world: &mut World| assert_started(world, &WORLDS[index]))
        .add()
        .step(format!(
            "{name}: the window streams in, the start docks and the board posts"
        ))
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(move |world: &mut World, _, _| {
            if start_is_ready(world) {
                world.insert_resource(StartReady(index));
            }
        })
        .until(when(move |world| {
            world
                .get_resource::<StartReady>()
                .is_some_and(|ready| ready.0 == index)
        }))
        .deadline(WORLD_SECS)
        .add();
    if spec.showcase {
        script = script
            .step(format!("{name}: warm the starting station to Ally"))
            .on_enter(move |world: &mut World| warm_the_start(world, seed))
            .until(when(|world| {
                start_station(world).is_some_and(|station| {
                    world.get::<Allegiance>(station) == Some(&Allegiance::Player)
                })
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{name}: hire one escort"))
            .on_enter(hire_the_escort)
            .each(|world: &mut World, _, _| refuse_a_refused_hire(world))
            .until(when(|world| fleet_len(world) == 1))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{name}: accept a Procure"))
            .on_enter(move |world: &mut World| accept_a_procure(world, seed))
            .until(when(|world| world.resource::<Jobs>().held().len() == 1))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{name}: the sensors find a derelict hull"))
            .until(when(|world| !world.resource::<Chart>().wrecks().is_empty()))
            .deadline(WRECK_SECS)
            .add();
    }
    script
        .step(format!("{name}: stay docked"))
        .until(frames(STAY_FRAMES[index]))
        .add()
        .step(format!("{name}: open the pause menu"))
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step(format!("{name}: let escape go"))
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_named(
            &format!("{name}: back to the main menu"),
            BACK_TO_MENU_BUTTON,
            state_is(GameStates::MainMenu),
            LEAVE_SECS,
        )
        // The menu comes back only once the leave save is on disk.
        .step(format!("{name}: the leave save is on disk"))
        .on_enter(move |_: &mut World| {
            let root = nova_assets::storage::worlds_root()
                .expect("load screen: the isolated profile names a worlds root");
            let header = root.join(slug_of(name)).join("world.ron");
            assert!(
                header.is_file(),
                "load screen: '{name}' left no header at {}",
                header.display()
            );
        })
        .add()
}

/// The corner's offer and field note are answered and laid out nowhere.
fn corner_is_answered(world: &World) -> bool {
    *world.resource::<TrainingPromptSetting>() == TrainingPromptSetting::Hidden
        && *world.resource::<FieldNoteSetting>() == FieldNoteSetting::Hidden
        && ui_node_rect(world, TRAINING_PROMPT).is_none()
        && ui_node_rect(world, FIELD_NOTE).is_none()
}

/// Assert the saved headers and the Load screen agree with [`WORLDS`].
fn assert_the_load_screen(world: &mut World) {
    assert!(
        corner_is_answered(world),
        "load screen: the New pilot offer or the field note is still up"
    );
    assert!(
        ui_node_rect(world, LOAD_PREVIEW).is_some(),
        "load screen: the details preview is not laid out"
    );
    let root = nova_assets::storage::worlds_root()
        .expect("load screen: the isolated profile names a worlds root");
    let listings = list_worlds(
        &root,
        world.resource::<LoadedSectionPacks>(),
        world.resource::<GameStartRoles>(),
    )
    .unwrap_or_else(|refusal| panic!("load screen: the worlds root refused: {refusal}"));
    let mut slugs: Vec<String> = WORLDS.iter().map(|spec| slug_of(spec.name)).collect();
    slugs.sort();
    let listed: Vec<String> = listings.iter().map(|row| row.folder.slug.clone()).collect();
    assert_eq!(
        listed, slugs,
        "load screen: the worlds root holds other worlds"
    );

    let showcase = WORLDS
        .iter()
        .find(|spec| spec.showcase)
        .expect("load screen: one world is the showcase");
    assert_eq!(
        slugs.first(),
        Some(&slug_of(showcase.name)),
        "load screen: '{}' must sort first, so Load selects it",
        showcase.name
    );
    let header = listings
        .iter()
        .find(|row| row.folder.slug == slug_of(showcase.name))
        .and_then(|row| row.header.as_ref().ok())
        .unwrap_or_else(|| panic!("load screen: '{}' has no readable header", showcase.name));
    assert_eq!(
        header.format, WORLD_SAVE_VERSION,
        "load screen: the save format"
    );
    assert_eq!(
        header.fleet.escorts, 1,
        "load screen: the showcase save's escorts"
    );
    assert_eq!(
        header.objectives.len(),
        2,
        "load screen: the showcase save's open objectives: {:?}",
        header.objectives
    );

    for slug in &slugs {
        assert!(
            ui_node_rect(world, &format!("{LOAD_ROW_PREFIX}{slug}")).is_some(),
            "load screen: no laid-out row for '{slug}'"
        );
    }
    assert_eq!(
        count_named(world, "Load World Row Invalid"),
        0,
        "load screen: a row reads Invalid"
    );
    assert_eq!(
        count_named(world, "Load World Row Icon"),
        3,
        "load screen: role icons"
    );
    assert_eq!(
        count_named(world, "Load World Row Play Time"),
        3,
        "load screen: play times"
    );
    assert_eq!(
        count_named(world, "Load World Row Saved"),
        3,
        "load screen: save ages"
    );
    let mut roles: Vec<String> = WORLDS
        .iter()
        .map(|spec| role_name(world, spec.role))
        .collect();
    roles.sort();
    assert_eq!(
        texts_of(world, "Load World Row Status"),
        roles,
        "load screen: the rows' start roles"
    );

    assert_eq!(
        text_of(world, LOAD_DETAILS_NAME).as_deref(),
        Some(showcase.name),
        "load screen: the details show another world"
    );
    let labels = shown_chart_labels(world);
    for label in ["STN-1", "WRK-1", "YOU"] {
        assert!(
            labels.iter().any(|shown| shown == label),
            "load screen: the preview does not show '{label}', it shows {labels:?}"
        );
    }
    assert_eq!(
        detail_value(world, "Load World Details Fleet Ship").as_deref(),
        Some(header.hull.as_str()),
        "load screen: the FLEET block's ship"
    );
    assert_eq!(
        detail_value(world, "Load World Details Fleet Escorts").as_deref(),
        Some("1"),
        "load screen: the FLEET block's escorts"
    );
    assert_eq!(
        count_named(world, "Load World Details Objective"),
        2,
        "load screen: the details' open objectives"
    );
    info!(
        "load screen: rows {slugs:?}, labels {labels:?}, ship '{}', objectives {:?}",
        header.hull,
        header
            .objectives
            .iter()
            .map(|objective| objective.title.as_str())
            .collect::<Vec<_>>()
    );
}

/// The walk: make each world, then Load.
fn load_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "answer the New pilot offer: Not now",
            TRAINING_PROMPT_DISMISS,
            when(|world| {
                *world.resource::<TrainingPromptSetting>() == TrainingPromptSetting::Hidden
                    && ui_node_rect(world, TRAINING_PROMPT).is_none()
            }),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "switch the field note off: Don't show again",
            FIELD_NOTE_DISMISS,
            when(corner_is_answered),
            BEAT_DEADLINE_SECS,
        );
    for index in 0..WORLDS.len() {
        script = play_world(script, index);
    }
    script
        .step("settle the menu with Continue")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "open Load",
            LOAD_BUTTON,
            ui_node_present(LOAD_PREVIEW),
            BEAT_DEADLINE_SECS,
        )
        // Load is greyed and a row reads CHECKING until its whole state
        // checks out.
        .step("every world's state checks out")
        .until(when(|world| {
            world.try_query::<&Name>().is_some_and(|mut names| {
                !names
                    .iter(world)
                    .any(|name| name.as_str() == LOAD_ROW_CHECKING)
            })
        }))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Off the rows, so no hover tint sits on one in the still.
        .step("rest the pointer on the details name")
        .on_enter(hover_named(LOAD_DETAILS_NAME))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("check the rows and the details")
        .on_enter(|world: &mut World| {
            assert_the_load_screen(world);
            hide_dev_overlays(world);
        })
        .add()
        // The news figure ships with the post, so the footer's build id stays
        // out of it.
        .step("capture the Load screen")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            hide_menu_version(world);
            shoot(world, SHOT);
        })
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
