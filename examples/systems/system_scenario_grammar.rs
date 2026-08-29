//! system_scenario_grammar: the scenario LANGUAGE, end to end - config in code,
//! loaded with variables, event handlers, filters and actions, and asserted
//! live over repeated rounds.
//!
//! The config exercises the event grammar broadly: `OnStart` spawning objects,
//! creating a trigger volume, posting objectives, binding a HUD readout and
//! seeding variables; `OnDestroyed` with an entity-type filter incrementing a
//! tally; an expression-filtered `OnDestroyed` advancing a beat once the tally
//! crosses a threshold; an expression-filtered `OnUpdate` promoting the beat
//! again and, with arithmetic over the scenario's own variables, closing each
//! ROUND; `OnEnter`/`OnExit` on the trigger volume; `OnNeutralized` on a
//! disarmed escort; a keyed timer and filtered `OnTimerEnd`; and
//! `ObjectiveComplete` when the ring is finally clear.
//!
//! It also measures `once`: the transitions carry it instead of a latch
//! filter, and one UNFILTERED `once` OnUpdate handler counts its own runs.
//! Without retirement that counter would read the frame count of the whole
//! run; the report asserts it reads 1.
//!
//! Every beat waits on what the SCENARIO wrote - its own variables - rather
//! than on a wall-clock settle, so a slow load or an llvmpipe frame stall
//! delays the run instead of truncating it, and a beat that never resolves
//! error-exits naming itself.
//!
//! Building the `ScenarioConfig` in code (instead of loading a named one from
//! `GameScenarios`, which system_menu_boot's boot flow covers) is the modding
//! surface, and it is what keeps this fixture story-free.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_scenario_grammar --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `scenario probe: variables seeded`,
//! #           `scenario probe: round 1 ticked ...`,
//! #           `scenario probe: handlers, filters and expressions all ticked`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_authoring::scenario_helpers::prelude::*;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_scenario_grammar")]
#[command(version = "1.0.0")]
#[command(about = "Scenario language showcase: variables, events, filters and actions over repeated rounds. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario this example builds and loads. Shared with the smoke-test
/// assertion so both agree on what "loaded" means.
const SCENARIO_ID: &str = "scenario_showcase";

/// How many rounds the script works the ring down in. Each round is a full
/// pass through the grammar: kills counted, the round arithmetic closing on
/// the scenario's own variables, assertions gated on both.
const ROUNDS: usize = 3;

/// Kills per round - also the expression filter's beat threshold, so the
/// beat chain completes inside round 1.
const KILLS_PER_ROUND: usize = 2;

/// How many asteroids OnStart spawns: exactly the rounds' worth, so the last
/// round leaves the ring empty and the clear-the-ring objective completes.
const ASTEROID_COUNT: usize = ROUNDS * KILLS_PER_ROUND;

/// The rock the trigger volume is built around, and the one the script pushes
/// back out of it. The area beats run before any round, so no round can have
/// destroyed it yet.
const AREA_ROCK: usize = ASTEROID_COUNT - 1;

/// The trigger volume's scenario id and radius. Small enough that no other
/// ring member is inside it (the ring members sit ~42 u apart).
const AREA_ID: &str = "rock_ring_trigger";
const AREA_RADIUS: f32 = 8.0;

/// The escort: an armed, engined ship that is DISARMED rather than destroyed,
/// so the run reaches `OnNeutralized` - a ship out of the fight with its hull
/// intact, which no amount of `OnDestroyed` covers.
const ESCORT_ID: &str = "escort";
const ESCORT_GUNS: &str = "escort_guns";
const ESCORT_DRIVE: &str = "escort_drive";

/// The two objectives: one completed by the neutralize, one by the cleared
/// ring.
const OBJECTIVE_DISARM: &str = "disarm_the_escort";
const OBJECTIVE_RING: &str = "clear_the_ring";

