//! DIVERGING BRANCHES driven end to end on a synthetic scenario defined inline
//! in this file. Content is authored here as a RON string with generic ids
//! (`beacon_1`, `spaceship_1`), parsed through the real
//! [`Content`](nova_modding::prelude::Content) vocabulary and registered as
//! real handlers, so no installed mod owns this coverage.
//!
//! The ENGINE contract a branching scenario leans on:
//!
//! 1. `OnEnter` routes by BOTH parties - the area entered and who entered it -
//!    so two beacons drive two different handlers;
//! 2. a branch handler may SPAWN a ship mid-scenario, arriving with an
//!    `engage_delay` telegraph, and the other branch spawns none;
//! 3. a branch handler's `SetSkybox` resolves its cubemap through the real
//!    `AssetServer` and runs to completion in a headless app;
//! 4. the two branches settle DISTINCT outcome messages;
//! 5. a branch that latches the terminal act SYNCHRONOUSLY opens no death
//!    window, while the branch that keeps a live act stays losable;
//! 6. only one branch chains a `NextScenario`; the terminal one queues nothing.
//!
//! Standalone: `cargo test -p nova_assets --test scenario_branch_choice`.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, GameEventsPlugin, OnDefeatedEvent, OnDefeatedEventInfo, OnDestroyedEvent,
    OnDestroyedEventInfo, OnEnterEvent, OnEnterEventInfo, OnNeutralizedEvent,
    OnNeutralizedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::GameObjectives;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

/// The fixture: one choice, two beacons, two endings. `beacon_1` is the FIGHT
/// branch (spawns `spaceship_2`, which must be broken to win and can kill you
/// first); `beacon_2` is the TERMINAL branch (latches the closing act at once
/// and settles a deferred outcome with no fight at all).
///
/// Acts: 1 = choosing, 2 = fighting, 3 = closed. `choice`: 1 = fight,
/// 2 = terminal.
const SCENARIO_RON: &str = r#"[
    Scenario((
        id: "scenario_1",
        name: "Scenario One",
        description: "Synthetic branching fixture.",
        cubemap: "dep://base/textures/cubemap.png",
        watches: [
            (variable: "scenario_elapsed", query: Scenario((property: Elapsed))),
        ],
        events: [
            (
                name: OnStart,
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "choice", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "close_gate", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "close_said", expression: Term(Factor(Literal(Number(0.0)))))),
                    SpawnScenarioObject((
                        base: (id: "spaceship_1", name: "Ship One", position: (0.0, 0.0, 40.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player(()),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "beacon_1", name: "Beacon One", position: (90.0, 0.0, -100.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "ONE", radius: 3.0, color: Srgba((red: 1.0, green: 0.7, blue: 0.2, alpha: 1.0)), area_radius: Some(25.0))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "beacon_2", name: "Beacon Two", position: (-90.0, 0.0, -100.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "TWO", radius: 3.0, color: Srgba((red: 1.0, green: 0.25, blue: 0.2, alpha: 1.0)), area_radius: Some(25.0))),
                    )),
                ],
            ),
            // The FIGHT branch: opens act 2, swaps the sky, and the hostile
            // ARRIVES telegraphed rather than existing from frame zero.
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("beacon_1"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "choice", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(2.0)))))),
                    SetSkybox((cubemap: "dep://base/textures/cubemap_alt.png")),
                    StoryMessage((speaker: "Speaker One", text: "Something is closing on you.")),
                    SpawnScenarioObject((
                        base: (id: "spaceship_2", name: "Ship Two", position: (0.0, 30.0, -260.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: AI((engage_delay: Some(8.0))),
                            hull: Inline((sections: [])),
                        )),
                    )),
                ],
            ),
            // The TERMINAL branch: act 3 SYNCHRONOUSLY with the choice, so the
            // act < 3 death gate can never trip afterwards. The overlay is
            // deferred a beat behind the line.
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("beacon_2"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "choice", expression: Term(Factor(Literal(Number(2.0)))))),
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(3.0)))))),
                    VariableSet((key: "close_gate", expression: Add(Factor(Name("scenario_elapsed")), Term(Factor(Literal(Number(3.0))))))),
                    StoryMessage((speaker: "Speaker One", text: "Nothing left to chase.")),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("choice"))), Term(Factor(Literal(Number(2.0))))))),
                    Expression((Equal(Term(Factor(Name("close_said"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("close_gate"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("scenario_elapsed"))), Term(Factor(Name("close_gate")))))),
                ],
                actions: [
                    VariableSet((key: "close_said", expression: Term(Factor(Literal(Number(1.0)))))),
                    Outcome((outcome: Victory, message: Some("WALKED AWAY"))),
                ],
            ),
            // The FIGHT branch's win, on either lifecycle end. Only this branch
            // chains onward.
            (
                name: OnDestroyed,
                filters: [
                    Entity((id: Some("spaceship_2"))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(2.0))))))),
                ],
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(3.0)))))),
                    Outcome((outcome: Victory, message: Some("STOOD YOUR GROUND"))),
                    NextScenario((scenario_id: "scenario_2", linger: true)),
                ],
            ),
            (
                name: OnNeutralized,
                filters: [
                    Entity((id: Some("spaceship_2"))),
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(2.0))))))),
                ],
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(3.0)))))),
                    Outcome((outcome: Victory, message: Some("STOOD YOUR GROUND"))),
                    NextScenario((scenario_id: "scenario_2", linger: true)),
                ],
            ),
            (
                name: OnDefeated,
                filters: [
                    Entity((id: Some("spaceship_1"))),
                    Expression((LessThan(Term(Factor(Name("act"))), Term(Factor(Literal(Number(3.0))))))),
                ],
                actions: [
                    Outcome((outcome: Defeat, message: Some("LOST"))),
                    NextScenario((scenario_id: "scenario_1", linger: true)),
                ],
            ),
        ],
    )),
]"#;

