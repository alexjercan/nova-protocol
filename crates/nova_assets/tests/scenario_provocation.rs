//! NEUTRAL-UNTIL-PROVOKED ships and the CLOCK-PACED opening, driven end to end
//! on a synthetic scenario defined inline in this file. Content is authored
//! here as a RON string with generic ids (`spaceship_2`, `area_1`), parsed
//! through the real [`Content`](nova_modding::prelude::Content) vocabulary and
//! registered as real handlers, so no installed mod owns this coverage.
//!
//! The ENGINE contract a stealth-shaped scenario leans on:
//!
//! 1. a ship spawned through the REAL `SpawnScenarioObject` action carries its
//!    authored `Allegiance` COMPONENT once the production command flush runs -
//!    the value the AI targets by, not a rig-side variable;
//! 2. `SetAllegiance` overwrites that live component, and one handler can flip
//!    several ships at once;
//! 3. both `OnEnter` (a `CreateScenarioArea` sensor zone) and
//!    `OnCombatLockStart` (a radar paint) reach the same wake;
//! 4. a shared one-shot variable composes those triggers: once any of them has
//!    stamped it, the others are inert;
//! 5. the reserved `player_speed` readout is filterable content vocabulary,
//!    and combines with `scenario_elapsed` into a warn -> rearm -> countdown ->
//!    trip machine whose countdown can be CANCELLED in time;
//! 6. an objective posts LAZILY out of a clock-paced cascade, so a `GameObjectives`
//!    entry appears only after the briefing hands off.
//!
//! Standalone: `cargo test -p nova_assets --test scenario_provocation`.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, EntityId, EventAction, GameEventInfo, GameEventsPlugin, LockEventInfo,
    OnCombatLockStartEvent, OnEnterEvent, OnEnterEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::{Allegiance, GameObjectives};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