/// The first step's name, so `loop_from` restarts the cycle at the scene
/// reload without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "load the grammar showcase";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: `assert_scenario_loaded` fails the run
    // unless the code-built scenario loads under its id with real content; the
    // beats below then work the grammar and assert every layer ticked.
    #[cfg(feature = "debug")]
    {
        let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            // OnStart seeds `round` and `rocks_destroyed`, so waiting for both
            // is also the gate a looped scene reload needs: the old cycle's
            // variables outlive the load replacing them (ScenarioLoaded fires
            // BEFORE OnStart runs, task 20260720-014142).
            .step(LOAD_STEP)
            .enter(GameStates::Loading)
            .until(and(
                scenario_variable_is("round", 1.0),
                scenario_variable_is("rocks_destroyed", 0.0),
            ))
            .deadline(30.0)
            .add()
            // The scene is live again: close the reload interval so a frame
            // capture excludes it. A no-op on the first cycle.
            .step("close the reload interval")
            .on_enter(nova_probe::capture_reload_end)
            .add()
            .step("check what OnStart seeded")
            .on_enter(assert_seed)
            .add()
            // OnEnter: the volume is created around a rock that is already
            // there, and the fresh contact pair is what fires the event.
            .step("see the scenario timer finish")
            .until(scenario_variable_is("timer_ended", 1.0))
            .deadline(5.0)
            .add()
            .step("see the trigger volume swallow a rock")
            .until(scenario_variable_is("area_entries", 1.0))
            .deadline(20.0)
            .add()
            // OnExit: the only half of the pair that needs the body to MOVE,
            // so the script pushes the rock out rather than despawning it (a
            // despawned collider fires no CollisionEnd).
            .step("push the rock back out of the trigger volume")
            .on_enter(push_rock_out)
            .until(scenario_variable_is("area_exits", 1.0))
            .deadline(20.0)
            .add()
            // OnNeutralized: strip the escort's guns and drive and let the
            // integrity layer decide it is out of the fight. The handler
            // completes an objective, so ObjectiveComplete rides along.
            .step("disarm the escort into a neutralize")
            .on_enter(disarm_escort)
            .until(scenario_variable_is("escort_neutralized", 1.0))
            .deadline(30.0)
            .add();

        // The rounds. Each one destroys its share of the ring and advances on
        // the scenario's OWN arithmetic closing the round (`round` counts up
        // through `rocks_destroyed > round * KILLS_PER_ROUND - 1`), never on a
        // settle window.
        for round in 1..=ROUNDS {
            script = script
                .step(format!("round {round}: work the ring down"))
                .on_enter(destroy_a_round_of_rocks)
                .until(and(
                    scenario_variable_is("rocks_destroyed", (round * KILLS_PER_ROUND) as f64),
                    scenario_variable_is("round", (round + 1) as f64),
                ))
                .deadline(20.0)
                .add()
                .step(format!("round {round}: the grammar ticked"))
                .on_enter(report_round(round))
                .add();
        }

        app.add_plugins(nova_screenshot(
            script
                // The emptied ring completes its objective through an
                // expression-filtered OnUpdate latch.
                .step("the cleared ring completes its objective")
                .until(scenario_variable_is("ring_cleared", 1.0))
                .deadline(20.0)
                .add()
                // The script's last beat: `nova_screenshot` appends the
                // capture beat behind it, and the driver reports done after
                // that - so a clean pass ends here rather than idling out a
                // runway.
                .step("report the whole grammar")
                .on_enter(report_grammar)
                .add()
                // Enrolled in capture looping (task 20260720-000616): when a
                // frame capture outlives the script, the scene reloads and the
                // rounds replay, so the capture measures ACTIVITY.
                .loop_from(LOAD_STEP)
                .on_loop(reload_the_showcase),
        ));
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        // Run-timeline recorder (inert unless NOVA_PROBE_TIMELINE is set):
        // this example exercises the whole scenario language, so its recorded
        // timeline doubles as the recorder's stability probe.
        // Every tally this scenario keeps is one-way by design. A reload
        // re-seeds them, which the checker forgets at teardown.
        app.add_plugins(nova_probe::NovaProbePlugin::default().monotonic([
            "beat",
            "rocks_destroyed",
            "round",
            "area_entries",
            "area_exits",
            "escort_neutralized",
            "timer_ended",
            "ring_cleared",
            "once_ticks",
        ]));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the
    // load happened by OnEnter(Playing), and loading in that same schedule
    // is an unordered race against the check.
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_showcase);
}

