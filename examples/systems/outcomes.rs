//! outcomes: the whole outcome arc, composed and walked in a live app.
//!
//! Die -> the Defeat overlay -> Retry -> a demonstrably CLEAN reload -> kill
//! the hostile -> the objective completes and the CHECKPOINT lands -> Continue
//! -> the next scenario is loaded. Four systems (chaining, Defeat, Retry
//! reload-clean, Victory + CHECKPOINT) that are each pinned headlessly in unit
//! tests, here in ONE rendered run through the real buttons.
//!
//! The fixtures are two ~50-line `ScenarioConfig` values built in Rust, not
//! shipped story RON: the compiler catches scenario-grammar changes, and the
//! run never needs touching when the campaign is rebalanced. Both are
//! `hidden: true`, so registering them adds no rows to the Scenarios picker.
//!
//! A CHECKPOINT is not a type. It is `Outcome(Victory)` plus a LINGERING
//! `NextScenario` queued in the same handler, observable as
//! `NovaEventWorld::next_scenario` and released by exactly what the overlay's
//! Continue/Retry button calls.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example outcomes --features debug
//! # look for: `outcomes: defeat overlay up, retrying`,
//! #           `outcomes: reload is clean`,
//! #           `outcomes: victory + checkpoint queued for 'outcome_probe_b'`,
//! #           `outcomes: chained into outcome_probe_b`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "outcomes")]
#[command(version = "1.0.0")]
#[command(about = "The composed outcome arc: Defeat, Retry, Victory, CHECKPOINT and the chain", long_about = None)]
struct Cli;

/// The scenario the run boots into and retries.
const SCENARIO_A: &str = "outcome_probe_a";

/// The chain target the CHECKPOINT queues.
const SCENARIO_B: &str = "outcome_probe_b";

/// The scenario object ids the script kills and waits on.
const PLAYER_ID: &str = "player_ship";
const HOSTILE_ID: &str = "hostile";

/// The objective scenario A posts and completes on the kill.
const OBJECTIVE_ID: &str = "clear_the_hostile";