// --- content plumbing -------------------------------------------------------

/// Two frames, then run the world out to LIVE.
///
/// The two frames dispatch the event just fired; `settle_spawns` is what makes
/// the NEXT one land. This scenario spawns from handlers the rig registers, so
/// firing an event can leave the world settling - and the dispatcher holds
/// every handler until those objects are in. A fixed frame count would pass or
/// fail on machine load rather than on logic.
fn step(app: &mut App) {
    app.update();
    app.update();
    nova_scenario::test_support::settle_spawns(app);
}

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

fn spawn_by_id<'a>(event: &'a ScenarioEventConfig, id: &str) -> Option<&'a ScenarioObjectConfig> {
    spawns(event).into_iter().find(|s| s.base.id == id)
}

/// Does this handler's `Entity` filter name `id` as the subject?
fn filtered_on(event: &ScenarioEventConfig, id: &str) -> bool {
    event
        .filters
        .iter()
        .any(|f| matches!(f, EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some(id)))
}

// --- app harness ------------------------------------------------------------

fn slice_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The fight branch's `SetSkybox` reads the AssetServer to start the cubemap
    // load, exactly as in production. Register the asset plumbing so the
    // handler runs to completion rather than panicking on a missing resource
    // (no scenario camera is present, so the swap no-ops after the load starts).
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

/// Fire an `OnEnter` of `area` by `entrant` (the loader's pairing: the area is
/// the subject, the entrant the other party).
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
    step(app);
}

fn destroy(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDefeatedEvent>(OnDefeatedEventInfo {
                id: info.id.clone(),
                type_name: info.type_name.clone(),
            });
            commands.fire::<OnDestroyedEvent>(info.clone());
        })
        .expect("fire direct-destruction lifecycle");
    step(app);
}

fn neutralize(app: &mut App, id: &str) {
    let info = OnNeutralizedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnDefeatedEvent>(OnDefeatedEventInfo {
                id: info.id.clone(),
                type_name: info.type_name.clone(),
            });
            commands.fire::<OnNeutralizedEvent>(info.clone());
        })
        .expect("fire neutralization lifecycle");
    step(app);
}

