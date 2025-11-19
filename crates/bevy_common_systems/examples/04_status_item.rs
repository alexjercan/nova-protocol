use std::{process::Command, sync::Arc};

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "04_status_item")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to use the status bar", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(PhysicsPlugins::default());

    #[cfg(feature = "debug")]
    app.add_plugins(InspectorDebugPlugin);
    if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
        app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
    }

    app.add_plugins(custom_plugin);

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_plugins(StatusBarPlugin);

    app.add_systems(Startup, setup_camera);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((status_bar(StatusBarRootConfig::default()),));

    commands.spawn(status_bar_item(StatusBarItemConfig {
        icon: None,
        value_fn: |_| {
            let output = Command::new("uname")
                .arg("-r")
                .output()
                .expect("Failed to execute uname");

            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(Arc::new(result) as Arc<dyn StatusValue>)
        },
        color_fn: |_| Some(Color::srgb(1.0, 1.0, 1.0)),
        prefix: "kernel".to_string(),
        suffix: "".to_string(),
    }));
    commands.spawn((status_bar_item(StatusBarItemConfig {
        icon: None,
        value_fn: status_fps_value_fn(),
        color_fn: status_fps_color_fn(),
        prefix: "".to_string(),
        suffix: "fps".to_string(),
    }),));
    commands.spawn((status_bar_item(StatusBarItemConfig {
        icon: None,
        value_fn: status_version_value_fn(env!("CARGO_PKG_VERSION")),
        color_fn: status_version_color_fn(),
        prefix: "v".to_string(),
        suffix: "".to_string(),
    }),));
}
