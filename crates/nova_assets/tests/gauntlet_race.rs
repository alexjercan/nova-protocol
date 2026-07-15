//! Production-faithful behavior test for the Gauntlet Run portal mod's slalom
//! gating (task 20260715-224803). It loads the ACTUAL shipped
//! `webmods/gauntlet/gauntlet.content.ron`, registers its `OnEnter` handlers
//! exactly the way the scenario loader does (`EventHandler::from(event.name)`
//! then `add_filter`/`add_action` for each), and drives the REAL area->body
//! contact bridge (`ScenarioAreaPlugin` + avian physics) - NOT hand-fired
//! events (lesson `scripted-walks-skip-the-bridges`). It proves the two things
//! that make the course *playable* rather than a static scene:
//!
//!   1. entering GATE 1's trigger area while `gate == 1` advances `gate` to 2 -
//!      the race actually progresses through the shipped data + real bridge; and
//!   2. entering GATE 2's trigger area while `gate == 1` does NOT advance `gate`
//!      - the `Expression(gate == 2)` guard makes the gates strictly sequential.
//!      A separate unguarded probe handler in that same test confirms the body
//!      really did enter the area (delivery guard: the stimulus fired, the
//!      guarded action still correctly did nothing).
//!
//! The rig mirrors `an_area_spawned_around_a_body_fires_on_enter` in
//! `crates/nova_scenario/src/objects/area.rs`: zero gravity, manual fixed steps,
//! `ScenarioAreaPlugin` only, and an area spawned AROUND an already-settled body
//! (full containment - avian still starts the fresh overlapping pair).

use core::time::Duration;

use avian3d::prelude::{Collider, ColliderDensity, Gravity, PhysicsPlugins, RigidBody, Sensor};
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};
use nova_events::prelude::{EntityId, EntityTypeName};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;

/// The exact bytes the portal ships, compiled in so the test breaks if the data
/// drifts. Relative to this source file: tests/ -> nova_assets/ -> crates/ -> root.
const GAUNTLET_RON: &str = include_str!("../../../webmods/gauntlet/gauntlet.content.ron");

/// Deserialize the shipped gauntlet content and pull out its one scenario.
fn gauntlet_scenario() -> ScenarioConfig {
    let items: Vec<Content> = ron::de::from_str(GAUNTLET_RON).expect("gauntlet RON parses");
    items
        .into_iter()
        .find_map(|c| match c {
            Content::Scenario(s) => Some(s),
            Content::Section(_) => None,
        })
        .expect("gauntlet content contains a Scenario")
}

/// The area.rs rig: headless app with real physics + the scenario area/event
/// plumbing, `gate` seeded to `start_gate` the way the scenario's OnStart would.
fn race_app(start_gate: f64) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        PhysicsPlugins::default(),
    ));
    app.insert_resource(Gravity(Vec3::ZERO));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        0.02,
    )));
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    app.init_resource::<NovaEventWorld>();
    app.init_resource::<GameObjectives>();
    app.add_plugins(ScenarioAreaPlugin);
    app.finish();

    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable("gate".to_string(), VariableLiteral::Number(start_gate));
    app
}