/// The fixture: two Neutral patrols the player is meant to slip past, a watch
/// zone, a paint trigger, an overspeed machine, and a clock-paced opening that
/// hands off to the first objective.
///
/// `spotted` is the shared one-shot every wake stamps; `speed_warned` runs
/// 0 (quiet) -> 1 (warned) -> 2 (armed) -> 3 (counting down).
const SCENARIO_RON: &str = r#"[
    Scenario((
        id: "scenario_1",
        name: "Scenario One",
        description: "Synthetic provocation fixture.",
        cubemap: "dep://base/textures/cubemap.png",
        watches: [
            (variable: "scenario_elapsed", query: Scenario((property: Elapsed))),
        ],
        events: [
            (
                name: OnStart,
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "spotted", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "speed_warned", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "speed_deadline", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "open_step", expression: Term(Factor(Literal(Number(0.0)))))),
                    SpawnScenarioObject((
                        base: (id: "spaceship_1", name: "Ship One", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player(()),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "spaceship_2", name: "Ship Two", position: (60.0, 0.0, -80.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI((patrol: [(60.0, 0.0, -80.0), (100.0, 10.0, -140.0)])),
                            allegiance: Some(Neutral),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "spaceship_3", name: "Ship Three", position: (-60.0, 0.0, -160.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI((patrol: [(-60.0, 0.0, -160.0), (-20.0, 10.0, -220.0)])),
                            allegiance: Some(Neutral),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    CreateScenarioArea((id: "area_1", name: "Watch Zone", position: (80.0, 0.0, -110.0), rotation: (0.0, 0.0, 0.0, 1.0), radius: 24.0)),
                ],
            ),
            // The clock-paced opening: a line, then the hand-off that finally
            // posts the objective. No objective shares a frame with the brief.
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("open_step"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("scenario_elapsed"))), Term(Factor(Literal(Number(2.0))))))),
                ],
                actions: [
                    VariableSet((key: "open_step", expression: Term(Factor(Literal(Number(1.0)))))),
                    StoryMessage((speaker: "Speaker One", text: "Stand by.")),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("open_step"))), Term(Factor(Literal(Number(1.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("scenario_elapsed"))), Term(Factor(Literal(Number(10.0))))))),
                ],
                actions: [
                    VariableSet((key: "open_step", expression: Term(Factor(Literal(Number(2.0)))))),
                    Objective((id: "objective_1", message: "Slip the channel unseen.")),
                ],
            ),
            // Wake 1: blundering into the watch zone.
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("area_1"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "spotted", expression: Term(Factor(Literal(Number(1.0)))))),
                    SetAllegiance((id: "spaceship_2", allegiance: Enemy)),
                    SetAllegiance((id: "spaceship_3", allegiance: Enemy)),
                ],
            ),
            // Wake 2: painting one of them.
            (
                name: OnCombatLockStart,
                filters: [
                    Entity((id: Some("spaceship_3"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "spotted", expression: Term(Factor(Literal(Number(1.0)))))),
                    SetAllegiance((id: "spaceship_2", allegiance: Enemy)),
                    SetAllegiance((id: "spaceship_3", allegiance: Enemy)),
                ],
            ),
            // Wake 3, in four beats: warn, rearm, count down, trip. Every beat
            // gates `spotted == 0`, so a prior wake disarms the whole machine.
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("speed_warned"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("player_speed"))), Term(Factor(Literal(Number(8.0))))))),
                ],
                actions: [
                    VariableSet((key: "speed_warned", expression: Term(Factor(Literal(Number(1.0)))))),
                    StoryMessage((speaker: "Speaker One", text: "Ease off.")),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("speed_warned"))), Term(Factor(Literal(Number(1.0))))))),
                    Expression((LessThan(Term(Factor(Name("player_speed"))), Term(Factor(Literal(Number(7.0))))))),
                ],
                actions: [
                    VariableSet((key: "speed_warned", expression: Term(Factor(Literal(Number(2.0)))))),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("speed_warned"))), Term(Factor(Literal(Number(2.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("player_speed"))), Term(Factor(Literal(Number(8.0))))))),
                ],
                actions: [
                    VariableSet((key: "speed_warned", expression: Term(Factor(Literal(Number(3.0)))))),
                    VariableSet((key: "speed_deadline", expression: Add(Factor(Name("scenario_elapsed")), Term(Factor(Literal(Number(3.5))))))),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("speed_warned"))), Term(Factor(Literal(Number(3.0))))))),
                    Expression((LessThan(Term(Factor(Name("player_speed"))), Term(Factor(Literal(Number(7.0))))))),
                ],
                actions: [
                    VariableSet((key: "speed_warned", expression: Term(Factor(Literal(Number(2.0)))))),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("spotted"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((Equal(Term(Factor(Name("speed_warned"))), Term(Factor(Literal(Number(3.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("player_speed"))), Term(Factor(Literal(Number(8.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("scenario_elapsed"))), Term(Factor(Name("speed_deadline")))))),
                ],
                actions: [
                    VariableSet((key: "spotted", expression: Term(Factor(Literal(Number(1.0)))))),
                    SetAllegiance((id: "spaceship_2", allegiance: Enemy)),
                    SetAllegiance((id: "spaceship_3", allegiance: Enemy)),
                ],
            ),
        ],
    )),
]"#;

/// The two neutral patrols the walks wake.
const PATROLS: [&str; 2] = ["spaceship_2", "spaceship_3"];

// --- content plumbing -------------------------------------------------------

fn scenario_from(ron_str: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron_str).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_)
            | Content::Campaign(_)
            | Content::Style(_)
            | Content::Ship(_)
            | Content::Impact(_) => None,
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

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> &'a ScenarioObjectConfig {
    event
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) if config.base.id == id => Some(config),
            _ => None,
        })
        .unwrap_or_else(|| panic!("OnStart spawns '{id}'"))
}

// --- app harness ------------------------------------------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
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
        app.world_mut().spawn(event.build_handler());
    }
}

/// Spawn the two patrols through their REAL OnStart `SpawnScenarioObject`
/// configs (the same `EventAction` the loader runs), then tick so the
/// production command flush applies them - the ships exist as scoped entities
/// carrying the authored `Allegiance`, exactly as in the game.
fn spawn_patrols(app: &mut App, scenario: &ScenarioConfig) {
    let start = on_start(scenario);
    for id in PATROLS {
        let config = spawn_by_id(start, id).clone();
        let mut event_world = app.world_mut().resource_mut::<NovaEventWorld>();
        config.action(&mut event_world, &GameEventInfo::default());
    }
    app.update();
    app.update();
    // Two ship spawns are the most expensive commands there are, and the flush
    // applies them under a per-frame budget - so two ticks sits exactly on the
    // edge, and a third patrol would silently leave the world settling with
    // every later handler held. Wait on the WORLD, not on a frame count.
    nova_scenario::test_support::settle_spawns(app);
}

