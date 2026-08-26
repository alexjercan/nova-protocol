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
//!
//! What IS driven here is the cover. Sixteen files re-read and a registry
//! rebuilt cost more than a frame, and paid in the open they read as a freeze
//! - so [`ContentReload`] runs the reload as a small machine: the panel goes up
//! on the press, the read waits until it has been rendered, and the resource
//! lives until the files have landed and the frame after the merge came back
//! smooth. `nova_core` draws the panel for as long as the resource exists.

/// Glob-import surface: `use nova_assets::reload::prelude::*`.
pub mod prelude {
    pub use super::{
        raise_reload_cover, reload_content, request_reload_on_key, settle_reload, ContentReload,
        ReloadContent, RELOAD_KEY,
    };
}

use bevy::{asset::AssetPath, prelude::*};
use nova_modding::prelude::{BundleAsset, ContentAsset};

use crate::mod_set::DownloadedMods;

/// The key that reads everything again, anywhere in the game.
pub const RELOAD_KEY: KeyCode = KeyCode::F5;

/// How long the cover stays up at minimum, seconds. Stops a reload that lands
/// inside two frames from flashing a panel up and down.
const COVER_MIN_DWELL: f32 = 0.6;
/// The frame delta the cover calls "settled", seconds: the merge frame is a
/// long one, so this is what holds the panel over it.
const COVER_SETTLED_DELTA: f32 = 0.05;
/// Hard cap on the cover, seconds. A file that never comes back must not take
/// the game with it.
const COVER_MAX_DWELL: f32 = 6.0;

/// Where a reload has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReloadPhase {
    /// The cover is up but has not been RENDERED yet. The read waits one frame:
    /// asked for in the frame the panel is spawned, it would freeze the game
    /// before the panel it is hiding behind ever appeared.
    Covering,
    /// The read has gone out; the files are on their way back.
    Reading,
    /// The files landed. The merge runs on one of the next frames, and the
    /// panel is here to cover exactly that.
    Settling,
}

/// A reload in flight, and the cover over it.
///
/// Present from the press until the content is back, which is the whole of what
/// the loading screen needs to know. One at a time: a second press while this
/// stands is the same reload being asked for twice.
#[derive(Resource, Debug)]
pub struct ContentReload {
    /// `Time<Real>` elapsed when the reload was asked for.
    started: f32,
    phase: ReloadPhase,
    /// Content files re-read and not yet back.
    pending: usize,
}

impl Default for ContentReload {
    /// A cover just raised, with nothing read yet. Production always goes
    /// through [`raise_reload_cover`]; this exists so the screen that DRAWS the
    /// cover can be tested without an asset server behind it.
    fn default() -> Self {
        Self {
            started: 0.0,
            phase: ReloadPhase::Covering,
            pending: 0,
        }
    }
}

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

/// Answer a [`ReloadContent`] by raising the cover, and nothing else.
///
/// The work waits for [`reload_content`] on the NEXT frame, once the panel this
/// puts up has been on screen for one. A reload already in flight swallows the
/// press: pressing F5 twice is asking for the same reload twice.
pub fn raise_reload_cover(
    mut asked: MessageReader<ReloadContent>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    reload: Option<Res<ContentReload>>,
) {
    if asked.read().count() == 0 || reload.is_some() {
        return;
    }
    commands.insert_resource(ContentReload {
        started: time.elapsed_secs(),
        phase: ReloadPhase::Covering,
        pending: 0,
    });
}

/// Do the reading, behind the cover: re-read the mod-cache index, and re-read
/// every bundle and content file the asset server is holding.
///
/// Both halves are needed and neither is enough. The index is the only place a
/// mod that did not exist at boot - the editor's own save, the first time it is
/// written - is named at all. The re-reads are the only thing that gets NEW
/// BYTES into a file the server already loaded, because `load` on a path it
/// holds hands back what it has.
pub fn reload_content(
    asset_server: Res<AssetServer>,
    bundles: Res<Assets<BundleAsset>>,
    contents: Res<Assets<ContentAsset>>,
    mut downloaded: ResMut<DownloadedMods>,
    reload: Option<ResMut<ContentReload>>,
) {
    let Some(mut reload) = reload else {
        return;
    };
    // `is_added` is the rendered frame: the cover was inserted in the frame
    // that ran before this one, so this is the first frame a player has seen
    // it.
    if reload.phase != ReloadPhase::Covering || reload.is_added() {
        return;
    }
    let bundle_paths = bundles
        .ids()
        .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned));
    let content_paths: Vec<AssetPath<'static>> = contents
        .ids()
        .filter_map(|id| asset_server.get_path(id).map(AssetPath::into_owned))
        .collect();
    // Only the content files are counted back in: they are the ones that carry
    // an `AssetEvent` this crate reads, and the ones the merge waits on.
    reload.pending = content_paths.len();
    reload.phase = ReloadPhase::Reading;
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
    info!("reload_content: re-reading {files} content file(s) from disk");
}

/// Take the cover down once the content is back: never while a re-read file is
/// still out, and then after the minimum dwell on the first frame back under
/// [`COVER_SETTLED_DELTA`], or at the hard cap.
///
/// The settle test is what covers the MERGE. Every file landing wakes
/// [`remerge_on_replaced_content`], the registries are rebuilt on one of the
/// next frames, and that frame is a long one - so the panel stays up until a
/// frame comes back short, which is the frame after the rebuild.
///
/// The cap beats a file that is still out, which is where this parts company
/// with the scenario screen's spawn gate: a slow scene is working, while a read
/// that has not landed by then is a loader that failed and will not land at
/// all.
///
/// Events are drained before the resource is looked at, so a file that landed
/// while no reload was in flight cannot be counted against the next one.
pub fn settle_reload(
    mut events: MessageReader<AssetEvent<ContentAsset>>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    reload: Option<ResMut<ContentReload>>,
) {
    let landed = events
        .read()
        .filter(|event| matches!(event, AssetEvent::Modified { .. }))
        .count();
    let Some(mut reload) = reload else {
        return;
    };
    if reload.phase == ReloadPhase::Reading {
        reload.pending = reload.pending.saturating_sub(landed);
        if reload.pending == 0 {
            reload.phase = ReloadPhase::Settling;
        }
    }
    let held = time.elapsed_secs() - reload.started;
    let settled = reload.phase == ReloadPhase::Settling
        && held >= COVER_MIN_DWELL
        && time.delta_secs() <= COVER_SETTLED_DELTA;
    if settled || held >= COVER_MAX_DWELL {
        commands.remove_resource::<ContentReload>();
    }
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
