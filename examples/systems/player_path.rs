//! player_path: a playable scenario, played - by the real input pipeline, and
//! played AGAIN through the same loop point.
//!
//! One armed player ship, one hostile rock dead ahead, one nav beacon
//! beyond it. The script performs the exact gestures a player would, in
//! order: raise (RMB) + radar hold (CTRL) to combat-lock the rock, gun it
//! down with LMB (the turret holds the combat lock), lower and radar again to travel-lock the beacon, press G to engage
//! GOTO, and start the flight leg toward the beacon's trigger area. The
//! SCENARIO watches the run through its own event handlers - a kill tally,
//! a travel-lock acquisition, an arrival flag - so the assertions read "the
//! scenario saw the player do it", not "the script poked some components".
//! Headless, the run completes at "GOTO engaged and closing"; the arrival
//! flag still fires interactively (llvmpipe throttles unfocused smoke
//! windows too hard for a full flight leg).
//!
//! Every beat waits on what the scenario SAW - a seeded variable, a live lock,
//! a published closing speed - so a slow load or an llvmpipe frame stall delays
//! the play instead of truncating it, and a beat that never resolves inside its
//! deadline error-exits naming that beat.
//!
//! The whole gesture chain runs [`ROUNDS`] times, each round separated by the
//! same scenario reload the capture loop point uses. A chain that only ever
//! holds on a freshly booted app is a weaker claim than one that holds on a
//! RELOADED one: the second round is what would catch state surviving a
//! teardown - a stale lock, an input latched down, a handler bound to the dead
//! scene.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example player_path --features debug
//! # look for: `autopilot: step `round 1: sweep the prey into a combat lock` begins`,
//! #           `player_path: round 1 - prey destroyed, waypoint locked, GOTO closing at ...`,
//! #           `player_path: round 2 - prey destroyed, waypoint locked, GOTO closing at ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_authoring::scenario_helpers::number;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "player_path")]
#[command(version = "1.0.0")]
#[command(about = "A playable scenario driven through the real input pipeline, watched by its own event handlers", long_about = None)]
struct Cli;

const SCENARIO_ID: &str = "playable_run";

#[cfg(feature = "debug")]
/// How many times the script plays the whole gesture chain, each round after
/// the first re-entered through a scenario reload. Two, not more: every round
/// is a real flight leg driven through the input pipeline, and under llvmpipe
/// that is seconds of wall clock per round for a claim the SECOND round
/// already makes (the chain survives a teardown).
const ROUNDS: usize = 2;

