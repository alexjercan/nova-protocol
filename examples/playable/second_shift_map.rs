//! second_shift_map: the fixed spatial-layout candidate for the wreck-return
//! chapter of Nova Protocol's replacement campaign.
//!
//! The player approaches the old carrier site from outside a long field of
//! hand-authored wreck fragments and asteroids. Three evidence marks sit among
//! the debris. Five cleanup ships wait at their intended entry point behind the
//! large planetoid. Beacons and marker text describe candidate beats, but this
//! bench has no objectives or mission sequencing yet.
//!
//! Use `--pilot camera` for the accelerated free camera. Keys 1-5 select the
//! overview, approach, wreck, cleanup-entry, and escape views.
//! ```text
//! cargo run --example second_shift_map --features debug
//! cargo run --example second_shift_map --features debug -- --pilot camera
//! ```

#[path = "shared/first_shift_stage.rs"]
mod stage;

use std::collections::HashSet;

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use nova_authoring::prelude::*;
use nova_protocol::prelude::*;

// These landmarks and rocks are the exact fixed stage from first_shift_map.
// Only the chapter-specific ships, wreckage, and review labels change.
const PLAYER_START_POS: Meters3 = Meters3::new(-1_200.0, 300.0, -5_300.0);
const APPROACH_MARKER_POS: Meters3 = Meters3::new(-600.0, 250.0, -5_000.0);
const WRECK_CENTER: Meters3 = Meters3::new(1_300.0, 0.0, -2_800.0);
const BRIDGE_EVIDENCE_POS: Meters3 = Meters3::new(-1_000.0, 500.0, 2_500.0);
const ENGINEERING_EVIDENCE_POS: Meters3 = Meters3::new(700.0, 500.0, -1_800.0);
const RELAY_EVIDENCE_POS: Meters3 = Meters3::new(2_300.0, 700.0, -3_650.0);
const CLEANUP_ENTRY_POS: Meters3 = Meters3::new(7_900.0, 250.0, -6_500.0);
const QUIET_ROUTE_POS: Meters3 = Meters3::new(1_400.0, 500.0, -2_800.0);
const EXTRACTION_POS: Meters3 = Meters3::new(-1_500.0, 300.0, -5_700.0);

const WRECK_SCATTER: [Meters3; 4] = [
    Meters3::new(-140.0, 420.0, 90.0),
    Meters3::new(180.0, -380.0, -120.0),
    Meters3::new(110.0, 500.0, -160.0),
    Meters3::new(-190.0, -440.0, 140.0),
];

const WRECK_PLACEMENTS: [(Meters3, Vec3); 28] = [
    (stage::CARRIER_POS, Vec3::new(0.08, 0.15, -0.06)),
    (
        Meters3::new(-550.0, 160.0, 650.0),
        Vec3::new(0.45, -0.25, 0.20),
    ),
    (
        Meters3::new(650.0, -220.0, 350.0),
        Vec3::new(-0.30, 0.55, 0.35),
    ),
    (
        Meters3::new(-300.0, -350.0, 0.0),
        Vec3::new(0.70, 0.20, -0.45),
    ),
    (
        Meters3::new(400.0, 320.0, -450.0),
        Vec3::new(-0.40, -0.65, 0.15),
    ),
    (
        Meters3::new(-900.0, 100.0, -950.0),
        Vec3::new(0.20, 0.80, 0.50),
    ),
    (
        Meters3::new(950.0, -280.0, -1_400.0),
        Vec3::new(-0.65, 0.15, -0.20),
    ),
    (
        Meters3::new(-650.0, 480.0, -1_950.0),
        Vec3::new(0.35, -0.45, 0.75),
    ),
    (
        Meters3::new(100.0, -420.0, -1_000.0),
        Vec3::new(-0.15, 0.95, -0.55),
    ),
    (
        Meters3::new(500.0, -180.0, -1_300.0),
        Vec3::new(0.60, 0.35, 0.10),
    ),
    (
        Meters3::new(900.0, 300.0, -1_650.0),
        Vec3::new(-0.50, -0.20, 0.65),
    ),
    (
        Meters3::new(1_250.0, 120.0, -2_000.0),
        Vec3::new(0.25, 0.70, -0.35),
    ),
    (
        Meters3::new(350.0, 500.0, 1_750.0),
        Vec3::new(0.90, -0.15, 0.40),
    ),
    (
        Meters3::new(-650.0, -450.0, 1_800.0),
        Vec3::new(-0.25, 1.10, 0.65),
    ),
    (
        Meters3::new(-950.0, 150.0, 1_450.0),
        Vec3::new(0.55, 0.30, -0.80),
    ),
    (
        Meters3::new(-1_200.0, 600.0, 1_100.0),
        Vec3::new(-0.70, -0.35, 0.20),
    ),
    (
        Meters3::new(-1_050.0, -650.0, 700.0),
        Vec3::new(0.15, 0.85, 0.75),
    ),
    (
        Meters3::new(-1_200.0, -500.0, 300.0),
        Vec3::new(-0.45, 0.10, -0.95),
    ),
    (
        Meters3::new(-950.0, 600.0, -100.0),
        Vec3::new(0.80, -0.60, 0.30),
    ),
    (
        Meters3::new(-700.0, 750.0, -450.0),
        Vec3::new(-0.10, 0.50, 1.05),
    ),
    (
        Meters3::new(-400.0, -700.0, -700.0),
        Vec3::new(0.65, 0.95, -0.25),
    ),
    (
        Meters3::new(0.0, 250.0, -950.0),
        Vec3::new(-0.85, -0.20, 0.55),
    ),
    (
        Meters3::new(350.0, 450.0, -1_200.0),
        Vec3::new(0.30, -1.00, -0.50),
    ),
    (
        Meters3::new(650.0, -650.0, -1_450.0),
        Vec3::new(1.05, 0.25, 0.10),
    ),
    (
        Meters3::new(900.0, 700.0, -1_700.0),
        Vec3::new(-0.35, 0.70, -0.75),
    ),
    (
        Meters3::new(1_100.0, 400.0, -1_850.0),
        Vec3::new(0.50, -0.85, 0.60),
    ),
    (
        Meters3::new(1_450.0, -500.0, -2_100.0),
        Vec3::new(-0.95, 0.40, 0.25),
    ),
    (
        Meters3::new(700.0, 650.0, -2_250.0),
        Vec3::new(0.20, 1.05, -0.40),
    ),
];

