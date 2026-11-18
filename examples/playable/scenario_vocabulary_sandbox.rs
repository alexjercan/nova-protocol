//! scenario_vocabulary_sandbox: play the open world under a scenario that
//! answers each job, stance, sector and refusal event with a comms line, and
//! trigger each one by hand.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, without a save folder. New Game plays
//! `scenario_vocabulary_sandbox`: the shipped open world under its own id
//! with the handlers of this sandbox at the end. The handlers pass the
//! content lint at start, or the run stops. Each handler says a comms line
//! whose speaker is the block that ran, and counts in a variable the panel
//! reads:
//!
//! | block | how to trigger it |
//! | - | - |
//! | `OnDocked`: `PostJob`, `GiveItems` | the first dock, which the start makes: a job for 3 Rations at 40 cr is on the board and 3 Rations are in the hold |
//! | `OnJobAccepted` | TAB > Jobs, Accept a job |
//! | `OnJobHandedIn`, `GrantCredits` | Hand in: the scenario's own job (`event.post_key`) pays 25 cr more, once |
//! | `OnJobDropped`, `event.cause` | accept another job, select it in Your jobs, Drop: `Abandoned`. A job the world takes reads `Lost` |
//! | `OnJobDone` | do the work of a located job (recover, escort, distress, neutralize). A procurement has no such step |
//! | `OnUndocked`: `TakeItems`, then `OnActionRefused` | undock: the scenario asks for 1000 Rations and is refused `short_items` |
//! | `SetStance`, then `OnStanceChanged` | the first undock after a hand-in asks for Neutral with the station's civilization, which starts Ally; the next dock asks for Ally again |
//! | `OnSectorEntered` | fly across a sector face; `--start edge` starts 300 m from one |
//!
//! `--start docked` (default) leaves the player docked at the starting
//! station. `--start edge` undocks, which fires the refusal, and jumps the
//! player 300 m inside the nearest face of its sector, at rest, bow on the
//! face: burn ahead and the sector notice and the `OnSectorEntered` line
//! show.
//!
//! ```text
//! nix develop --command cargo run --example scenario_vocabulary_sandbox --features debug
//! nix develop --command cargo run --example scenario_vocabulary_sandbox --features debug -- --start edge
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging and checks it. Docked, the posted job stands
//! on the board with its goods in the hold; the walk then accepts it and
//! hands it in by the messages the Jobs tab writes, undocks, and reads each
//! counter and the stance. From the edge, the refusal is counted and a jump
//! across the face counts one sector. `NOVA_CAPTURE=1` shoots
//! `scenario-vocabulary-sandbox.png` and, docked,
//! `scenario-vocabulary-sandbox-chain.png`.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use std::collections::HashSet;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{
    nova_debug::diagnostics,
    nova_interface::jobs::{JobAcceptCommand, JobHandInCommand},
    prelude::*,
};
use nova_world::prelude::{WorldClock, WorldConfig};
use open_world_stage::OpenWorldStartPlugin;

/// Where the run starts.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxStartType {
    /// Docked at the starting station, for the job blocks.
    #[default]
    Docked,
    /// In flight, 300 m inside a sector face, for `OnSectorEntered`.
    Edge,
}

impl SandboxStartType {
    /// The intro line.
    fn intro(self) -> &'static str {
        match self {
            Self::Docked => {
                "A scenario answers each job step in comms: Tab, Jobs, Accept, Hand in, undock."
            }
            Self::Edge => "A sector face is 300 m ahead: burn across it and read the comms.",
        }
    }
}

#[derive(Parser, Resource, Clone, Copy)]
#[command(name = "scenario_vocabulary_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Play the open world under a scenario that answers each job, stance, sector and \
             refusal event with a comms line, and trigger each one by hand",
    long_about = None
)]
struct Cli {
    /// Where the run starts.
    #[arg(long, value_enum, default_value_t = SandboxStartType::Docked)]
    start: SandboxStartType,
}

