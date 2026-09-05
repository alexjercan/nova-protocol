//! first_shift_map: the fixed spatial layout candidate for Nova Protocol's
//! replacement opening, with the accepted ship candidates included for scale.
//!
//! The scene exposes the whole tutorial route at once: the carrier and launch,
//! first flight beacon, the cutter-only rock plate between the carrier and
//! planetoids, GOTO approach, inspection planetoid, and the larger body that hides the
//! warship's emergence point. Objective
//! markers and the HUD list are review labels, not final mission wording.
//!
//! Choose a pilot from the CLI. `camera` keeps the accelerated free camera and
//! keys 1-5 for the overview, launch, salvage, orbit, and ambush views:
//! ```text
//! cargo run --example first_shift_map --features debug -- --pilot warship
//! cargo run --example first_shift_map --features debug -- --pilot cutter
//! cargo run --example first_shift_map --features debug -- --pilot camera
//! ```

#[path = "shared/first_shift_stage.rs"]
mod stage;

use std::collections::{BTreeMap, HashSet};

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

const PLAYER_START_POS: Meters3 = Meters3::new(-1_100.0, 0.0, 2_500.0);
const START_BEACON_POS: Meters3 = Meters3::new(-1_100.0, 250.0, 2_500.0);
const HOME_BEACON_POS: Meters3 = Meters3::new(-1_000.0, 500.0, 2_500.0);
const FLIGHT_BEACON_POS: Meters3 = Meters3::new(0.0, 100.0, -900.0);
const SALVAGE_CENTER: Meters3 = Meters3::new(1_400.0, 0.0, -2_800.0);
const APPROACH_POS: Meters3 = Meters3::new(-2_500.0, 700.0, -5_700.0);
const WARSHIP_POS: Meters3 = Meters3::new(7_900.0, 250.0, -6_500.0);
const EMERGENCE_BEACON_POS: Meters3 = Meters3::new(7_900.0, 650.0, -6_500.0);

const CRATE_POSITIONS: [Meters3; 3] = [
    Meters3::new(2_800.0, 20.0, -3_800.0),
    Meters3::new(2_300.0, 20.0, -4_250.0),
    Meters3::new(1_700.0, 20.0, -4_400.0),
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Resource, ValueEnum)]
enum Pilot {
    Cutter,
    #[default]
    Warship,
    Camera,
}

#[derive(Parser)]
#[command(name = "first_shift_map")]
struct Cli {
    /// Select the controlled ship, or retain the free review camera.
    #[arg(long, value_enum, default_value_t)]
    pilot: Pilot,
}

/// Frames the loaded map is held before its shot. Sized to outlast the
/// frame-time window below: the capture has to open and close inside one step.
#[cfg(feature = "debug")]
const HOLD_FRAMES: u32 = 460;
/// Warmup and measured frames of the frame-time window. This is the most
/// populated authored scene in the game, and nothing else measures it.
#[cfg(feature = "debug")]
const FRAMETIME_WINDOW: (u32, u32) = (60, 300);
/// The still a harnessed run writes.
#[cfg(feature = "debug")]
const SHOT_NAME: &str = "first-shift-map.png";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(map_plugin).build();
    app.insert_resource(cli.pilot);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants, so `probe run` grades this
        // example instead of hanging on an app with nothing to end it.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        // The frame-time claim is declared only when a measured run asks for
        // it, so `probe run` stays a correctness walk and the sweep still gets
        // its artifact.
        if nova_probe::probe_armed() {
            let (warmup, frames) = FRAMETIME_WINDOW;
            app.add_plugins(nova_probe::nova_frametime().window(warmup, frames));
        }
        app.add_plugins(map_script());
        app.add_systems(Update, freeze_bodies.run_if(camera_pilot));
    }

    app.run()
}

#[cfg(feature = "debug")]
fn camera_pilot(pilot: Res<Pilot>) -> bool {
    *pilot == Pilot::Camera
}

/// The probe script every harnessed run walks: reach the loaded map, hold it
/// long enough for an ARMED frame-time window to close inside one step, then
/// shoot it and exit. An unarmed run walks the identical steps and measures
/// nothing, so this is also the smoke path.
#[cfg(feature = "debug")]
fn map_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("wait for the map")
        .enter(GameStates::Loading)
        .until(and(
            state_is(GameStates::Playing),
            scenario_camera_present(),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("hold the loaded map")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("shoot the map")
        .on_enter(|world: &mut World| shoot(world, SHOT_NAME))
        .until(shot_written(SHOT_NAME))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}

fn map_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_map);
    app.add_systems(Update, (frame_new_camera, accelerate_camera, select_view));
}

fn load_map(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    ships: Res<GameShips>,
    pilot: Res<Pilot>,
) {
    let scenario = map_scenario(&game_assets, *pilot);
    refuse_broken(&scenario, &sections, &ships);
    commands.trigger(LoadScenario(scenario));
}