fn pump_clock(app: &mut App, to_secs: f64) {
    seed_var(app, "scenario_elapsed", to_secs);
    step(app);
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

fn outcome_message(app: &App) -> Option<String> {
    app.world()
        .resource::<CurrentOutcome>()
        .0
        .as_ref()
        .and_then(|outcome| outcome.message.clone())
}

fn queued_next(app: &App) -> Option<(String, bool)> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| (next.scenario_id.clone(), next.linger))
}

/// The machine seeded the way OnStart does, with a defined clock base.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    for (key, value) in [
        ("act", 1.0),
        ("choice", 0.0),
        ("close_gate", 0.0),
        ("close_said", 0.0),
        ("scenario_elapsed", 30.0),
    ] {
        seed_var(&mut app, key, value);
    }
    app
}

// --- structural pins --------------------------------------------------------

#[test]
fn the_arriving_hostile_is_spawned_by_one_branch_only() {
    // The whole point of a mid-scenario arrival: it exists in exactly one
    // handler, so the other branch structurally cannot summon it.
    let scenario = scenario_from(SCENARIO_RON);
    let spawn_sites: Vec<_> = scenario
        .events
        .iter()
        .filter(|e| spawn_by_id(e, "spaceship_2").is_some())
        .collect();
    assert_eq!(
        spawn_sites.len(),
        1,
        "exactly ONE handler spawns the arriving hostile"
    );
    assert!(
        matches!(spawn_sites[0].name, EventConfig::OnEnter),
        "and it arrives on a branch entry, not at OnStart"
    );

    // It arrives TELEGRAPHED: an engage_delay grace is what makes an arrival
    // readable before it is lethal.
    let arrival = spawn_by_id(spawn_sites[0], "spaceship_2").expect("the branch spawns it");
    let ScenarioObjectKind::Spaceship(ship) = &arrival.kind else {
        panic!("the arrival is a spaceship");
    };
    let SpaceshipController::AI(ai) = &ship.controller else {
        panic!("the arrival is AI-controlled");
    };
    assert_eq!(
        ai.engage_delay,
        Some(8.0),
        "the arriving hostile carries its engage_delay grace"
    );
}

#[test]
fn only_the_fight_branch_chains_onward() {
    let scenario = scenario_from(SCENARIO_RON);
    let chains: Vec<_> = scenario
        .events
        .iter()
        .filter(|e| {
            e.actions.iter().any(|a| {
                matches!(a, EventActionConfig::NextScenario(next) if next.scenario_id == "scenario_2")
            })
        })
        .collect();
    assert_eq!(
        chains.len(),
        2,
        "the two equivalent fight wins (destroyed / neutralized) chain onward"
    );
    assert!(
        chains.iter().all(|e| filtered_on(e, "spaceship_2")),
        "both chains hang off the arriving hostile's death, not the terminal branch"
    );
}

// --- the two branches -------------------------------------------------------

#[test]
fn the_fight_branch_opens_the_fight_and_wins_by_breaking_the_arrival() {
    let scenario = scenario_from(SCENARIO_RON);

    for kill in [destroy as fn(&mut App, &str), neutralize] {
        let mut app = armed_app(&scenario);

        enter(&mut app, "beacon_1", "spaceship_1");
        assert_eq!(number_var(&app, "choice"), Some(1.0), "the branch is taken");
        assert_eq!(number_var(&app, "act"), Some(2.0), "the fight act opens");
        assert_eq!(
            outcome_kind(&app),
            None,
            "taking the branch is not a win - the arrival still has to die"
        );

        kill(&mut app, "spaceship_2");
        assert_eq!(
            number_var(&app, "act"),
            Some(3.0),
            "the kill closes the act"
        );
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
        assert_eq!(outcome_message(&app).as_deref(), Some("STOOD YOUR GROUND"));
        assert_eq!(queued_next(&app), Some(("scenario_2".to_string(), true)));
    }
}

