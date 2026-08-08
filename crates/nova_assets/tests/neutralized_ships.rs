//! Production-faithful scenario tests for the NEUTRALIZED (combat-dead) signal.
//! A ship that was armed and has lost all working weapons AND thrusters fires
//! `OnNeutralizedEvent` instead of being destroyed; the shipped scenarios carry
//! `OnNeutralized` sibling handlers so a beaten ship counts as beaten without
//! its hull being ground to zero. This loads the ACTUAL shipped RON, registers
//! its real handlers the way the loader does, and drives the act machine by
//! firing `OnNeutralizedEvent` - the same info the gameplay neutralize system
//! emits.
//!
//! The physical predicate (weapons + thrusters gone => the event) is pinned in
//! `nova_gameplay::integrity::neutralize`; what this file owns is the SCENARIO
//! DATA's consumption of the event: an enemy kill-objective completes on
//! neutralize, the player's neutralize is an immediate terminal Defeat, and the
//! act guards give once-semantics so a later real destruction cannot double-fire.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nova_events::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, OnNeutralizedEvent,
    OnNeutralizedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::GameObjectives;
use nova_scenario::prelude::*;

const BROADSIDE_RON: &str = include_str!("../../../assets/base/scenarios/broadside.content.ron");
const BROADSIDE_GUNSHIP_RON: &str =
    include_str!("../../../assets/base/scenarios/broadside_gunship.content.ron");
const SHAKEDOWN_RON: &str =
    include_str!("../../../assets/base/scenarios/shakedown_run.content.ron");
const LIFELINE_RON: &str = include_str!("../../../assets/base/scenarios/lifeline.content.ron");

fn scenario_from(ron: &str) -> ScenarioConfig {
    let items: Vec<nova_modding::prelude::Content> =
        ron::de::from_str(ron).expect("content parses");
    items
        .into_iter()
        .find_map(|c| match c {
            nova_modding::prelude::Content::Scenario(s) => Some(s),
            _ => None,
        })
        .expect("content contains a Scenario")
}

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

/// Fire the scenario `OnNeutralized` for a ship id - the same info the gameplay
/// neutralize system emits - and pump the handlers + queued commands through.
fn neutralize(app: &mut App, id: &str) {
    let info = OnNeutralizedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<OnNeutralizedEvent>(info.clone());
        })
        .expect("fire OnNeutralized");
    app.update();
    app.update();
}

/// The destroy counterpart, for the once-semantics cross-check.
fn destroy(app: &mut App, id: &str) {
    let info = nova_events::prelude::OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "spaceship".to_string(),
    };
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.fire::<nova_events::prelude::OnDestroyedEvent>(info.clone());
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

/// The immediate annoyance fixed at scenario level: neutralizing an enemy
/// kill-objective completes it exactly like destroying it, so a ship whose guns
/// and engines are gone counts as down without grinding its hull to zero. Both
/// corvettes neutralized wins part one and chains onward.
#[test]
fn neutralizing_both_corvettes_wins_part_one() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "corvette_a_down", 0.0);
    seed_var(&mut app, "corvette_b_down", 0.0);
    seed_var(&mut app, "hauler_lost", 0.0);

    // Delivery guard: nothing advances on its own.
    app.update();
    assert_eq!(number_var(&app, "act"), Some(1.0));

    neutralize(&mut app, "corvette_a");
    assert_eq!(
        number_var(&app, "corvette_a_down"),
        Some(1.0),
        "neutralizing a corvette marks it down (delivery guard for the act assert)"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(1.0),
        "one corvette is not enough"
    );

    neutralize(&mut app, "corvette_b");
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "both corvettes neutralized wins part one"
    );
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the broken ambush is a Victory beat"
    );
}

/// The gunship boss: neutralizing it declares Victory and chains into Lifeline,
/// the same terminal beat destroying it would - and the act guard gives
/// once-semantics, so a later real destruction of the drifting wreck cannot
/// re-open the win gate.
#[test]
fn neutralizing_the_gunship_wins_and_does_not_double_fire() {
    let scenario = scenario_from(BROADSIDE_GUNSHIP_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "hauler_lost", 0.0);

    app.update();
    assert_eq!(outcome_kind(&app), None, "no outcome before the neutralize");

    neutralize(&mut app, "gunship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the gunship neutralize wins the slice"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "the win advanced the act"
    );
    let next = app
        .world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .expect("chapter three is queued")
        .scenario_id
        .clone();
    assert_eq!(next, "lifeline", "the chain enters Lifeline");

    // Once-semantics: the wreck is later blown up for real. The gunship's
    // OnDestroyed win gate is act == 1; the neutralize already advanced to 2,
    // so the destruction declares nothing new.
    destroy(&mut app, "gunship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "a later real destruction does not re-declare / overwrite the win"
    );
    assert_eq!(number_var(&app, "act"), Some(2.0), "the act does not skip");
}

/// The player consequence: being neutralized is an IMMEDIATE Defeat that closes the act, mirroring the
/// player-death path so the last-write-wins outcome cannot be overwritten.
#[test]
fn neutralizing_the_player_is_a_terminal_defeat() {
    // The guarded part: broadside_gunship's player handler sets terminal act 3.
    let scenario = scenario_from(BROADSIDE_GUNSHIP_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);

    neutralize(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "losing all weapons + thrusters is a player Defeat"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the player's neutralize is terminal (closes the win gate)"
    );
    let next = app
        .world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .expect("a retry is queued")
        .scenario_id
        .clone();
    assert_eq!(next, "broadside_gunship", "the retry is the current part");

    // And the simple (unguarded) case still declares Defeat.
    let shakedown = scenario_from(SHAKEDOWN_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &shakedown);
    neutralize(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Defeat),
        "shakedown's player neutralize is also a Defeat"
    );
}

/// Lifeline is the biggest scenario and carries the most siblings (7 raiders +
/// the act-1-guarded player Defeat). Its raider handlers are unguarded flag
/// sets: neutralizing a raider marks it down exactly like destroying it, and a
/// player neutralize on the live act is a terminal Defeat that retries the lane.
#[test]
fn lifeline_raiders_and_player_neutralize_as_expected() {
    let scenario = scenario_from(LIFELINE_RON);

    // A first-wave raider: neutralize marks its kill flag down.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "r1a_down", 0.0);
    app.update();
    assert_eq!(
        number_var(&app, "r1a_down"),
        Some(0.0),
        "nothing on its own"
    );

    neutralize(&mut app, "raider_1a");
    assert_eq!(
        number_var(&app, "r1a_down"),
        Some(1.0),
        "neutralizing a raider marks it down like a kill"
    );

    // The player neutralize on the live (act 1) part: terminal Defeat, retry.
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    neutralize(&mut app, "player_spaceship");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    assert_eq!(
        number_var(&app, "act"),
        Some(3.0),
        "the player's neutralize closes the act (last-write-wins guard)"
    );
    let next = app
        .world()
        .resource::<NovaEventWorld>()
        .next_scenario
        .as_ref()
        .expect("a retry is queued")
        .scenario_id
        .clone();
    assert_eq!(next, "lifeline", "the retry is the lane itself");
}