fn map_scenario(game_assets: &GameAssets, pilot: Pilot) -> ScenarioConfig {
    assert_crates_clear_rocks();
    let mut actions = vec![
        spawn(ship_object(
            "player_cutter",
            "Player Maintenance Cutter",
            PLAYER_START_POS,
            Quat::IDENTITY,
            controller_for(pilot, Pilot::Cutter),
            BLOCK_CUTTER_SHIP_ID,
        )),
        spawn(ship_object(
            "industrial_carrier",
            "Industrial Carrier",
            stage::CARRIER_POS,
            Quat::IDENTITY,
            SpaceshipController::None,
            BLOCK_CARRIER_SHIP_ID,
        )),
        spawn(ship_object(
            "stolen_warship",
            "Hidden Stolen Warship",
            WARSHIP_POS,
            facing(WARSHIP_POS, stage::CARRIER_POS),
            controller_for(pilot, Pilot::Warship),
            BLOCK_WARSHIP_SHIP_ID,
        )),
        spawn(beacon(
            "start_marker",
            "START",
            START_BEACON_POS,
            Color::srgb(0.2, 1.0, 0.3),
        )),
        spawn(beacon(
            "carrier_berth",
            "HOME / ATTACK DESTINATION",
            HOME_BEACON_POS,
            Color::srgb(0.2, 0.55, 1.0),
        )),
        spawn(beacon(
            "flight_beacon",
            "1 FLIGHT CHECK",
            FLIGHT_BEACON_POS,
            Color::srgb(0.2, 0.9, 1.0),
        )),
        spawn(beacon(
            "salvage_marker",
            "2 SALVAGE",
            SALVAGE_CENTER,
            Color::srgb(1.0, 0.75, 0.2),
        )),
        spawn(beacon(
            "approach_beacon",
            "3 GOTO APPROACH",
            APPROACH_POS,
            Color::srgb(0.75, 0.35, 1.0),
        )),
        spawn(beacon(
            "emergence_marker",
            "5 HIDDEN WARSHIP",
            EMERGENCE_BEACON_POS,
            Color::srgb(1.0, 0.2, 0.15),
        )),
    ];

    actions.extend(
        stage::belt(&game_assets.asteroid_texture)
            .into_iter()
            .map(spawn),
    );
    for (index, position) in CRATE_POSITIONS.into_iter().enumerate() {
        actions.push(spawn(crate_object(index + 1, position)));
    }

    actions.extend(pilot_objectives(pilot));
    actions.extend([
        marker("player_cutter", "PLAYER SPAWN - GREEN MARKER"),
        marker(
            "industrial_carrier",
            "CARRIER - BEHIND PLAYER / ATTACK TARGET",
        ),
        marker("carrier_berth", "BLUE - HOME / ATTACK DESTINATION"),
        marker("flight_beacon", "1 MANUAL FLIGHT"),
        marker("salvage_marker", "2 CRATES + CUTTER-ONLY ROCK PLATE"),
        marker("approach_beacon", "3 LOCK + GOTO APPROACH"),
        marker(
            "inspection_planetoid",
            "4 SMALL PLANETOID - INSPECT + ORBIT",
        ),
        marker("concealment_planetoid", "LARGE PLANETOID - HIDES WARSHIP"),
        marker("stolen_warship", "WARSHIP - HIDDEN BEFORE ATTACK"),
        marker("emergence_marker", "5 RED - WARSHIP EMERGENCE"),
    ]);
    actions.extend(
        ThreePointRig::around(
            "first_shift_map",
            Meters3::new(2_000.0, 0.0, -2_500.0),
            25.0,
        )
        .actions(),
    );

    ScenarioConfig {
        description: "First Shift map and sight-line candidate".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "first_shift_map".to_string(),
            "First Shift Map".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

fn assert_crates_clear_rocks() {
    // Asteroid runtime geometry can reach the exported maximum noise factor.
    // Convert to engine units only at this geometry boundary and reserve the
    // crate's own 15 m half-envelope beyond that worst-case surface.
    let crate_clearance = Meters(15.0).to_engine();
    for crate_position in CRATE_POSITIONS {
        for (rock_position, radius) in stage::SALVAGE_ROCKS {
            let separation = crate_position
                .to_engine()
                .distance(rock_position.to_engine());
            let required = radius.to_engine() * ASTEROID_GEOMETRIC_FACTOR_MAX + crate_clearance;
            assert!(
                separation > required,
                "crate at {crate_position:?} intersects the worst-case rock at {rock_position:?}",
            );
        }
    }
}

fn spawn(object: ScenarioObjectConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(object)
}

fn objective(id: &str, message: &str) -> EventActionConfig {
    EventActionConfig::Objective(ObjectiveActionConfig::new(id, message))
}

fn pilot_objectives(pilot: Pilot) -> Vec<EventActionConfig> {
    match pilot {
        Pilot::Cutter => vec![
            objective("route", "Fly from green to the nearby cyan flight beacon."),
            objective(
                "salvage",
                "Recover three maintenance crates from the rock cluster.",
            ),
            objective("orbit", "Approach and orbit the inspection planetoid."),
            objective("return", "Return toward the blue carrier marker."),
        ],
        Pilot::Warship => vec![
            objective(
                "attack_run",
                "Clear the concealment planetoid and approach the blue carrier marker.",
            ),
            objective(
                "weapon_test",
                "Fire PDCs with [LMB], railguns with [R], and torpedoes with [F].",
            ),
        ],
        Pilot::Camera => vec![objective(
            "review",
            "Review the route and sight lines; keys 1-5 select fixed views.",
        )],
    }
}

fn marker(target: &str, label: &str) -> EventActionConfig {
    EventActionConfig::ObjectiveMarkerAttach(ObjectiveMarkerAttachActionConfig::new(target, label))
}

fn beacon(id: &str, label: &str, position: Meters3, color: Color) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: Meters(20.0),
            color,
            area_radius: None,
            lock_signature: None,
        }),
    }
}

