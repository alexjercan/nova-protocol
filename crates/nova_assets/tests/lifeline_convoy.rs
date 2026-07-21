//! Production-faithful behavior + layout rig for Lifeline, chapter three's
//! convoy defense (task 20260721-160957, spike tasks/20260721-155249/
//! SPIKE.md). Loads the ACTUAL shipped `lifeline.content.ron`, registers
//! its real handlers the way the loader does, and drives the act machine
//! with the same event infos the engine emits - plus computed layout pins
//! over the shipped spawns, so the fairness constraints cannot silently
//! rot:
//!
//! 1. the convoy is the ally mechanism in shipped form: `controller: None`
//!    haulers with `allegiance: Some(Player)` (targetable, cannot chase);
//! 2. every raider spawn sits outside the threat envelope's design floor
//!    of EVERY friendly anchor (player spawn and both haulers) - the
//!    telegraphed-arrival contract;
//! 3. waves stage on clock AND previous-wave clears (no stacking);
//! 4. the relief bell wins with the convoy alive; the early clear wins
//!    sooner; both banner variants track the convoy's fate;
//! 5. losing BOTH haulers or the player is a Defeat that retries THIS
//!    scenario (the checkpoint contract).
//!
//! Harness mirrors broadside_assault.rs (same slice: handlers + the
//! per-frame OnUpdate pulse; `cargo test -p nova_assets --test
//! lifeline_convoy`).

use bevy::{ecs::system::RunSystemOnce, math::Vec3, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    EntityId, OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::Allegiance;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const LIFELINE_RON: &str = include_str!("../../../assets/base/scenarios/lifeline.content.ron");
const BASE_BUNDLE_RON: &str = include_str!("../../../assets/base/base.bundle.ron");

/// The design floor (u) between any triggered raider spawn and every
/// friendly anchor. The true threat envelopes are derived by `content
/// lint`'s balance audit (which runs in CI and is clean); this pin holds
/// the authored MARGIN so a nearer respawn shows up as a failing test, not
/// only as a lint warning to be acked away.
const RAIDER_SPAWN_MIN_RANGE: f32 = 700.0;

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

fn on_start(scenario: &ScenarioConfig) -> &ScenarioEventConfig {
    scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("has OnStart")
}

fn spawns(event: &ScenarioEventConfig) -> Vec<&ScenarioObjectConfig> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) => Some(config),
            _ => None,
        })
        .collect()
}

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a ScenarioObjectConfig {
    spawns(event)
        .into_iter()
        .find(|s| s.base.id == id)
        .unwrap_or_else(|| panic!("OnStart spawns '{id}'"))
}

/// Every spaceship spawned by ANY handler, with the handler's index -
/// OnStart ships and the triggered raider waves alike.
fn all_ship_spawns(scenario: &ScenarioConfig) -> Vec<(&str, Vec3, &SpaceshipConfig)> {
    scenario
        .events
        .iter()
        .flat_map(|e| e.actions.iter())
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(s) => match &s.kind {
                ScenarioObjectKind::Spaceship(ship) => {
                    Some((s.base.id.as_str(), s.base.position, ship))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

// --- app harness (mirrors broadside_assault.rs) -----------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });
    app
}

fn seed_var(app: &mut App, key: &str, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(key.to_string(), VariableLiteral::Number(value));
}

fn register_non_start_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        let mut handler = EventHandler::<NovaEventWorld>::from(event.name);
        for filter in event.filters.iter() {
            handler.add_filter(filter.clone());
        }
        for action in event.actions.iter() {
            handler.add_action(action.clone());
        }
        app.world_mut().spawn(handler);
    }
}

/// Seed the whole OnStart variable block the way OnStart would (the rig
/// registers only the non-start handlers, mirroring broadside_assault.rs).
fn seed_live_defense(app: &mut App) {
    for (key, value) in [
        ("act", 1.0),
        ("queen_down", 0.0),
        ("meridian_down", 0.0),
        ("w1_up", 0.0),
        ("w2_up", 0.0),
        ("w3_up", 0.0),
        ("r1a_down", 0.0),
        ("r1b_down", 0.0),
        ("r2a_down", 0.0),
        ("r2b_down", 0.0),
        ("r2c_down", 0.0),
        ("r3a_down", 0.0),
        ("r3b_down", 0.0),
        ("hello_said", 0.0),
        ("w1_clear_said", 0.0),
        ("w2_clear_said", 0.0),
        ("relief_remaining", 240.0),
        ("scenario_elapsed", 0.0),
    ] {
        seed_var(app, key, value);
    }
}

