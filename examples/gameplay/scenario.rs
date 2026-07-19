//! scenario: the scenario LANGUAGE, end to end - config in code, loaded
//! with variables, event handlers, filters and actions, and asserted live.
//!
//! The config exercises the whole event grammar: `OnStart` spawning objects
//! and seeding variables, `OnDestroyed` with an entity-type filter
//! incrementing a tally, an expression-filtered `OnDestroyed` advancing a
//! beat once the tally crosses a threshold, and an expression-filtered
//! `OnUpdate` promoting the beat again - variables, arithmetic, comparisons
//! and per-event filters all in play. Building the `ScenarioConfig` in code
//! (instead of loading a named one from `GameScenarios`, which
//! menu_newgame's boot flow covers) is the modding surface.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example scenario --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `scenario probe: variables seeded`,
//! #           `scenario probe: handlers, filters and expressions all ticked`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "scenario")]
#[command(version = "1.0.0")]
#[command(about = "Scenario language showcase: variables, events, filters and actions", long_about = None)]
struct Cli;

/// The scenario this example builds and loads. Shared with the smoke-test
/// assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "scenario_showcase";

/// How many asteroids OnStart spawns (also the loaded-object count).
const ASTEROID_COUNT: usize = 6;

/// How many asteroid kills advance the beat (the expression filter's
/// threshold).
const BEAT_KILLS: f64 = 2.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: `assert_scenario_loaded` fails the run
    // unless the code-built scenario loads under its id with real content;
    // the probe script then destroys asteroids and asserts the event
    // handlers, filters and variable expressions all actually ticked.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<ScenarioProbe>();
        app.add_plugins(nova_autopilot().input(autopilot_scenario_probe));
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        // Run-timeline recorder (inert unless NOVA_PERF_TIMELINE is set):
        // this example exercises the whole scenario language, so its recorded
        // timeline doubles as the recorder's stability probe.
        app.add_plugins(nova_probe::nova_timeline());
        // Continuous invariants (inert unless NOVA_PERF_INVARIANTS is set):
        // the beat gate and the destruction tally only ever advance in this
        // scenario's design.
        app.add_plugins(nova_probe::nova_invariants().monotonic(["beat", "rocks_destroyed"]));
        // Frame-time capture (inert unless NOVA_PERF is set): fleet-wide
        // wiring, task 20260719-210443.
        app.add_plugins(nova_probe::nova_frametime());
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the
    // load happened by OnEnter(Playing), and loading in that same schedule
    // is an unordered race against the check.
    app.add_systems(
        OnEnter(GameAssetsStates::Loaded),
        |mut commands: Commands, game_assets: Res<GameAssets>| {
            commands.trigger(LoadScenario(showcase(&game_assets)));
        },
    );
}

/// Shorthand: a literal number as a variable expression.
fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

/// Shorthand: a variable reference as a variable expression.
fn name(key: &str) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(VariableFactorNode::new_name(
        key.to_string(),
    )))
}

/// The showcase scenario: every part of the event grammar in one config.
fn showcase(game_assets: &GameAssets) -> ScenarioConfig {
    // OnStart: spawn the asteroid ring and seed the variables. Deterministic
    // ring, not random: the probe (and a reader) should see the same scene
    // every run.
    let mut start_actions: Vec<EventActionConfig> = (0..ASTEROID_COUNT)
        .map(|i| {
            let angle = i as f32 / ASTEROID_COUNT as f32 * std::f32::consts::TAU;
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: format!("rock_{i}"),
                    name: format!("Rock {i}"),
                    position: Vec3::new(angle.cos() * 40.0, 0.0, angle.sin() * 40.0 - 60.0),
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    impact_sound: Some("base/sounds/impact.wav".into()),
                    destroy_sound: Some("base/sounds/explosion.wav".into()),
                    radius: 2.0,
                    texture: game_assets.asteroid_texture.clone().into(),
                    health: 50.0,
                    surface_gravity: None,
                    invulnerable: false,
                    lock_signature: None,
                }),
            })
        })
        .collect();
    start_actions.push(EventActionConfig::VariableSet(VariableSetActionConfig {
        key: "beat".to_string(),
        expression: number(1.0),
    }));
    start_actions.push(EventActionConfig::VariableSet(VariableSetActionConfig {
        key: "rocks_destroyed".to_string(),
        expression: number(0.0),
    }));

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: start_actions,
        },
        // Every destroyed asteroid bumps the tally (entity-type filter +
        // variable arithmetic).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: None,
                type_name: Some("asteroid".to_string()),
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "rocks_destroyed".to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(
                        "rocks_destroyed".to_string(),
                    )),
                    number(1.0),
                ),
            })],
        },
        // Enough kills advance the beat (expression filters: tally reached
        // the threshold while still on beat 1).
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_greater_than(
                        name("rocks_destroyed"),
                        number(BEAT_KILLS - 1.0),
                    ),
                )),
                EventFilterConfig::Expression(ExpressionFilterConfig(
                    VariableConditionNode::new_equals(name("beat"), number(1.0)),
                )),
            ],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "beat".to_string(),
                expression: number(2.0),
            })],
        },
        // The per-frame pulse promotes beat 2 -> 3 (OnUpdate + expression
        // filter; the beat change makes it fire exactly once).
        ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_equals(name("beat"), number(2.0)),
            ))],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "beat".to_string(),
                expression: number(3.0),
            })],
        },
    ];

    ScenarioConfig {
        id: SCENARIO_ID.to_string(),
        name: "Scenario Showcase".to_string(),
        description: "Variables, events, filters and actions in one scenario.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
        ..Default::default()
    }
}

