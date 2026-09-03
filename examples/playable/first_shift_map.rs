//! first_shift_map: the fixed spatial layout candidate for Nova Protocol's
//! replacement opening, with the accepted ship candidates included for scale.
//!
//! The scene exposes the whole tutorial route at once: the carrier and launch,
//! first flight beacon, salvage cluster, GOTO approach, inspection planetoid,
//! and the larger body that hides the warship's emergence point. Objective
//! markers and the HUD list are review labels, not final mission wording.
//!
//! Choose a pilot from the CLI. `camera` keeps the accelerated free camera and
//! keys 1-5 for the overview, launch, salvage, orbit, and ambush views:
//! ```text
//! cargo run --example first_shift_map --features debug -- --pilot warship
//! cargo run --example first_shift_map --features debug -- --pilot cutter
//! cargo run --example first_shift_map --features debug -- --pilot camera
//! ```

#[path = "shared/first_shift.rs"]
mod first_shift;

use std::collections::{BTreeMap, HashSet};

use bevy::prelude::*;
use clap::{Parser, ValueEnum};
use nova_protocol::prelude::*;

const PLAYER_START_POS: Meters3 = Meters3::new(-100.0, 0.0, 1_000.0);
const START_BEACON_POS: Meters3 = Meters3::new(-100.0, 250.0, 1_000.0);
const CARRIER_POS: Meters3 = Meters3::new(0.0, 0.0, 1_000.0);
const HOME_BEACON_POS: Meters3 = Meters3::new(0.0, 500.0, 1_000.0);
const FLIGHT_BEACON_POS: Meters3 = Meters3::new(-500.0, 0.0, -1_000.0);
const SALVAGE_CENTER: Meters3 = Meters3::new(0.0, 200.0, -6_000.0);
const APPROACH_POS: Meters3 = Meters3::new(3_000.0, 700.0, -4_800.0);
const INSPECTION_POS: Meters3 = Meters3::new(4_500.0, -400.0, -6_500.0);
const CONCEALMENT_POS: Meters3 = Meters3::new(-4_500.0, 300.0, -6_500.0);
const WARSHIP_POS: Meters3 = Meters3::new(-7_900.0, 250.0, -6_500.0);
const EMERGENCE_BEACON_POS: Meters3 = Meters3::new(-7_900.0, 650.0, -6_500.0);

const INSPECTION_RADIUS: Meters = Meters(200.0);
const CONCEALMENT_RADIUS: Meters = Meters(500.0);
const WARSHIP_RAILGUN_POWER_MULTIPLIER: f32 = 200.0;
const WARSHIP_RAILGUN_DAMAGE: f32 = 500.0;
const WARSHIP_RAILGUN_RAKE_RADIUS: Meters = Meters(30.0);

const WARSHIP_RAILGUN_IDS: [&str; 2] = ["railgun_port", "railgun_starboard"];

const WARSHIP_PDC_IDS: [&str; 10] = [
    "pdc_forward_port",
    "pdc_forward_starboard",
    "pdc_aft_port",
    "pdc_aft_starboard",
    "pdc_dorsal_port",
    "pdc_dorsal_starboard",
    "pdc_ventral_forward_port",
    "pdc_ventral_forward_starboard",
    "pdc_ventral_aft_port",
    "pdc_ventral_aft_starboard",
];

const WARSHIP_TORPEDO_IDS: [&str; 6] = [
    "bastion_bay_port_forward",
    "bastion_bay_port_midships",
    "bastion_bay_port_aft",
    "bastion_bay_starboard_forward",
    "bastion_bay_starboard_midships",
    "bastion_bay_starboard_aft",
];

const ROCK_OFFSETS: [Meters3; 14] = [
    Meters3::new(-520.0, 80.0, 260.0),
    Meters3::new(-390.0, -180.0, -220.0),
    Meters3::new(-260.0, 310.0, 30.0),
    Meters3::new(-100.0, -260.0, 340.0),
    Meters3::new(40.0, 170.0, -380.0),
    Meters3::new(180.0, -90.0, 110.0),
    Meters3::new(320.0, 260.0, -170.0),
    Meters3::new(470.0, -210.0, 290.0),
    Meters3::new(610.0, 120.0, -40.0),
    Meters3::new(-650.0, 390.0, -90.0),
    Meters3::new(-210.0, 520.0, -430.0),
    Meters3::new(140.0, -430.0, -150.0),
    Meters3::new(530.0, 440.0, 410.0),
    Meters3::new(760.0, -120.0, -330.0),
];