/// The world seed and the start role: `system_job_distress`'s, whose
/// starting station posts a board.
const SANDBOX_SEED: u32 = 16;
const SANDBOX_ROLE: &str = "bounty_hunter";
/// The scenario New Game plays: the open world plus this sandbox's handlers.
const FIXTURE_SCENARIO_ID: &str = "scenario_vocabulary_sandbox";
/// The player's ship, by the id the open world spawns it under.
const PLAYER_ID: &str = "player";
/// The key the scenario posts its job under.
const POST_KEY: &str = "sandbox";
/// The goods the posted job asks for, how many, and how many `GiveItems`
/// puts aboard.
const GOODS: &str = "Rations";
const GOODS_ASKED: u32 = 3;
/// What the posted job pays, in credits.
const POST_REWARD_CR: u32 = 40;
/// What the hand-in of the posted job grants over its reward, in credits.
const HAND_IN_GRANT_CR: i32 = 25;
/// What the undock asks for: more than any hold carries.
const GOODS_TAKEN: u32 = 1_000;
/// The stance the undock after a hand-in asks for. The start role's
/// civilization starts Ally, and a Neutral station still docks and trades.
/// The next dock asks for [`STANCE_HOME`] again.
const STANCE_ASKED: StanceType = StanceType::Neutral;
const STANCE_HOME: StanceType = StanceType::Ally;
/// Seconds a line of this sandbox stays in comms.
const CUE_DWELL_SECS: f32 = 10.0;
/// How far inside the sector face `--start edge` puts the player.
const EDGE_STANDOFF: Meters = Meters(300.0);

/// The counters the handlers keep: the variable, the block it counts and
/// how to trigger the block by hand.
const BLOCKS: [(&str, &str, &str); 10] = [
    (
        "posted",
        "OnDocked: PostJob, GiveItems",
        "the first dock, made by the start",
    ),
    ("accepted", "OnJobAccepted", "TAB > Jobs > Accept"),
    ("done", "OnJobDone", "do the work of a located job"),
    ("handed_in", "OnJobHandedIn", "TAB > Jobs > Hand in"),
    ("granted", "GrantCredits", "hand in the Rations job"),
    ("dropped", "OnJobDropped", "accept a job, select it, Drop"),
    (
        "refused",
        "OnUndocked: TakeItems > OnActionRefused",
        "undock",
    ),
    (
        "stance_asked",
        "SetStance",
        "undock after a hand-in, then dock",
    ),
    ("stance_changed", "OnStanceChanged", "follows SetStance"),
    ("sectors", "OnSectorEntered", "fly across a sector face"),
];

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.insert_resource(cli);
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(cli.start.intro()));
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        register_the_scenario.in_set(RuntimeScenarioSystems),
    );
    app.add_systems(
        Update,
        (
            pin_new_game_start,
            drive_the_sandbox.run_if(in_state(GameStates::Playing)),
        ),
    );
    app.add_systems(
        PostUpdate,
        draw_the_blocks
            .in_set(DiagnosticsSystems)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script(cli.start));

    app.run()
}

/// Keep the declared New Game start on the sandbox's scenario: the bundle
/// merge writes the base bundle's start by command at content load.
fn pin_new_game_start(mut start: ResMut<NewGameStart>) {
    if start.0.as_deref() != Some(FIXTURE_SCENARIO_ID) {
        start.0 = Some(FIXTURE_SCENARIO_ID.to_string());
    }
}

fn expression(text: &str) -> VariableExpressionNode {
    text.parse().unwrap_or_else(|error| {
        panic!("scenario_vocabulary_sandbox: '{text}' is no expression: {error}")
    })
}

/// `key = <text>`.
fn set(key: &str, text: &str) -> EventActionConfig {
    EventActionConfig::VariableSet(VariableSetActionConfig {
        key: key.to_string(),
        expression: expression(text),
    })
}

/// `key = key + 1`.
fn count(key: &str) -> EventActionConfig {
    set(key, &format!("{key} + 1"))
}

