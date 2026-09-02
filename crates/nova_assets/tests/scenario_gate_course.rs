//! An ORDERED GATE COURSE driven end to end on a synthetic scenario defined
//! inline in this file. Content is authored here as a RON string with generic
//! ids (`gate_1`..`gate_3`, `spaceship_1`), parsed through the real
//! [`Content`](nova_modding::prelude::Content) vocabulary and registered as
//! real handlers, so no installed mod owns this coverage.
//!
//! The ENGINE contract a sequenced, timed course leans on:
//!
//! 1. a `gate == N` expression filter sequences `OnEnter` handlers strictly -
//!    entering a later gate early is INERT, not a skip;
//! 2. the `other_id` half of the `Entity` filter means only the named ship
//!    advances the course;
//! 3. an area is NOT a one-shot: re-entering the same zone fires again, so a
//!    penalty zone can count repeats;
//! 4. two mutually exclusive `Outcome` branches on one trigger select by a
//!    counter variable, so the same crossing yields a different banner;
//! 5. a terminal gate value disarms the loss handler, so a wreck after the
//!    finish declares nothing;
//! 6. `HudReadout` binds a named slot to a scenario variable with a format and
//!    a visibility - the display half of the variable vocabulary.
//!
//! Standalone: `cargo test -p nova_assets --test scenario_gate_course`.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, GameEventsPlugin, OnDestroyedEvent, OnDestroyedEventInfo, OnEnterEvent,
    OnEnterEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::GameObjectives;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

/// The fixture: three gates then a finish, a repeatable penalty zone, and two
/// win banners keyed on the penalty count. `gate` runs 1..4 and latches at 5.
const SCENARIO_RON: &str = r#"[
    Scenario((
        id: "scenario_1",
        name: "Scenario One",
        description: "Synthetic gate-course fixture.",
        cubemap: "dep://base/textures/cubemap.png",
        watches: [
            (variable: "scenario_elapsed", query: Scenario((property: Elapsed))),
        ],
        events: [
            (
                name: OnStart,
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "penalty", expression: Term(Factor(Literal(Number(0.0)))))),
                    HudReadout((slot: "readout_1", variable: "scenario_elapsed", format: Time, label: Some("TIME"), visible: true)),
                    SpawnScenarioObject((
                        base: (id: "spaceship_1", name: "Ship One", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player(()),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "gate_1", name: "Gate One", position: (0.0, 0.0, -2000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "1", radius: 30.0, color: Srgba((red: 0.4, green: 0.8, blue: 1.0, alpha: 1.0)), area_radius: Some(300.0))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "gate_2", name: "Gate Two", position: (0.0, 0.0, -4000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "2", radius: 30.0, color: Srgba((red: 0.4, green: 0.8, blue: 1.0, alpha: 1.0)), area_radius: Some(300.0))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "gate_3", name: "Gate Three", position: (0.0, 0.0, -6000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "3", radius: 30.0, color: Srgba((red: 0.4, green: 0.8, blue: 1.0, alpha: 1.0)), area_radius: Some(300.0))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "finish_1", name: "Finish", position: (0.0, 0.0, -8000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Beacon((label: "FINISH", radius: 30.0, color: Srgba((red: 1.0, green: 0.8, blue: 0.2, alpha: 1.0)), area_radius: Some(300.0))),
                    )),
                    CreateScenarioArea((id: "area_1", name: "Penalty Zone", position: (400.0, 0.0, -3000.0), rotation: (0.0, 0.0, 0.0, 1.0), radius: 400.0)),
                ],
            ),
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("gate_1"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(2.0)))))),
                ],
            ),
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("gate_2"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(2.0))))))),
                ],
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(3.0)))))),
                ],
            ),
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("gate_3"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(3.0))))))),
                ],
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(4.0)))))),
                ],
            ),
            // The penalty zone re-counts on every fresh entry, and is gated out
            // once the course is over.
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("area_1"), other_id: Some("spaceship_1"))),
                    Expression((LessThan(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(5.0))))))),
                ],
                actions: [
                    VariableSet((key: "penalty", expression: Add(Factor(Name("penalty")), Term(Factor(Literal(Number(1.0))))))),
                ],
            ),
            // Two mutually exclusive win branches on the same crossing.
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("finish_1"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(4.0))))))),
                    Expression((Equal(Term(Factor(Name("penalty"))), Term(Factor(Literal(Number(0.0))))))),
                ],
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(5.0)))))),
                    Outcome((outcome: Victory, message: Some("CLEAN RUN"))),
                ],
            ),
            (
                name: OnEnter,
                filters: [
                    Entity((id: Some("finish_1"), other_id: Some("spaceship_1"))),
                    Expression((Equal(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(4.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("penalty"))), Term(Factor(Literal(Number(0.0))))))),
                ],
                actions: [
                    VariableSet((key: "gate", expression: Term(Factor(Literal(Number(5.0)))))),
                    Outcome((outcome: Victory, message: Some("FINISHED"))),
                ],
            ),
            (
                name: OnDestroyed,
                filters: [
                    Entity((id: Some("spaceship_1"))),
                    Expression((LessThan(Term(Factor(Name("gate"))), Term(Factor(Literal(Number(5.0))))))),
                ],
                actions: [
                    Outcome((outcome: Defeat, message: Some("WRECKED"))),
                    NextScenario((scenario_id: "scenario_1", linger: true)),
                ],
            ),
        ],
    )),
]"#;