/// The first step's name, so `loop_from` restarts the cycle at the scene
/// reload without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "load the outcome probe";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the outcome overlay and its buttons live in
    // nova_menu, and `with_game_plugins` turns the menu plugin OFF (it also
    // owns the boot-into-MainMenu handoff, which this run must not take - it
    // goes Loading -> Playing). The plugin is built to stand alone in slim
    // apps, so adding it without the menu boot is supported, not a hack.
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    // Headless smoke-test harness: strictly linear beats, each waiting on a
    // world predicate the arc itself produces (an outcome, an overlay entity, a
    // re-seeded variable, a loaded scenario id) rather than on a guessed
    // duration. A shown outcome PAUSES virtual time, so a wall-clock beat after
    // the overlay would measure nothing; every beat past the kill is a world
    // observation for that reason as much as for llvmpipe.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(
            nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
                // Both fixtures seed `player_down`, so waiting for the ship AND
                // the seed is also the gate a looped scene reload needs: the
                // old cycle's variables outlive the load replacing them
                // (ScenarioLoaded fires BEFORE OnStart runs).
                .step(LOAD_STEP)
                .enter(GameStates::Loading)
                .until(and(
                    player_ship_present(),
                    scenario_variable_is("player_down", 0.0),
                ))
                .deadline(30.0)
                .add()
                // The scene is live again: close the reload interval so a frame
                // capture excludes it. A no-op on the first cycle.
                .step("close the reload interval")
                .on_enter(nova_probe::capture_reload_end)
                .add()
                // LOSE first. Overkill on the ship ROOT is the production
                // damage entry point; the declarative alternative (a scenario
                // action that damages the player) does not exist.
                .step("die to the overkill")
                .on_enter(kill_object(PLAYER_ID))
                .until(defeat_overlay_up())
                .deadline(20.0)
                .add()
                .step("report the defeat overlay")
                .on_enter(report_defeat_overlay)
                .add()
                // Retry through the real button. The reload is clean when the
                // outcome is gone, a FRESH player ship is up, the hostile is
                // back and the death latch has been re-seeded to zero - the
                // last one is what proves OnStart ran again rather than the old
                // event world surviving.
                .step("retry into a clean reload")
                .on_enter(activate_named("Outcome Primary Button"))
                .until(and(
                    and(outcome_cleared(), player_ship_present()),
                    and(
                        object_present(HOSTILE_ID),
                        scenario_variable_is("player_down", 0.0),
                    ),
                ))
                .deadline(20.0)
                .add()
                .step("report the clean reload")
                .on_enter(report_clean_reload)
                .add()
                // WIN. The kill completes the objective and posts
                // Victory + the lingering chain - the CHECKPOINT.
                .step("kill the hostile")
                .on_enter(kill_object(HOSTILE_ID))
                .until(victory_overlay_up())
                .deadline(20.0)
                .add()
                .step("report the victory checkpoint")
                .on_enter(report_victory_checkpoint)
                .add()
                // Continue through the same button, and end on B actually being
                // the loaded scenario rather than on the switch being queued.
                .step("continue into the chained scenario")
                .on_enter(activate_named("Outcome Primary Button"))
                .until(and(
                    current_scenario_is(SCENARIO_B),
                    scenario_variable_is("in_probe_b", 1.0),
                ))
                .deadline(20.0)
                .add()
                // Last beat: the driver reports done after it, so a clean pass
                // ends here rather than idling out a runway.
                .step("report the chain")
                .on_enter(report_chain)
                .add()
                // Enrolled in capture looping: when a frame capture outlives the
                // script the scene reloads and the arc replays, so the capture
                // measures ACTIVITY rather than an idle tail.
                .loop_from(LOAD_STEP)
                .on_loop(reload_the_probe),
        );
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_A));
        // Run-timeline recorder (inert unless NOVA_PERF_TIMELINE names an output
        // path): the beats below push markers, so an armed run reads
        // "die -> retry -> kill -> continue" beside the scenario's own events.
        app.add_plugins(nova_probe::nova_timeline());
        // Continuous invariants (inert unless NOVA_PERF_INVARIANTS is set):
        // `hostile_down` is a one-way latch across the WHOLE run - seeded 0 by
        // both loads of A and set to 1 by the kill, so the Retry reload writes
        // 0 over 0 and never regresses. `player_down` is deliberately NOT
        // claimed: the death sets it to 1 and the Retry re-seeds it to 0, which
        // is the point of the beat, not a violation.
        app.add_plugins(nova_probe::nova_invariants().monotonic(["hostile_down"]));
        // Frame-time capture (inert unless NOVA_PERF is set): fleet-wide wiring.
        app.add_plugins(nova_probe::nova_frametime());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    // On assets-Loaded, not on Playing: `assert_scenario_loaded` checks the load
    // happened by OnEnter(Playing), and loading in that same schedule is an
    // unordered race against the check.
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_probe);
}

/// Register BOTH fixtures in [`GameScenarios`], then boot into A.
///
/// The registration is load-bearing, not tidiness: a queued `NextScenario`
/// carries only an id, and the switch resolves it against this resource -
/// unloading to the menu on a miss. Without it, the Retry requeue and the
/// Continue chain would each silently drop out of the run. A shipped scenario
/// reaches the resource through the content-bundle merge; a code-built one has
/// to put itself there.
fn setup_probe(
    mut commands: Commands,
    mut scenarios: ResMut<GameScenarios>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    let a = outcome_probe_a(&game_assets, &sections);
    let b = outcome_probe_b(&game_assets);
    scenarios.insert(a.id.clone(), a.clone());
    scenarios.insert(b.id.clone(), b);
    commands.trigger(LoadScenario(a));
}

/// Shorthand: a literal number as a variable expression.
fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