const ROCK_RADII: [Meters; 14] = [
    Meters(28.0),
    Meters(18.0),
    Meters(35.0),
    Meters(22.0),
    Meters(15.0),
    Meters(30.0),
    Meters(20.0),
    Meters(26.0),
    Meters(14.0),
    Meters(32.0),
    Meters(17.0),
    Meters(24.0),
    Meters(20.0),
    Meters(30.0),
];

const CRATE_POSITIONS: [Meters3; 3] = [
    Meters3::new(-585.0, -57.0, -6_414.0),
    Meters3::new(105.0, 590.0, -5_620.0),
    Meters3::new(681.0, 423.0, -6_314.0),
];

const AMBIENT_ROCKS: [(Meters3, Meters); 20] = [
    (Meters3::new(-6_000.0, 1_000.0, -1_000.0), Meters(55.0)),
    (Meters3::new(-4_200.0, -900.0, -2_500.0), Meters(40.0)),
    (Meters3::new(-2_500.0, 1_300.0, -1_500.0), Meters(65.0)),
    (Meters3::new(-1_800.0, -1_100.0, -3_800.0), Meters(35.0)),
    (Meters3::new(500.0, 1_000.0, -2_500.0), Meters(45.0)),
    (Meters3::new(1_600.0, -900.0, -1_200.0), Meters(60.0)),
    (Meters3::new(3_200.0, 1_300.0, -2_700.0), Meters(38.0)),
    (Meters3::new(5_200.0, -1_000.0, -2_000.0), Meters(70.0)),
    (Meters3::new(7_000.0, 700.0, -3_500.0), Meters(42.0)),
    (Meters3::new(8_000.0, -1_200.0, -5_000.0), Meters(55.0)),
    (Meters3::new(-8_000.0, 1_500.0, -3_500.0), Meters(48.0)),
    (Meters3::new(-8_500.0, -1_000.0, -8_000.0), Meters(75.0)),
    (Meters3::new(-6_500.0, 1_800.0, -10_000.0), Meters(44.0)),
    (Meters3::new(-2_500.0, -1_500.0, -10_000.0), Meters(62.0)),
    (Meters3::new(1_000.0, 1_700.0, -9_500.0), Meters(40.0)),
    (Meters3::new(3_500.0, -1_400.0, -10_000.0), Meters(68.0)),
    (Meters3::new(6_500.0, 1_600.0, -9_500.0), Meters(50.0)),
    (Meters3::new(8_500.0, -800.0, -8_000.0), Meters(72.0)),
    (Meters3::new(9_000.0, 1_200.0, -6_000.0), Meters(46.0)),
    (Meters3::new(7_000.0, -1_600.0, -7_000.0), Meters(58.0)),
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

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(map_plugin).build();
    app.insert_resource(cli.pilot);

    #[cfg(feature = "debug")]
    app.add_systems(Update, freeze_bodies.run_if(camera_pilot));

    app.run()
}

fn camera_pilot(pilot: Res<Pilot>) -> bool {
    *pilot == Pilot::Camera
}

fn map_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_map);
    app.add_systems(Update, (frame_new_camera, accelerate_camera, select_view));
}

fn load_map(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    pilot: Res<Pilot>,
) {
    let scenario = map_scenario(&game_assets, &sections, *pilot);
    refuse_broken(&scenario, &sections);
    commands.trigger(LoadScenario(scenario));
}

