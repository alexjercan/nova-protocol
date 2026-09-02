//! A self-contained Bevy app driven end to end by [`AutopilotPlugin`].
//!
//! This is the crate's standalone proof and its reference script: a real
//! `DefaultPlugins` app with its own three-state machine, its own input
//! handling and its own scene, driven through NAMED PREDICATE STEPS and exited
//! by the completion protocol. It imports no `nova_*` crate but
//! `nova_autopilot` itself, so a run that builds is also a run that proves the
//! crate has no game coupling.
//!
//! Every beat here waits on something OBSERVED - the cube exists, the game read
//! the thrust, a real `bevy_ui` node reported the press - rather than on a
//! guessed duration, which is what keeps the run honest when a software
//! renderer collapses a wall-clock window into a handful of frames.
//!
//! Both halves of the synthesized input are exercised against the real thing: a
//! held key read through Bevy's own `ButtonInput`, and a pointer click routed
//! through the `bevy_ui` picking backend to the widget under it.
//!
//! Run it armed (it does nothing without the env var):
//!
//! ```sh
//! NOVA_AUTOPILOT=1 cargo run -p nova_autopilot --example driven_app
//! ```
//!
//! Set [`STALL_ENV`] as well to append a step whose predicate can never hold,
//! which demonstrates (and lets the test assert) the other end of the contract:
//! a stalled beat error-exits NAMING itself instead of quietly passing.
//!
//! `tests/autopilot_example.rs` runs both shapes headless and asserts on the
//! exit status and the log lines. The probe run-harness reuses this shape for
//! its correctness and profiling runs. (Named without the crate path on
//! purpose: the no-coupling proof greps this directory for `nova_*` mentions.)

use bevy::prelude::*;
// Both preludes export a `not`; an explicit import picks the predicate one.
use nova_autopilot::predicate::not;
use nova_autopilot::prelude::*;

/// Set to append the deliberately unsatisfiable step. Demo-only: it exists so
/// the crate's end-to-end test can assert the abort path in the SAME real app
/// as the success path, rather than in a second example that could drift.
const STALL_ENV: &str = "DRIVEN_APP_STALL";

/// The example's own state machine - deliberately NOT a Nova type: the driver
/// is generic over `S: States`, and this is what exercises that generic.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Boot,
    Flying,
    Done,
}

/// The one thing the autopilot's synthesized key press is supposed to cause.
/// The "thrust" step waits on it, so a run that stops driving stalls on a named
/// beat instead of passing over an untouched scene.
#[derive(Resource, Default)]
struct Thrust {
    moved: bool,
}

/// The driven object.
#[derive(Component)]
struct Cube;

/// Where the on-screen button's CENTER sits, in logical window pixels. The
/// pointer beat clicks exactly here, so the node below is laid out around it.
const BUTTON_CENTER: Vec2 = Vec2::new(160.0, 120.0);

/// Side of the square button, logical pixels.
const BUTTON_SIZE: f32 = 80.0;

/// The other thing the autopilot's synthesized input is supposed to cause: a
/// real `bevy_ui` node, hit by the real picking backend, reporting a press.
/// The "click the button" step waits on it, so a `click_at` that lands
/// somewhere the widget is not stalls on a named beat.
#[derive(Resource, Default)]
struct Button {
    pressed: bool,
}

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.init_state::<DemoState>();
    app.init_resource::<Thrust>();
    app.init_resource::<Button>();
    app.add_plugins(script());
    app.add_systems(Startup, spawn_scene);
    app.add_systems(Update, thrust);
    app.add_systems(OnEnter(DemoState::Done), assert_the_cube_moved);
    app.run()
}

/// The script, as a list of beats named after what each one WAITS FOR.
fn script() -> AutopilotPlugin<DemoState> {
    let plugin = AutopilotPlugin::new()
        // The scene is spawned by `Startup`, not by a timer: wait for the thing
        // itself rather than for however long spawning it usually takes.
        .step("spawn the cube")
        .until(any_entity::<With<Cube>>())
        .deadline(10.0)
        .add()
        // A held key is ONE press beat: Bevy's input collection clears the
        // `just_*` edges but keeps the press, so nothing has to re-press it
        // every frame. The step ends when the GAME reports it read the thrust.
        .step("thrust until the game moves the cube")
        .enter(DemoState::Flying)
        .on_enter(press_key(KeyCode::Space))
        .until(resource_where::<Thrust>(|thrust| thrust.moved))
        .deadline(10.0)
        .add()
        .step("coast")
        .on_enter(release_key(KeyCode::Space))
        .until(elapsed(0.5))
        .add()
        // The pointer half of the contract: place the cursor over a real
        // `bevy_ui` node and press. `click_at` writes the window's own
        // `cursor_position` AND a `CursorMoved` message, which is what the
        // picking backend needs to know where the pointer is - writing only one
        // leaves the widget untouched, so this beat would stall.
        .step("click the button")
        .on_enter(click_at(BUTTON_CENTER, MouseButton::Left))
        .until(resource_where::<Button>(|button| button.pressed))
        .deadline(10.0)
        .add()
        .step("release the button")
        .on_enter(release_mouse(MouseButton::Left))
        .add()
        .step("settle in Done")
        .enter(DemoState::Done)
        .until(frames(2))
        .add();

    if std::env::var(STALL_ENV).is_err() {
        return plugin;
    }
    plugin
        // `not(any_entity)` over a cube that is never despawned: false forever,
        // by construction, so the deadline is the only way out of this beat.
        .step("wait for the cube to vanish")
        .until(not(any_entity::<With<Cube>>()))
        .deadline(1.0)
        .add()
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
    // A plain absolutely-positioned node, laid out so its CENTRE lands on
    // `BUTTON_CENTER` - the point the script clicks.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BUTTON_CENTER.x - BUTTON_SIZE / 2.0),
                top: Val::Px(BUTTON_CENTER.y - BUTTON_SIZE / 2.0),
                width: Val::Px(BUTTON_SIZE),
                height: Val::Px(BUTTON_SIZE),
                ..default()
            },
            BackgroundColor(Color::srgb(0.8, 0.3, 0.3)),
        ))
        .observe(on_button_pressed);
}

/// The game side of the pointer loop: a real `Pointer<Press>` from the real
/// `bevy_ui` picking backend, over the node the script aimed at. Nothing here
/// knows about the autopilot.
fn on_button_pressed(_press: On<Pointer<Press>>, mut button: ResMut<Button>) {
    if !button.pressed {
        button.pressed = true;
        info!("driven_app: the pointer pressed the button");
    }
}

/// The game side of the loop: read real `ButtonInput`, move the cube. Nothing
/// here knows about the autopilot - the press arrives through Bevy's own input
/// resource, which is the point of the step actions running after
/// `InputSystems`.
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
fn assert_the_cube_moved(thrust: Res<Thrust>, button: Res<Button>) {
    assert!(
        thrust.moved,
        "reached Done without the autopilot's input ever moving the cube"
    );
    assert!(
        button.pressed,
        "reached Done without the autopilot's pointer ever reaching the widget"
    );
}
