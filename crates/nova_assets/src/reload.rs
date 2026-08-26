//! Reading the content back off disk, by restarting the game.
//!
//! Content reaches the registries ONCE: the asset server loads every bundle and
//! content file at boot, [`register_bundles`](crate::merge::register_bundles)
//! overlays them into `GameSections` / `GameScenarios` / the rest, and that is
//! the game's content for the run. A scenario saved from the editor lands in
//! the mod cache a moment later and nothing goes looking for it - which is why
//! seeing it in the Scenarios list used to mean quitting and starting again.
//!
//! The Wesnoth answer: one key that reads everything again. [`ReloadContent`]
//! asks; this module re-reads the mod-cache index, tells the asset server to
//! re-read every content file it holds, and puts the game back into
//! `GameAssetsStates::Loading` - the boot path, with the boot loading screen
//! over it, ending in the main menu.
//!
//! A RESTART rather than a live swap. The content the registries hand out is
//! read by everything already on screen - a flown ship's sections, a scenario's
//! script, the catalog behind the parts rail - and re-reading it under them is
//! the sort of thing that goes wrong quietly. Restarting is the one way to be
//! sure everything downstream sees one version of the content. So the gesture
//! is only offered where there is nothing to lose: F5 in the main menu, and the
//! way out of the mods panel, the editor and a scenario.
//!
//! A file that never comes back is the boot path's problem, and it already has
//! an answer: `GameAssetsStates::Failed` names it and stops.

/// Glob-import surface: `use nova_assets::reload::prelude::*`.
pub mod prelude {
    pub use super::{request_reload_on_key, restart_for_content, ReloadContent, RELOAD_KEY};
}

use bevy::{asset::AssetPath, prelude::*};
use nova_gameplay::prelude::GameStates;
use nova_modding::prelude::{BundleAsset, ContentAsset};

use crate::{mod_set::DownloadedMods, plugin::GameAssetsStates};

/// The key that reads everything again, from the main menu.
pub const RELOAD_KEY: KeyCode = KeyCode::F5;

/// Read every piece of content off disk again, and come back up on it.
///
/// Sent by [`request_reload_on_key`], and by any screen that is being LEFT
/// after what is on disk changed - the mods panel, the editor, a scenario.
#[derive(Message, Debug, Clone, Copy)]
pub struct ReloadContent;

/// [`RELOAD_KEY`] in the main menu.
///
/// Registered with the menu gate on it: a restart takes the game back to the
/// menu, so anywhere else the key would throw away whatever is on screen.
pub fn request_reload_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut asked: MessageWriter<ReloadContent>,
) {
    if keys.just_pressed(RELOAD_KEY) {
        asked.write(ReloadContent);
    }
}

/// Answer a [`ReloadContent`]: re-read the mod-cache index and every bundle and
/// content file the asset server holds, then re-enter the boot load.
///
/// Both halves of the read are needed and neither is enough. The index is the
/// only place a mod that did not exist at boot - the editor's own save, the
/// first time it is written - is named at all. The re-reads are the only thing
/// that gets NEW BYTES into a file the server already loaded, because `load` on
/// a path it holds hands back what it has.
///
/// The states are set in the same frame as the reads. `bevy_asset_loader` waits
/// on the collection it is handed, the boot loading screen goes up at
/// `OnEnter(Loading)`, and `OnEnter(Processing)` re-runs the whole merge - so
/// the game comes back on the content that is now on disk rather than on what
/// it booted with.
pub fn restart_for_content(
    mut asked: MessageReader<ReloadContent>,
    asset_server: Res<AssetServer>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
    mut downloaded: ResMut<DownloadedMods>,
    mut assets_state: ResMut<NextState<GameAssetsStates>>,
    mut game_state: ResMut<NextState<GameStates>>,
) {
    if asked.read().count() == 0 {
        return;
    }
    let bundle_paths = bundles
        .ids()
        .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned));
    let content_paths = contents
        .ids()
        .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned));
    let paths: Vec<AssetPath<'static>> = bundle_paths.chain(content_paths).collect();
    let files = paths.len();
    for path in paths {
        asset_server.reload(path);
    }
    // The index LAST, so the mods it names are looked for against a server
    // that has already been told its copies are stale.
    #[cfg(not(target_arch = "wasm32"))]
    crate::mod_set::refresh_downloaded_mods(&asset_server, &mut downloaded);
    // Web has no local cache to re-read; the merge still wants waking, because
    // the re-read files above land as replacements.
    #[cfg(target_arch = "wasm32")]
    downloaded.set_changed();
    // Both states, together. The asset state runs the load; the game state is
    // what the boot hook at `OnEnter(Loaded)` reads to decide it may hand the
    // player back to the menu.
    assets_state.set(GameAssetsStates::Loading);
    game_state.set(GameStates::Loading);
    info!("restart_for_content: re-reading {files} content file(s) from disk");
}

/// Re-merge when a content file comes back CHANGED.
///
/// The restart's late half. The boot load waits on the `GameAssets` collection,
/// which is the shipped content; a MOD's content file is loaded off the cache
/// index and is not in it, so one can land after the merge at
/// `OnEnter(Processing)` has already run. The merge is gated on the installed
/// set, which a re-saved mod does not change - the replaced file is the only
/// signal there is.
pub fn remerge_on_replaced_content(
    mut events: MessageReader<AssetEvent<ContentAsset>>,
    mut downloaded: ResMut<DownloadedMods>,
) {
    let mut replaced = false;
    for event in events.read() {
        replaced |= matches!(event, AssetEvent::Modified { .. });
    }
    if replaced {
        downloaded.set_changed();
    }
}

#[cfg(test)]
mod tests;