fn setup_showcase(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(showcase(&game_assets, &sections)));
}

/// Where ring member `i` sits: a deterministic ring, not random - the probe
/// (and a reader) should see the same scene every run.
fn rock_position(i: usize) -> Vec3 {
    let angle = i as f32 / ASTEROID_COUNT as f32 * std::f32::consts::TAU;
    Vec3::new(angle.cos() * 40.0, 0.0, angle.sin() * 40.0 - 60.0)
}

/// The showcase scenario: every part of the event grammar in one config.
fn showcase(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // The escort exists to be DISARMED: a weapon section and a thruster
    // section, both destroyable, on a hull that survives losing them. Nobody
    // drives it - `OnNeutralized` is an integrity verdict, not an AI one.
    let escort = fixtures::ship(
        sections,
        SpaceshipController::None,
        &[
            SectionSpec::new("escort_controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new(
                "escort_hull",
                "reinforced_hull_section",
                Vec3::new(0.0, 0.0, 1.0),
            ),
            SectionSpec::new(
                ESCORT_DRIVE,
                "basic_thruster_section",
                Vec3::new(0.0, 0.0, 2.0),
            ),
            SectionSpec::new(
                ESCORT_GUNS,
                "pdc_kinetic_turret_section",
                Vec3::new(0.0, 0.75, 0.0),
            ),
        ],
    );

    // OnStart: the ring, the escort, the trigger volume, the objectives, the
    // HUD readout and the seeds.
    let mut start_actions: Vec<EventActionConfig> = (0..ASTEROID_COUNT)
        .map(|i| {
            EventActionConfig::SpawnScenarioObject(fixtures::asteroid(
                game_assets,
                &format!("rock_{i}"),
                &format!("Rock {i}"),
                rock_position(i),
                2.0,
                // Unsigned: the ring is cleared by hand, never radar-locked.
                None,
            ))
        })
        .collect();
    start_actions.push(EventActionConfig::SpawnScenarioObject(
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: ESCORT_ID.to_string(),
                name: "Escort".to_string(),
                position: Vec3::new(0.0, 0.0, -10.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(escort),
        },
    ));
    // The volume is created AROUND a rock that is already in the world: avian
    // starts a fresh contact pair even at full containment, so this fires
    // OnEnter without anything having to fly in.
    start_actions.push(EventActionConfig::CreateScenarioArea(ScenarioAreaConfig {
        id: AREA_ID.to_string(),
        name: "Rock Ring Trigger".to_string(),
        position: rock_position(AREA_ROCK),
        rotation: Quat::IDENTITY,
        radius: AREA_RADIUS,
    }));
    start_actions.push(EventActionConfig::Objective(ObjectiveActionConfig::new(
        OBJECTIVE_DISARM,
        "Disarm the escort",
    )));
    start_actions.push(EventActionConfig::Objective(ObjectiveActionConfig::new(
        OBJECTIVE_RING,
        "Clear the ring",
    )));
    // The display half of the variable vocabulary: the HUD reads the tally
    // live off the event world, so the readout is bound, not pushed.
    start_actions.push(EventActionConfig::HudReadout(HudReadoutActionConfig {
        slot: "rocks".to_string(),
        variable: "rocks_destroyed".to_string(),
        format: HudReadoutFormatConfig::Integer,
        label: Some("ROCKS".to_string()),
        visible: true,
    }));
    start_actions.extend([
        set_number("beat", 1.0),
        set_number("round", 1.0),
        set_number("rocks_destroyed", 0.0),
        set_number("area_entries", 0.0),
        set_number("area_exits", 0.0),
        set_number("escort_neutralized", 0.0),
        set_number("timer_ended", 0.0),
        set_number("ring_cleared", 0.0),
        set_number("once_ticks", 0.0),
        EventActionConfig::TimerStart(TimerStartActionConfig {
            key: "grammar_delay".to_string(),
            seconds: number(0.25),
        }),
    ]);
    // The showcase lights itself: the engine spawns no light, so a scenario
    // that authors none renders black.
    start_actions.extend(ThreePointRig::around("showcase", Vec3::ZERO, 5.0).actions());

    let events = vec![
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: start_actions,
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnTimerEnd,
            once: true,
            filters: vec![EventFilterConfig::Timer(TimerFilterConfig {
                key: "grammar_delay".to_string(),
            })],
            actions: vec![set_number("timer_ended", 1.0)],
        },
        // Every destroyed asteroid bumps the tally (entity-type filter +
        // variable arithmetic).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: false,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: None,
                type_name: Some("asteroid".to_string()),
                ..default()
            })],
            actions: vec![increment_variable("rocks_destroyed")],
        },
        // Enough kills advance the beat (expression filters: tally reached
        // the threshold). `once` is what makes it a transition rather than a
        // repeat - the old "and beat is still 1" guard was the handler asking
        // whether it had already run.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnDestroyed,
            once: true,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    variable("rocks_destroyed"),
                    number(KILLS_PER_ROUND as f64 - 1.0),
                ),
            ))],
            actions: vec![set_number("beat", 2.0)],
        },
        // The per-frame pulse promotes beat 2 -> 3 (OnUpdate + expression
        // filter, retiring on the pass).
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_equals(variable("beat"), number(2.0)),
            ))],
            actions: vec![set_number("beat", 3.0)],
        },
        // The ROUND counter, and the reason the script never needs a settle
        // window: the scenario decides when a round is over, by arithmetic
        // over its own variables - `rocks_destroyed > round * kills - 1`.
        // Self-limiting, since the last increment puts the threshold out of
        // the ring's reach.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: false,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    variable("rocks_destroyed"),
                    VariableExpressionNode::new_subtract(
                        VariableTermNode::new_multiply(
                            VariableFactorNode::new_name("round".to_string()),
                            VariableTermNode::new_factor(VariableFactorNode::new_literal(
                                VariableLiteral::Number(KILLS_PER_ROUND as f64),
                            )),
                        ),
                        number(1.0),
                    ),
                ),
            ))],
            actions: vec![increment_variable("round")],
        },
        // OnEnter/OnExit on the trigger volume, filtered to the pair (the
        // area's id, the body's id) the area plugin reports.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnEnter,
            once: false,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(AREA_ID.to_string()),
                other_id: Some(format!("rock_{AREA_ROCK}")),
                ..default()
            })],
            actions: vec![increment_variable("area_entries")],
        },
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnExit,
            once: false,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(AREA_ID.to_string()),
                other_id: Some(format!("rock_{AREA_ROCK}")),
                ..default()
            })],
            actions: vec![increment_variable("area_exits")],
        },
        // OnNeutralized: the escort lost its guns and its drive, so it is out
        // of the fight with the hull still intact - a verdict `OnDestroyed`
        // never delivers. Completing an objective here is the other half.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnNeutralized,
            once: true,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(ESCORT_ID.to_string()),
                ..default()
            })],
            actions: vec![
                set_number("escort_neutralized", 1.0),
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: OBJECTIVE_DISARM.to_string(),
                }),
            ],
        },
        // The emptied ring completes its objective. `once` is what makes an
        // every-frame handler fire exactly once; the only filter left is the
        // one about the ring.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![EventFilterConfig::Expression(ExpressionFilterConfig(
                VariableConditionNode::new_greater_than(
                    variable("rocks_destroyed"),
                    number(ASTEROID_COUNT as f64 - 1.0),
                ),
            ))],
            actions: vec![
                set_number("ring_cleared", 1.0),
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: OBJECTIVE_RING.to_string(),
                }),
            ],
        },
        // The retirement itself, measured: an UNFILTERED OnUpdate handler,
        // which without `once` would tick on every frame of every round. The
        // report asserts the counter reads exactly 1.
        ScenarioEventConfig {
            label: None,
            name: EventConfig::OnUpdate,
            once: true,
            filters: vec![],
            actions: vec![increment_variable("once_ticks")],
        },
    ];

    ScenarioConfig {
        description: "Variables, events, filters and actions in one scenario.".to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Scenario Showcase".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Restart the cycle on an autopilot loop: mark the reload interval and
/// re-trigger the SAME showcase scenario (task 20260720-000616). The step
/// after the loop point closes the interval once the fixture has re-seeded
/// itself.
///
/// Silently does nothing before the loader has inserted the asset resources -
/// a loop cannot happen that early, but reading them unguarded would panic.
#[cfg(feature = "debug")]
fn reload_the_showcase(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(showcase(&game_assets, &sections)));
}