fn only_if(text: &str) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(text.parse().unwrap_or_else(
        |error| panic!("scenario_vocabulary_sandbox: '{text}' is no condition: {error}"),
    )))
}

/// The player as the ship that docks or undocks: a generated trader docks
/// too.
fn by_the_player() -> EventFilterConfig {
    only_if(&format!("event.other_id == \"{PLAYER_ID}\""))
}

/// A comms line whose speaker is the block that ran.
fn say(block: &str, text: &str) -> EventActionConfig {
    EventActionConfig::NarrativeCue(NarrativeCueActionConfig {
        accent: default_comms_accent(),
        speaker: block.to_string(),
        text: text.to_string(),
        dwell: Some(CUE_DWELL_SECS),
        icon: None,
    })
}

fn handler(
    label: &str,
    name: EventConfig,
    once: bool,
    filters: Vec<EventFilterConfig>,
    actions: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    ScenarioEventConfig {
        label: Some(label.to_string()),
        position: None,
        name,
        once,
        filters,
        actions,
    }
}

/// The handlers of the sandbox, in the order the scenario lists them.
fn the_handlers() -> Vec<ScenarioEventConfig> {
    let goods = ItemDesignId::from(GOODS);
    let mut handlers = vec![
        handler(
            "post a job and give its goods on the first dock",
            EventConfig::OnDocked,
            true,
            vec![by_the_player()],
            vec![
                // Under `OnDocked` the hull docked with is `id`.
                EventActionConfig::PostJob(PostJobActionConfig {
                    station: "$id".to_string(),
                    key: POST_KEY.to_string(),
                    item: goods.clone(),
                    count: GOODS_ASKED,
                    reward_cr: POST_REWARD_CR,
                }),
                EventActionConfig::GiveItems(GiveItemsActionConfig {
                    ship: PLAYER_ID.to_string(),
                    item: goods.clone(),
                    count: GOODS_ASKED,
                }),
                count("posted"),
                say(
                    "PostJob",
                    "A job for 3 Rations at 40 cr is on this board. GiveItems put 3 Rations in \
                     your hold.",
                ),
            ],
        ),
        handler(
            "answer an accept",
            EventConfig::OnJobAccepted,
            false,
            vec![],
            vec![
                count("accepted"),
                say("OnJobAccepted", "The scenario saw you accept a job."),
            ],
        ),
        handler(
            "answer a done",
            EventConfig::OnJobDone,
            false,
            vec![],
            vec![
                count("done"),
                say("OnJobDone", "The work of a located job is done."),
            ],
        ),
        handler(
            "answer a hand-in",
            EventConfig::OnJobHandedIn,
            false,
            vec![],
            vec![
                count("handed_in"),
                say("OnJobHandedIn", "The scenario saw you hand a job in."),
            ],
        ),
        handler(
            "grant on the hand-in of the posted job",
            EventConfig::OnJobHandedIn,
            true,
            vec![only_if(&format!("event.post_key == \"{POST_KEY}\""))],
            vec![
                EventActionConfig::GrantCredits(GrantCreditsActionConfig {
                    ship: PLAYER_ID.to_string(),
                    amount: HAND_IN_GRANT_CR,
                }),
                count("granted"),
                say(
                    "GrantCredits",
                    "25 cr more for the job the scenario posted: event.post_key is sandbox.",
                ),
            ],
        ),
        handler(
            "take more than the hold carries on the undock",
            EventConfig::OnUndocked,
            false,
            vec![by_the_player()],
            vec![EventActionConfig::TakeItems(TakeItemsActionConfig {
                ship: PLAYER_ID.to_string(),
                item: goods,
                count: GOODS_TAKEN,
            })],
        ),
        handler(
            "answer the refused take",
            EventConfig::OnActionRefused,
            false,
            vec![
                only_if("event.action == \"TakeItems\""),
                only_if("event.reason == \"short_items\""),
            ],
            vec![
                count("refused"),
                say(
                    "OnActionRefused",
                    "TakeItems was refused as short_items: the undock asked for 1000 Rations.",
                ),
            ],
        ),
        handler(
            "ask for a stance on the undock after a hand-in",
            EventConfig::OnUndocked,
            true,
            vec![by_the_player(), only_if("handed_in > 0")],
            vec![
                // Under `OnUndocked` the hull let go of is `id`.
                EventActionConfig::SetStance(SetStanceActionConfig {
                    body: "$id".to_string(),
                    stance: STANCE_ASKED,
                }),
                count("stance_asked"),
                say(
                    "SetStance",
                    "After a hand-in the scenario asks for Neutral with this station's \
                     civilization. Dock again and it asks for Ally.",
                ),
            ],
        ),
        handler(
            "ask for the first stance again on the next dock",
            EventConfig::OnDocked,
            true,
            vec![by_the_player(), only_if("stance_asked > 0")],
            vec![
                EventActionConfig::SetStance(SetStanceActionConfig {
                    body: "$id".to_string(),
                    stance: STANCE_HOME,
                }),
                count("stance_asked"),
                say(
                    "SetStance",
                    "The scenario asks for Ally with this station's civilization again.",
                ),
            ],
        ),
        handler(
            "answer a sector crossing",
            EventConfig::OnSectorEntered,
            false,
            vec![],
            vec![
                count("sectors"),
                say("OnSectorEntered", "You crossed into another sector."),
            ],
        ),
    ];
    for (cause, text) in [
        (
            "Abandoned",
            "event.cause is Abandoned: you dropped the job.",
        ),
        ("Lost", "event.cause is Lost: the world took the job."),
    ] {
        handlers.push(handler(
            &format!("answer a job dropped as {cause}"),
            EventConfig::OnJobDropped,
            false,
            vec![only_if(&format!("event.cause == \"{cause}\""))],
            vec![count("dropped"), say("OnJobDropped", text)],
        ));
    }
    for stance in ["Ally", "Neutral", "Enemy"] {
        handlers.push(handler(
            &format!("answer a stance that moved to {stance}"),
            EventConfig::OnStanceChanged,
            false,
            vec![only_if(&format!("event.to == \"{stance}\""))],
            vec![
                count("stance_changed"),
                say("OnStanceChanged", &format!("event.to is {stance}.")),
            ],
        ));
    }
    handlers
}

