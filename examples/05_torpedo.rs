use avian3d::prelude::*;
use bevy::prelude::*;
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
    app.add_systems(
        Update,
        (
            update_target_position,
            torpedo_sync_system,
            torpedo_thrust_system,
        )
            .chain(),
    );
}

#[derive(Component, Debug, Clone, Reflect)]
struct ExampleTargetMarker;

#[derive(Component, Debug, Clone, Reflect)]
struct TorpedoMarker;

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct TorpedoTargetPosition(pub Vec3);

fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        SkyboxConfig {
            cubemap: game_assets.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_2,
            0.0,
            0.0,
        )),
        GlobalTransform::default(),
    ));

    let mut rng = rand::rng();

    // Target
    commands.spawn((
        ExampleTargetMarker,
        base_scenario_object(&BaseScenarioObjectConfig {
            id: "target_01".to_string(),
            name: "Torpedo Target".to_string(),
            position: Vec3::new(
                rng.random_range(-10.0..10.0),
                rng.random_range(-10.0..10.0),
                rng.random_range(-10.0..10.0),
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

    // Torpedo
    commands.spawn((
        TorpedoMarker,
        TorpedoTargetPosition(Vec3::ZERO),
        base_scenario_object(&BaseScenarioObjectConfig {
            id: "torpedo_01".to_string(),
            name: "Torpedo".to_string(),
            position: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            health: 100.0,
        }),
        children![
            (
                base_section(BaseSectionConfig {
                    id: "torpedo_controller".to_string(),
                    name: "Torpedo Controller".to_string(),
                    description: "The controller for the torpedo warhead".to_string(),
                    mass: 1.0,
                }),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)).with_rotation(
                    Quat::from_euler(EulerRot::XYZ, std::f32::consts::FRAC_PI_2, 0.0, 0.0)
                ),
                ControllerSectionRenderMarker,
                Mesh3d(meshes.add(Cylinder::new(0.2, 1.0))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.8, 0.8))),
                controller_section(ControllerSectionConfig {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 10.0,
                    render_mesh: None,
                }),
            ),
            (
                base_section(BaseSectionConfig {
                    id: "torpedo_thruster".to_string(),
                    name: "Torpedo Thruster".to_string(),
                    description: "The thruster for the torpedo".to_string(),
                    mass: 1.0,
                }),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.5)),
                ThrusterSectionRenderMarker,
                thruster_section(ThrusterSectionConfig {
                    magnitude: 1.0,
                    render_mesh: None,
                }),
                SpaceshipThrusterInputBinding(vec![KeyCode::KeyQ.into()]),
                children![(
                    Name::new("Thruster Exhaust"),
                    Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                        .with_translation(Vec3::new(0.0, 0.0, 0.05)),
                    ThrusterExhaustConfig {
                        exhaust_height: 0.1,
                        exhaust_max: 1.0,
                        exhaust_radius: 0.15,
                        emissive_color: LinearRgba::new(10.0, 5.0, 0.0, 1.0),
                    },
                )],
            )
        ],
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

fn torpedo_sync_system(
    q_torpedo: Query<(&Transform, &TorpedoTargetPosition, &LinearVelocity), With<TorpedoMarker>>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
) {
    for (mut controller_input, ChildOf(torpedo)) in &mut q_controller {
        if let Ok((torpedo_transform, torpedo_target_position, linear_velocity)) =
            q_torpedo.get(*torpedo)
        {
            let to_target = (**torpedo_target_position - torpedo_transform.translation).normalize();
            let forward = torpedo_transform.forward();

            let velocity = **linear_velocity;
            let sideways = velocity - forward * velocity.dot(forward.into());
            let drift_correction = -sideways * 0.05;

            let desired_dir = (to_target + drift_correction).normalize();
            let new_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, desired_dir);

            **controller_input = new_rotation;
        }
    }
}

fn torpedo_thrust_system(
    q_torpedo: Query<(&Transform, &TorpedoTargetPosition, &LinearVelocity), With<TorpedoMarker>>,
    mut q_thruster: Query<(&mut ThrusterSectionInput, &ChildOf), With<ThrusterSectionMarker>>,
) {
    for (mut thruster_input, ChildOf(torpedo)) in &mut q_thruster {
        if let Ok((torpedo_transform, torpedo_target_position, linear_velocity)) =
            q_torpedo.get(*torpedo)
        {
            let to_target = (**torpedo_target_position - torpedo_transform.translation).normalize();
            let forward = torpedo_transform.forward();

            let alignment = forward.dot(to_target).clamp(0.0, 1.0);

            let velocity = **linear_velocity;
            let sideways = velocity - forward * velocity.dot(forward.into());
            let drift_correction = -sideways.length() * 0.1;

            let steering = (alignment + drift_correction).clamp(0.0, 1.0);
            **thruster_input = steering;
        }
    }
}
