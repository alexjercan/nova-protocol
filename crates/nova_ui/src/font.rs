//! The shared UI typeface handle.

use bevy::prelude::*;

/// The game's shared UI typeface (Iosevka Term), loaded once at startup and
/// read by every text-spawning site instead of a per-spawn `asset_server.load`.
///
/// `nova_assets` preloads the font through its `BootAssets` collection and
/// inserts this resource at `OnExit(GameAssetsStates::Boot)` (which runs before
/// `OnEnter(GameAssetsStates::Loading)`), so it exists before any UI (the boot
/// loading screen included) spawns text. Sites read the
/// handle through [`UiFont::handle`]; a headless rig that never ran the loader
/// simply has no `UiFont` resource and falls back to the engine default font.
#[derive(Resource, Clone, Debug, Default)]
pub struct UiFont(pub Handle<Font>);

impl UiFont {
    /// The typeface handle held by this resource.
    pub fn handle(&self) -> Handle<Font> {
        self.0.clone()
    }
}
