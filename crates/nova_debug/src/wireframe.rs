//! A plugin that enables a global wireframe debug view and allows toggling it with a key press.
//!
//! This plugin adds the Bevy WireframePlugin and exposes a simple debug mode that can be turned
//! on or off at runtime. When enabled, all meshes in the scene are rendered in wireframe mode.
//!
//! Usage:
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_debug::wireframe::WireframeDebugPlugin;
//! # fn demo(app: &mut App) {
//! app.add_plugins(WireframeDebugPlugin);
//! # }
//! ```
//!
//! Nova owns this because the wireframe pass is one of the four layers
//! [`crate::DebugPlugin`] raises together on F11. The F11 key itself is read
//! ONCE, by `DebugPlugin`, which also overrides this `DebugEnabled` with
//! `DEBUG_LAYER_STARTS_ON` so the layer boots OFF.

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};

/// A resource that stores whether wireframe debug mode is currently enabled.
///
/// Other systems can read or write this value to control debug mode. The
/// WireframeDebugPlugin automatically updates the global wireframe setting
/// based on this resource.
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut, PartialEq, Eq, Hash)]
pub struct DebugEnabled(pub bool);

/// A plugin that enables global wireframe rendering and allows toggling it at runtime.
///
/// This plugin:
/// - Inserts the `DebugEnabled` resource (default: true)
/// - Registers Bevy's built in `WireframePlugin`
/// - Updates the global wireframe configuration each frame
pub struct WireframeDebugPlugin;

impl Plugin for WireframeDebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugEnabled(true));
        app.add_plugins(WireframePlugin::default());
        app.insert_resource(WireframeConfig {
            global: true,
            ..default()
        });
        app.add_systems(Update, enable_wireframe);
    }
}

/// Update the wireframe configuration whenever the debug state changes.
fn enable_wireframe(mut wireframe_config: ResMut<WireframeConfig>, debug: Res<DebugEnabled>) {
    if debug.is_changed() {
        wireframe_config.global = **debug;
    }
}

/// `WireframeDebugPlugin` and its `DebugEnabled` flag.
pub mod prelude {
    pub use super::{DebugEnabled, WireframeDebugPlugin};
}
