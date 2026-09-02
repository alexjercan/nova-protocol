//! The scenario ACT MACHINE, driven end to end on a synthetic scenario defined
//! inline in this file. Content is authored here as a RON string with generic
//! ids (`spaceship_1`, `scenario_1`), parsed through the real
//! [`Content`](nova_modding::prelude::Content) vocabulary, registered as real
//! handlers the way the loader does, and driven with the same event infos the
//! engine emits. No installed mod is read, so no mod can hold this coverage
//! hostage.
//!
//! What this pins is the ENGINE contract every act-structured scenario leans
//! on:
//!
//! 1. the defeat lifecycle (`OnDefeated` + `OnDestroyed` / `OnNeutralized`)
//!    drives scenario variables;
//! 2. a per-target one-shot guard makes a kill count ONCE, so destroying an
//!    already-neutralized wreck never double-counts;
//! 3. a DEFERRED outcome (`gate = scenario_elapsed + delay`, then a one-shot
//!    `OnUpdate` filtered on `scenario_elapsed > gate`) leaves the frame the
//!    kill landed in outcome-free and settles a beat later, once the clock is
//!    pumped;
//! 4. a Victory queues `NextScenario` with `linger`, and a Defeat queues the
//!    retry of THIS scenario;
//! 5. a multi-condition win gate stays shut while ANY condition is open;
//! 6. an act latch closes the scenario: a death after the win declares
//!    nothing, and a settled outcome is never overwritten.
//!
//! Standalone: `cargo test -p nova_assets --test scenario_act_machine`
//! (nova_assets unifies the serde feature across the workspace).

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, GameEventsPlugin, OnDefeatedEvent, OnDefeatedEventInfo, OnDestroyedEvent,
    OnDestroyedEventInfo, OnNeutralizedEvent, OnNeutralizedEventInfo, OnUpdateEvent,
    OnUpdateEventInfo,
};
use nova_gameplay::prelude::GameObjectives;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

/// The fixture: a two-hostile clear with a station, a deferred Victory and a
/// retry-on-death Defeat - the shape every act-structured scenario shares,
/// with none of anybody's story on it.
///
/// Acts: 1 = fighting, 2 = settled. `kills` counts broken hostiles, each
/// guarded by its own `<id>_down` one-shot; `station_down` is the second win
/// condition; `win_gate`/`win_said` defer the overlay a beat behind the kill.
const SCENARIO_RON: &str = r#"[
    Scenario((
        id: "scenario_1",
        name: "Scenario One",
        description: "Synthetic act-machine fixture.",
        cubemap: "dep://base/textures/cubemap.png",
        watches: [
            (variable: "scenario_elapsed", query: Scenario((property: Elapsed))),
        ],
        events: [
            (
                name: OnStart,
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "kills", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "spaceship_2_down", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "spaceship_3_down", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "station_down", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "win_gate", expression: Term(Factor(Literal(Number(0.0)))))),
                    VariableSet((key: "win_said", expression: Term(Factor(Literal(Number(0.0)))))),
                    Objective((id: "objective_1", message: "Break the pair, then the station.")),
                    SpawnScenarioObject((
                        base: (id: "spaceship_1", name: "Ship One", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((
                            controller: Player(()),
                            hull: Inline((sections: [])),
                        )),
                    )),
                    SpawnScenarioObject((
                        base: (id: "spaceship_2", name: "Ship Two", position: (0.0, 0.0, -6000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((controller: AI(()), hull: Inline((sections: [])))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "spaceship_3", name: "Ship Three", position: (0.0, 0.0, -6200.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((controller: AI(()), hull: Inline((sections: [])))),
                    )),
                    SpawnScenarioObject((
                        base: (id: "station_1", name: "Station One", position: (0.0, 0.0, -9000.0), rotation: (0.0, 0.0, 0.0, 1.0)),
                        kind: Spaceship((controller: AI(()), hull: Inline((sections: [])))),
                    )),
                ],
            ),
            // Each hostile has ONE counting handler per lifecycle end, both
            // guarded by the same `<id>_down` one-shot: a neutralized ship that
            // is later destroyed must not count twice.
            (
                name: OnDefeated,
                filters: [
                    Entity((id: Some("spaceship_2"))),
                    Expression((Equal(Term(Factor(Name("spaceship_2_down"))), Term(Factor(Literal(Number(0.0))))))),
                ],
                actions: [
                    VariableSet((key: "spaceship_2_down", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "kills", expression: Add(Factor(Name("kills")), Term(Factor(Literal(Number(1.0))))))),
                ],
            ),
            (
                name: OnDefeated,
                filters: [
                    Entity((id: Some("spaceship_3"))),
                    Expression((Equal(Term(Factor(Name("spaceship_3_down"))), Term(Factor(Literal(Number(0.0))))))),
                ],
                actions: [
                    VariableSet((key: "spaceship_3_down", expression: Term(Factor(Literal(Number(1.0)))))),
                    VariableSet((key: "kills", expression: Add(Factor(Name("kills")), Term(Factor(Literal(Number(1.0))))))),
                ],
            ),
            (
                name: OnDefeated,
                filters: [
                    Entity((id: Some("station_1"))),
                    Expression((Equal(Term(Factor(Name("station_down"))), Term(Factor(Literal(Number(0.0))))))),
                ],
                actions: [
                    VariableSet((key: "station_down", expression: Term(Factor(Literal(Number(1.0)))))),
                ],
            ),
            // The win ARMS on the last condition and speaks; the overlay is a
            // separate handler a beat later, so no Outcome shares a frame with
            // a StoryMessage.
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(1.0))))))),
                    Expression((Equal(Term(Factor(Name("kills"))), Term(Factor(Literal(Number(2.0))))))),
                    Expression((Equal(Term(Factor(Name("station_down"))), Term(Factor(Literal(Number(1.0))))))),
                ],
                actions: [
                    VariableSet((key: "act", expression: Term(Factor(Literal(Number(2.0)))))),
                    VariableSet((key: "win_gate", expression: Add(Factor(Name("scenario_elapsed")), Term(Factor(Literal(Number(3.0))))))),
                    ObjectiveComplete((id: "objective_1")),
                    StoryMessage((speaker: "Speaker One", text: "Field is clear.")),
                ],
            ),
            (
                name: OnUpdate,
                filters: [
                    Expression((Equal(Term(Factor(Name("act"))), Term(Factor(Literal(Number(2.0))))))),
                    Expression((Equal(Term(Factor(Name("win_said"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("win_gate"))), Term(Factor(Literal(Number(0.0))))))),
                    Expression((GreaterThan(Term(Factor(Name("scenario_elapsed"))), Term(Factor(Name("win_gate")))))),
                ],
                actions: [
                    VariableSet((key: "win_said", expression: Term(Factor(Literal(Number(1.0)))))),
                    Outcome((outcome: Victory, message: Some("CLEARED"))),
                    NextScenario((scenario_id: "scenario_2", linger: true)),
                ],
            ),
            // Defeat is gated on the LIVE act, so a death under the banner
            // cannot flip an earned win.
            (
                name: OnDefeated,
                filters: [
                    Entity((id: Some("spaceship_1"))),
                    Expression((LessThan(Term(Factor(Name("act"))), Term(Factor(Literal(Number(2.0))))))),
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

fn seeded_keys(event: &ScenarioEventConfig) -> Vec<&str> {
    event
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(set) => Some(set.key.as_str()),
            _ => None,
        })
        .collect()
}

// --- app harness ------------------------------------------------------------

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

/// Register every handler EXCEPT OnStart, the way the loader does. OnStart is
/// replayed by hand in [`armed_app`] so each walk starts from a known machine
/// state; that OnStart really seeds it is pinned structurally below
/// (rig-supplies-precondition).
fn register_non_start_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| !matches!(e.name, EventConfig::OnStart))
    {
        app.world_mut().spawn(event.build_handler());
    }
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
    app.update();
    app.update();
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
    app.update();
    app.update();
}