/// The first step's name, so `loop_from` can restart the cycle at the scene
/// reload without repeating the string.
#[cfg(feature = "debug")]
const LOAD_STEP: &str = "load the run";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: named beats drive the REAL gestures
    // (raise, radar, fire, lower, radar, GOTO) and each one advances on what
    // the SCENARIO saw - its own event handlers' variables, the live lock
    // components - rather than on a guessed duration. That matters here
    // beyond tidiness: under CI's llvmpipe throttle frames hit Bevy's
    // max-delta cap, so a wall-clock window holds a fraction of the sim
    // seconds it looks like it does, and the old 24 s runway was margin
    // guesswork. The per-step deadlines below are what that runway became;
    // their sum stays well under DEFAULT_DEADLINE_SECS (120 s) so a stalled
    // beat is named rather than swallowed by the generic hang detector.
    #[cfg(feature = "debug")]
    {
        let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            // The ship is spawned by an OnStart handler, and that same
            // handler seeds `target_down` - so waiting for BOTH is also
            // the gate a looped scene reload needs, since the old cycle's
            // variables outlive the load replacing them (20260720-014142:
            // ScenarioLoaded fires BEFORE OnStart runs, and the first real
            // loop crashed in that gap).
            .step(LOAD_STEP)
            .enter(GameStates::Loading)
            .until(and(
                player_ship_present(),
                scenario_variable_is("target_down", 0.0),
            ))
            .deadline(30.0)
            .add();

        for round in 1..=ROUNDS {
            if round > 1 {
                // Re-enter the run the way the capture loop point does: drop
                // whatever the last round left held (GOTO is still down), tear
                // the scene down and reload it, and wait on the SAME gate the
                // boot step uses. A latched key or a lock surviving the
                // teardown fails the next round's beats, which is the point.
                script = script
                    .step(format!("round {round}: reload the run"))
                    .on_enter(reload_the_run)
                    .until(and(
                        player_ship_present(),
                        scenario_variable_is("target_down", 0.0),
                    ))
                    .deadline(30.0)
                    .add();
            }
            script = script
                // The scene is live: close the reload interval so the capture
                // excludes it from the frame-time stats. A no-op in round 1 of
                // a clean pass, where no reload was ever in flight.
                .step(format!("round {round}: close the reload interval"))
                .on_enter(nova_probe::capture_reload_end)
                .add()
                .step(format!("round {round}: raise the stance"))
                .on_enter(beat(press_mouse(MouseButton::Right), "beat: raised"))
                .until(elapsed(0.3))
                .add()
                // Hold the sweep until the combat lock is LIVE. The radar
                // writes it under the sweep, and waiting on the component
                // rather than the clock is what survives llvmpipe stutter,
                // where a wall-clock window can collapse into a single frame.
                .step(format!("round {round}: sweep the prey into a combat lock"))
                .on_enter(beat(
                    press_key(KeyCode::ControlLeft),
                    "beat: radar combat sweep",
                ))
                .until(combat_lock_live())
                .deadline(20.0)
                .add()
                .step(format!("round {round}: gun the prey down"))
                .on_enter(open_fire)
                .until(scenario_variable_is("target_down", 1.0))
                .deadline(20.0)
                .add()
                .step(format!("round {round}: lower the stance"))
                .on_enter(lower_stance)
                .until(elapsed(0.3))
                .add()
                // With the prey gone the waypoint is the sweep's only
                // candidate; same component-not-clock reasoning as above.
                .step(format!(
                    "round {round}: sweep the waypoint into a travel lock"
                ))
                .until(travel_lock_live())
                .on_enter(beat(
                    press_key(KeyCode::ControlLeft),
                    "beat: radar travel sweep",
                ))
                .deadline(20.0)
                .add()
                // GOTO with the real key, then wait until the autopilot is
                // demonstrably FLYING the locked leg. Full area arrival is
                // deliberately NOT in the headless contract: llvmpipe
                // throttles unfocused smoke windows too hard for a
                // multi-second flight leg (it arrives fine standalone and
                // interactively, and the area/OnEnter machinery is exercised
                // by the shipped scenarios and by systems/scenario_grammar).
                // The `arrived` handler stays in the scenario for interactive
                // runs.
                .step(format!("round {round}: engage the goto and start closing"))
                .on_enter(engage_goto)
                .until(and(scenario_variable_is("leg", 1.0), goto_closing()))
                .deadline(20.0)
                .add()
                // The round's verdict: the scenario saw the whole chain.
                .step(format!("round {round}: report the flown leg"))
                .on_enter(report_flown_leg(round))
                .add();
        }

        app.add_plugins(
            script
                // Enrolled in capture looping (task 20260720-000616): when a
                // frame capture outlives the script, the scene reloads and the
                // rounds replay, so the capture measures ACTIVITY rather than
                // an idle tail.
                .loop_from(LOAD_STEP)
                .on_loop(reload_the_run),
        );
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        // Run-timeline recorder (inert unless NOVA_PERF_TIMELINE names an
        // output path): the script below pushes its beats as markers, so an
        // armed run reads "raise -> sweep -> fire -> ... -> done" in the same
        // JSONL stream as the scenario's own events and variables.
        // target_down and leg are one-way latches in this scenario's design
        // (0 -> 1 on kill / lock), so a decrease is a real regression. A
        // round's reload re-seeds both, which is not one: the checker forgets
        // its monotonic memory on `ScenarioLoaded`, so each round is its own
        // life.
        app.add_plugins(nova_probe::NovaProbePlugin::default().monotonic(["target_down", "leg"]));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_run);
}

fn setup_run(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(playable_run(&game_assets, &sections)));
}