/// Read a Number variable from the live event world, or panic with context.
#[cfg(feature = "debug")]
fn number_variable(world: &World, key: &str) -> f64 {
    match world.resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("scenario probe: variable {key} should be a number, got {other:?}"),
    }
}

/// The scenario-scoped roots carrying `id`, matched by a predicate over the id.
#[cfg(feature = "debug")]
fn scoped_entities(world: &mut World, matches: impl Fn(&str) -> bool) -> Vec<(String, Entity)> {
    let mut query = world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
    let mut found: Vec<(String, Entity)> = query
        .iter(world)
        .filter(|(_, id)| matches(&id.0))
        .map(|(entity, id)| (id.0.clone(), entity))
        .collect();
    found.sort_by(|(a, _), (b, _)| a.cmp(b));
    found
}

/// The step that guards the seed: the load step already waited on `round` and
/// the tally, so this pins the rest of what OnStart claims to have done.
#[cfg(feature = "debug")]
fn assert_seed(world: &mut World) {
    assert_eq!(
        number_variable(world, "beat"),
        1.0,
        "OnStart seeds beat = 1"
    );
    assert_eq!(
        number_variable(world, "escort_neutralized"),
        0.0,
        "OnStart seeds the escort latch at 0"
    );
    assert_eq!(
        number_variable(world, "ring_cleared"),
        0.0,
        "OnStart seeds the ring latch at 0"
    );
    let objectives = world.resource::<GameObjectives>();
    for id in [OBJECTIVE_DISARM, OBJECTIVE_RING] {
        assert!(
            objectives.objectives.iter().any(|live| live.id == id),
            "OnStart must post objective '{id}'"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: onstart seeds variables and objectives",
        serde_json::json!({}),
    );
    info!("scenario probe: variables seeded");
}

/// Push the contained rock out of the trigger volume, so the area's OTHER
/// half - `OnExit` - fires. It has to MOVE: avian reports no `CollisionEnd`
/// for a collider that was despawned, so destroying the rock would never exit.
#[cfg(feature = "debug")]
fn push_rock_out(world: &mut World) {
    let id = format!("rock_{AREA_ROCK}");
    let (_, rock) = scoped_entities(world, |live| live == id)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("scenario probe: no '{id}' to push out of the trigger volume"));
    // Straight up, where no other ring member sits: 40 u/s clears the 8 u
    // sphere in a fraction of a second even under an llvmpipe frame stall.
    world
        .entity_mut(rock)
        .insert(avian3d::prelude::LinearVelocity(Vec3::Y * 40.0));
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: push out of the area", json_at(t));
}

