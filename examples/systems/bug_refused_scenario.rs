//! bug_refused_scenario: a scenario the loader refuses ENDS the one it was
//! going to replace.
//!
//! The campaign case, walked over the shipped app: a leg opens with a player
//! ship, an objective, a comms line and a LINGERING chain queued behind it, and
//! the advance key releases that chain onto a scenario whose content the
//! runtime gate refuses.
//!
//! ONE SUBJECT: what a refusal leaves behind. The report is the easy half. The
//! half this range exists for is the scenario that was already running - it
//! must be torn down (no scoped entity, no current scenario, no event world, no
//! objective or comms mirror) before the player is told, and the cursor the
//! flight grab took must come back, because the only thing on screen is a modal
//! with a button on it. Main Menu is the way out, and it works.
//!
//! The fixtures are two small `ScenarioConfig` values built in Rust rather than
//! a broken mod on disk: the refusal has to be a CONTENT error the gate files,
//! and the compiler then holds the fixture to the same grammar the game ships.
//! They are registered in [`RuntimeScenarioSystems`], the ordering handle for a
//! scenario with no content file behind it, so the `--scenario` membership
//! check in the same transition cannot run before the id it resolves exists,
//! and a later re-merge keeps them (a merge replaces only what content
//! publishes).
//!
//! The refusal is the subject, so the loader's own report of it is expected
//! noise and this range clamps that target out of the log. Nothing here reads a
//! log line - every claim is read off the world.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example bug_refused_scenario --features debug
//! # look for: `refused: the live scenario is torn down, not left running`,
//! #           `refused: the cursor is free and Main Menu is the way out`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "bug_refused_scenario")]
#[command(version = "1.0.0")]
#[command(
    about = "A refused next scenario tears the live one down, frees the cursor and offers the menu. Autopilot-only correctness range - play the game to use the menu",
    long_about = None
)]
struct Cli;

/// The live scenario the run plays, and the chain it queues.
#[cfg(feature = "debug")]
const SCENARIO_A: &str = "refused_probe_live";
/// The chain target, registered and BROKEN: its content is refused at start.
#[cfg(feature = "debug")]
const SCENARIO_B: &str = "refused_probe_broken";
/// What [`SCENARIO_B`] chains to, and what nothing registers - the content
/// error the gate files against it.
#[cfg(feature = "debug")]
const MISSING: &str = "refused_probe_missing";

/// The player object [`SCENARIO_A`] spawns.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "player_ship";
/// The objective it posts, mirrored into the HUD list.
#[cfg(feature = "debug")]
const OBJECTIVE_ID: &str = "hold_the_line";
/// The variable it seeds, which lives in the event world.
#[cfg(feature = "debug")]
const LIVE_MARK: &str = "in_probe_a";

/// The report's own nodes, and the front door its way out reaches, by `Name`.
#[cfg(feature = "debug")]
const REPORT_MENU_BUTTON: &str = "Start Failure Menu Button";
/// The panel a load raises. A refused load has nothing to raise it for.
#[cfg(feature = "debug")]
const LOADING_PANEL: &str = "Scenario Loading Screen";
#[cfg(feature = "debug")]
const REPORT_ISSUE: &str = "Start Failure Issue";
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";

/// Seconds a boot, a scenario load or a return to the menu is given. Sized to
/// outlast a software-rendered CI GPU and kept under the harness completion
/// deadline, so a stall names THIS beat.
#[cfg(feature = "debug")]
const BOOT_SECS: f32 = 90.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    #[cfg(feature = "debug")]
    quiet_the_refusal();

    // The binary's `--scenario <id>`: the launch lands on the same
    // OnEnter(Playing) loader a click on Play uses, and the app keeps its menu
    // - which the refusal's one way out has to reach.
    #[cfg(feature = "debug")]
    let startup = Some(StartupScenario::Id(SCENARIO_A.to_string()));
    #[cfg(not(feature = "debug"))]
    let startup = None;

    // The same app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, startup);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            register_probe_scenarios.in_set(RuntimeScenarioSystems),
        );
        // No frame-time claim: the run is a load, a refusal and a menu, and the
        // number that would come out of that window says nothing about the
        // game's speed.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(refused_script());
    }

    app.run()
}

/// Clamp the loader's own refusal report out of the log.
///
/// The refusal is what this range stages, and the probe's log gate cannot tell
/// a staged ERROR from a real one. Nothing here reads a log line, so the target
/// that reports it goes quiet for this process only.
#[cfg(feature = "debug")]
fn quiet_the_refusal() {
    let rust_log = match std::env::var("RUST_LOG") {
        Ok(existing) if !existing.is_empty() => {
            format!("{existing},nova_scenario::loader::lifecycle=off")
        }
        _ => "nova_scenario::loader::lifecycle=off".to_string(),
    };
    std::env::set_var("RUST_LOG", rust_log);
}