/// Register the sandbox's scenario: the shipped open world under its own
/// id, its `OnStart` seeding the counters, and the handlers at the end.
///
/// # Panics
///
/// When the scenario fails the content lint.
fn register_the_scenario(
    mut scenarios: ResMut<GameScenarios>,
    sections: Res<GameSections>,
    ships: Res<GameShipDesigns>,
    items: Res<GameItems>,
    styles: Res<GameStyles>,
    profiles: Res<GameTradeProfiles>,
) {
    let mut scenario = scenarios
        .get(OPEN_WORLD_SCENARIO_ID)
        .cloned()
        .expect("scenario_vocabulary_sandbox: the base bundle registers the open world");
    scenario.id = FIXTURE_SCENARIO_ID.to_string();
    scenario.name = "Scenario Vocabulary Sandbox".to_string();
    let start = scenario
        .events
        .iter_mut()
        .find(|event| event.name == EventConfig::OnStart)
        .expect("scenario_vocabulary_sandbox: the open world opens with OnStart");
    for (key, ..) in BLOCKS {
        start.actions.push(set(key, "0"));
    }
    scenario.events.extend(the_handlers());

    let issues = lint_scenario(
        &scenario,
        &KnownSections::from_configs(sections.iter()),
        &KnownShipDesigns::from_configs(ships.iter()),
        &KnownStationDesigns::default(),
        &HashSet::from([scenario.id.clone()]),
        &items,
        &styles,
        &profiles,
    );
    let errors: Vec<&str> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "scenario_vocabulary_sandbox: the scenario failed the content lint:\n  {}",
        errors.join("\n  ")
    );
    info!(
        "scenario_vocabulary_sandbox: the scenario passed the content lint with {} handler(s)",
        scenario.events.len()
    );
    scenarios.insert(FIXTURE_SCENARIO_ID.to_string(), scenario);
}