/// Stamp the scenario clock and tick, so a clock-gated handler can fire. The
/// rig runs no clock of its own, so `scenario_elapsed` reads whatever was last
/// stamped (the time-gated-content-needs-a-clock-pump lesson).
fn pump_clock(app: &mut App, to_secs: f64) {
    seed_var(app, "scenario_elapsed", to_secs);
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

fn queued_next(app: &App) -> Option<(String, bool)> {
    app.world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .map(|next| (next.scenario_id.clone(), next.linger))
}

/// The machine seeded the way OnStart does, with the clock given a defined
/// base so a `scenario_elapsed + delay` stamp lands on a real deadline.
fn armed_app(scenario: &ScenarioConfig) -> App {
    let mut app = slice_app();
    register_non_start_handlers(&mut app, scenario);
    for (key, value) in [
        ("act", 1.0),
        ("kills", 0.0),
        ("spaceship_2_down", 0.0),
        ("spaceship_3_down", 0.0),
        ("station_down", 0.0),
        ("win_gate", 0.0),
        ("win_said", 0.0),
        ("scenario_elapsed", 30.0),
    ] {
        seed_var(&mut app, key, value);
    }
    app
}

/// Break the whole defence: both hostiles and the station.
fn clear_the_field(app: &mut App) {
    destroy(app, "spaceship_2");
    destroy(app, "spaceship_3");
    destroy(app, "station_1");
}

// --- structural pin ---------------------------------------------------------

#[test]
fn on_start_seeds_every_gate_the_walks_assume() {
    // The walks seed the machine by hand; this pins that OnStart establishes
    // it. An undefined gate variable fails its filter closed forever, so a
    // one-shot guard that OnStart forgets is a permanently dead handler.
    let scenario = scenario_from(SCENARIO_RON);
    let start = on_start(&scenario);
    let keys = seeded_keys(start);
    for key in [
        "act",
        "kills",
        "spaceship_2_down",
        "spaceship_3_down",
        "station_down",
        "win_gate",
        "win_said",
    ] {
        assert!(keys.contains(&key), "OnStart must seed '{key}'");
    }
}

// --- the counting lifecycle -------------------------------------------------

#[test]
fn each_defeat_counts_once_however_the_ship_died() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    destroy(&mut app, "spaceship_2");
    assert_eq!(number_var(&app, "kills"), Some(1.0));
    assert_eq!(outcome_kind(&app), None, "one kill is not the field");

    // A neutralized ship counts on the SAME `OnDefeated` lifecycle...
    neutralize(&mut app, "spaceship_3");
    assert_eq!(number_var(&app, "kills"), Some(2.0));
    assert_eq!(number_var(&app, "spaceship_3_down"), Some(1.0));

    // ...and destroying that wreck afterwards must not count it again.
    destroy(&mut app, "spaceship_3");
    assert_eq!(
        number_var(&app, "kills"),
        Some(2.0),
        "destroying a neutralized wreck double-counted the kill"
    );
}