/// The gate ids in course order, paired with the `gate` value each is armed at.
const GATE_CHAIN: [(&str, f64); 4] = [
    ("gate_1", 1.0),
    ("gate_2", 2.0),
    ("gate_3", 3.0),
    ("finish_1", 4.0),
];

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

// --- app harness ------------------------------------------------------------

fn course_app(scenario: &ScenarioConfig) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.init_resource::<CurrentOutcome>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        app.world_mut().spawn(event.build_handler());
    }
    app
}

fn seed_var(app: &mut App, key: &str, value: f64) {
    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable(key.to_string(), VariableLiteral::Number(value));
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

/// The course armed at its start, the way OnStart seeds it.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = course_app(scenario);
    seed_var(&mut app, "gate", 1.0);
    seed_var(&mut app, "penalty", 0.0);
    app
}

// --- the sequencer ----------------------------------------------------------

#[test]
fn gates_advance_only_in_order_and_only_for_the_named_ship() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    // Delivery guard: nothing advances on its own.
    app.update();
    assert_eq!(number_var(&app, "gate"), Some(1.0));

    // Skipping ahead is inert: gate 3 while armed at 1.
    enter(&mut app, "gate_3", "spaceship_1");
    assert_eq!(
        number_var(&app, "gate"),
        Some(1.0),
        "an out-of-order entry must not advance the course"
    );

    // The wrong ship through the right gate is inert (the other_id filter).
    enter(&mut app, "gate_1", "gate_2");
    assert_eq!(
        number_var(&app, "gate"),
        Some(1.0),
        "only the named ship advances the course"
    );

    // The whole chain in order, each entry arming exactly the next.
    for (id, arming) in GATE_CHAIN {
        assert_eq!(
            number_var(&app, "gate"),
            Some(arming),
            "gate is armed to {arming} before entering '{id}'"
        );
        enter(&mut app, id, "spaceship_1");
    }
    assert_eq!(
        number_var(&app, "gate"),
        Some(5.0),
        "crossing the finish latches the terminal state"
    );
}

// --- the repeatable zone ----------------------------------------------------

#[test]
fn a_zone_recounts_on_every_fresh_entry_until_the_course_closes() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    enter(&mut app, "area_1", "spaceship_1");
    assert_eq!(number_var(&app, "penalty"), Some(1.0));
    enter(&mut app, "area_1", "spaceship_1");
    assert_eq!(
        number_var(&app, "penalty"),
        Some(2.0),
        "re-entering the same zone counts again - an area is not a one-shot"
    );

    // Past the finish the zone is gated out, so a post-win pass cannot spoil a
    // banner already earned.
    seed_var(&mut app, "gate", 5.0);
    enter(&mut app, "area_1", "spaceship_1");
    assert_eq!(number_var(&app, "penalty"), Some(2.0));
}

// --- the two win branches ---------------------------------------------------

#[test]
fn the_finish_banner_is_selected_by_the_counter() {
    let scenario = scenario_from(SCENARIO_RON);

    for (penalty, banner) in [(0.0, "CLEAN RUN"), (2.0, "FINISHED")] {
        let mut app = armed_app(&scenario);
        seed_var(&mut app, "gate", 4.0);
        seed_var(&mut app, "penalty", penalty);

        app.update();
        assert_eq!(outcome_kind(&app), None, "no outcome before the finish");

        enter(&mut app, "finish_1", "spaceship_1");
        assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));
        assert_eq!(
            outcome_message(&app).as_deref(),
            Some(banner),
            "a {penalty}-penalty finish must take its own branch"
        );
    }
}

// --- the loss and its terminal gate -----------------------------------------

#[test]
fn a_wreck_before_the_finish_declares_defeat_with_a_retry() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);
    seed_var(&mut app, "gate", 3.0);

    destroy(&mut app, "spaceship_1");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    let next = app
        .world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .clone()
        .expect("a retry is queued");
    assert_eq!(
        next.scenario_id, "scenario_1",
        "the retry re-runs the course"
    );
    assert!(next.linger, "the retry lingers behind the overlay");
}

#[test]
fn a_wreck_after_the_finish_declares_nothing() {
    // The Defeat handler is gated on the terminal gate value: a death blast
    // after the win must not flip it. The test above is this one's delivery
    // guard.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);
    seed_var(&mut app, "gate", 5.0);

    destroy(&mut app, "spaceship_1");
    assert_eq!(
        outcome_kind(&app),
        None,
        "no Defeat once the course is finished"
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "no retry queued over an earned Victory"
    );
}

// --- the readout ------------------------------------------------------------

#[test]
fn on_start_binds_a_hud_readout_to_a_scenario_variable() {
    // The display half of the variable vocabulary: a named slot, the variable
    // it tracks, the format it renders in, and that it is SHOWN rather than
    // cleared.
    let scenario = scenario_from(SCENARIO_RON);
    let readout = on_start(&scenario)
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::HudReadout(config) => Some(config),
            _ => None,
        })
        .expect("OnStart shows a HudReadout");
    assert_eq!(readout.slot, "readout_1");
    assert_eq!(readout.variable, "scenario_elapsed");
    assert_eq!(readout.format, HudReadoutFormatConfig::Time);
    assert!(readout.visible, "the readout is shown, not cleared");
}
