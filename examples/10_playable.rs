//! 10_playable: a playable scenario, played - by the real input pipeline.
//!
//! One armed player ship, one hostile rock dead ahead, one nav beacon
//! beyond it. The script performs the exact gestures a player would, in
//! order: raise (RMB) + radar hold (CTRL) to combat-lock the rock, gun it
//! down (the turret holds the combat lock), lower and radar again to travel-lock the beacon, press G to engage
//! GOTO, and start the flight leg toward the beacon's trigger area. The
//! SCENARIO watches the run through its own event handlers - a kill tally,
//! a travel-lock echo, an arrival flag - so the assertions read "the
//! scenario saw the player do it", not "the script poked some components".
//! Headless, the run completes at "GOTO engaged and closing"; the arrival
//! flag still fires interactively (llvmpipe throttles unfocused smoke
//! windows too hard for a full flight leg).
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! BCS_AUTOPILOT=1 cargo run --example 10_playable --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `playable: prey destroyed, waypoint locked, GOTO closing at ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "10_playable")]
#[command(version = "1.0.0")]
#[command(about = "A playable scenario driven through the real input pipeline, watched by its own event handlers", long_about = None)]
struct Cli;

const SCENARIO_ID: &str = "playable_run";

/// Total autopilot window, seconds. The run needs a kill, two radar
/// gestures and a ~40 u GOTO leg; the stock 6 s preset is too tight, so
/// this example holds its own longer window like 07_com_range/11_hud_range.
#[cfg(feature = "debug")]
const WINDOW_SECS: f32 = 18.0;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    // Headless smoke-test harness: a staged script drives the REAL gestures
    // (raise, radar, fire, lower, radar, GOTO) and the assertion reads the
    // SCENARIO's own variables - the run only passes if the event handlers
    // saw the play happen.
    #[cfg(feature = "debug")]
    {
        app.init_resource::<PlayableScript>();
        app.add_plugins(
            AutopilotPlugin::<GameStates>::new()
                .hold(GameStates::Loading, WINDOW_SECS)
                .input(playable_script),
        );
        app.add_plugins(nova_screenshot());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
    }

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_run);
}

fn setup_run(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(playable_run(&game_assets, &sections)));
}

/// Shorthand: a literal number as a variable expression.
fn number(value: f64) -> VariableExpressionNode {
    VariableExpressionNode::new_term(VariableTermNode::new_factor(
        VariableFactorNode::new_literal(VariableLiteral::Number(value)),
    ))
}

/// The run: an armed player ship, a hostile rock dead ahead, a nav beacon
/// beyond it, and handlers that echo each beat into a variable.
fn playable_run(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };

    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([(
                "guns".to_string(),
                vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
            )]),
            speed_cap: None,
            infinite_ammo: true,
            lock_refire_secs: None,
        }),
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
            SpaceshipSectionConfig {
                id: "main_drive".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("basic_thruster_section")),
                modifications: vec![],
            },
            SpaceshipSectionConfig {
                id: "guns".to_string(),
                position: Vec3::new(0.0, 1.0, -1.0),
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(section("better_turret_section")),
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
                        id: "player_ship".to_string(),
                        name: "Player Ship".to_string(),
                        position: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(ship),
                }),
                // The prey: dead ahead on the default look ray, lockable
                // (an unsigned rock is invisible to the radar).
                EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "prey".to_string(),
                        name: "Prey".to_string(),
                        position: Vec3::new(0.0, 0.0, -40.0),
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                        impact_sound: Some("base/sounds/impact.wav".into()),
                        destroy_sound: Some("base/sounds/explosion.wav".into()),
                        radius: 2.0,
                        texture: game_assets.asteroid_texture.clone().into(),
                        health: 60.0,
                        surface_gravity: None,
                        invulnerable: false,
                        lock_signature: Some(1000.0),
                    }),
                }),
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
            ],
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
            name: EventConfig::OnTravelLock,
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
        id: SCENARIO_ID.to_string(),
        name: "Playable Run".to_string(),
        description: "Kill the prey, lock the waypoint, fly there.".to_string(),
        cubemap: game_assets.cubemap.clone().into(),
        events,
        ..Default::default()
    }
}

/// Stage tracker for the playable script.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct PlayableScript {
    playing_since: Option<f32>,
    raised: bool,
    radar_combat: bool,
    fired: bool,
    lowered: bool,
    lowered_at: Option<f32>,
    radar_travel: bool,
    engaged_goto: bool,
    done: bool,
}

/// Read a Number variable from the live event world.
#[cfg(feature = "debug")]
fn number_variable(world: &World, key: &str) -> f64 {
    match world.resource::<NovaEventWorld>().get_variable(key) {
        Some(VariableLiteral::Number(value)) => *value,
        other => panic!("playable: variable {key} should be a number, got {other:?}"),
    }
}

/// The staged play, all through the real input pipeline. Timeline is
/// relative to entering Playing; the backstop fails the run loudly if the
/// script never finishes (vacuous-pass guard).
#[cfg(feature = "debug")]
fn playable_script(world: &mut World, elapsed: f32) {
    if elapsed > WINDOW_SECS - 0.5 && !world.resource::<PlayableScript>().done {
        let script = world.resource::<PlayableScript>();
        panic!(
            "playable: the run never finished (raised={} combat={} fired={} \
             travel={} goto={} done={})",
            script.raised,
            script.radar_combat,
            script.fired,
            script.radar_travel,
            script.engaged_goto,
            script.done
        );
    }
    if *world.resource::<State<GameStates>>().get() != GameStates::Playing {
        return;
    }
    let playing_since = {
        let mut script = world.resource_mut::<PlayableScript>();
        *script.playing_since.get_or_insert(elapsed)
    };
    let t = elapsed - playing_since;
    if world.resource::<PlayableScript>().done {
        return;
    }

    // Beat 1: raise, sweep, and the radar combat-locks the prey dead ahead.
    if t > 0.2 && !world.resource::<PlayableScript>().raised {
        world.resource_mut::<PlayableScript>().raised = true;
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
    }
    if t > 0.5 && !world.resource::<PlayableScript>().radar_combat {
        world.resource_mut::<PlayableScript>().radar_combat = true;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
    }
    // Beat 2: hold the sweep until the combat lock is LIVE (the radar
    // writes it under the sweep; waiting on the component instead of the
    // clock survives llvmpipe frame stutter, where a wall-clock window can
    // collapse into a single frame), assert it locked the PREY, then
    // release and hold fire until the kill.
    if world.resource::<PlayableScript>().radar_combat && !world.resource::<PlayableScript>().fired
    {
        let player = {
            let mut q = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
            q.single(world)
                .expect("playable: the player ship must exist")
        };
        let combat = world.get::<CombatLock>(player).and_then(|lock| lock.0);
        if let Some(combat) = combat {
            // The raised sweep must have locked the PREY, not the beacon
            // (the radar pick is purely angular; the first collinear
            // geometry locked the waypoint and the shots flew 30 u past
            // their target).
            let combat_id = world
                .get::<EntityId>(combat)
                .map(|id| id.0.clone())
                .unwrap_or_default();
            assert_eq!(
                combat_id, "prey",
                "playable: the combat lock must be on the prey"
            );
            world.resource_mut::<PlayableScript>().fired = true;
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(KeyCode::ControlLeft);
        }
    }
    if world.resource::<PlayableScript>().fired && !world.resource::<PlayableScript>().lowered {
        // Hold fire through the kill window (the stance stays raised, so
        // the safety cannot interrupt the burst).
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
    }
    // Beat 3: lower once the SCENARIO confirms the kill, then sweep again -
    // with the prey gone the waypoint is the sweep's only candidate.
    if world.resource::<PlayableScript>().fired
        && !world.resource::<PlayableScript>().lowered
        && number_variable(world, "target_down") == 1.0
    {
        world.resource_mut::<PlayableScript>().lowered = true;
        world.resource_mut::<PlayableScript>().lowered_at = Some(t);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Space);
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Right);
    }
    let lowered_at = world.resource::<PlayableScript>().lowered_at;
    if let Some(lowered_at) = lowered_at {
        if t > lowered_at + 0.3 && !world.resource::<PlayableScript>().radar_travel {
            world.resource_mut::<PlayableScript>().radar_travel = true;
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ControlLeft);
        }
        // Beat 4: hold the sweep until the TRAVEL lock is live (same
        // frame-stutter reasoning as the combat sweep), then release and
        // engage GOTO with the real key.
        if world.resource::<PlayableScript>().radar_travel
            && !world.resource::<PlayableScript>().engaged_goto
        {
            let player = {
                let mut q = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
                q.single(world)
                    .expect("playable: the player ship must exist")
            };
            if world
                .get::<TravelLock>(player)
                .is_some_and(|lock| lock.0.is_some())
            {
                world.resource_mut::<PlayableScript>().engaged_goto = true;
                world
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .release(KeyCode::ControlLeft);
                world
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::KeyG);
            }
        }
    }
    // Beat 5: done when the scenario saw the kill and the travel lock, and
    // the autopilot is demonstrably FLYING the locked leg (engaged, with a
    // positive closing speed on its published telemetry). Full area
    // arrival is deliberately NOT in the headless contract: under the
    // smoke suite llvmpipe throttles unfocused windows, so a multi-second
    // flight leg gets too few sim seconds to complete (it arrives fine
    // standalone and interactively; the area/OnEnter machinery is
    // exercised by the shipped scenarios). The `arrived` handler stays in
    // the scenario for interactive runs.
    if !world.resource::<PlayableScript>().done
        && number_variable(world, "target_down") == 1.0
        && number_variable(world, "leg") == 1.0
    {
        let player = {
            let mut q = world.query_filtered::<Entity, With<PlayerSpaceshipMarker>>();
            q.single(world)
                .expect("playable: the player ship must exist")
        };
        let closing = world
            .get::<ManeuverTelemetry>(player)
            .filter(|_| world.get::<Autopilot>(player).is_some())
            .map(|telemetry| telemetry.closing_speed);
        if closing.is_some_and(|speed| speed > 0.1) {
            info!(
                "playable: prey destroyed, waypoint locked, GOTO closing at {:.2} u/s",
                closing.unwrap_or_default()
            );
            world.resource_mut::<PlayableScript>().done = true;
        }
    }
}
