//! The global wireframe pass.
//!
//! Adds Bevy's [`WireframePlugin`] with the global pass off. The pass is one
//! of the dev overlays: [`DebugOverlays::wireframe`] turns it on, through
//! [`apply_debug_overlays`].
//!
//! [`DebugOverlays::wireframe`]: crate::hud::overlays::DebugOverlays::wireframe
//! [`apply_debug_overlays`]: crate::hud::overlays::apply_debug_overlays

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};

/// Adds the wireframe pass, off.
pub struct WireframeDebugPlugin;

impl Plugin for WireframeDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WireframePlugin::default());
        app.insert_resource(WireframeConfig {
            global: false,
            ..default()
        });
    }
}

/// `WireframeDebugPlugin`.
pub mod prelude {
    pub use super::WireframeDebugPlugin;
}