// --- the deferred outcome ---------------------------------------------------

#[test]
fn the_victory_overlay_lands_a_beat_after_the_win_arms() {
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    clear_the_field(&mut app);
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "the last kill closes the act immediately"
    );
    assert_eq!(
        outcome_kind(&app),
        None,
        "the win line plays first; the overlay is a beat behind"
    );
    let win_gate = number_var(&app, "win_gate").expect("the win stamps a deadline");
    assert!(win_gate > 0.0, "the deadline is stamped off the live clock");

    // Time short of the deadline settles nothing.
    pump_clock(&mut app, win_gate - 1.0);
    assert_eq!(outcome_kind(&app), None, "the breather has not elapsed");

    pump_clock(&mut app, win_gate + 1.0);
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the deferred overlay lands once the breather elapses"
    );
    assert_eq!(outcome_message(&app).as_deref(), Some("CLEARED"));
    assert_eq!(number_var(&app, "win_said"), Some(1.0), "one-shot spent");
    assert_eq!(
        queued_next(&app),
        Some(("scenario_2".to_string(), true)),
        "the win chains onward and lingers"
    );
}

#[test]
fn a_partial_clear_never_arms_the_win() {
    let scenario = scenario_from(SCENARIO_RON);

    // Both hostiles down, the station still standing.
    let mut app = armed_app(&scenario);
    destroy(&mut app, "spaceship_2");
    destroy(&mut app, "spaceship_3");
    pump_clock(&mut app, 200.0);
    assert_eq!(number_var(&app, "kills"), Some(2.0));
    assert_eq!(number_var(&app, "act"), Some(1.0), "the act stays open");
    assert_eq!(outcome_kind(&app), None, "the station must fall too");

    // The station down, a hostile still up.
    let mut app = armed_app(&scenario);
    destroy(&mut app, "station_1");
    destroy(&mut app, "spaceship_2");
    pump_clock(&mut app, 200.0);
    assert_eq!(number_var(&app, "station_down"), Some(1.0));
    assert_eq!(
        outcome_kind(&app),
        None,
        "a surviving hostile denies the win"
    );
}

// --- the loss and the latch -------------------------------------------------

#[test]
fn a_player_death_declares_defeat_and_requeues_this_scenario() {
    let scenario = scenario_from(SCENARIO_RON);

    for kill in [destroy as fn(&mut App, &str), neutralize] {
        let mut app = armed_app(&scenario);
        kill(&mut app, "spaceship_1");
        assert_eq!(
            outcome_kind(&app),
            Some(ScenarioOutcomeKind::Defeat),
            "losing the player loses the run"
        );
        assert_eq!(
            queued_next(&app),
            Some(("scenario_1".to_string(), true)),
            "the retry is THIS scenario, and it lingers"
        );
    }
}

#[test]
fn a_death_after_the_win_declares_nothing() {
    // The act latch is what makes an earned win safe: debris under the gold
    // banner must not flip it (the act-gating lesson). The live-act Defeat
    // test above is this test's delivery guard.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = armed_app(&scenario);

    clear_the_field(&mut app);
    let win_gate = number_var(&app, "win_gate").expect("the win stamps a deadline");
    pump_clock(&mut app, win_gate + 1.0);
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Victory));

    destroy(&mut app, "spaceship_1");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "a late death cannot overwrite the settled Victory"
    );
    assert_eq!(
        queued_next(&app),
        Some(("scenario_2".to_string(), true)),
        "and cannot replace the queued chain with a retry"
    );
}

#[test]
fn a_death_in_a_closed_act_declares_nothing_at_all() {
    // Closed act, overlay not yet up (the window between the win arming and
    // its deferred overlay): no Defeat, no retry.
    let scenario = scenario_from(SCENARIO_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 2.0);
    seed_var(&mut app, "win_gate", 0.0);
    seed_var(&mut app, "win_said", 0.0);

    destroy(&mut app, "spaceship_1");
    assert_eq!(outcome_kind(&app), None, "no Defeat over a closed act");
    assert_eq!(queued_next(&app), None, "no retry over a closed act");
}
