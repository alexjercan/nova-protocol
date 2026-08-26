//! Reading the content back off disk, without restarting the game.
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
//! re-read every content file it holds, and lets the existing change-gated
//! merge do the rest.
//!
//! The reload is ASYNCHRONOUS - the merge cannot run in the same frame the
//! files are re-read - so nothing here re-merges by hand. It leans on the two
//! signals the pipeline already has: a changed installed set, and a content
//! asset that was replaced.

/// Glob-import surface: `use nova_assets::reload::prelude::*`.
pub mod prelude {
    pub use super::{reload_content, request_reload_on_key, ReloadContent, RELOAD_KEY};
}

use bevy::{asset::AssetPath, prelude::*};
use nova_modding::prelude::{BundleAsset, ContentAsset};

use crate::mod_set::DownloadedMods;

/// The key that reads everything again, anywhere in the game.
pub const RELOAD_KEY: KeyCode = KeyCode::F5;

/// Read every piece of content off disk again.
///
/// Sent by [`request_reload_on_key`], and by any screen that has just changed
/// what is ON disk - the editor's save, the mod portal's install.
#[derive(Message, Debug, Clone, Copy)]
pub struct ReloadContent;

/// [`RELOAD_KEY`], anywhere: the reload is a property of the content pipeline
/// rather than of whichever screen happens to be up, and a builder who just
/// saved is not in a mood to find the right one.
pub fn request_reload_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut asked: MessageWriter<ReloadContent>,
) {
    if keys.just_pressed(RELOAD_KEY) {
        asked.write(ReloadContent);
    }
}

/// Answer a [`ReloadContent`]: re-read the mod-cache index, and re-read every
/// bundle and content file the asset server is holding.
///
/// Both halves are needed and neither is enough. The index is the only place a
/// mod that did not exist at boot - the editor's own save, the first time it is
/// written - is named at all. The re-reads are the only thing that gets NEW
/// BYTES into a file the server already loaded, because `load` on a path it
/// holds hands back what it has.
pub fn reload_content(
    mut asked: MessageReader<ReloadContent>,
    asset_server: Res<AssetServer>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
    mut downloaded: ResMut<DownloadedMods>,
) {
    if asked.read().count() == 0 {
        return;
    }
    let paths: Vec<AssetPath<'static>> = bundles
        .ids()
        .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned))
        .chain(
            contents
                .ids()
                .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned)),
        )
        .collect();
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
    info!("reload_content: re-reading {files} content file(s) from disk");
}

/// Re-merge when a content file comes back CHANGED.
///
/// The reload's other half, and the one that runs frames later: the asset
/// server replaces each file when its read lands, and the merge is gated on the
/// installed set rather than on the files. Without this a re-saved mod - one
/// whose id the index already knew - would be re-read into a registry nobody
/// asked to rebuild.
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
