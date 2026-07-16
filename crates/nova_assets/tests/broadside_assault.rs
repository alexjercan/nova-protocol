//! Production-faithful behavior tests for Broadside, the chapter-two slice
//! (task 20260708-203659). Loads the ACTUAL shipped
//! `assets/base/scenarios/broadside.content.ron`, registers its real
//! `OnDestroyed`/`OnUpdate` handlers the way the loader does, and drives the
//! act machine by firing `OnDestroyedEvent` - the same info the integrity
//! bridge emits when a ship root dies (the physical bridge itself is pinned
//! in nova_scenario/nova_gameplay; what this file owns is the SCENARIO
//! DATA's consumption of it, the arena_combat.rs division of labor).
//!
//! Structural pins ride along for what the behavior rig seeds for itself
//! (rig-supplies-precondition): the OnStart stage - player loadout, the
//! NEUTRAL hauler, the trigger area, the act/flag seeds - plus the
//! base-bundle membership (a scenario missing from base.bundle.ron ships as
//! dead data: declared-but-not-loaded) and the story chain into and out of
//! the neighboring scenarios.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    EntityId, OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_gameplay::prelude::Allegiance;
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const BROADSIDE_RON: &str = include_str!("../../../assets/base/scenarios/broadside.content.ron");
const SHAKEDOWN_RON: &str =
    include_str!("../../../assets/base/scenarios/shakedown_run.content.ron");
const ASTEROID_FIELD_RON: &str =
    include_str!("../../../assets/base/scenarios/asteroid_field.content.ron");
const BASE_BUNDLE_RON: &str = include_str!("../../../assets/base/base.bundle.ron");

fn scenario_from(ron: &str) -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ron).expect("content RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("content contains a Scenario")
}

/// Headless app with the scenario event plumbing plus the per-frame
/// `OnUpdate` pulse (production's `fire_on_update` is crate-private to
/// nova_scenario). Act-machine variables are seeded per test the way
/// OnStart would; the OnStart wiring itself is pinned structurally below.
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

/// Fire the scenario `OnDestroyed` for a ship root id - the same info the
/// integrity bridge emits - and pump the handlers + queued commands through.
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

#[test]
fn escalation_needs_both_corvettes_down_then_spawns_the_gunship() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);
    seed_var(&mut app, "corvette_a_down", 0.0);
    seed_var(&mut app, "corvette_b_down", 0.0);

    // Delivery guard: nothing advances on its own.
    app.update();
    assert_eq!(number_var(&app, "act"), Some(1.0));

    destroy(&mut app, "corvette_a");
    assert_eq!(
        number_var(&app, "corvette_a_down"),
        Some(1.0),
        "the kill handler ran (delivery guard for the act assert below)"
    );
    assert_eq!(
        number_var(&app, "act"),
        Some(1.0),
        "one corvette is not enough to escalate"
    );

    // Double OnDestroyed for the same ship (multi-collider bodies can): the
    // flag is idempotent, the act must not skip.
    destroy(&mut app, "corvette_a");
    assert_eq!(number_var(&app, "corvette_a_down"), Some(1.0));
    assert_eq!(number_var(&app, "act"), Some(1.0));

    destroy(&mut app, "corvette_b");
    assert_eq!(
        number_var(&app, "act"),
        Some(2.0),
        "both corvettes down escalates to act 2"
    );

    // The escalation's gunship spawn went through the production drain: the
    // root entity exists under its scenario id.
    let mut q = app.world_mut().query::<&EntityId>();
    assert!(
        q.iter(app.world()).any(|id| **id == *"gunship"),
        "act 2 spawns the gunship"
    );
}

#[test]
fn killing_the_gunship_declares_victory_with_no_queued_next() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 2.0);

    app.update();
    assert_eq!(outcome_kind(&app), None, "no outcome before the kill");

    destroy(&mut app, "gunship");
    assert_eq!(
        outcome_kind(&app),
        Some(ScenarioOutcomeKind::Victory),
        "the gunship kill wins the slice"
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "end of the base story: nothing queued, the overlay offers Main Menu"
    );
}

#[test]
fn player_death_declares_defeat_with_a_lingering_retry() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 1.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(outcome_kind(&app), Some(ScenarioOutcomeKind::Defeat));
    let world = app.world().resource::<NovaEventWorld>();
    let next = world.next_scenario.as_ref().expect("a retry is queued");
    assert_eq!(next.scenario_id, "broadside");
    assert!(next.linger, "the retry lingers behind the overlay");
}

/// Review R1.3: a player death AFTER the win (act 3 - the gunship's death
/// blast, a rock under the gold banner) declares NOTHING - the earned
/// Victory must not flip to Defeat. The act-1 test above is this test's
/// delivery guard: the same destroy on a live act does declare.
#[test]
fn player_death_after_the_win_declares_nothing() {
    let scenario = scenario_from(BROADSIDE_RON);
    let mut app = slice_app();
    register_non_start_handlers(&mut app, &scenario);
    seed_var(&mut app, "act", 3.0);

    destroy(&mut app, "player_spaceship");
    assert_eq!(
        outcome_kind(&app),
        None,
        "no Defeat after the win (the real flow holds Victory here)"
    );
    assert!(
        app.world()
            .resource::<NovaEventWorld>()
            .next_scenario
            .is_none(),
        "no retry queued over the earned Victory"
    );
}