/// The run: an armed player ship, a hostile rock dead ahead, a nav beacon
/// beyond it, and handlers that echo each beat into a variable.
fn playable_run(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let ship = fixtures::ship(
        sections,
        SpaceshipController::Player(PlayerControllerConfig {
            // The shipped fire bindings (the base scenarios map turrets to
            // LMB / RightTrigger2). NOT Space/RightTrigger: both also bind
            // FlightBurnInput in the flight rig, so a held fire key would
            // burn the main drive too and coast the ship past the beacon,
            // out of the radar cone (task 20260718-235837).
            input_mapping: HashMap::from([(
                "guns".to_string(),
                vec![
                    MouseButton::Left.into(),
                    GamepadButton::RightTrigger2.into(),
                ],
            )]),
            speed_cap: None,
            infinite_ammo: true,
        }),
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
            SectionSpec::new(
                "main_drive",
                "basic_thruster_section",
                Vec3::new(0.0, 0.0, 2.0),
            ),
            SectionSpec::new(
                "guns",
                "pdc_kinetic_turret_section",
                Vec3::new(0.0, 0.75, 0.0),
            ),
        ],
    );

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "player_ship".to_string(),
                        name: "Player Ship".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                }),
                // The prey: dead ahead on the default look ray, lockable
                // (an unsigned rock is invisible to the radar).
                EventActionConfig::SpawnScenarioObject(fixtures::asteroid(
                    game_assets,
                    "prey",
                    "Prey",
                    Vec3::new(0.0, 0.0, -40.0),
                    // Small enough for real gunfire to exhaust its geometry.
                    0.25,
                    Some(1000.0),
                )),
                // The waypoint: off the boresight (the radar pick is
                // purely angular, and with the camera above the hull a
                // FARTHER on-axis object aligns better with the look ray -
                // collinear placement made the raised sweep lock the beacon
                // instead of the prey). 14 degrees off keeps it inside the
                // sweep cone for the nav lock once the prey is gone.
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "waypoint".to_string(),
                        name: "Waypoint".to_string(),
                        position: Vec3::new(14.0, 0.0, -55.0),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Beacon(BeaconConfig {
                        label: "WAYPOINT".to_string(),
                        radius: 1.5,
                        color: Color::srgb(0.3, 0.9, 0.9),
                        area_radius: Some(18.0),
                        lock_signature: None,
                    }),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "target_down".to_string(),
                    expression: number(0.0),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "leg".to_string(),
                    expression: number(0.0),
                }),
                EventActionConfig::VariableSet(VariableSetActionConfig {
                    key: "arrived".to_string(),
                    expression: number(0.0),
                }),
            ]
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            .into_iter()
            .chain(ThreePointRig::around("path", Vec3::ZERO, 5.0).actions())
            .collect(),
        },
        // The kill, seen by the scenario.
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("prey".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "target_down".to_string(),
                expression: number(1.0),
            })],
        },
        // The travel lock on the waypoint, seen by the scenario.
        ScenarioEventConfig {
            name: EventConfig::OnTravelLockStart,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("waypoint".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "leg".to_string(),
                expression: number(1.0),
            })],
        },
        // The arrival, seen by the scenario (the beacon is its own area).
        ScenarioEventConfig {
            name: EventConfig::OnEnter,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("waypoint".to_string()),
                type_name: None,
                other_id: Some("player_ship".to_string()),
                ..default()
            })],
            actions: vec![EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "arrived".to_string(),
                expression: number(1.0),
            })],
        },
    ];

    ScenarioConfig {
        description: "Kill the prey, lock the waypoint, fly there.".to_string(),
        watches: vec![WatchConfig {
            variable: "player_speed".to_string(),
            query: QueryConfig::Entity(EntityQuery {
                filter: EntityQueryFilter {
                    id: "player_ship".to_string(),
                },
                property: EntityProperty::Speed,
            }),
        }],
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Playable Run".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Let go of everything the last round left held. GOTO is the one that
/// matters - a still-pressed G would re-engage the autopilot on the fresh ship
/// before its beat, and the run would prove the chain on a lock it never took.
#[cfg(feature = "debug")]
fn release_all_held_keys(world: &mut World) {
    release_key(KeyCode::KeyG)(world);
    release_key(KeyCode::ControlLeft)(world);
    release_mouse(MouseButton::Left)(world);
    release_mouse(MouseButton::Right)(world);
}

/// Restart the cycle on an autopilot loop: release the held inputs, mark the
/// reload interval and re-trigger the same run (task 20260720-000616). The
/// step that follows the reload closes the interval once the scenario has
/// re-seeded itself.
///
/// Silently does nothing before the loader has inserted the asset resources -
/// a loop cannot happen that early, but reading them unguarded would panic.
#[cfg(feature = "debug")]
fn reload_the_run(world: &mut World) {
    release_all_held_keys(world);
    let (Some(game_assets), Some(sections)) = (
        world.get_resource::<GameAssets>().cloned(),
        world.get_resource::<GameSections>().cloned(),
    ) else {
        return;
    };
    nova_probe::capture_reload_begin(world);
    world.trigger(LoadScenario(playable_run(&game_assets, &sections)));
}

/// Wrap a gesture so it also pushes a timeline marker under `label` - the run
/// timeline then reads "raise -> sweep -> fire -> ..." in the same JSONL stream
/// as the scenario's own events.
#[cfg(feature = "debug")]
fn beat(
    gesture: impl Fn(&mut World) + Send + Sync + 'static,
    label: &'static str,
) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let t = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(world, label, serde_json::json!({ "t": t }));
        gesture(world);
    }
}

/// Advance once the player carries a live combat lock.
#[cfg(feature = "debug")]
fn combat_lock_live() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| player_lock::<CombatLock>(world, |lock| lock.0).is_some())
}

/// Advance once the player carries a live travel lock.
#[cfg(feature = "debug")]
fn travel_lock_live() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| player_lock::<TravelLock>(world, |lock| lock.0).is_some())
}