/// Filter an event down to one scenario object id.
fn by_id(id: &str) -> EventFilterConfig {
    EventFilterConfig::Entity(EntityFilterConfig {
        id: Some(id.to_string()),
        type_name: None,
        ..default()
    })
}

/// Scenario A: one player ship, one killable hostile, one objective, and the
/// two `OnDestroyed` handlers that turn a death into a Defeat-with-Retry and
/// the kill into a Victory-with-CHECKPOINT.
///
/// `auto_advance_secs` is deliberately absent from both outcomes: an overlay
/// that advances its own chain after N seconds would let the run pass without
/// the script's `Activate` ever being the cause - a vacuous proof of the exact
/// path this example exists to walk.
fn outcome_probe_a(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_controller_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "hull".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("reinforced_hull_section")),
                modifications: vec![],
            },
        ],
    };

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: PLAYER_ID.to_string(),
                        name: "Player Ship".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                }),
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: HOSTILE_ID.to_string(),
                        name: "Hostile".to_string(),
                        position: Vec3::new(0.0, 0.0, -40.0),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        impact_sound: Some("base/sounds/impact.wav".into()),
                        destroy_sound: Some("base/sounds/explosion.wav".into()),
                        radius: 2.0,
                        texture: game_assets.asteroid_texture.clone().into(),
                        health: 60.0,
                        mass: None,
                        invulnerable: false,
                        lock_signature: Some(1000.0),
                    }),
                }),
                EventActionConfig::Objective(ObjectiveActionConfig::new(
                    OBJECTIVE_ID,
                    "Destroy the hostile",
                )),
                // The two latches. Seeded here and NOWHERE else, so their value
                // after a reload is the proof that OnStart ran again.
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "player_down".to_string(),
                    expression: number(0.0),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "hostile_down".to_string(),
                    expression: number(0.0),
                }),
            ]
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            .into_iter()
            .chain(ThreePointRig::around("arena", Vec3::ZERO, 2.0).actions())
            .collect(),
        },
        // The DEFEAT path. The requeue of A is what makes the overlay show a
        // Retry button at all: `primary` is None unless something is queued.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![by_id(PLAYER_ID)],
            actions: vec![
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "player_down".to_string(),
                    expression: number(1.0),
                }),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Defeat,
                    "Hull breached.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: SCENARIO_A.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
        // The VICTORY path, and the CHECKPOINT: `Outcome(Victory)` plus a
        // LINGERING `NextScenario` in the same handler. `linger: true` is
        // required rather than stylistic - the scenario lint warns on an
        // `Outcome` composed with a non-lingering switch, because the instant
        // cut tears the scenario down before the overlay can show.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![by_id(HOSTILE_ID)],
            actions: vec![
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "hostile_down".to_string(),
                    expression: number(1.0),
                }),
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: OBJECTIVE_ID.to_string(),
                }),
                EventActionConfig::Outcome(OutcomeActionConfig::new(
                    ScenarioOutcomeKind::Victory,
                    "Hostile cleared.",
                )),
                EventActionConfig::NextScenario(NextScenarioActionConfig {
                    scenario_id: SCENARIO_B.to_string(),
                    linger: true,
                    delay: None,
                }),
            ],
        },
    ];

    ScenarioConfig {
        description: "Die for the Defeat overlay, kill for the Victory checkpoint.".to_string(),
        hidden: true,
        events,
        ..ScenarioConfig::new(
            SCENARIO_A.to_string(),
            "Outcome Probe A".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Scenario B: the chain target. One object and one variable seeded on
/// `OnStart`, so the chain's ARRIVAL is observable rather than inferred from
/// the switch having been queued.
fn outcome_probe_b(game_assets: &GameAssets) -> ScenarioConfig {
    let events = vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: vec![
            EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: "chain_marker".to_string(),
                    name: "Chain Marker".to_string(),
                    position: Vec3::new(0.0, 0.0, -30.0),
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Beacon(BeaconConfig {
                    label: "ARRIVED".to_string(),
                    radius: 1.5,
                    color: Color::srgb(0.3, 0.9, 0.9),
                    area_radius: None,
                    lock_signature: None,
                }),
            }),
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "in_probe_b".to_string(),
                expression: number(1.0),
            }),
        ],
    }];

    ScenarioConfig {
        description: "The chain target, so the CHECKPOINT's arrival is observable.".to_string(),
        hidden: true,
        events,
        ..ScenarioConfig::new(
            SCENARIO_B.to_string(),
            "Outcome Probe B".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Restart the cycle on an autopilot loop: mark the reload interval and
/// re-trigger scenario A. The step after the loop point closes the interval
/// once the fixture has re-seeded itself.
///
/// Silently does nothing before the loader has inserted the asset resources - a
/// loop cannot happen that early, but reading them unguarded would panic.
#[cfg(feature = "debug")]
fn reload_the_probe(world: &mut World) {
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(outcome_probe_a(&game_assets, &sections)));
}

/// The live outcome kind, if one is declared.
#[cfg(feature = "debug")]
fn outcome_kind(world: &World) -> Option<ScenarioOutcomeKind> {
    world
        .get_resource::<CurrentOutcome>()
        .and_then(|outcome| outcome.0.as_ref())
        .map(|config| config.outcome)
}

/// The entity carrying `name`, if any - how the script finds the real overlay
/// button instead of clicking screen coordinates.
#[cfg(feature = "debug")]
fn entity_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find(|(_, live)| live.as_str() == name)
        .map(|(entity, _)| entity)
}