/// The OnStart stage the behavior rigs seed for themselves, pinned on the
/// shipped data (rig-supplies-precondition): variables, the player loadout,
/// the neutral hauler, and the trigger area all come from OnStart.
#[test]
fn on_start_stages_the_slice() {
    let scenario = scenario_from(BROADSIDE_RON);
    assert!(!scenario.hidden, "broadside is a Scenarios-picker entry");

    let on_start = scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("broadside has an OnStart");

    let seeded: Vec<&str> = on_start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::VariableSet(v) => Some(v.key.as_str()),
            _ => None,
        })
        .collect();
    for key in ["act", "corvette_a_down", "corvette_b_down"] {
        assert!(seeded.contains(&key), "OnStart seeds '{key}'");
    }

    let ships: Vec<&ScenarioObjectConfig> = on_start
        .actions
        .iter()
        .filter_map(|a| match a {
            EventActionConfig::SpawnScenarioObject(config) => Some(config),
            _ => None,
        })
        .collect();

    let player = ships
        .iter()
        .find(|s| s.base.id == "player_spaceship")
        .expect("OnStart spawns the player");
    let ScenarioObjectKind::Spaceship(player_ship) = &player.kind else {
        panic!("player is a spaceship");
    };
    assert!(
        matches!(player_ship.controller, SpaceshipController::Player(_)),
        "player-controlled"
    );
    assert!(
        player_ship
            .sections
            .iter()
            .any(|s| matches!(&s.source, SectionSource::Prototype(p) if p == "torpedo_section")),
        "the full loadout includes a torpedo bay"
    );

    let hauler = ships
        .iter()
        .find(|s| s.base.id == "hauler")
        .expect("OnStart spawns the hauler");
    let ScenarioObjectKind::Spaceship(hauler_ship) = &hauler.kind else {
        panic!("hauler is a spaceship");
    };
    assert_eq!(
        hauler_ship.allegiance,
        Some(Allegiance::Neutral),
        "the hauler is authored NEUTRAL - no AI may target it"
    );
    assert!(
        matches!(hauler_ship.controller, SpaceshipController::None),
        "the hauler drifts (no controller)"
    );

    assert!(
        on_start.actions.iter().any(
            |a| matches!(a, EventActionConfig::CreateScenarioArea(area) if area.id == "hauler_area")
        ),
        "OnStart creates the ambush trigger area"
    );
}

/// A scenario absent from base.bundle.ron is dead data the game never loads
/// (declared-but-not-loaded).
#[test]
fn base_bundle_ships_broadside() {
    assert!(
        BASE_BUNDLE_RON.contains("scenarios/broadside.content.ron"),
        "base.bundle.ron lists broadside"
    );
}

/// The story chain, pinned at both ends: shakedown's chapter win chains into
/// broadside behind a Victory overlay, and the asteroid_field retrofits
/// (outcome review R1.8) declare their outcomes instead of switching
/// silently.
#[test]
fn story_chain_declares_outcomes_at_both_ends() {
    let shakedown = scenario_from(SHAKEDOWN_RON);
    let chapter_win = shakedown
        .events
        .iter()
        .find(|e| {
            e.actions.iter().any(|a| {
                matches!(
                    a,
                    EventActionConfig::NextScenario(next) if next.scenario_id == "broadside"
                )
            })
        })
        .expect("shakedown chains into broadside");
    assert!(
        chapter_win.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory
        )),
        "the chain rides a Victory overlay (Continue)"
    );
    let chained = chapter_win
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::NextScenario(next) => Some(next),
            _ => None,
        })
        .expect("chain action present");
    assert!(chained.linger, "the chain lingers behind the overlay");

    let field = scenario_from(ASTEROID_FIELD_RON);
    let field_death = field
        .events
        .iter()
        .find(|e| {
            matches!(e.name, EventConfig::OnDestroyed)
                && e.filters.iter().any(|f| matches!(
                    f,
                    EventFilterConfig::Entity(entity) if entity.id.as_deref() == Some("player_spaceship")
                ))
        })
        .expect("asteroid_field has a player-death handler");
    assert!(
        field_death.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Defeat
        )),
        "the sandbox death restart declares Defeat (R1.8 retrofit)"
    );

    // The OTHER half of the retrofit (slice review R1.1 - the first
    // application was lost in a retry): the zone-clear switch to
    // asteroid_next declares Victory.
    let zone_clear = field
        .events
        .iter()
        .find(|e| {
            e.actions.iter().any(|a| {
                matches!(
                    a,
                    EventActionConfig::NextScenario(next) if next.scenario_id == "asteroid_next"
                )
            })
        })
        .expect("asteroid_field has the zone-clear switch");
    assert!(
        zone_clear.actions.iter().any(|a| matches!(
            a,
            EventActionConfig::Outcome(o) if o.outcome == ScenarioOutcomeKind::Victory
        )),
        "zone-clear declares Victory (R1.8's second half, review R1.1)"
    );
}
