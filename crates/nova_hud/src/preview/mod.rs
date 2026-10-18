//! The 3D previews the main menu and the interface share: the chart scene
//! ([`chart`]) the Map's chart view, the New Game seed preview and the Load
//! preview draw, and the hull view ([`hull`]) the role step and the station
//! cards show. Each host spawns its own cameras on its own render layers;
//! this module owns what the scenes look like.

use bevy::{app::HierarchyPropagatePlugin, camera::visibility::RenderLayers, prelude::*};

pub mod chart;
pub mod hull;

/// Glob-import surface: `use nova_hud::preview::prelude::*`.
pub mod prelude {
    pub use super::{chart::prelude::*, hull::prelude::*, PreviewPlugin, PreviewSystems};
}

/// System set for the previews' per-frame work: the hull turntables.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreviewSystems;

/// Draws the chart tile textures at startup, propagates render layers down
/// a preview's hierarchy (a preview section's meshes arrive as children)
/// and turns the hull views. Each host adds it; a second add is skipped, so
/// the menu and the interface in one app share one.
pub struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, chart::init_chart_tiles);
        if !app.is_plugin_added::<HierarchyPropagatePlugin<RenderLayers>>() {
            app.add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PostUpdate));
        }
        app.add_systems(Update, hull::turn_hull_previews.in_set(PreviewSystems));
    }
}