fn destroy(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDestroyedEvent>(info.clone());
        })
        .expect("fire OnDestroyed");
    app.update();
    app.update();
}

fn pump(app: &mut App) {
    app.update();
    app.update();
}

fn number_var(app: &App, key: &str) -> Option<f64> {
    match app.world().resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

fn outcome_kind(app: &App) -> Option<ScenarioOutcomeKind> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .map(|outcome| outcome.outcome)
}

fn outcome_message(app: &App) -> String {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .and_then(|outcome| outcome.message.clone())
        .unwrap_or_default()
}

fn ship_in_world(app: &mut App, id: &str) -> bool {
    let mut q = app.world_mut().query::<&EntityId>();
    q.iter(app.world()).any(|e| **e == *id)
}

// --- the stage --------------------------------------------------------------

/// OnStart stages the whole lane: the ally-mechanism convoy (None
/// controller + Player allegiance - targetable, cannot chase), the frame
/// beacons, the two cover tiers, the seeded state, the countdown readout,
/// and the picker face (visible chapter head + thumbnail).
#[test]
fn on_start_stages_the_lane() {
    let scenario = scenario_from(LIFELINE_RON);
    assert!(!scenario.hidden, "the chapter head is a picker entry");
    assert!(
        scenario.thumbnail.is_some(),
        "picker entries carry the placeholder thumbnail"
    );

    let start = on_start(&scenario);
    for (id, allegiance) in [
        ("hauler_queen", Some(Allegiance::Player)),
        ("hauler_meridian", Some(Allegiance::Player)),
    ] {
        let ship = spawn_by_id(start, id);
        let ScenarioObjectKind::Spaceship(config) = &ship.kind else {
            panic!("{id} is a spaceship");
        };
        assert_eq!(
            config.allegiance, allegiance,
            "{id} is on the player's side (the ally mechanism)"
        );
        assert!(
            matches!(config.controller, SpaceshipController::None),
            "{id} is stalled: no controller, so it can never chase"
        );
    }
    spawn_by_id(start, "player_spaceship");
    spawn_by_id(start, "beacon_transfer");
    spawn_by_id(start, "beacon_lane");

    let boulders: Vec<_> = spawns(start)
        .into_iter()
        .filter(|s| match &s.kind {
            ScenarioObjectKind::Asteroid(rock) => rock.invulnerable,
            _ => false,
        })
        .collect();
    assert_eq!(boulders.len(), 4, "four invulnerable lane boulders");
    assert!(
        start
            .actions
            .iter()
            .any(|a| matches!(a, EventActionConfig::ScatterObjects(_))),
        "the destructible chaff tier scatters on start"
    );

    let readout = start
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::HudReadout(r) => Some(r),
            _ => None,
        })
        .expect("the relief countdown readout fires on start");
    assert_eq!(readout.slot, "relief");
    assert_eq!(readout.variable, "relief_remaining");
    assert!(matches!(readout.format, HudReadoutFormat::Time));
    assert!(readout.visible);

    let stories: Vec<_> = start
        .actions
        .iter()
        .filter(|a| matches!(a, EventActionConfig::StoryMessage(_)))
        .collect();
    assert_eq!(stories.len(), 1, "one comms line per beat: the dispatch");
}

/// The base bundle registers the chapter head.
#[test]
fn base_bundle_ships_lifeline() {
    assert!(
        BASE_BUNDLE_RON.contains("scenarios/lifeline.content.ron"),
        "base.bundle.ron lists the generated lifeline file"
    );
}

