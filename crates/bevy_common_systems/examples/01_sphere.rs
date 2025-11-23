use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "01_sphere")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create a basic sphere in bevy_common_systems", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(PhysicsPlugins::default());

    #[cfg(feature = "debug")]
    app.add_plugins(InspectorDebugPlugin);
    #[cfg(feature = "debug")]
    app.add_plugins(WireframeDebugPlugin);

    app.add_plugins(bevy_enhanced_input::EnhancedInputPlugin);
    app.add_plugins(WASDCameraPlugin);
    app.add_plugins(WASDCameraControllerPlugin);

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = TriangleMeshBuilder::new_octahedron(3).build();

    commands.spawn((
        Name::new("Sphere"),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.7, 0.6),
            ..Default::default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
    ));

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