/// Stage tracker for the scenario probe.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ScenarioProbe {
    seeded_at: Option<f32>,
    destroyed: bool,
    asserted: bool,
}

/// Read a Number variable from the live event world, or panic with context.
#[cfg(feature = "debug")]
fn number_variable(world: &World, key: &str) -> f64 {
    match world.resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("scenario probe: variable {key} should be a number, got {other:?}"),
    }
}

/// Autopilot script: assert the OnStart seed, then destroy [`BEAT_KILLS`]
/// asteroids through the production damage path and assert the whole
/// handler chain ticked - the tally counted, the expression filter advanced
/// the beat, and the OnUpdate pulse promoted it again.
#[cfg(feature = "debug")]
fn autopilot_scenario_probe(world: &mut World, elapsed: f32) {
    // Backstop before the state gate: if the window is about to close and
    // the probe never completed (loading ate the window, a stage stalled),
    // fail loudly instead of vacuously passing.
    if elapsed > nova_protocol::nova_debug::harness::NOVA_AUTOPILOT_SECS - 0.3
        && !world.resource::<ScenarioProbe>().asserted
    {
        panic!("scenario probe: never completed within the autopilot window");
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    if world.resource::<ScenarioProbe>().asserted {
        return;
    }

    let Some(seeded_at) = world.resource::<ScenarioProbe>().seeded_at else {
        // Stage 1: the OnStart handlers seeded the variables.
        assert_eq!(
            number_variable(world, "beat"),
            1.0,
            "OnStart must seed beat = 1"
        );
        assert_eq!(
            number_variable(world, "rocks_destroyed"),
            0.0,
            "OnStart must seed the tally at 0"
        );
        info!("scenario probe: variables seeded");
        world.resource_mut::<ScenarioProbe>().seeded_at = Some(elapsed);
        return;
    };

    if !world.resource::<ScenarioProbe>().destroyed {
        // Stage 2: destroy exactly BEAT_KILLS rocks through the production
        // damage path. Damage lands on the asteroid's health-carrying CHILD
        // node, the way real rounds do - the id-carrying root has no Health
        // (the integrity bridge in objects/asteroid.rs documents exactly
        // this hierarchy, task 20260713-150343).
        let rock_roots: Vec<Entity> = {
            let mut q = world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
            q.iter(world)
                .filter(|(_, id)| id.0.starts_with("rock_"))
                .map(|(entity, _)| entity)
                .take(BEAT_KILLS as usize)
                .collect()
        };
        assert_eq!(
            rock_roots.len(),
            BEAT_KILLS as usize,
            "scenario probe: expected {BEAT_KILLS} rocks to destroy"
        );
        let nodes: Vec<Entity> = {
            let mut q = world.query_filtered::<(Entity, &ChildOf), With<Health>>();
            q.iter(world)
                .filter(|(_, child_of)| rock_roots.contains(&child_of.parent()))
                .map(|(entity, _)| entity)
                .collect()
        };
        assert_eq!(
            nodes.len(),
            BEAT_KILLS as usize,
            "scenario probe: each rock must carry one health node"
        );
        for node in nodes {
            world.trigger(HealthApplyDamage {
                entity: node,
                source: None,
                amount: 1e6,
            });
        }
        world.resource_mut::<ScenarioProbe>().destroyed = true;
        return;
    }

    // Stage 3: give the destroy -> event -> variable chain a beat to flush,
    // then assert every layer ticked.
    if elapsed < seeded_at + 1.5 {
        return;
    }
    assert_eq!(
        number_variable(world, "rocks_destroyed"),
        BEAT_KILLS,
        "the OnDestroyed tally handler must have counted the kills"
    );
    assert_eq!(
        number_variable(world, "beat"),
        3.0,
        "the expression filter must advance the beat (2) and the OnUpdate \
         pulse must promote it (3)"
    );
    info!("scenario probe: handlers, filters and expressions all ticked");
    world.resource_mut::<ScenarioProbe>().asserted = true;
}