/// Register both fixtures into the merged registry.
///
/// The registration is load-bearing three times over: the launch request
/// resolves its id against this resource, a queued `NextScenario` carries only
/// an id and resolves it here too, and the runtime gate lints the chain's
/// target against the same registry - which is what makes [`MISSING`] a content
/// error rather than a silent miss.
#[cfg(feature = "debug")]
fn register_probe_scenarios(
    mut scenarios: ResMut<GameScenarios>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    let live = refused_probe_live(&game_assets, &sections);
    let broken = refused_probe_broken(&game_assets);
    scenarios.insert(live.id.clone(), live);
    scenarios.insert(broken.id.clone(), broken);
}

/// The live scenario: a player ship, an objective, a comms line, a seeded
/// variable, and the LINGERING chain the advance key releases.
///
/// Every one of those is a mirror the refusal has to clear, which is why the
/// fixture carries them: an empty scenario would prove the teardown ran without
/// proving what it reached.
#[cfg(feature = "debug")]
fn refused_probe_live(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                SpaceshipSectionConfig {
                    id: "controller".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("basic_controller_section")),
                },
                SpaceshipSectionConfig {
                    id: "hull".to_string(),
                    position: Vec3::new(0.0, 0.0, 1.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(section("reinforced_hull_section")),
                },
            ],
            ..default()
        }),
        ..default()
    };

    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: vec![
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: PLAYER_ID.to_string(),
                    name: "Player Ship".to_string(),
                    position: Meters3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Spaceship(ship),
            }),
            EventActionConfig::Objective(ObjectiveActionConfig::new(OBJECTIVE_ID, "Hold the line")),
            EventActionConfig::NarrativeCue(NarrativeCueActionConfig {
                channel: "comms".to_string(),
                speaker: "Fleet".to_string(),
                text: "Stand by for the next leg.".to_string(),
                dwell: None,
                icon: None,
            }),
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: LIVE_MARK.to_string(),
                expression: nova_authoring::scenario_helpers::number(1.0),
            }),
            // The chain, queued and waiting for the player: `linger: true` is
            // what makes the advance key - not a timer - the cause of the
            // switch this range walks.
            EventActionConfig::NextScenario(NextScenarioActionConfig {
                scenario_id: SCENARIO_B.to_string(),
                linger: true,
                delay: None,
            }),
        ]
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        .into_iter()
        .chain(ThreePointRig::around("arena", Meters3::ZERO, 2.0).actions())
        .collect(),
    }];

    ScenarioConfig {
        description: "A live leg with a chain queued behind it.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_A.to_string(),
            "Refused Probe: Live".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The chain target, registered and BROKEN: it chains on to a scenario nothing
/// registers, which is an Error-level content finding and therefore a refusal
/// at start rather than a half-spawned scene.
///
/// The shape of a campaign whose next leg was removed with a mod.
#[cfg(feature = "debug")]
fn refused_probe_broken(game_assets: &GameAssets) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: vec![EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id: MISSING.to_string(),
            linger: false,
            delay: None,
        })],
    }];

    ScenarioConfig {
        description: "The leg whose own chain target is gone.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_B.to_string(),
            "Refused Probe: Broken".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn refused_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("refused: the leg opens and is live")
        .until(and(
            state_is(GameStates::Playing),
            and(player_ship_present(), scenario_variable_is(LIVE_MARK, 1.0)),
        ))
        .deadline(BOOT_SECS)
        .add()
        // The guard the whole range rests on: what the refusal has to end is
        // demonstrably running first.
        .step("refused: the leg being left is really there")
        .on_enter(|world: &mut World| {
            assert!(
                scoped_entities(world) > 0,
                "the live leg has scenario-scoped entities to tear down"
            );
            assert!(
                !world.resource::<GameObjectives>().objectives.is_empty(),
                "the live leg posted its objective into the HUD mirror"
            );
            assert!(
                !world.resource::<StoryFeed>().0.is_empty(),
                "the live leg pushed its comms line into the feed"
            );
            assert!(
                world
                    .resource::<NovaEventWorld>()
                    .next_scenario
                    .as_ref()
                    .is_some_and(|next| next.scenario_id == SCENARIO_B),
                "the chain is queued and lingering, waiting on the player"
            );
        })
        .add()
        // The gesture: the advance key releases the queued chain, exactly as a
        // campaign's next leg is taken.
        .step("refused: press Advance")
        .on_enter(press_action("scenario_advance"))
        .add()
        .step("refused: release Advance")
        .on_enter(release_action("scenario_advance"))
        .until(ui_node_present(REPORT_MENU_BUTTON))
        .deadline(BOOT_SECS)
        .add()
        .step("refused: the live leg is gone")
        .on_enter(|world: &mut World| {
            let scoped = scoped_entities(world);
            assert_eq!(
                scoped, 0,
                "a refused scenario tears the old one down: {scoped} scoped entities still live"
            );
            assert_eq!(
                count::<PlayerSpaceshipMarker>(world),
                0,
                "the player ship of the leg that ended must not fly on under the report"
            );
            // Nothing of the old leg is left to tick, and every system the
            // loader gates on `scenario_is_live` is off with it.
            assert!(
                world
                    .get_resource::<CurrentScenario>()
                    .and_then(|current| current.0.as_ref())
                    .is_none(),
                "no scenario is current after a refusal, so `scenario_is_live` is false"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a refused scenario ends the one it replaces",
                serde_json::json!({ "scoped": scoped }),
            );
            info!("refused: the live scenario is torn down, not left running");
        })
        .add()
        .step("refused: the mirrors go with it")
        .on_enter(|world: &mut World| {
            let variables = world.resource::<NovaEventWorld>().variables().count();
            assert_eq!(
                variables, 0,
                "the event world goes with the scenario: {variables} variable(s) survived"
            );
            assert!(
                world.resource::<GameObjectives>().objectives.is_empty(),
                "the objective list belongs to the scenario that posted it"
            );
            assert!(
                world.resource::<StoryFeed>().0.is_empty(),
                "the comms feed belongs to the scenario that spoke"
            );
            assert!(
                world.resource::<NovaEventWorld>().next_scenario.is_none(),
                "the chain that was refused is not still queued"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the refusal clears the scenario mirrors",
                serde_json::json!({ "variables": variables }),
            );
        })
        .add()
        .step("refused: the report says what went wrong")
        .on_enter(|world: &mut World| {
            let issues = named_texts(world, REPORT_ISSUE);
            assert!(
                issues.iter().any(|issue| issue.contains(MISSING)),
                "the report names the content error it refused on: {issues:?}"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the refusal report names the issue",
                serde_json::json!({ "issues": issues }),
            );

            // The LOADING panel is a full-screen node over everything. Left up
            // over a dead end, it hides the report and eats the click that is
            // the only way out of it.
            let panels = named_nodes(world, LOADING_PANEL);
            assert_eq!(
                panels, 0,
                "a refused load raises no loading panel over its own report; {panels} up"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the refusal leaves no loading panel over the report",
                serde_json::json!({ "panels": panels }),
            );

            let (grabbed, visible) = cursor_state(world);
            assert!(
                !grabbed && visible,
                "the report is a modal with a button on it: the flight grab must be \
                 released like the outcome overlay's (grabbed={grabbed}, visible={visible})"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the refusal frees the cursor",
                serde_json::json!({ "visible": visible }),
            );
            info!("refused: the cursor is free and Main Menu is the way out");
        })
        .add()
        // ...and the one road out actually goes there.
        .step("refused: click Main Menu")
        .on_enter(click_named(REPORT_MENU_BUTTON))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("refused: release Main Menu")
        .on_enter(release_mouse(MouseButton::Left))
        .until(and(
            state_is(GameStates::MainMenu),
            ui_node_present(NEW_GAME_BUTTON),
        ))
        .deadline(BOOT_SECS)
        .add()
        .step("refused: the menu is the way out")
        .on_enter(|world: &mut World| {
            nova_probe::probe_marker(
                world,
                "outcome: the refusal report reaches the main menu",
                serde_json::json!({}),
            );
            info!("refused: back at the front door");
        })
        .add()
}