/// Strip the escort's guns and drive through the production damage path, and
/// let the integrity layer reach its own neutralize verdict. Deliberately not
/// a hit on the ship root: that would DESTROY it, which is the event this beat
/// exists to distinguish itself from.
#[cfg(feature = "debug")]
fn disarm_escort(world: &mut World) {
    let targets = {
        let mut query =
            world.query_filtered::<(Entity, &EntityId), (With<SectionMarker>, With<Health>)>();
        query
            .iter(world)
            .filter(|(_, id)| id.0 == ESCORT_GUNS || id.0 == ESCORT_DRIVE)
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        targets.len(),
        2,
        "scenario probe: the escort must carry a live guns and drive section to strip"
    );
    for target in targets {
        world.trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1e6,
        });
    }
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: disarm the escort", json_at(t));
}

/// Exhaust one round's worth of the ring, lowest id first, through the
/// production located-damage path.
#[cfg(feature = "debug")]
fn destroy_a_round_of_rocks(world: &mut World) {
    let rocks: Vec<Entity> = scoped_entities(world, |id| id.starts_with("rock_"))
        .into_iter()
        .map(|(_, entity)| entity)
        .take(KILLS_PER_ROUND)
        .collect();
    assert_eq!(
        rocks.len(),
        KILLS_PER_ROUND,
        "scenario probe: a round needs {KILLS_PER_ROUND} live rocks"
    );
    for rock in rocks {
        let node = world
            .get::<Children>(rock)
            .and_then(|children| children.iter().next())
            .expect("scenario probe: a rock carries one collider node");
        let at = world
            .get::<GlobalTransform>(rock)
            .map_or(Vec3::ZERO, GlobalTransform::translation);
        let mut commands = world.commands();
        apply_damage(
            &mut commands,
            node,
            None,
            2_000_000.0,
            DamageType::Kinetic,
            Some(at),
        );
        world.flush();
    }
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: destroy a round of rocks", json_at(t));
}