/// Where the staging stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The start's window is streaming in and the start is docking the
    /// player at its starting station.
    #[default]
    Streaming,
    /// `--start edge`: the player let go of the starting station.
    Leaving,
    /// The player has the run.
    Ready,
}

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
}

/// The player's cell, the axis of its nearest face, the side of the cell
/// the face stands on, and the distance to the face.
fn nearest_face(at: Meters3, edge: Meters) -> (SectorCoord, usize, f32, Meters) {
    let cell = SectorCoord::containing(at, edge);
    let local = at.get() - cell.centre(edge).get();
    let half = edge.get() / 2.0;
    let (axis, gap) = (0..3)
        .map(|axis| (axis, half - local[axis].abs()))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("three axes");
    let side = if local[axis] < 0.0 { -1.0 } else { 1.0 };
    (cell, axis, side, Meters(gap))
}

/// Jump the player [`EDGE_STANDOFF`] inside the nearest face of its sector,
/// at rest, bow on the face.
fn jump_to_the_edge(world: &mut World, player: Entity) {
    let edge = open_world_stage::sector_edge(world)
        .expect("scenario_vocabulary_sandbox: the world is armed");
    let at = Meters3::from_engine(
        world
            .get::<Position>(player)
            .expect("scenario_vocabulary_sandbox: the player has a physics position")
            .0,
    );
    let (cell, axis, side, gap) = nearest_face(at, edge);
    let mut out = Vec3::ZERO;
    out[axis] = side;
    let target = Meters3(at.get() + out * (gap.0 - EDGE_STANDOFF.0));
    open_world_stage::jump_facing(world, player, target, out);
    info!(
        "scenario_vocabulary_sandbox: the player stands {} m inside a face of sector {cell}",
        EDGE_STANDOFF.0
    );
}