/// Read the LIVE `Allegiance` component off a spawned scenario ship.
fn ship_allegiance(app: &mut App, id: &str) -> Allegiance {
    let mut query = app.world_mut().query::<(&EntityId, &Allegiance)>();
    query
        .iter(app.world())
        .find(|(entity_id, _)| entity_id.0 == id)
        .map(|(_, allegiance)| *allegiance)
        .unwrap_or_else(|| panic!("ship '{id}' exists with an Allegiance"))
}

fn assert_patrols_are(app: &mut App, expected: Allegiance, why: &str) {
    for id in PATROLS {
        assert_eq!(ship_allegiance(app, id), expected, "{id}: {why}");
    }
}

fn enter(app: &mut App, area: &str, entrant: &str) {
    let info = OnEnterEventInfo {
        id: area.to_string(),
        other_id: entrant.to_string(),
        other_type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnEnterEvent>(info.clone());
        })
        .expect("fire OnEnter");
    app.update();
    app.update();
}

fn combat_lock(app: &mut App, id: &str, other_id: &str) {
    let info = LockEventInfo {
        id: id.to_string(),
        other_id: other_id.to_string(),
        other_type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnCombatLockStartEvent>(info.clone());
        })
        .expect("fire OnCombatLockStart");
    app.update();
    app.update();
}

/// Stamp the scenario clock and tick (the time-gated-content-needs-a-clock-pump
/// lesson: the rig runs no clock of its own).
fn pump_clock(app: &mut App, to_secs: f64) {
    seed_var(app, "scenario_elapsed", to_secs);
    app.update();
    app.update();
}

/// Stamp the reserved `player_speed` readout and tick. In production
/// `track_player_speed` writes it off the player ship's velocity every live
/// tick (unit-pinned in nova_scenario); here the CONTENT handlers that consume
/// it are what is under test.
fn pump_speed(app: &mut App, to_units: f64) {
    seed_var(app, "player_speed", to_units);
    app.update();
    app.update();
}

fn number_var(app: &App, key: &str) -> Option<f64> {
    match app.world().resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

fn has_objective(app: &App, id: &str) -> bool {
    app.world()
        .resource::<GameObjectives>()
        .objectives
        .iter()
        .any(|o| o.id == id)
}

/// The machine seeded the way OnStart does, with both patrols spawned through
/// the real spawn action.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    for (key, value) in [
        ("act", 1.0),
        ("spotted", 0.0),
        ("speed_warned", 0.0),
        ("speed_deadline", 0.0),
        ("open_step", 0.0),
        ("player_speed", 0.0),
        ("scenario_elapsed", 0.0),
    ] {
        seed_var(&mut app, key, value);
    }
    spawn_patrols(&mut app, scenario);
    app
}

// --- the authored allegiance reaches the live component ---------------------

#[test]
fn a_spawned_ship_carries_its_authored_allegiance_component() {
    // The stealth contract starts here: the ships the player must slip past
    // are NEUTRAL bystanders on the component the AI targets by, not a
    // rig-side variable standing in for one.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);
    assert_patrols_are(&mut app, Allegiance::Neutral, "spawns asleep");
}

// --- the wakes --------------------------------------------------------------

#[test]
fn entering_the_watch_zone_wakes_every_named_ship() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "area_1", "spaceship_1");
    assert_eq!(
        number_var(&app, "spotted"),
        Some(1.0),
        "the zone stamps the shared one-shot"
    );
    assert_patrols_are(
        &mut app,
        Allegiance::Enemy,
        "one handler flips every ship it names",
    );
}

#[test]
fn painting_a_sleeping_ship_wakes_them_too() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    combat_lock(&mut app, "spaceship_3", "spaceship_1");
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    assert_patrols_are(&mut app, Allegiance::Enemy, "a paint reaches the same wake");
}