/// Advance once the engaged autopilot publishes a positive closing speed - the
/// proof that the ship is FLYING the locked leg, not merely pointed at it.
#[cfg(feature = "debug")]
fn goto_closing() -> std::sync::Arc<dyn Fn(&World) -> bool + Send + Sync> {
    std::sync::Arc::new(|world: &World| closing_speed(world).is_some_and(|speed| speed > 0.1))
}

/// The lock entity `read` pulls out of the player's `L` component, if any.
#[cfg(feature = "debug")]
fn player_lock<L: Component>(world: &World, read: impl Fn(&L) -> Option<Entity>) -> Option<Entity> {
    let mut query = world.try_query_filtered::<(&L, ()), With<PlayerSpaceshipMarker>>()?;
    let (lock, ()) = query.iter(world).next()?;
    read(lock)
}

/// The engaged autopilot's published closing speed, if the player has one.
#[cfg(feature = "debug")]
fn closing_speed(world: &World) -> Option<f32> {
    let mut query = world
        .try_query_filtered::<(&ManeuverTelemetry, &Autopilot), With<PlayerSpaceshipMarker>>()?;
    query
        .iter(world)
        .next()
        .map(|(telemetry, _)| telemetry.closing_speed)
}

/// Release the sweep and hold fire. The raised sweep must have locked the PREY,
/// not the beacon: the radar pick is purely angular, and the first collinear
/// geometry locked the waypoint so the shots flew 30 u past their target.
///
/// The stance stays raised through the burst, so the safety cannot interrupt
/// it. LMB is the shipped fire binding: the fire key must not overlap the
/// flight rig, or the burst would also burn the ship off its mark
/// (20260718-235837). One press holds until the release beat - Bevy's input
/// collection only clears the `just_*` edges.
#[cfg(feature = "debug")]
fn open_fire(world: &mut World) {
    let combat = player_lock::<CombatLock>(world, |lock| lock.0)
        .expect("player_path: the sweep step advanced without a combat lock");
    let combat_id = world
        .get::<EntityId>(combat)
        .map(|id| id.0.clone())
        .unwrap_or_default();
    assert_eq!(
        combat_id, "prey",
        "player_path: the combat lock must be on the prey"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the combat lock is on the prey",
        serde_json::json!({ "combat_lock": combat_id.clone() }),
    );
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(
        world,
        "beat: fire",
        serde_json::json!({ "t": t, "combat_lock": combat_id }),
    );
    release_key(KeyCode::ControlLeft)(world);
    press_mouse(MouseButton::Left)(world);
}

/// Cease fire and lower the stance, so the next sweep runs in the travel slot.
#[cfg(feature = "debug")]
fn lower_stance(world: &mut World) {
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: lowered", serde_json::json!({ "t": t }));
    release_mouse(MouseButton::Left)(world);
    release_mouse(MouseButton::Right)(world);
}

/// Release the travel sweep and engage GOTO with the real key.
#[cfg(feature = "debug")]
fn engage_goto(world: &mut World) {
    let t = world.resource::<Time>().elapsed_secs();
    nova_probe::probe_marker(world, "beat: goto engaged", serde_json::json!({ "t": t }));
    release_key(KeyCode::ControlLeft)(world);
    press_key(KeyCode::KeyG)(world);
}

/// Each round's last beat: the scenario saw the kill and the travel lock, and
/// the autopilot is flying the leg. Asserted per round, not once, so a chain
/// that only works on a freshly booted app fails here rather than passing.
#[cfg(feature = "debug")]
fn report_flown_leg(round: usize) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let closing =
            closing_speed(world).expect("player_path: the goto step advanced without telemetry");
        assert_eq!(
            variable(world, "target_down"),
            Some(1.0),
            "round {round}: the scenario must have seen the kill"
        );
        assert_eq!(
            variable(world, "leg"),
            Some(1.0),
            "round {round}: the scenario must have seen the travel lock"
        );
        assert!(
            variable(world, "player_speed").is_some_and(|speed| speed >= 0.0),
            "round {round}: the typed entity-speed watch must publish a number"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the scenario saw the kill",
            serde_json::json!({ "round": round }),
        );
        nova_probe::probe_marker(
            world,
            "outcome: the scenario saw the travel lock",
            serde_json::json!({ "round": round }),
        );
        nova_probe::probe_marker(
            world,
            "outcome: the entity speed watch publishes a number",
            serde_json::json!({ "round": round }),
        );
        info!(
            "player_path: round {round} - prey destroyed, waypoint locked, GOTO closing at \
             {closing:.2} u/s"
        );
        let t = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "beat: done",
            serde_json::json!({ "t": t, "round": round, "closing_speed": closing }),
        );
    }
}

/// A Number scenario variable, if the live event world holds one under `key`.
#[cfg(feature = "debug")]
fn variable(world: &World, key: &str) -> Option<f64> {
    match world.resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(value)) => Some(*value),
        _ => None,
    }
}