/// One placed catalog ship. `ship` is the CATALOG id, so the layout is flown
/// with the hulls the campaign ships rather than copies of them.
fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    ship: &str,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller,
            hull: hull(ship),
            ..default()
        }),
    }
}

fn controller_for(pilot: Pilot, ship: Pilot) -> SpaceshipController {
    if pilot != ship {
        return SpaceshipController::None;
    }

    let input_mapping = if ship == Pilot::Warship {
        let mut bindings = BTreeMap::new();
        for id in BLOCK_WARSHIP_TURRET_IDS {
            bindings.insert(id.to_string(), vec![MouseButton::Left.into()]);
        }
        for id in BLOCK_WARSHIP_RAILGUN_IDS {
            bindings.insert(id.to_string(), vec![KeyCode::KeyR.into()]);
        }
        for id in BLOCK_WARSHIP_BAY_IDS {
            bindings.insert(id.to_string(), vec![KeyCode::KeyF.into()]);
        }
        bindings
    } else {
        BTreeMap::new()
    };

    SpaceshipController::Player(PlayerControllerConfig {
        input_mapping,
        speed_cap: None,
    })
}

fn facing(from: Meters3, target: Meters3) -> Quat {
    Transform::from_translation(from.to_engine())
        .looking_at(target.to_engine(), Vec3::Y)
        .rotation
}

fn crate_object(index: usize, position: Meters3) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("maintenance_crate_{index}"),
            name: format!("Maintenance Crate {index}"),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
            size: Meters(15.0),
            area_radius: Meters(80.0),
            pickup_sound: None,
        }),
    }
}

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections, ships: &GameShips) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::from_configs(ships.iter()),
        &HashSet::from([scenario.id.clone()]),
    );
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == LintSeverity::Error)
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        errors.is_empty(),
        "first_shift_map: candidate layout failed content lint:\n  {}",
        errors.join("\n  "),
    );
}

const OVERVIEW_EYE: Meters3 = Meters3::new(500.0, 12_000.0, 6_000.0);
const OVERVIEW_TARGET: Meters3 = Meters3::new(500.0, 0.0, -4_500.0);

fn frame_new_camera(
    mut cameras: Query<&mut Transform, (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>)>,
) {
    for mut transform in &mut cameras {
        set_view(&mut transform, OVERVIEW_EYE, OVERVIEW_TARGET);
    }
}

fn accelerate_camera(mut cameras: Query<&mut WASDCamera>) {
    for mut camera in &mut cameras {
        camera.wasd_sensitivity = 2.0;
    }
}

fn select_view(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut cameras: Query<
        (Entity, &mut Transform),
        (With<ScenarioCameraMarker>, With<WASDCameraController>),
    >,
) {
    let view = if keys.just_pressed(KeyCode::Digit1) {
        Some((OVERVIEW_EYE, OVERVIEW_TARGET))
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some((
            PLAYER_START_POS + Meters3::new(350.0, 250.0, 500.0),
            PLAYER_START_POS,
        ))
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some((
            SALVAGE_CENTER + Meters3::new(0.0, 1_500.0, 1_800.0),
            SALVAGE_CENTER,
        ))
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some((
            stage::INSPECTION_POS + Meters3::new(0.0, 2_500.0, 2_800.0),
            stage::INSPECTION_POS,
        ))
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some((
            stage::CONCEALMENT_POS + Meters3::new(2_800.0, 3_800.0, 3_200.0),
            stage::CONCEALMENT_POS,
        ))
    } else {
        None
    };

    if let Some((eye, target)) = view {
        for (entity, mut transform) in &mut cameras {
            set_view(&mut transform, eye, target);
            // Reinsert the rig so its private target state starts at the new
            // pose instead of restoring the previous free-fly pose next frame.
            commands
                .entity(entity)
                .remove::<WASDCamera>()
                .insert(WASDCamera {
                    wasd_sensitivity: 2.0,
                    ..default()
                });
        }
    }
}

fn set_view(transform: &mut Transform, eye: Meters3, target: Meters3) {
    *transform =
        Transform::from_translation(eye.to_engine()).looking_at(target.to_engine(), Vec3::Y);
}