/// The telegraphed-arrival contract, computed from the shipped data: every
/// raider spawn (triggered waves included) keeps the design floor from the
/// player spawn AND both haulers, so no wave arrives inside anyone's
/// envelope (authored-vs-derived: the exact envelopes live in the balance
/// audit; this pins the authored margin).
#[test]
fn raider_spawns_keep_the_design_floor_from_every_friendly() {
    let scenario = scenario_from(LIFELINE_RON);
    let start = on_start(&scenario);
    let anchors = [
        spawn_by_id(start, "player_spaceship").base.position,
        spawn_by_id(start, "hauler_queen").base.position,
        spawn_by_id(start, "hauler_meridian").base.position,
    ];
    let raiders: Vec<_> = all_ship_spawns(&scenario)
        .into_iter()
        .filter(|(id, _, _)| id.starts_with("raider_"))
        .collect();
    assert_eq!(raiders.len(), 7, "seven raiders across the three waves");
    for (id, pos, ship) in raiders {
        for anchor in anchors {
            assert!(
                pos.distance(anchor) >= RAIDER_SPAWN_MIN_RANGE,
                "{id} spawns {:.0}u from a friendly anchor (floor {})",
                pos.distance(anchor),
                RAIDER_SPAWN_MIN_RANGE
            );
        }
        let SpaceshipController::AI(ai) = &ship.controller else {
            panic!("{id} is AI-flown");
        };
        assert!(
            ai.engage_delay.unwrap_or(0.0) > 0.0,
            "{id} arrives under a grace (telegraphed)"
        );
    }
}

// --- the act machine --------------------------------------------------------