/// Advance once `kind` is declared AND the overlay's primary button exists.
///
/// Never the outcome alone: the outcome resource lands a frame before the
/// overlay spawns (a PostUpdate write, synced next Update), so gating on the
/// resource is a race against the button the next beat presses.
///
/// The button rather than the overlay, because the overlay is not enough. Both
/// spawn in one command batch (`crates/nova_menu/src/outcome.rs`), so there is
/// no lag BETWEEN them - but the button is conditional on something being
/// queued, so an overlay can exist without one. Waiting on the button makes a
/// fixture that forgets its lingering `NextScenario` stall on a named step
/// instead of panicking in the next beat's `activate_named`.
#[cfg(feature = "debug")]
fn outcome_overlay_up(
    kind: ScenarioOutcomeKind,
) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        outcome_kind(world) == Some(kind)
            && world.try_query::<&Name>().is_some_and(|mut query| {
                query
                    .iter(world)
                    .any(|name| name.as_str() == "Outcome Primary Button")
            })
    })
}

#[cfg(feature = "debug")]
fn defeat_overlay_up() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    outcome_overlay_up(ScenarioOutcomeKind::Defeat)
}

#[cfg(feature = "debug")]
fn victory_overlay_up() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    outcome_overlay_up(ScenarioOutcomeKind::Victory)
}

/// Advance once no outcome is declared - the first half of "the reload is
/// clean", since scenario teardown is what clears it.
#[cfg(feature = "debug")]
fn outcome_cleared() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| outcome_kind(world).is_none())
}

/// Advance once a scenario-scoped object carries `id`.
#[cfg(feature = "debug")]
fn object_present(id: &'static str) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        world
            .try_query_filtered::<&EntityId, With<ScenarioScopedMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|live| live.0 == id))
    })
}

/// Advance once `id` is the LOADED scenario - the chain's arrival, not its
/// queueing.
#[cfg(feature = "debug")]
fn current_scenario_is(id: &'static str) -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<CurrentScenario>()
            .and_then(|current| current.0.as_ref())
            .is_some_and(|live| live.id == id)
    })
}