/// The cleanup group, in the order the campaign introduces it: two unarmed
/// hulls, two armed escorts, then the leader. The catalog id is part of the
/// placement so the bench poses the shipped craft.
const CLEANUP_PLACEMENTS: [(Meters3, &str, &str, &str); 5] = [
    (
        Meters3::new(7_700.0, 450.0, -6_100.0),
        "cleanup_skiff",
        "Cleanup Skiff",
        BLOCK_SKIFF_SHIP_ID,
    ),
    (
        Meters3::new(8_050.0, -100.0, -6_200.0),
        "cleanup_tug",
        "Cleanup Tug",
        BLOCK_TUG_SHIP_ID,
    ),
    (
        Meters3::new(7_750.0, -350.0, -6_750.0),
        "cleanup_picket",
        "Cleanup Picket",
        BLOCK_PICKET_SHIP_ID,
    ),
    (
        Meters3::new(8_250.0, 500.0, -6_850.0),
        "cleanup_claw",
        "Cleanup Claw",
        BLOCK_CLAW_SHIP_ID,
    ),
    (
        Meters3::new(8_450.0, 100.0, -6_450.0),
        "cleanup_leader",
        "Cleanup Leader",
        BLOCK_CLEANUP_LEADER_SHIP_ID,
    ),
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Resource, ValueEnum)]
enum Pilot {
    #[default]
    Cutter,
    Camera,
}

#[derive(Parser)]
#[command(name = "second_shift_map")]
struct Cli {
    /// Fly the cutter or retain the free review camera.
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
const SHOT_NAME: &str = "second-shift-map.png";

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
    let mut actions = vec![
        spawn(ship_object(
            "player_cutter",
            "Maintenance Cutter",
            PLAYER_START_POS,
            facing(PLAYER_START_POS, WRECK_CENTER),
            if pilot == Pilot::Cutter {
                SpaceshipController::Player(PlayerControllerConfig::default())
            } else {
                SpaceshipController::None
            },
            BLOCK_CUTTER_SHIP_ID,
        )),
        spawn(beacon(
            "approach_marker",
            "APPROACH WRECK FIELD",
            APPROACH_MARKER_POS,
            Color::srgb(0.2, 1.0, 0.35),
        )),
        spawn(beacon(
            "wreck_marker",
            "SEARCH CARRIER WRECKAGE",
            WRECK_CENTER + Meters3::new(0.0, 700.0, 0.0),
            Color::srgb(1.0, 0.7, 0.15),
        )),
        spawn(beacon(
            "bridge_evidence",
            "EVIDENCE - BRIDGE RECORDER",
            BRIDGE_EVIDENCE_POS,
            Color::srgb(0.2, 0.85, 1.0),
        )),
        spawn(beacon(
            "engineering_evidence",
            "EVIDENCE - ENGINEERING LOG",
            ENGINEERING_EVIDENCE_POS,
            Color::srgb(0.2, 0.85, 1.0),
        )),
        spawn(beacon(
            "relay_evidence",
            "EVIDENCE - DISTRESS RELAY",
            RELAY_EVIDENCE_POS,
            Color::srgb(0.2, 0.85, 1.0),
        )),
        spawn(beacon(
            "cleanup_entry",
            "CLEANUP GROUP ENTRY",
            CLEANUP_ENTRY_POS,
            Color::srgb(1.0, 0.2, 0.15),
        )),
        spawn(beacon(
            "quiet_route",
            "QUIET ROUTE THROUGH ROCKS",
            QUIET_ROUTE_POS,
            Color::srgb(0.65, 0.4, 1.0),
        )),
        spawn(beacon(
            "extraction",
            "ESCAPE / COMMUNICATIONS",
            EXTRACTION_POS,
            Color::srgb(0.15, 1.0, 0.65),
        )),
    ];