fn map_scenario(game_assets: &GameAssets, sections: &GameSections, pilot: Pilot) -> ScenarioConfig {
    assert_crates_clear_rocks();
    let mut actions = vec![
        spawn(ship_object(
            "player_cutter",
            "Player Maintenance Cutter",
            PLAYER_START_POS,
            Quat::IDENTITY,
            controller_for(pilot, Pilot::Cutter),
            first_shift::maintenance_cutter(),
        )),
        spawn(ship_object(
            "industrial_carrier",
            "Industrial Carrier",
            CARRIER_POS,
            Quat::IDENTITY,
            SpaceshipController::None,
            first_shift::industrial_carrier(),
        )),
        spawn(ship_object(
            "stolen_warship",
            "Hidden Stolen Warship",
            WARSHIP_POS,
            facing(WARSHIP_POS, CARRIER_POS),
            controller_for(pilot, Pilot::Warship),
            tuned_warship(sections),
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
        spawn(asteroid(
            "inspection_planetoid",
            "Inspection Planetoid",
            INSPECTION_POS,
            INSPECTION_RADIUS,
            Some(27_000.0),
            &game_assets.asteroid_texture,
        )),
        spawn(asteroid(
            "concealment_planetoid",
            "Concealment Planetoid",
            CONCEALMENT_POS,
            CONCEALMENT_RADIUS,
            Some(20_000.0),
            &game_assets.asteroid_texture,
        )),
        spawn(beacon(
            "emergence_marker",
            "5 HIDDEN WARSHIP",
            EMERGENCE_BEACON_POS,
            Color::srgb(1.0, 0.2, 0.15),
        )),
    ];

    for (index, (offset, radius)) in ROCK_OFFSETS.iter().zip(ROCK_RADII).enumerate() {
        actions.push(spawn(asteroid(
            &format!("salvage_rock_{index}"),
            &format!("Salvage Rock {}", index + 1),
            SALVAGE_CENTER + *offset,
            radius,
            None,
            &game_assets.asteroid_texture,
        )));
    }
    for (index, position) in CRATE_POSITIONS.into_iter().enumerate() {
        actions.push(spawn(crate_object(index + 1, position)));
    }
    for (index, (position, radius)) in AMBIENT_ROCKS.into_iter().enumerate() {
        actions.push(spawn(asteroid(
            &format!("ambient_rock_{index}"),
            &format!("Ambient Rock {}", index + 1),
            position,
            radius,
            None,
            &game_assets.asteroid_texture,
        )));
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
        marker(
            "salvage_marker",
            "2 CRATES + ASTEROID FIELD BETWEEN BOTH PLANETOIDS",
        ),
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
        for (offset, radius) in ROCK_OFFSETS.into_iter().zip(ROCK_RADII) {
            let rock_position = SALVAGE_CENTER + offset;
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

fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    controller: SpaceshipController,
    hull: ShipHull,
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
            hull: ShipSource::Inline(hull),
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
        for id in WARSHIP_PDC_IDS {
            bindings.insert(id.to_string(), vec![MouseButton::Left.into()]);
        }
        for id in WARSHIP_RAILGUN_IDS {
            bindings.insert(id.to_string(), vec![KeyCode::KeyR.into()]);
        }
        for id in WARSHIP_TORPEDO_IDS {
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

fn tuned_warship(sections: &GameSections) -> ShipHull {
    let mut railgun = sections
        .iter()
        .find(|section| section.base.id == "railgun_lance_section")
        .cloned()
        .expect("first_shift_map: railgun prototype is loaded");
    let SectionKind::Railgun(config) = &mut railgun.kind else {
        panic!("first_shift_map: railgun prototype has the wrong section kind");
    };
    config.slug_damage = WARSHIP_RAILGUN_DAMAGE;
    config.slug_power *= WARSHIP_RAILGUN_POWER_MULTIPLIER;
    config.rake_radius = Some(WARSHIP_RAILGUN_RAKE_RADIUS);

    let mut hull = first_shift::stolen_warship();
    for section in &mut hull.sections {
        if WARSHIP_RAILGUN_IDS.contains(&section.id.as_str()) {
            section.source = SectionSource::Inline(railgun.clone());
        }
    }
    hull
}

fn facing(from: Meters3, target: Meters3) -> Quat {
    Transform::from_translation(from.to_engine())
        .looking_at(target.to_engine(), Vec3::Y)
        .rotation
}

fn asteroid(
    id: &str,
    name: &str,
    position: Meters3,
    radius: Meters,
    mass: Option<f32>,
    texture: &Handle<Image>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            material: None,
            destroy_sound: None,
            radius,
            texture: texture.clone().into(),
            mass,
            invulnerable: true,
            seed: None,
            lock_signature: None,
        }),
    }
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

fn refuse_broken(scenario: &ScenarioConfig, sections: &GameSections) {
    let known = KnownSections::from_configs(sections.iter());
    let issues = lint_scenario(
        scenario,
        &known,
        &KnownShips::default(),
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
            INSPECTION_POS + Meters3::new(0.0, 2_500.0, 2_800.0),
            INSPECTION_POS,
        ))
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some((
            CONCEALMENT_POS + Meters3::new(2_800.0, 3_800.0, 3_200.0),
            CONCEALMENT_POS,
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
