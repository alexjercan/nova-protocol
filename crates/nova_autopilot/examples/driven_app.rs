//! A self-contained Bevy app driven end to end by [`AutopilotPlugin`].
//!
//! This is the crate's standalone proof: a real `DefaultPlugins` app with its
//! own three-state machine, its own input handling and its own scene, driven
//! `Boot -> Flying -> Done` by the autopilot and exited by the completion
//! protocol. It imports no `nova_*` crate but `nova_autopilot` itself, so a
//! run that builds is also a run that proves the crate has no game coupling.
//!
//! Run it armed (it does nothing without the env var):
//!
//! ```sh
//! NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app
//! ```
//!
//! `tests/autopilot_example.rs` runs exactly that headless and asserts on the
//! exit status and the log lines. The probe run-harness reuses this shape for
//! its correctness and profiling runs. (Named without the crate path on
//! purpose: the no-coupling proof greps this directory for `nova_*` mentions.)

use bevy::prelude::*;
use nova_autopilot::autopilot::AutopilotPlugin;

/// The example's own state machine - deliberately NOT a Nova type: the driver
/// is generic over `S: States`, and this is what exercises that generic.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Boot,
    Flying,
    Done,
}

/// The one thing the autopilot's input closure is supposed to cause. Read by
/// the `Done` guard below, so a run that stops driving fails the process.
#[derive(Resource, Default)]
struct Thrust {
    moved: bool,
}

/// The driven object.
#[derive(Component)]
struct Cube;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.init_state::<DemoState>();
    app.init_resource::<Thrust>();
    app.add_plugins(
        AutopilotPlugin::new()
            .hold(DemoState::Boot, 0.5)
            .hold(DemoState::Flying, 2.0)
            .hold(DemoState::Done, 0.5)
            .input(|world, _elapsed| {
                // Gated to Flying: the closure runs in EVERY state, and a
                // press outside the gameplay state is exactly what the plugin
                // docs warn about.
                if *world.resource::<State<DemoState>>().get() != DemoState::Flying {
                    return;
                }
                world
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Space);
            }),
    );
    app.add_systems(Startup, spawn_scene);
    app.add_systems(Update, thrust);
    app.add_systems(OnEnter(DemoState::Done), assert_the_cube_moved);
    app.run()
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Cube,
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.4, 0.6, 0.9))),
        Transform::default(),
    ));
}

/// The game side of the loop: read real `ButtonInput`, move the cube. Nothing
/// here knows about the autopilot - the press arrives through Bevy's own input
/// resource, which is the point of the closure running after `InputSystems`.
fn thrust(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut thrust: ResMut<Thrust>,
    mut cube: Single<&mut Transform, With<Cube>>,
) {
    if !input.pressed(KeyCode::Space) {
        return;
    }
    cube.translation.y += 2.0 * time.delta_secs();
    if !thrust.moved {
        thrust.moved = true;
        info!("driven_app: thrust moved the cube");
    }
}

/// In-example behavior assertion: a driven run that stopped driving must FAIL
/// the process, not exit green on an untouched scene.
fn assert_the_cube_moved(thrust: Res<Thrust>) {
    assert!(
        thrust.moved,
        "reached Done without the autopilot's input ever moving the cube"
    );
}
