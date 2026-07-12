//! A Bevy plugin that adds various debugging tools.

use bevy::{camera::RenderTarget, prelude::*};
use bevy_common_systems::{debug::harness::AUTOPILOT_ENV, prelude::*};
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;
use nova_gameplay::GameStates;

pub mod gravity;
pub mod harness;
pub mod sections;

pub mod prelude {
    // Only the presets are the intended entry point. The raw `AutopilotPlugin` /
    // `ScreenshotPlugin` types stay reachable via `nova_debug::harness::` for the
    // rare bespoke-timeline case, so glob-importing this prelude does not clash
    // with Bevy's own `bevy::render::view::screenshot::ScreenshotPlugin`.
    pub use super::{
        debugdump,
        harness::{assert_scenario_loaded, nova_autopilot, nova_screenshot},
        DebugPlugin,
    };
}

/// The keycode to toggle debug mode.
pub const DEBUG_TOGGLE_KEYCODE: KeyCode = KeyCode::F11;

/// Resource with debug toggle state.
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct DebugEnabled(pub bool);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugSystems;

/// A plugin that adds various debugging tools.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugEnabled(true));

        app.add_plugins(InspectorDebugPlugin);
        app.add_plugins(WireframeDebugPlugin);
        app.add_plugins(sections::SectionsDebugPlugin);
        app.add_plugins(gravity::GravityDebugPlugin);

        app.add_systems(Update, (toggle_debug_mode, keep_inspector_on_window_camera));

        // Under the headless autopilot (`nova_debug::harness`), confirm the
        // asset loader actually reached gameplay before the clean exit, so a run
        // that silently stalls in `Loading` fails the smoke test instead of
        // passing on `autopilot: cycle complete, no panic` alone.
        if std::env::var(AUTOPILOT_ENV).is_ok() {
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("nova harness: reached Playing")
            });
        }

        app.configure_sets(
            Update,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
        app.configure_sets(
            PostUpdate,
            DebugSystems.run_if(resource_equals(DebugEnabled(true))),
        );
    }
}

fn toggle_debug_mode(mut debug: ResMut<DebugEnabled>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(DEBUG_TOGGLE_KEYCODE) {
        **debug = !**debug;
    }
}

/// Keep the inspector's `PrimaryEguiContext` on a window-targeting camera, and
/// off any camera that renders to an `Image`.
///
/// bcs's `InspectorDebugPlugin` assigns the primary egui context to the FIRST
/// camera added (a "first camera wins" observer). That breaks once a second
/// camera renders to a texture instead of the window - nova's target-inset RTT
/// camera (task 20260710-104421): if its `Add` fires before the scene camera's,
/// the inspector egui lands inside the inset's texture instead of the window.
/// This reconcile makes the placement order-independent: an `Image`-target
/// camera never owns the context, and a window camera always does. Runs
/// unconditionally (even with the inspector toggled off) so the context is
/// already in the right place when it is toggled back on.
fn keep_inspector_on_window_camera(
    mut commands: Commands,
    q_cameras: Query<(Entity, Option<&RenderTarget>, Has<PrimaryEguiContext>), With<Camera>>,
) {
    let renders_to_image =
        |target: Option<&RenderTarget>| matches!(target, Some(RenderTarget::Image(_)));

    let mut window_has_context = false;
    let mut first_window_camera = None;
    for (entity, target, has_context) in &q_cameras {
        if renders_to_image(target) {
            if has_context {
                // An offscreen (RTT) camera must never own the inspector UI.
                commands.entity(entity).remove::<PrimaryEguiContext>();
            }
        } else {
            first_window_camera.get_or_insert(entity);
            window_has_context |= has_context;
        }
    }

    // If the context is not (or no longer) on a window camera - e.g. it was just
    // pulled off the inset above - hand it to one. Removal and insertion flush
    // together, so there is no frame with zero primary contexts.
    if !window_has_context {
        if let Some(entity) = first_window_camera {
            commands.entity(entity).insert(PrimaryEguiContext);
        }
    }
}

pub fn debugdump(app: &mut App) {
    bevy_mod_debugdump::print_schedule_graph(app, Update);
    // bevy_mod_debugdump::print_schedule_graph(app, PostUpdate);
    // bevy_mod_debugdump::print_schedule_graph(app, FixedUpdate);
}