#[test]
fn a_prior_wake_disarms_every_other_trigger() {
    // The one-shot composition: `spotted` is what makes five independent
    // triggers behave as one.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "area_1", "spaceship_1");
    assert_eq!(number_var(&app, "spotted"), Some(1.0));

    // Neither a paint nor a hot burn does anything after the first wake.
    combat_lock(&mut app, "spaceship_3", "spaceship_1");
    pump_speed(&mut app, 20.0);
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(0.0),
        "an already-spotted run never warns on speed"
    );
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    assert_patrols_are(&mut app, Allegiance::Enemy, "and nothing re-flips");
}

// --- the overspeed machine --------------------------------------------------

#[test]
fn overspeed_warns_then_trips_only_on_a_held_fresh_breach() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    // First breach: WARNS only. Nobody wakes.
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(1.0));
    assert_eq!(
        number_var(&app, "spotted"),
        Some(0.0),
        "a warning is not a spotting"
    );
    assert_patrols_are(&mut app, Allegiance::Neutral, "still asleep");

    // A single CONTINUOUS burn never advances past the warning: the countdown
    // needs a rearm below the band first.
    for _ in 0..3 {
        pump_speed(&mut app, 12.0);
    }
    assert_eq!(
        number_var(&app, "speed_warned"),
        Some(1.0),
        "a continuous burn never rearms, so it never arms the countdown"
    );

    // Slowing under the rearm band ARMS the countdown, silently.
    pump_speed(&mut app, 6.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(2.0));
    assert_eq!(number_var(&app, "spotted"), Some(0.0));

    // A FRESH breach starts the countdown and stamps a deadline off the clock.
    // It does not trip yet - this is the reaction window.
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(3.0));
    assert_eq!(
        number_var(&app, "speed_deadline"),
        Some(3.5),
        "the deadline is scenario_elapsed (0) + 3.5"
    );
    assert_eq!(number_var(&app, "spotted"), Some(0.0));

    // Still hot but BEFORE the deadline: no wake.
    pump_clock(&mut app, 2.0);
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_patrols_are(&mut app, Allegiance::Neutral, "the run is still dark");

    // Held past the deadline: the wake lands, on the same live component the
    // zone and paint triggers flip.
    pump_clock(&mut app, 4.0);
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    assert_patrols_are(&mut app, Allegiance::Enemy, "holding the burn wakes them");
}

#[test]
fn easing_off_during_the_countdown_cancels_the_wake() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    // Warn, rearm, then a fresh breach starts the countdown.
    pump_speed(&mut app, 9.0);
    pump_speed(&mut app, 6.0);
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(3.0));
    assert_eq!(number_var(&app, "speed_deadline"), Some(3.5));

    // Ease off in time: the countdown CANCELS back to armed, silently.
    pump_clock(&mut app, 2.0);
    pump_speed(&mut app, 6.0);
    assert_eq!(number_var(&app, "speed_warned"), Some(2.0));
    assert_eq!(number_var(&app, "spotted"), Some(0.0));

    // Even well past the OLD deadline: no wake - the state is armed, not
    // counting.
    pump_clock(&mut app, 10.0);
    assert_eq!(number_var(&app, "spotted"), Some(0.0));
    assert_patrols_are(&mut app, Allegiance::Neutral, "the reprieve held");

    // A fresh breach starts a NEW countdown off the CURRENT clock; held past
    // it, they wake. The reprieve granted no immunity.
    pump_speed(&mut app, 9.0);
    assert_eq!(number_var(&app, "speed_deadline"), Some(13.5));
    pump_clock(&mut app, 14.0);
    assert_eq!(number_var(&app, "spotted"), Some(1.0));
    assert_patrols_are(&mut app, Allegiance::Enemy, "the second breach landed");
}

// --- the lazy objective -----------------------------------------------------

#[test]
fn the_objective_posts_only_after_the_clock_paced_hand_off() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    app.update();
    assert!(
        !has_objective(&app, "objective_1"),
        "no objective before the briefing runs"
    );
    assert_eq!(number_var(&app, "open_step"), Some(0.0));

    // One pump per threshold, so each cascade step actually fires.
    pump_clock(&mut app, 3.0);
    assert_eq!(number_var(&app, "open_step"), Some(1.0));
    assert!(
        !has_objective(&app, "objective_1"),
        "the first beat speaks; the objective is still held back"
    );

    pump_clock(&mut app, 11.0);
    assert_eq!(number_var(&app, "open_step"), Some(2.0));
    assert!(
        has_objective(&app, "objective_1"),
        "the hand-off is what finally posts the objective"
    );
}