/// Kill the scenario object `id` with an overkill through the production damage
/// entry point, aimed at the node that actually carries the `Health`.
///
/// Which node that is depends on the object: a ship root carries the aggregate
/// health its sections sum into, but an asteroid root carries only markers -
/// its collider and `Health` live on a CHILD (`asteroid_scenario_object`).
/// `HealthApplyDamage` propagates child -> parent and never downwards, so an
/// overkill on an asteroid root is silently absorbed by nothing. Walking to the
/// `Health` bearer is what makes one helper kill both the player and the rock.
#[cfg(feature = "debug")]
fn kill_object(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let root = {
            let mut query =
                world.query_filtered::<(Entity, &EntityId), With<ScenarioScopedMarker>>();
            query
                .iter(world)
                .find(|(_, live)| live.0 == id)
                .map(|(entity, _)| entity)
        };
        let root = root.unwrap_or_else(|| panic!("outcomes: no scenario object '{id}' to kill"));
        let target = health_bearer(world, root)
            .unwrap_or_else(|| panic!("outcomes: scenario object '{id}' carries no Health"));
        let t = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(world, "beat: kill", serde_json::json!({ "t": t, "id": id }));
        world.trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1e6,
        });
    }
}

/// The nearest `Health`-carrying node at or beneath `entity`, depth first.
#[cfg(feature = "debug")]
fn health_bearer(world: &World, entity: Entity) -> Option<Entity> {
    if world.get::<Health>(entity).is_some() {
        return Some(entity);
    }
    let children: Vec<Entity> = world
        .get::<Children>(entity)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .find_map(|child| health_bearer(world, child))
}

/// Press the overlay button named `name` the way the widget layer does.
/// Deliberately not a `click_at` on screen coordinates: this run proves the
/// mechanics behind the button, and a coordinate click would make it a layout
/// test that fails for unrelated reasons.
#[cfg(feature = "debug")]
fn activate_named(name: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let button = entity_by_name(world, name)
            .unwrap_or_else(|| panic!("outcomes: the overlay has no '{name}'"));
        let t = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "beat: activate",
            serde_json::json!({ "t": t, "button": name }),
        );
        world.trigger(bevy::ui_widgets::Activate { entity: button });
    }
}

/// The Defeat arc's first landmark, reported where the next beat's button is
/// already known to exist - the step's predicate held the Defeat outcome and
/// the button before this ran.
#[cfg(feature = "debug")]
fn report_defeat_overlay(world: &mut World) {
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: defeat overlay", serde_json::json!({ "t": t }));
    info!("outcomes: defeat overlay up, retrying");
}

/// The Retry landed on a genuinely fresh scenario: the step's predicate already
/// held the ship, the hostile and the re-seeded latch, so this only records it
/// and pins the one thing a predicate cannot - that no outcome survived.
#[cfg(feature = "debug")]
fn report_clean_reload(world: &mut World) {
    assert!(
        outcome_kind(world).is_none(),
        "outcomes: the Retry reload must clear the outcome"
    );
    info!("outcomes: reload is clean - fresh ship, hostile back, latch re-seeded");
}

/// The CHECKPOINT, asserted where it is observable: the objective left the HUD
/// list and a LINGERING switch to B is queued next to the Victory.
#[cfg(feature = "debug")]
fn report_victory_checkpoint(world: &mut World) {
    let objectives = world.resource::<GameObjectives>();
    assert!(
        !objectives
            .objectives
            .iter()
            .any(|objective| objective.id == OBJECTIVE_ID),
        "outcomes: the kill must complete objective '{OBJECTIVE_ID}'"
    );
    let next = world
        .resource::<NovaEventWorld>()
        .next_scenario
        .clone()
        .expect("outcomes: a Victory CHECKPOINT queues the next scenario");
    assert_eq!(
        next.scenario_id, SCENARIO_B,
        "outcomes: the CHECKPOINT must queue the chain target"
    );
    assert!(
        next.linger,
        "outcomes: the queued switch must LINGER, or the overlay never shows"
    );
    info!("outcomes: victory + checkpoint queued for '{SCENARIO_B}'");
}

/// The script's last beat: Continue actually loaded B.
#[cfg(feature = "debug")]
fn report_chain(world: &mut World) {
    info!("outcomes: chained into {SCENARIO_B}");
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "beat: done",
        serde_json::json!({ "t": t, "scenario": SCENARIO_B }),
    );
}