#[test]
fn the_terminal_branch_closes_with_no_fight_and_chains_nothing() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "beacon_2", "spaceship_1");
    assert_eq!(number_var(&app, "choice"), Some(2.0), "the branch is taken");
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the terminal act latches SYNCHRONOUSLY with the choice"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "the line plays first; the overlay is a beat behind"
    );

    pump_clock(&mut app, 100.0);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
    assert_eq!(outcome_message(&app).as_deref(), Some("WALKED AWAY"));
    assert_eq!(
        queued_next(&app),
        None,
        "the terminal ending queues no NextScenario"
    );
}

#[test]
fn the_two_endings_settle_distinct_messages() {
    let scenario = scenario_from(SCENARIO_RON);

    let mut fight = armed_app(&scenario);
    enter(&mut fight, "beacon_1", "spaceship_1");
    destroy(&mut fight, "spaceship_2");

    let mut terminal = armed_app(&scenario);
    enter(&mut terminal, "beacon_2", "spaceship_1");
    pump_clock(&mut terminal, 100.0);

    let fight_message = outcome_message(&fight).expect("the fight branch settles");
    let terminal_message = outcome_message(&terminal).expect("the terminal branch settles");
    assert_eq!(outcome_kind(&fight), Some(ScenarioOutcomeKind::Victory));
    assert_eq!(outcome_kind(&terminal), Some(ScenarioOutcomeKind::Victory));
    assert_ne!(
        fight_message, terminal_message,
        "the two endings must carry DISTINCT terminal messages, not the same text"
    );
}

// --- the death window -------------------------------------------------------

#[test]
fn the_fight_branch_is_losable_and_requeues_this_scenario() {
    let scenario = scenario_from(SCENARIO_RON);

    for kill in [destroy as fn(&mut App, &str), neutralize] {
        let mut app = armed_app(&scenario);
        enter(&mut app, "beacon_1", "spaceship_1");
        assert_eq!(number_var(&app, "act"), Some(2.0));

        kill(&mut app, "spaceship_1");
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
        assert_eq!(
            queued_next(&app),
            Some(("scenario_1".to_string(), true)),
            "the retry is THIS scenario"
        );
    }
}

#[test]
fn the_terminal_branch_opens_no_death_window() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    // Latch the terminal act BEFORE any death.
    enter(&mut app, "beacon_2", "spaceship_1");
    assert_eq!(number_var(&app, "act"), Some(3.0));

    // A death after the latch (debris, a stray) must not flip the earned close.
    destroy(&mut app, "spaceship_1");
    assert_ne!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "no Defeat once the terminal act is latched"
    );

    // And the deferred close still lands.
    pump_clock(&mut app, 100.0);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
}

#[test]
fn a_settled_outcome_is_not_overwritten() {
    // Both endings latch act 3; no later handler may overwrite a settled
    // Outcome (the outcome-is-last-write-wins-close-the-act lesson).
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 3.0);
    seed_var(&mut app, "choice", 1.0);
    seed_var(&mut app, "close_gate", 0.0);
    seed_var(&mut app, "close_said", 0.0);

    destroy(&mut app, "spaceship_1");
    assert_eq!(outcome_kind(&app), None, "no Defeat over a closed act");
    assert_eq!(queued_next(&app), None, "no retry over a closed act");
}

// --- the OnEnter pairing ----------------------------------------------------

#[test]
fn an_entry_by_the_wrong_party_drives_nothing() {
    // The `other_id` half of the Entity filter is what keeps a drifting rock
    // or an escort from taking the player's choice.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "beacon_1", "spaceship_2");
    assert_eq!(
        number_var(&app, "choice"),
        Some(0.0),
        "only the named entrant takes the branch"
    );
    assert_eq!(number_var(&app, "act"), Some(1.0));

    // And a second entry after the choice is closed by the act guard.
    enter(&mut app, "beacon_1", "spaceship_1");
    assert_eq!(number_var(&app, "choice"), Some(1.0));
    enter(&mut app, "beacon_2", "spaceship_1");
    assert_eq!(
        number_var(&app, "choice"),
        Some(1.0),
        "the act guard makes the choice one-way"
    );
}
