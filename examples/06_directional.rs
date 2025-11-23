use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "06_directional")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how the directional HUD works", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = App::new();

    // Base plugins for the things to work
    app.add_plugins(DefaultPlugins);
    app.add_plugins(PhysicsPlugins::default());

    #[cfg(feature = "debug")]
    app.add_plugins(InspectorDebugPlugin);

    // WASD Camera for navigation
    app.add_plugins(bevy_enhanced_input::EnhancedInputPlugin);
    app.add_plugins(WASDCameraPlugin);
    app.add_plugins(WASDCameraControllerPlugin);

    // The Velocity HUD plugin (what we test here)
    app.add_plugins(VelocityHudPlugin);

    // Required by the HUD Plugin
    app.add_plugins(DirectionalSphereOrbitPlugin);

    app.add_systems(Startup, (setup_camera, setup_hud));

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
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
}

fn setup_hud(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let entity = commands
        .spawn((
            Name::new("Target"),
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 0.2),
                ..default()
            })),
            LinearVelocity(Vec3::new(0.0, 0.0, -10.0)),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    commands.spawn((velocity_hud(VelocityHudConfig {
        radius: 5.0,
        sharpness: 20.0,
        target: entity,
    }),));
}