/// Advance the staging one phase at a time.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let docked = world.get::<DockedShip>(player).is_some();
    let next = match phase {
        SandboxPhaseType::Ready => return,
        SandboxPhaseType::Streaming => {
            if !(open_world_stage::window_settled(world) && docked) {
                return;
            }
            match world.resource::<Cli>().start {
                SandboxStartType::Docked => {
                    info!("scenario_vocabulary_sandbox: docked at the starting station");
                    SandboxPhaseType::Ready
                }
                SandboxStartType::Edge => {
                    world.trigger(DockingReleaseRequest { entity: player });
                    SandboxPhaseType::Leaving
                }
            }
        }
        SandboxPhaseType::Leaving => {
            if docked {
                return;
            }
            jump_to_the_edge(world, player);
            SandboxPhaseType::Ready
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// How often the block counted in `key` ran.
fn counted(events: &NovaEventWorld, key: &str) -> u32 {
    match events.get_variable(key) {
        Some(VariableLiteral::Number(value)) => *value as u32,
        _ => 0,
    }
}

/// The panel: one line per block with its count and its trigger, and the
/// distance to the nearest sector face.
fn draw_the_blocks(
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    events: Res<NovaEventWorld>,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<(&GlobalTransform, Option<&DockedShip>), With<PlayerSpaceshipMarker>>,
) {
    if sandbox.phase != SandboxPhaseType::Ready {
        panel.line(format!("Staging the sandbox: {:?}", sandbox.phase));
        return;
    }
    let Ok((at, docked)) = q_player.single() else {
        return;
    };
    let interface = pause.is_some_and(|pause| *pause.get() == PauseStates::Interface);
    // The interface draws its panes where the table stands.
    if !interface {
        for (key, block, how) in BLOCKS {
            panel.line(format!("x{} {block}  ({how})", counted(&events, key)));
        }
    }
    if let Some(config) = config {
        let (cell, _, _, gap) =
            nearest_face(Meters3::from_engine(at.translation()), config.sector_edge);
        panel.line(format!("Sector {cell}, nearest face {:.0} m", gap.0));
    }
    if interface {
        panel.hint("Jobs", "Accept, Hand in, select a held job and Drop");
    } else if docked.is_some() {
        panel.hint("TAB", "Jobs > Accept > Hand in");
        panel.hint("D", "undock: the refused TakeItems");
    } else {
        panel.hint("W", "burn across the sector face");
        panel.hint("Ctrl", "hold on the station to lock it, then D to dock");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS};

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 800.0;
    /// Seconds the sector tracker gets to see the jump across the face.
    const BEAT_SECS: f32 = 120.0;
    /// Frames the scenario gets to drain its events.
    const SETTLE_FRAMES: u32 = 10;

    /// The frames the capture path writes: the staging, and the comms after
    /// the docked walk's undock.
    const SHOT: &str = "scenario-vocabulary-sandbox.png";
    const CHAIN_SHOT: &str = "scenario-vocabulary-sandbox-chain.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn block(world: &World, key: &str) -> u32 {
        counted(world.resource::<NovaEventWorld>(), key)
    }

    /// The docked staging: the first dock posted the job on the starting
    /// station's board and gave its goods, and the station's civilization
    /// has the stance the scenario sets back, so both `SetStance` asks move
    /// it.
    fn assert_the_docked_start(world: &mut World) {
        let player =
            open_world_stage::the_player(world).expect("scenario_vocabulary_sandbox: a player");
        assert!(
            world.get::<DockedShip>(player).is_some(),
            "scenario_vocabulary_sandbox: the player starts docked"
        );
        assert_eq!(
            block(world, "posted"),
            1,
            "scenario_vocabulary_sandbox: the first dock ran the PostJob handler once"
        );
        let start = world.resource::<OpenWorldSession>().start.id.clone();
        let station = open_world_stage::body_by_id(world, &start)
            .expect("scenario_vocabulary_sandbox: the starting station stands");
        let goods = ItemDesignId::from(GOODS);
        let board = world
            .get::<JobBoard>(station)
            .expect("scenario_vocabulary_sandbox: the starting station posts a board");
        let posted = board.0.iter().any(|job| {
            job.reward.total_cr() == POST_REWARD_CR
                && matches!(
                    &job.kind,
                    JobType::Procure { item, count } if *item == goods && *count == GOODS_ASKED
                )
        });
        assert!(
            posted,
            "scenario_vocabulary_sandbox: the board lists the posted job: {board:?}"
        );
        let held = world
            .get::<ShipInventory>(player)
            .expect("scenario_vocabulary_sandbox: the player carries a hold")
            .count(&goods);
        assert!(
            held >= GOODS_ASKED,
            "scenario_vocabulary_sandbox: the hold fills the posted job: {held} {GOODS}"
        );
        let civ = world
            .get::<TraderMarket>(station)
            .expect("scenario_vocabulary_sandbox: the starting station prices goods")
            .civilization;
        let now = world.resource::<WorldClock>().play_seconds;
        let stance =
            world
                .resource::<CivilizationStances>()
                .effective(&civ, seeded_stance(&civ), now);
        assert_eq!(
            stance,
            STANCE_HOME,
            "scenario_vocabulary_sandbox: '{}' starts at the stance the scenario sets back",
            civ.name()
        );
        info!(
            "scenario_vocabulary_sandbox: PASS docked with the posted job on the board, {held} \
             {GOODS} aboard, '{}' at {stance:?}",
            civ.name()
        );
    }

    /// The starting station.
    fn the_start_station(world: &mut World) -> Entity {
        let start = world.resource::<OpenWorldSession>().start.id.clone();
        open_world_stage::body_by_id(world, &start)
            .expect("scenario_vocabulary_sandbox: the starting station stands")
    }

    /// The job the scenario posted on the starting station's board.
    fn the_posted_job(world: &mut World) -> Job {
        let station = the_start_station(world);
        let goods = ItemDesignId::from(GOODS);
        let job = world
            .get::<JobBoard>(station)
            .and_then(|board| {
                board.0.iter().find(|job| {
                    job.reward.total_cr() == POST_REWARD_CR
                        && matches!(
                            &job.kind,
                            JobType::Procure { item, count }
                                if *item == goods && *count == GOODS_ASKED
                        )
                })
            })
            .cloned()
            .expect("scenario_vocabulary_sandbox: the board lists the posted job");
        job
    }

    fn credits(world: &mut World) -> u32 {
        let player =
            open_world_stage::the_player(world).expect("scenario_vocabulary_sandbox: a player");
        world
            .get::<ShipCredits>(player)
            .expect("scenario_vocabulary_sandbox: the player has a balance")
            .0
    }

    /// The balance before the walk's hand-in.
    #[derive(Resource)]
    struct BalanceBefore(u32);

    /// After the accept, the hand-in and the undock: each block ran once,
    /// the hand-in paid the reward and the grant, and the stance moved.
    fn assert_the_chain(world: &mut World) {
        let counts: Vec<(&str, u32)> = BLOCKS
            .iter()
            .map(|(key, ..)| (*key, block(world, key)))
            .collect();
        let wanted = [
            ("posted", 1),
            ("accepted", 1),
            ("done", 0),
            ("handed_in", 1),
            ("granted", 1),
            ("dropped", 0),
            ("refused", 1),
            ("stance_asked", 1),
            ("stance_changed", 1),
            ("sectors", 0),
        ];
        assert_eq!(counts, wanted, "scenario_vocabulary_sandbox: the counters");
        let station = the_start_station(world);
        let civ = world
            .get::<TraderMarket>(station)
            .expect("scenario_vocabulary_sandbox: the starting station prices goods")
            .civilization;
        let now = world.resource::<WorldClock>().play_seconds;
        let stance =
            world
                .resource::<CivilizationStances>()
                .effective(&civ, seeded_stance(&civ), now);
        assert_eq!(
            stance, STANCE_ASKED,
            "scenario_vocabulary_sandbox: SetStance moved the stance"
        );
        let before = world.resource::<BalanceBefore>().0;
        let paid = credits(world) - before;
        assert!(
            paid >= POST_REWARD_CR + HAND_IN_GRANT_CR as u32,
            "scenario_vocabulary_sandbox: the hand-in paid the reward and the grant: {paid} cr"
        );
        info!(
            "scenario_vocabulary_sandbox: PASS accept, hand-in (+{paid} cr), refused take and \
             SetStance to {stance:?} each ran once"
        );
    }

    /// The docked walk past the staging: accept and hand in the posted job
    /// by the messages the Jobs tab writes, undock, and read the counters.
    fn walk_the_chain(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("scenario vocabulary sandbox: accept the posted job")
            .on_enter(|world: &mut World| {
                let job = the_posted_job(world);
                let before = credits(world);
                world.insert_resource(BalanceBefore(before));
                world.write_message(JobAcceptCommand { id: job.id });
            })
            .until(when(|world| block(world, "accepted") == 1))
            .deadline(BEAT_SECS)
            .add()
            .step("scenario vocabulary sandbox: hand the posted job in")
            .on_enter(|world: &mut World| {
                let job = world
                    .resource::<Jobs>()
                    .held()
                    .first()
                    .cloned()
                    .expect("scenario_vocabulary_sandbox: the posted job is held");
                world.write_message(JobHandInCommand { id: job.id });
            })
            .until(when(|world| {
                block(world, "handed_in") == 1 && block(world, "granted") == 1
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("scenario vocabulary sandbox: undock")
            .on_enter(|world: &mut World| {
                let player = open_world_stage::the_player(world)
                    .expect("scenario_vocabulary_sandbox: a player");
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(when(|world| {
                block(world, "refused") == 1 && block(world, "stance_changed") == 1
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("scenario vocabulary sandbox: check the counters")
            .on_enter(assert_the_chain)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("scenario vocabulary sandbox: shoot the comms")
            .on_enter(|world: &mut World| shoot(world, CHAIN_SHOT))
            .until(shot_written(CHAIN_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }

    /// The edge staging: the undock was refused its take, and the player
    /// stands free [`EDGE_STANDOFF`] inside a sector face.
    fn assert_the_edge_start(world: &mut World) {
        let player =
            open_world_stage::the_player(world).expect("scenario_vocabulary_sandbox: a player");
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "scenario_vocabulary_sandbox: the player starts free"
        );
        assert_eq!(
            (block(world, "posted"), block(world, "refused")),
            (1, 1),
            "scenario_vocabulary_sandbox: the dock posted once and the undock was refused once"
        );
        let edge = open_world_stage::sector_edge(world)
            .expect("scenario_vocabulary_sandbox: the world is armed");
        let at = Meters3::from_engine(world.get::<Position>(player).expect("a position").0);
        let (cell, _, _, gap) = nearest_face(at, edge);
        assert!(
            (gap.0 - EDGE_STANDOFF.0).abs() < 20.0,
            "scenario_vocabulary_sandbox: the player stands {} m inside a face: {:.0} m",
            EDGE_STANDOFF.0,
            gap.0
        );
        assert_eq!(
            block(world, "sectors"),
            0,
            "scenario_vocabulary_sandbox: no sector is entered before the flight"
        );
        info!(
            "scenario_vocabulary_sandbox: PASS free {:.0} m inside a face of sector {cell}, the \
             take refused once",
            gap.0
        );
    }

    /// Jump the player across the face it stands at. A hand run flies it.
    fn jump_across_the_face(world: &mut World) {
        let player =
            open_world_stage::the_player(world).expect("scenario_vocabulary_sandbox: a player");
        let edge = open_world_stage::sector_edge(world)
            .expect("scenario_vocabulary_sandbox: the world is armed");
        let at = Meters3::from_engine(world.get::<Position>(player).expect("a position").0);
        let (_, axis, side, gap) = nearest_face(at, edge);
        let mut out = Vec3::ZERO;
        out[axis] = side;
        open_world_stage::jump_to(
            world,
            player,
            Meters3(at.get() + out * (gap.0 + EDGE_STANDOFF.0)),
        );
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script(start: SandboxStartType) -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("scenario vocabulary sandbox: start and stage")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("scenario vocabulary sandbox: settle the staging")
            .until(frames(SETTLE_FRAMES))
            .add();
        let shoot_the_view = |script: AutopilotPlugin<GameStates>| {
            script
                .step("scenario vocabulary sandbox: shoot the view")
                .on_enter(|world: &mut World| shoot(world, SHOT))
                .until(shot_written(SHOT))
                .deadline(SHOT_DEADLINE_SECS)
                .add()
        };
        match start {
            SandboxStartType::Docked => walk_the_chain(shoot_the_view(
                script
                    .step("scenario vocabulary sandbox: check the docked start")
                    .on_enter(assert_the_docked_start)
                    .until(frames(SETTLE_FRAMES))
                    .add(),
            )),
            SandboxStartType::Edge => shoot_the_view(
                script
                    .step("scenario vocabulary sandbox: check the edge start")
                    .on_enter(assert_the_edge_start)
                    .until(frames(SETTLE_FRAMES))
                    .add()
                    .step("scenario vocabulary sandbox: jump across the face")
                    .on_enter(jump_across_the_face)
                    .until(when(|world| block(world, "sectors") == 1))
                    .deadline(BEAT_SECS)
                    .add()
                    .step("scenario vocabulary sandbox: log the crossing")
                    .on_enter(|_: &mut World| {
                        info!(
                            "scenario_vocabulary_sandbox: PASS the jump across the face counted \
                             one sector"
                        );
                    })
                    .until(frames(SETTLE_FRAMES))
                    .add(),
            ),
        }
    }
}
