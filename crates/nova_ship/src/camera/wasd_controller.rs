//! WASD camera controller using bevy_enhanced_input for input handling.
//!
//! This plugin works together with the [`WASDCamera`] component from the
//! sibling [`wasd`](super::wasd) rig. It sets up input bindings for:
//! - WASD movement
//! - Mouse look for yaw and pitch
//! - Vertical movement using the space and shift keys
//! - Enable/disable mouse look using the right mouse button
//!
//! The plugin converts user input into updates to the [`WASDCameraInput`] component,
//! which can then be processed by the `WASDCameraPlugin` to update camera transform.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_input::prelude::{mouse_sensitivity, MousePath};

use super::wasd::{WASDCamera, WASDCameraInput};

/// Glob-import surface for the WASD camera input controller.
pub mod prelude {
    pub use super::{WASDCameraController, WASDCameraControllerPlugin};
}

/// Component that marks an entity as having a WASD camera controller.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
pub struct WASDCameraController;

/// Internal marker component used to define the input context.
#[derive(Component, Debug, Clone)]
struct WASDCameraInputMarker;

/// Component that tracks whether mouse look is currently enabled.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, Reflect)]
struct WASDCameraLookEnabled(bool);

/// Input action for movement along the horizontal plane.
#[derive(InputAction)]
#[action_output(Vec2)]
struct WASDCameraInputMove;

/// Input action for mouse look (yaw and pitch).
#[derive(InputAction)]
#[action_output(Vec2)]
struct WASDCameraInputLook;

/// Input action to enable or disable mouse look.
#[derive(InputAction)]
#[action_output(bool)]
struct WASDCameraInputEnableLook;

/// Input action for vertical movement (space/up and shift/down).
#[derive(InputAction)]
#[action_output(f32)]
struct WASDCameraInputVertical;

/// Plugin that sets up WASD camera controls.
///
/// Automatically initializes input bindings, updates the [`WASDCameraInput`]
/// component from player input, and manages enabling/disabling mouse look.
pub struct WASDCameraControllerPlugin;

impl Plugin for WASDCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        trace!("WASDCameraControllerPlugin: build");

        app.add_input_context::<WASDCameraInputMarker>();

        app.add_observer(setup_wasd_camera);
        app.add_observer(destroy_wasd_camera);

        app.add_observer(on_wasd_input);
        app.add_observer(on_wasd_input_completed);
        app.add_observer(on_mouse_input);
        app.add_observer(on_mouse_input_completed);
        app.add_observer(on_enable_look_input);
        app.add_observer(on_enable_look_input_completed);
        app.add_observer(on_vertical_input);
        app.add_observer(on_vertical_input_completed);
    }
}

/// Initializes a new WASD camera entity with default settings and input bindings.
fn setup_wasd_camera(insert: On<Insert, WASDCameraController>, mut commands: Commands) {
    let entity = insert.entity;
    trace!("setup_wasd_camera: entity {:?}", entity);

    commands.entity(entity).insert((
        Camera3d::default(),
        WASDCamera {
            wasd_sensitivity: 0.1,
            ..default()
        },
        WASDCameraLookEnabled(false),
        WASDCameraInputMarker,
        actions!(
            WASDCameraInputMarker[
                (
                    Name::new("Input: WASD Camera Move"),
                    Action::<WASDCameraInputMove>::new(),
                    Bindings::spawn((
                        Cardinal::wasd_keys().with(Scale::splat(1.0)),
                        Axial::left_stick().with(Scale::splat(1.0)),
                    )),
                ),
                (
                    Name::new("Input: WASD Camera Look"),
                    Action::<WASDCameraInputLook>::new(),
                    Bindings::spawn((
                        Spawn((
                            Binding::mouse_motion(),
                            mouse_sensitivity(MousePath::FreeCamera),
                            Negate::none(),
                        )),
                        Axial::right_stick().with((Scale::splat(1.0), Negate::none())),
                    )),
                ),
                (
                    Name::new("Input: WASD Camera Enable Look"),
                    Action::<WASDCameraInputEnableLook>::new(),
                    bindings![MouseButton::Right],
                ),
                (
                    Name::new("Input: WASD Camera Vertical"),
                    Action::<WASDCameraInputVertical>::new(),
                    Bindings::spawn((
                        Bidirectional::<Binding, Binding> {
                            positive: KeyCode::Space.into(),
                            negative: KeyCode::ShiftLeft.into(),
                        },
                    )),
                ),
            ]
        ),
    ));
}

/// Removes input components and bindings when the WASD camera controller is removed.
fn destroy_wasd_camera(remove: On<Remove, WASDCameraController>, mut commands: Commands) {
    let entity = remove.entity;
    trace!("destroy_wasd_camera: entity {:?}", entity);

    commands.entity(entity).try_remove::<(
        Actions<WASDCameraInputMarker>,
        WASDCamera,
        WASDCameraLookEnabled,
        WASDCameraInputMarker,
    )>();
}

/// Updates horizontal movement based on WASD input.
fn on_wasd_input(fire: On<Fire<WASDCameraInputMove>>, mut q_input: Query<&mut WASDCameraInput>) {
    for mut input in &mut q_input {
        input.wasd = fire.value;
    }
}