/// Per-round assertion: the tally counted this round's kills, the scenario's
/// own arithmetic closed the round, and the beat chain (expression-filtered
/// OnDestroyed, then the OnUpdate pulse) completed in round 1 and stayed put.
#[cfg(feature = "debug")]
fn report_round(round: usize) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let killed = (round * KILLS_PER_ROUND) as f64;
        assert_eq!(
            number_variable(world, "rocks_destroyed"),
            killed,
            "round {round}: the OnDestroyed tally handler must have counted the kills"
        );
        assert_eq!(
            number_variable(world, "round"),
            (round + 1) as f64,
            "round {round}: the scenario's own arithmetic must close the round"
        );
        assert_eq!(
            number_variable(world, "beat"),
            3.0,
            "round {round}: the expression filter must advance the beat (2) and the \
             OnUpdate pulse must promote it (3)"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the round arithmetic closes the round",
            serde_json::json!({}),
        );
        info!("scenario probe: round {round} ticked ({killed} rocks down)");
    }
}

/// The script's last beat: every grammar surface this scenario reaches has
/// left its mark, and both objectives are off the HUD list.
#[cfg(feature = "debug")]
fn report_grammar(world: &mut World) {
    assert_eq!(
        number_variable(world, "area_entries"),
        1.0,
        "the trigger volume must have reported exactly one entry"
    );
    assert_eq!(
        number_variable(world, "area_exits"),
        1.0,
        "the trigger volume must have reported exactly one exit"
    );
    assert_eq!(
        number_variable(world, "escort_neutralized"),
        1.0,
        "the disarmed escort must have fired OnNeutralized"
    );
    assert_eq!(
        number_variable(world, "rocks_destroyed"),
        ASTEROID_COUNT as f64,
        "every round's kills must be on the tally"
    );
    assert_eq!(
        number_variable(world, "once_ticks"),
        1.0,
        "a `once` handler runs on ONE frame: this one has no filter at all, so \
         without retirement it would tick on every frame of every round"
    );
    let objectives = world.resource::<GameObjectives>();
    for id in [OBJECTIVE_DISARM, OBJECTIVE_RING] {
        assert!(
            !objectives.objectives.iter().any(|live| live.id == id),
            "objective '{id}' must be completed by the end of the run"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the trigger volume fires on enter and exit",
        serde_json::json!({}),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the disarmed escort fires the neutralized handler",
        serde_json::json!({}),
    );
    nova_probe::probe_marker(
        world,
        "outcome: every kill reaches the tally",
        serde_json::json!({}),
    );
    nova_probe::probe_marker(
        world,
        "outcome: a once handler retires after one pass",
        serde_json::json!({}),
    );
    nova_probe::probe_marker(
        world,
        "outcome: both objectives complete",
        serde_json::json!({}),
    );
    info!("scenario probe: handlers, filters and expressions all ticked");
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: done", json_at(t));
}

/// The timeline marker payload every beat here shares.
#[cfg(feature = "debug")]
fn json_at(t: f32) -> serde_json::Value {
    serde_json::json!({ "t": t })
}