/// Register the scenario's real `OnEnter` handlers, exactly as `on_load_scenario`
/// does in the loader. OnStart is skipped: it spawns a full player ship from
/// section prototypes, which is not what this bridge test exercises.
fn register_on_enter_handlers(app: &mut App, scenario: &ScenarioConfig) {
    for event in scenario
        .events
        .iter()
        .filter(|e| matches!(e.name, EventConfig::OnEnter))
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

/// Spawn a settled player body at the origin (the id every gate filter keys off).
fn spawn_player(app: &mut App) {
    app.world_mut().spawn((
        EntityId::new("player_spaceship"),
        EntityTypeName::new("spaceship"),
        RigidBody::Dynamic,
        Collider::sphere(0.5),
        ColliderDensity(1.0),
        Transform::IDENTITY,
    ));
    for _ in 0..5 {
        app.update();
    }
}

/// Spawn a gate trigger area (id `gate_id`) around the origin - the exact bundle
/// `CreateScenarioObject`/the beacon's `area_radius` produces: a static sensor.
fn spawn_gate_area(app: &mut App, gate_id: &str, radius: f32) {
    app.world_mut().spawn((
        ScenarioAreaMarker,
        EntityId::new(gate_id.to_string()),
        RigidBody::Static,
        Collider::sphere(radius),
        Sensor,
        Transform::IDENTITY,
    ));
    for _ in 0..25 {
        app.update();
    }
}

fn gate_value(app: &App) -> Option<f64> {
    match app
        .world()
        .resource::<NovaEventWorld>()
        .get_variable("gate")
    {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    }
}

// The two bridge tests below are scoped to the `gate` ordering variable (the
// proof of sequencing); they intentionally do not assert the cosmetic
// ObjectiveComplete/marker actions the same handlers also perform.
#[test]
fn entering_gate_one_in_order_advances_the_race() {
    let scenario = gauntlet_scenario();
    let mut app = race_app(1.0);
    register_on_enter_handlers(&mut app, &scenario);

    spawn_player(&mut app);
    assert_eq!(
        gate_value(&app),
        Some(1.0),
        "delivery guard: race starts on gate 1, nothing has advanced it yet"
    );

    // The real GATE 1 trigger area, spawned around the player (full containment).
    spawn_gate_area(&mut app, "gauntlet_gate_1", 25.0);

    assert_eq!(
        gate_value(&app),
        Some(2.0),
        "threading GATE 1 while gate==1 must advance the race to gate 2 through \
         the real area bridge on the shipped RON"
    );
}

#[test]
fn entering_a_later_gate_out_of_order_does_not_advance() {
    let scenario = gauntlet_scenario();
    let mut app = race_app(1.0);
    register_on_enter_handlers(&mut app, &scenario);

    // Control probe: an UNGUARDED OnEnter on GATE 2, so we can prove the physical
    // enter actually fired even though the shipped (gate==2) handler stays inert.
    let mut probe = EventHandler::<NovaEventWorld>::from(EventConfig::OnEnter);
    probe.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
        id: Some("gauntlet_gate_2".to_string()),
        other_id: Some("player_spaceship".to_string()),
        ..Default::default()
    }));
    probe.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
        key: "probe_entered_gate_2".to_string(),
        expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
        )),
    }));
    app.world_mut().spawn(probe);

    spawn_player(&mut app);
    spawn_gate_area(&mut app, "gauntlet_gate_2", 25.0);

    // The stimulus really happened...
    assert!(
        matches!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("probe_entered_gate_2"),
            Some(VariableLiteral::Boolean(true))
        ),
        "delivery guard: the body must actually enter GATE 2's area"
    );
    // ...but the shipped handler's gate==2 guard kept the race on gate 1.
    assert_eq!(
        gate_value(&app),
        Some(1.0),
        "entering GATE 2 out of order (gate==1) must not advance the race"
    );
}

/// The two bridge tests above deliberately seed `gate=1` and skip `OnStart`, so
/// they cannot catch a broken OnStart. This structural check pins the OnStart
/// wiring that makes the course playable at all: without the player-ship spawn
/// there is nothing to fly, and without the `gate=1` seed every gate's
/// `Expression(gate==N)` filter evaluates to `Err(UndefinedVariable) -> false`
/// (nova_scenario `ExpressionFilterConfig::filter`) and the race soft-locks. It
/// asserts the shipped data, so a regression that drops either fails here rather
/// than shipping a green - but silently unplayable - portal mod.
#[test]
fn onstart_spawns_the_player_and_seeds_the_race() {
    let scenario = gauntlet_scenario();
    let onstart = scenario
        .events
        .iter()
        .find(|e| matches!(e.name, EventConfig::OnStart))
        .expect("gauntlet has an OnStart event");

    let spawns_player = onstart.actions.iter().any(|a| match a {
        EventActionConfig::SpawnScenarioObject(cfg) => {
            cfg.base.id == "player_spaceship"
                && matches!(cfg.kind, ScenarioObjectKind::Spaceship(_))
        }
        _ => false,
    });
    assert!(
        spawns_player,
        "OnStart must spawn a Spaceship with id `player_spaceship` - the change \
         that makes the gauntlet playable"
    );

    let gate_seed = onstart
        .actions
        .iter()
        .find_map(|a| match a {
            EventActionConfig::VariableSet(vs) if vs.key == "gate" => Some(vs),
            _ => None,
        })
        .expect("OnStart must seed the `gate` race counter");
    assert_eq!(
        gate_seed
            .expression
            .evaluate(&NovaEventWorld::default())
            .ok(),
        Some(VariableLiteral::Number(1.0)),
        "the race must start on gate 1, or every gate filter fails closed and \
         the course soft-locks"
    );
}
