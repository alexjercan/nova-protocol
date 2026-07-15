//! Production-faithful behavior test for the Demo Mod Arena's target-clear
//! win logic (task 20260715-224812). It loads the ACTUAL shipped
//! `assets/mods/demo/mod.content.ron`, registers the scenario's real
//! `OnDestroyed` + `OnUpdate` handlers the way the loader does, and drives the
//! win state machine by firing `OnDestroyedEvent` for each target id.
//!
//! Why firing the event by hand is faithful here: the *physical* bridge - a
//! turret kills an asteroid, whose node death fires `OnDestroyed` under the
//! ROOT's id - is already owned by nova_scenario's
//! `destroying_an_asteroid_node_fires_on_destroyed_for_the_root`. Those systems
//! are crate-private to nova_scenario, so this test cannot re-drive them; what
//! it owns instead is the ARENA DATA's consumption of that event: three per-id
//! `OnDestroyed` handlers each increment `destroyed`, and a one-shot `OnUpdate`
//! (gated `destroyed == 3 && arena_done == 0`) flips the win. The fired
//! `OnDestroyedEventInfo { id, type_name }` is exactly what the pinned bridge
//! emits (`ENTITY_ID_COMPONENT_NAME == "id"`), so the id filter matches the
//! same data path production produces.
//!
//! A second test pins the OnStart wiring the behavior rig seeds for itself
//! (player ship, three targets, `destroyed=0`) - a rig that supplies its own
//! precondition is blind to that precondition regressing.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_common_systems::prelude::{
    CommandsGameEventExt, EventHandler, GameEventsPlugin, GameObjectives,
};
use nova_events::prelude::{
    OnDestroyedEvent, OnDestroyedEventInfo, OnUpdateEvent, OnUpdateEventInfo,
};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

const ARENA_RON: &str = include_str!("../../../assets/mods/demo/mod.content.ron");

/// The shipped demo-mod content carries a Section overlay AND the arena
/// Scenario; pull out the scenario.
fn arena_scenario() -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(ARENA_RON).expect("demo mod RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("demo mod content contains a Scenario")
}

/// Headless app with the scenario event plumbing plus a per-frame `OnUpdate`
/// pulse (production's `fire_on_update`, which is crate-private to
/// nova_scenario). `destroyed`/`arena_done` are seeded the way OnStart would.
fn arena_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.add_systems(Update, |mut commands: Commands| {
        commands.fire::<OnUpdateEvent>(OnUpdateEventInfo);
    });

    {
        let mut world = app.world_mut().resource_mut::<NovaEventWorld>();
        world.insert_variable("destroyed".to_string(), VariableLiteral::Number(0.0));
        world.insert_variable("arena_done".to_string(), VariableLiteral::Number(0.0));
    }
    app
}

/// Register every handler EXCEPT OnStart (whose actions spawn asset-heavy
/// objects), exactly as `on_load_scenario` builds them.
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

/// Fire the scenario `OnDestroyed` for a target id - the same info the integrity
/// bridge emits for a destroyed asteroid root - and pump the handler through.
fn destroy_target(app: &mut App, id: &str) {
    let info = OnDestroyedEventInfo {
        id: id.to_string(),
        type_name: "asteroid".to_string(),
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

#[test]
fn destroying_all_three_targets_clears_the_arena() {
    let scenario = arena_scenario();
    let mut app = arena_app();
    register_non_start_handlers(&mut app, &scenario);

    // Delivery guard: nothing has advanced the win state yet.
    app.update();
    assert_eq!(number_var(&app, "destroyed"), Some(0.0));
    assert_eq!(
        number_var(&app, "arena_done"),
        Some(0.0),
        "the arena is not cleared before any target dies"
    );

    destroy_target(&mut app, "arena_target_1");
    destroy_target(&mut app, "arena_target_2");
    assert_eq!(
        number_var(&app, "destroyed"),
        Some(2.0),
        "each target's OnDestroyed increments the kill counter"
    );
    assert_eq!(
        number_var(&app, "arena_done"),
        Some(0.0),
        "two of three down is not a clear"
    );

    destroy_target(&mut app, "arena_target_3");
    // A few frames for the per-frame OnUpdate win gate to catch destroyed==3.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        number_var(&app, "destroyed"),
        Some(3.0),
        "all three counted"
    );
    assert_eq!(
        number_var(&app, "arena_done"),
        Some(1.0),
        "clearing all three targets must trip the one-shot win"
    );
}

#[test]
fn win_gate_is_one_shot_and_needs_the_full_count() {
    let scenario = arena_scenario();
    let mut app = arena_app();
    register_non_start_handlers(&mut app, &scenario);

    // Only two down: the win must never fire, no matter how many frames pass.
    destroy_target(&mut app, "arena_target_1");
    destroy_target(&mut app, "arena_target_2");
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        number_var(&app, "arena_done"),
        Some(0.0),
        "the win gate requires destroyed == 3"
    );

    // Third down trips it exactly once and it stays tripped.
    destroy_target(&mut app, "arena_target_3");
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        number_var(&app, "arena_done"),
        Some(1.0),
        "win tripped once"
    );
    assert_eq!(
        number_var(&app, "destroyed"),
        Some(3.0),
        "the counter does not run past 3 (no more targets fire OnDestroyed)"
    );
}

#[test]
fn onstart_spawns_the_player_targets_and_seeds_the_counter() {
    let scenario = arena_scenario();
    let onstart = scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("arena has an OnStart event");

    let spawns_player = onstart.actions.iter().any(|a| match a {
        EventActionConfig::SpawnScenarioObject(cfg) => {
            cfg.base.id == "player_spaceship"
                && matches!(cfg.kind, ScenarioObjectKind::Spaceship(_))
        }
        _ => false,
    });
    assert!(
        spawns_player,
        "OnStart must spawn a turreted `player_spaceship` - without it there is \
         nothing to shoot the targets with"
    );

    let target_count = onstart
        .actions
        .iter()
        .filter(|a| match a {
            EventActionConfig::SpawnScenarioObject(cfg) => {
                cfg.base.id.starts_with("arena_target_")
                    && matches!(cfg.kind, ScenarioObjectKind::Asteroid(_))
            }
            _ => false,
        })
        .count();
    assert_eq!(
        target_count, 3,
        "OnStart must spawn exactly the three destructible targets the win \
         counter expects"
    );

    let seeds_counter = onstart.actions.iter().any(|a| match a {
        EventActionConfig::VariableSet(vs) if vs.key == "destroyed" => {
            vs.expression.evaluate(&NovaEventWorld::default()).ok()
                == Some(VariableLiteral::Number(0.0))
        }
        _ => false,
    });
    assert!(
        seeds_counter,
        "OnStart must seed `destroyed = 0`, or the win gate can never reach a \
         known count"
    );
}