    for (index, (_old_position, rotation)) in WRECK_PLACEMENTS.into_iter().enumerate() {
        let position = if index == 0 {
            stage::CARRIER_POS
        } else {
            stage::SALVAGE_ROCKS[index - 1].0 + WRECK_SCATTER[index % WRECK_SCATTER.len()]
        };
        actions.push(spawn(ship_object(
            &format!("carrier_wreck_{index}"),
            &format!("Carrier Wreck Fragment {}", index + 1),
            position,
            Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
            SpaceshipController::None,
            wreck_piece(index),
        )));
    }

    actions.extend(
        stage::belt(&game_assets.asteroid_texture)
            .into_iter()
            .map(spawn),
    );

    for (position, id, name, ship) in CLEANUP_PLACEMENTS {
        actions.push(spawn(ship_object(
            id,
            name,
            position,
            facing(position, WRECK_CENTER),
            SpaceshipController::None,
            ship,
        )));
    }

    actions.extend([
        marker("player_cutter", "PLAYER START - FLY INTO THE FIELD"),
        marker("approach_marker", "ENTER THE OUTER DEBRIS"),
        marker("wreck_marker", "SEARCH THE DESTROYED CARRIER"),
        marker("bridge_evidence", "CANDIDATE EVIDENCE 1"),
        marker("engineering_evidence", "CANDIDATE EVIDENCE 2"),
        marker("relay_evidence", "CANDIDATE EVIDENCE 3"),
        marker("cleanup_entry", "FIVE SCAVENGERS ENTER HERE"),
        marker("quiet_route", "CANDIDATE UNDETECTED ESCAPE LINE"),
        marker("extraction", "CANDIDATE EXTRACTION / COMMS END"),
        marker("concealment_planetoid", "LARGE BODY MASKS CLEANUP ENTRY"),
    ]);
    actions.extend(
        ThreePointRig::around(
            "second_shift_map",
            Meters3::new(-500.0, 0.0, -1_000.0),
            30.0,
        )
        .actions(),
    );

    ScenarioConfig {
        description: "Second-scenario wreck field and cleanup-entry candidate".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions,
        }],
        ..ScenarioConfig::new(
            "second_shift_map".to_string(),
            "Second Shift Map".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Which piece of the Meridian sits at `index`, matching the campaign: the
/// bridge tower is still where the ship was, and most of a debris field is
/// small plating.
fn wreck_piece(index: usize) -> &'static str {
    if index == 0 {
        return BLOCK_WRECK_BRIDGE_SHIP_ID;
    }
    match index % 4 {
        0 => BLOCK_WRECK_SPINE_SHIP_ID,
        1 => BLOCK_WRECK_SHOULDER_SHIP_ID,
        _ => BLOCK_WRECK_PLATE_SHIP_ID,
    }
}

fn spawn(object: ScenarioObjectConfig) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(object)
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

/// One placed catalog ship. `ship` is the CATALOG id, so the field is dressed
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

fn facing(from: Meters3, target: Meters3) -> Quat {
    Transform::from_translation(from.to_engine())
        .looking_at(target.to_engine(), Vec3::Y)
        .rotation
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
        "second_shift_map: candidate layout failed content lint:\n  {}",
        errors.join("\n  "),
    );
}

const OVERVIEW_EYE: Meters3 = Meters3::new(0.0, 12_500.0, 6_500.0);
const OVERVIEW_TARGET: Meters3 = Meters3::new(-500.0, 0.0, -2_000.0);

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
            PLAYER_START_POS + Meters3::new(600.0, 500.0, 700.0),
            WRECK_CENTER,
        ))
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some((
            WRECK_CENTER + Meters3::new(1_800.0, 1_600.0, 2_000.0),
            WRECK_CENTER,
        ))
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some((
            CLEANUP_ENTRY_POS + Meters3::new(2_400.0, 2_000.0, 2_200.0),
            CLEANUP_ENTRY_POS,
        ))
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some((
            EXTRACTION_POS + Meters3::new(1_200.0, 1_400.0, 1_500.0),
            EXTRACTION_POS,
        ))
    } else {
        None
    };

    if let Some((eye, target)) = view {
        for (entity, mut transform) in &mut cameras {
            set_view(&mut transform, eye, target);
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