/// Waves stage on the clock AND the previous wave's clears - late clears
/// delay later waves instead of stacking them - and the early clear of the
/// last wave wins before the bell, with the whole-convoy banner.
#[test]
fn waves_stage_on_clock_and_clears_and_the_early_clear_wins() {
    let scenario = scenario_from(LIFELINE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_defense(&mut app);

    // Before the first mark: nothing spawns.
    pump(&mut app);
    assert!(
        !ship_in_world(&mut app, "raider_1a"),
        "no wave before W1_AT"
    );

    // Past W1_AT: wave one arrives.
    seed_var(&mut app, "scenario_elapsed", 30.0);
    pump(&mut app);
    assert!(ship_in_world(&mut app, "raider_1a"));
    assert!(ship_in_world(&mut app, "raider_1b"));
    assert_eq!(number_var(&app, "w1_up"), Some(1.0));

    // Past W2_AT with wave one still alive: wave two WAITS (no stacking).
    seed_var(&mut app, "scenario_elapsed", 100.0);
    pump(&mut app);
    assert!(
        !ship_in_world(&mut app, "raider_2a"),
        "wave two holds until wave one is cleared"
    );

    // Clear wave one: wave two arrives (the clock mark already passed).
    destroy(&mut app, "raider_1a");
    destroy(&mut app, "raider_1b");
    assert!(ship_in_world(&mut app, "raider_2a"));
    assert!(ship_in_world(&mut app, "raider_2c"));

    // Clear wave two before W3_AT: wave three waits on ITS clock mark.
    destroy(&mut app, "raider_2a");
    destroy(&mut app, "raider_2b");
    destroy(&mut app, "raider_2c");
    assert!(
        !ship_in_world(&mut app, "raider_3a"),
        "wave three holds until its clock mark"
    );
    seed_var(&mut app, "scenario_elapsed", 170.0);
    pump(&mut app);
    assert!(ship_in_world(&mut app, "raider_3a"));

    // Clear the last wave before the bell: the early win, convoy whole.
    destroy(&mut app, "raider_3a");
    destroy(&mut app, "raider_3b");
    assert_eq!(number_var(&app, "act"), Some(2.0), "the early clear wins");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
    assert!(
        outcome_message(&app).contains("before the relief wing"),
        "the early-clear banner: {}",
        outcome_message(&app)
    );
    assert!(
        outcome_message(&app).contains("convoy is whole"),
        "the whole-convoy variant: {}",
        outcome_message(&app)
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "chapter three part one ends the chain until the finale task"
    );
}

/// The relief bell wins with the convoy alive - and the banner tracks the
/// convoy's fate (whole vs half).
#[test]
fn the_relief_bell_wins_and_the_banner_tracks_the_convoy() {
    for (queen_dies, phrase) in [(false, "convoy is whole"), (true, "Half the convoy")] {
        let scenario = scenario_from(LIFELINE_RON);
        let mut app = slice_app();
        register_non_start_handlers(&mut app, &scenario);
        seed_live_defense(&mut app);

        if queen_dies {
            destroy(&mut app, "hauler_queen");
            assert_eq!(number_var(&app, "queen_down"), Some(1.0));
            assert_eq!(outcome_kind(&app), None, "one hauler down is not a loss");
        }

        seed_var(&mut app, "scenario_elapsed", 245.0);
        pump(&mut app);

        assert_eq!(number_var(&app, "act"), Some(2.0), "the bell wins");
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
        assert!(
            outcome_message(&app).contains(phrase),
            "banner variant (queen_dies={queen_dies}): {}",
            outcome_message(&app)
        );
    }
}

/// Losing BOTH haulers is the Defeat, it closes the win gate, and it
/// retries THIS scenario; each hauler death speaks through Belt Relay.
#[test]
fn losing_the_whole_convoy_is_the_defeat() {
    let scenario = scenario_from(LIFELINE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_defense(&mut app);

    destroy(&mut app, "hauler_queen");
    assert_eq!(outcome_kind(&app), None, "half the convoy is not the loss");
    destroy(&mut app, "hauler_meridian");

    assert_eq!(number_var(&app, "act"), Some(3.0), "the loss is terminal");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    let world = app.world().resource::<NovaEventWorld>();
    let next = world.next_scenario.as_ref().expect("a retry is queued");
    assert_eq!(next.scenario_id, "lifeline", "the retry is THIS scenario");
    assert!(next.linger, "the retry lingers behind the overlay");

    // The win gate is closed for good: even past the bell, act 3 holds.
    seed_var(&mut app, "scenario_elapsed", 245.0);
    pump(&mut app);
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the bell cannot overwrite the loss"
    );

    // The voice half, pinned on the shipped data: both hauler-death
    // handlers speak through Belt Relay.
    for hauler in ["hauler_queen", "hauler_meridian"] {
        let handler = scenario
            .events
            .iter()
            .find(|e| {
                matches!(e.name, EventConfig::OnDestroyed) && e.filters.iter().any(|f| {
                    matches!(
                        f,
                        EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some(hauler)
                    )
                })
            })
            .expect("the hauler death beat exists");
        assert!(
            handler.actions.iter().any(|a| matches!(
                a,
                EventActionConfig::StoryMessage(m) if m.speaker == "Belt Relay"
            )),
            "{hauler}'s beacon-dark beat speaks through Belt Relay"
        );
    }
}

/// The player's death on a live act retries the lane.
#[test]
fn player_death_retries_the_lane() {
    let scenario = scenario_from(LIFELINE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_defense(&mut app);

    destroy(&mut app, "player_spaceship");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the player's death is terminal (review R1.1)"
    );
    let world = app.world().resource::<NovaEventWorld>();
    let next = world.next_scenario.as_ref().expect("a retry is queued");
    assert_eq!(next.scenario_id, "lifeline");
    assert!(next.linger);

    // The mutual-destruction trade (review R1.1): the last raider dying
    // AFTER the player must not overwrite the Defeat with a Victory.
    seed_var(&mut app, "w3_up", 1.0);
    destroy(&mut app, "raider_3a");
    destroy(&mut app, "raider_3b");
    seed_var(&mut app, "scenario_elapsed", 245.0);
    pump(&mut app);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "no win gate opens after the player's death"
    );
}

/// The countdown is a DERIVED variable: the live handler recomputes
/// `relief_remaining = 240 - scenario_elapsed` every frame (only writing
/// `scenario_elapsed` itself is linted).
#[test]
fn the_countdown_tracks_the_clock() {
    let scenario = scenario_from(LIFELINE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_live_defense(&mut app);

    seed_var(&mut app, "scenario_elapsed", 100.0);
    app.update();
    assert_eq!(
        number_var(&app, "relief_remaining"),
        Some(140.0),
        "relief_remaining = 240 - clock"
    );
}