/// Resets horizontal movement when input is completed.
fn on_wasd_input_completed(
    _: On<Complete<WASDCameraInputMove>>,
    mut q_input: Query<&mut WASDCameraInput>,
) {
    for mut input in &mut q_input {
        input.wasd = Vec2::ZERO;
    }
}

/// Updates mouse look if enabled.
fn on_mouse_input(
    fire: On<Fire<WASDCameraInputLook>>,
    mut q_input: Query<(&mut WASDCameraInput, &WASDCameraLookEnabled)>,
) {
    for (mut input, enabled) in &mut q_input {
        if !**enabled {
            continue;
        }
        input.pan = fire.value;
    }
}

/// Resets mouse look when input is completed.
fn on_mouse_input_completed(
    _: On<Complete<WASDCameraInputLook>>,
    mut q_input: Query<&mut WASDCameraInput>,
) {
    for mut input in &mut q_input {
        input.pan = Vec2::ZERO;
    }
}

/// Enables mouse look when the enable input is fired.
fn on_enable_look_input(
    _: On<Fire<WASDCameraInputEnableLook>>,
    mut q_look_enabled: Query<&mut WASDCameraLookEnabled>,
) {
    for mut look_enabled in &mut q_look_enabled {
        **look_enabled = true;
    }
}

/// Disables mouse look and resets pan when enable input completes.
fn on_enable_look_input_completed(
    _: On<Complete<WASDCameraInputEnableLook>>,
    mut q_look_enabled: Query<(&mut WASDCameraInput, &mut WASDCameraLookEnabled)>,
) {
    for (mut input, mut look_enabled) in &mut q_look_enabled {
        input.pan = Vec2::ZERO;
        **look_enabled = false;
    }
}

/// Updates vertical movement based on space/shift input.
fn on_vertical_input(
    fire: On<Fire<WASDCameraInputVertical>>,
    mut q_input: Query<&mut WASDCameraInput>,
) {
    for mut input in &mut q_input {
        input.vertical = fire.value;
    }
}

/// Resets vertical movement when input is completed.
fn on_vertical_input_completed(
    _: On<Complete<WASDCameraInputVertical>>,
    mut q_input: Query<&mut WASDCameraInput>,
) {
    for mut input in &mut q_input {
        input.vertical = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::{mouse::MouseMotion, InputPlugin};
    use nova_input::prelude::{MouseSensitivity, MouseSensitivityPlugin};

    use super::*;
    use crate::camera::wasd::WASDCameraPlugin;

    /// A free camera with mouse look armed (right button held), which is the
    /// only state its pan reads in.
    fn free_camera_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(MouseSensitivityPlugin);
        app.add_plugins((WASDCameraPlugin, WASDCameraControllerPlugin));

        app.finish();
        app.cleanup();
        app.update();
        let camera = app.world_mut().spawn(WASDCameraController).id();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.update();
        (app, camera)
    }

    /// The free-camera sensitivity scales its mouse look and nothing else: not
    /// the keyboard movement beside it, and not the other two mouse paths.
    #[test]
    fn the_free_camera_sensitivity_scales_only_its_own_mouse_look() {
        let (mut app, camera) = free_camera_app();

        let sweep = |app: &mut App| {
            app.world_mut().write_message(MouseMotion {
                delta: Vec2::new(12.0, 0.0),
            });
            app.update();
            app.world().get::<WASDCameraInput>(camera).unwrap().pan.x
        };

        let at_default = sweep(&mut app);
        assert!(
            (at_default - 12.0 * MousePath::FreeCamera.default_raw()).abs() < 1e-6,
            "the rig starts on the free-camera default (got {at_default})"
        );

        app.world_mut()
            .resource_mut::<MouseSensitivity>()
            .set_percent(MousePath::FreeCamera, 300.0);
        let at_top = sweep(&mut app);
        assert!(
            (at_top - 12.0 * MousePath::FreeCamera.range().raw(300.0)).abs() < 1e-6,
            "the setting reaches a free camera that already existed (got {at_top})"
        );

        let mut sensitivity = app.world_mut().resource_mut::<MouseSensitivity>();
        sensitivity.set_percent(MousePath::Look, 300.0);
        sensitivity.set_percent(MousePath::Rcs, 500.0);
        assert!(
            (sweep(&mut app) - at_top).abs() < 1e-9,
            "the look and RCS sliders leave the free camera alone"
        );
    }

    /// Keyboard movement is not a mouse path. A W held while the sensitivity
    /// moves has to read the same either way - the setting names mouse look,
    /// and a `Scale` on the wrong binding would quietly make it a fly speed.
    #[test]
    fn the_free_camera_sensitivity_never_touches_keyboard_movement() {
        let (mut app, camera) = free_camera_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        let at_default = app.world().get::<WASDCameraInput>(camera).unwrap().wasd;
        assert_ne!(at_default, Vec2::ZERO, "W drives the camera");

        app.world_mut()
            .resource_mut::<MouseSensitivity>()
            .set_percent(MousePath::FreeCamera, 300.0);
        app.update();
        assert_eq!(
            app.world().get::<WASDCameraInput>(camera).unwrap().wasd,
            at_default,
            "keyboard movement keeps its own speed"
        );
    }
}