/// How many scenario-scoped entities are alive.
#[cfg(feature = "debug")]
fn scoped_entities(world: &World) -> usize {
    count::<ScenarioScopedMarker>(world)
}

/// How many entities carry `M`.
#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut query| query.iter(world).count())
}

/// How many laid-out nodes are called `name`.
#[cfg(feature = "debug")]
fn named_nodes(world: &mut World, name: &str) -> usize {
    let mut query = world.query_filtered::<&Name, With<Node>>();
    query
        .iter(world)
        .filter(|live| live.as_str() == name)
        .count()
}

/// Every `Text` on a node named `name`.
#[cfg(feature = "debug")]
fn named_texts(world: &mut World, name: &str) -> Vec<String> {
    let mut query = world.query::<(&Name, &Text)>();
    query
        .iter(world)
        .filter(|(live, _)| live.as_str() == name)
        .map(|(_, text)| text.0.clone())
        .collect()
}

/// The primary window's cursor: whether it is grabbed, and whether it is drawn.
#[cfg(feature = "debug")]
fn cursor_state(world: &mut World) -> (bool, bool) {
    let mut query =
        world.query_filtered::<&bevy::window::CursorOptions, With<bevy::window::PrimaryWindow>>();
    query
        .iter(world)
        .next()
        .map(|cursor| {
            (
                cursor.grab_mode != bevy::window::CursorGrabMode::None,
                cursor.visible,
            )
        })
        .expect("a rendered run has a primary window")
}
