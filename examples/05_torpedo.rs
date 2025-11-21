use bevy::{platform::collections::HashMap, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;
use rand::Rng;

#[derive(Parser)]
#[command(name = "05_torpedo")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create a basic tropedo in nova_protocol", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStates::Playing), setup_scenario);
    app.add_systems(Update, update_target_position);
}

#[derive(Component, Debug, Clone, Reflect)]
struct ExampleTargetMarker;

fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.trigger(LoadScenario(example(&game_assets, &sections)));

    let mut rng = rand::rng();

    // Target
    commands.spawn((
        ExampleTargetMarker,
        base_scenario_object(&BaseScenarioObjectConfig {
            id: "target_01".to_string(),
            name: "Torpedo Target".to_string(),
            position: Vec3::new(
                rng.random_range(-100.0..100.0),
                rng.random_range(-100.0..100.0),
                rng.random_range(-100.0..100.0),
            ),
            rotation: Quat::IDENTITY,
            health: 100.0,
        }),
        children![(
            Name::new("Target Marker"),
            Transform::from_translation(Vec3::ZERO),
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        )],
    ));
}

fn update_target_position(
    target: Single<&Transform, With<ExampleTargetMarker>>,
    mut q_torpedo: Query<&mut TorpedoTargetPosition>,
) {
    let target_transform = target.into_inner();

    for mut torpedo_target_position in &mut q_torpedo {
        **torpedo_target_position = target_transform.translation;
    }
}

pub fn example(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let mut objects = Vec::new();
    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: HashMap::from([
                (
                    "thruster".to_string(),
                    vec![KeyCode::Space.into(), GamepadButton::RightTrigger.into()],
                ),
                (
                    "torpedo".to_string(),
                    vec![
                        MouseButton::Left.into(),
                        GamepadButton::RightTrigger2.into(),
                    ],
                ),
            ]),
        }),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_controller_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_thruster_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "torpedo".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: sections
                    .get_section("torpedo_section")
                    .unwrap()
                    .clone(),
            },
        ],
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player Spaceship".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            health: 500.0,
        },
        kind: ScenarioObjectKind::Spaceship(spaceship),
    });

    let events = vec![
        // OnStart: Create the scenario objects
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .collect::<_>(),
        },
    ];

    ScenarioConfig {
        id: "asteroid_field".to_string(),
        name: "Asteroid Field".to_string(),
        description: "A dense asteroid field.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}
