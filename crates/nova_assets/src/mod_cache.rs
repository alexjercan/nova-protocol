//! The LOCAL mod cache: downloaded mods' file bytes plus the installed index,
//! and the `mods://` asset source that serves them (task 20260715-142906).
//!
//! Two halves, split per platform like `mod_prefs` (small index sync, bulk
//! bytes on the platform's bulk store):
//!
//! - The INDEX is a RON `Vec<InstalledModRecord>` of DOWNLOADED mods only (the
//!   shipped `mods.catalog.ron` stays the other half of the installed set).
//!   Native: `<data_root>/installed.mods.ron`. Web: `window.localStorage` under
//!   `nova_protocol.installed_mods`. Best-effort like mod_prefs: missing or
//!   corrupt reads as `None`, write failures are logged.
//! - The FILE BYTES live at `<data_root>/mods/<id>/<path>` on native, and in
//!   IndexedDB (database `nova-protocol`, object store `mod-files`, key
//!   `<id>/<path>`) on the web.
//!
//! `<data_root>` is `dirs::data_dir()/nova-protocol` (data, not config - the
//! config dir stays prefs-only), OVERRIDABLE via the `NOVA_MOD_CACHE_ROOT`
//! environment variable. The override is read both at `mods://`
//! source-registration time and by every native path helper here, so the asset
//! source and the cache always agree; tests point both at a temp dir with it.
//!
//! The `mods://` ASSET SOURCE serves the cached files to the asset server:
//! native reads the cache directory live through a `FileAssetReader`; the web
//! reads a shared in-memory [`Dir`](bevy::asset::io::memory::Dir)
//! (`MemoryAssetReader`) that a startup task hydrates from IndexedDB (the
//! [`ModsSourceDir`] resource keeps the `Dir` handle reachable - it is
//! Arc-shared, so filling it after registration is visible to the reader). A
//! downloaded mod's bundle then loads as `mods://<id>/<bundle>` through the
//! exact same loaders as a shipped one.
//!
//! The COMMIT discipline for a real download (write files first, index last)
//! belongs to the installer flow (task 20260715-163508); this module only
//! provides the primitives plus the native [`install_local`] composition used
//! by tests and local tooling.
//!
//! TRUST BOUNDARY (review 142906 R1.1): index records and bundle manifests are
//! DOWNLOADED input, guarded in layers. Ids and file paths are validated by
//! the shared, cfg-independent helpers below in the PUBLIC cache API (both
//! platforms) and again where the index is consumed (`load_downloaded_mods`
//! skips unsafe records with a warning). Escaping ASSET paths (a malicious
//! manifest can request one without touching the index) are rejected by bevy's
//! own `UnapprovedPathMode::Forbid` default at load time, and the native
//! source is additionally SANDBOXED at the reader layer
//! ([`SandboxedAssetReader`]) so containment does not depend on that default.

use std::path::Path;

#[cfg(target_arch = "wasm32")]
pub use backend::read_all_files;
use serde::{Deserialize, Serialize};

/// The `mods://` asset source id, registered by [`register_mods_source`].
pub const MODS_SOURCE: &str = "mods";

/// One DOWNLOADED (portal-installed) mod in the local cache index.
///
/// The downloaded half of the installed set - shipped mods stay declared in
/// `mods.catalog.ron`. `bundle` is the mod's `*.bundle.ron` manifest path
/// RELATIVE to the mod's own cache directory (mirroring the portal's
/// `PortalEntry::bundle`), so the runtime asset path is `mods://<id>/<bundle>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledModRecord {
    /// Stable mod id - the cache directory name, the enable/disable key, and
    /// the merge-overlay namespace (same id space as the shipped catalog).
    pub id: String,
    /// The installed version string (from the portal entry), kept so the
    /// updater can compare against the portal catalog.
    pub version: String,
    /// The bundle manifest path relative to the mod's cache directory
    /// (e.g. `my_mod.bundle.ron`).
    pub bundle: String,
}

/// The saved downloaded-mods index, or `None` if nothing has been saved yet
/// (or the store is unreadable/corrupt). `None` degrades to "no downloaded
/// mods", never a panic.
pub fn read_index() -> Option<Vec<InstalledModRecord>> {
    backend::read_index()
}

/// `<data_root>/portal_catalog.json` - where the portal client's last-good
/// catalog store lives (`portal::last_good_store`). Beside the cache ON
/// PURPOSE: the catalog is cached wire data, not a preference, and the cache
/// root's `NOVA_MOD_CACHE_ROOT` test override is what keeps integration rigs
/// out of the developer's real store.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn portal_catalog_store_path() -> Option<std::path::PathBuf> {
    backend::portal_catalog_store_path()
}

/// Persist the downloaded-mods index. Best-effort - failures are logged, not
/// returned (the mod_prefs idiom; the installer flow of task 163508 owns any
/// stricter commit discipline via the `*_at` primitives). A plain persist: the
/// UNTRUSTED direction is reading the on-disk index back, which
/// `load_downloaded_mods` validates record by record.
pub fn write_index(records: &[InstalledModRecord]) {
    backend::write_index(records);
}

/// Insert or replace `record` in the downloaded-mods index (keyed by id).
/// Best-effort like [`write_index`]. Used by the portal installer's wasm
/// commit (native goes through [`install_local`], whose `io::Result` carries
/// the stricter files-first-index-last discipline end to end).
pub fn upsert_index_record(record: InstalledModRecord) {
    let mut records = read_index().unwrap_or_default();
    match records.iter_mut().find(|r| r.id == record.id) {
        Some(existing) => *existing = record,
        None => records.push(record),
    }
    write_index(&records);
}

/// Drop `id`'s record from the downloaded-mods index (absent is fine).
/// Best-effort like [`write_index`]. The UNINSTALL flow removes the index
/// entry FIRST, files second - the reverse of the install order, so the index
/// never references files that are already gone.
pub fn remove_index_record(id: &str) {
    let mut records = read_index().unwrap_or_default();
    records.retain(|r| r.id != id);
    write_index(&records);
}

/// True when every component of `path` is a plain name - no `..`, no root, no
/// drive prefix, no `.` - so joining it under a directory can never escape it.
/// Shared by the path validators and the native source sandbox.
fn has_only_normal_components(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// True when `s` stays INSIDE the directory it is joined onto. Index/portal
/// data is downloaded input, so the cache boundary re-checks it instead of
/// trusting the portal generator's own validation. Cfg-INDEPENDENT: both the
/// native fs backend and the wasm IndexedDB backend are gated on this.
pub(crate) fn is_safe_rel_path(s: &str) -> bool {
    !s.is_empty() && has_only_normal_components(Path::new(s))
}

/// True when `id` is usable as a single cache directory name. Rejecting `/`
/// (and `\`) also keeps the wasm `<id>/<path>` IndexedDB keys UNAMBIGUOUS:
/// id "a" + path "b/c" cannot collide with id "a/b" + path "c" because the
/// latter id never passes (review 142906 R1.3).
pub(crate) fn is_safe_id(id: &str) -> bool {
    is_safe_rel_path(id) && !id.contains('/') && !id.contains('\\')
}

/// The shared public-API gate, applied BEFORE the cfg dispatch below so both
/// platform backends get identical validation (review 142906 R1.1/R1.3).
fn validate_file_op<'a>(id: &str, paths: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    if !is_safe_id(id) {
        return Err(format!("mod cache: unsafe mod id '{id}'"));
    }
    for path in paths {
        if !is_safe_rel_path(path) {
            return Err(format!("mod cache: unsafe mod file path '{path}'"));
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn invalid_input(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

/// Read one cached mod file's bytes, or `None` if absent/unreadable (or the
/// id/path is unsafe).
#[cfg(not(target_arch = "wasm32"))]
pub fn read_mod_file(id: &str, path: &str) -> Option<Vec<u8>> {
    validate_file_op(id, [path]).ok()?;
    backend::read_mod_file(id, path)
}

/// Store a downloaded mod's files under `<data_root>/mods/<id>/`.
#[cfg(not(target_arch = "wasm32"))]
pub fn store_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    validate_file_op(id, files.iter().map(|(p, _)| p.as_str())).map_err(invalid_input)?;
    backend::store_mod_files(id, files)
}

/// Remove a mod's cached files (missing files are fine), pruning the emptied
/// directories.
#[cfg(not(target_arch = "wasm32"))]
pub fn remove_mod_files(id: &str, paths: &[String]) -> std::io::Result<()> {
    validate_file_op(id, paths.iter().map(String::as_str)).map_err(invalid_input)?;
    backend::remove_mod_files(id, paths)
}

/// Install a mod into the local cache from in-memory files: store the bytes,
/// then upsert the index record (files first, index last - the same order a
/// failed install must leave a readable state in). Used by tests and local
/// tooling; the network installer (task 163508) composes the same primitives.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_local(
    id: &str,
    version: &str,
    bundle: &str,
    files: &[(String, Vec<u8>)],
) -> std::io::Result<()> {
    validate_file_op(id, files.iter().map(|(p, _)| p.as_str())).map_err(invalid_input)?;
    validate_file_op(id, [bundle]).map_err(invalid_input)?;
    backend::install_local(id, version, bundle, files)
}

/// Remove EVERY cached file of `id` (the whole `<data_root>/mods/<id>` tree,
/// whatever it holds - a version's exact file list, plus any orphans an older
/// install left behind). The uninstall flow's file half; a missing dir is fine.
#[cfg(not(target_arch = "wasm32"))]
pub fn remove_mod(id: &str) -> std::io::Result<()> {
    validate_file_op(id, std::iter::empty::<&str>()).map_err(invalid_input)?;
    backend::remove_mod(id)
}

/// Read one cached mod file's bytes, or `None` if absent/unreadable (or the
/// id/path is unsafe).
#[cfg(target_arch = "wasm32")]
pub async fn read_mod_file(id: &str, path: &str) -> Option<Vec<u8>> {
    validate_file_op(id, [path]).ok()?;
    backend::read_mod_file(id, path).await
}

/// Store a downloaded mod's files in the IndexedDB store.
#[cfg(target_arch = "wasm32")]
pub async fn store_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    validate_file_op(id, files.iter().map(|(p, _)| p.as_str()))?;
    backend::store_mod_files(id, files).await
}

/// Remove a mod's cached files (missing keys are fine - IDB delete is
/// idempotent).
#[cfg(target_arch = "wasm32")]
pub async fn remove_mod_files(id: &str, paths: &[String]) -> Result<(), String> {
    validate_file_op(id, paths.iter().map(String::as_str))?;
    backend::remove_mod_files(id, paths).await
}

/// Store a downloaded mod's files in ONE IndexedDB transaction, resolved on
/// the TRANSACTION's `complete` event - NOT per-request success, which is not
/// commit (review 142906 R1.4): a put whose request succeeded can still abort
/// at commit time (e.g. quota exceeded). All-or-nothing, which is exactly the
/// installer's files-first-index-last need: `Ok` here means every byte is
/// durably committed, so writing the index record afterwards can never
/// publish a mod whose files silently rolled back.
#[cfg(target_arch = "wasm32")]
pub async fn commit_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    validate_file_op(id, files.iter().map(|(p, _)| p.as_str()))?;
    backend::commit_mod_files(id, files).await
}

/// Remove EVERY cached file of `id` (all IndexedDB keys under `<id>/`,
/// whatever they are - a version's exact file list, plus any orphans an older
/// install left behind). Returns the removed keys so the caller can also
/// evict them from the in-memory `mods://` `Dir`. The uninstall flow's file
/// half; no keys is fine.
#[cfg(target_arch = "wasm32")]
pub async fn remove_mod(id: &str) -> Result<Vec<String>, String> {
    validate_file_op(id, std::iter::empty::<&str>())?;
    backend::remove_mod(id).await
}

/// Register the `mods://` asset source on `app`. MUST run before `AssetPlugin`
/// (typically before `DefaultPlugins`): bevy builds the registered sources when
/// `AssetPlugin` is added, not lazily (`AssetApp::register_asset_source`,
/// bevy_asset 0.19 src/lib.rs:563).
///
/// Native: a SANDBOXED `FileAssetReader` rooted at `<data_root>/mods`, reading
/// the cache live (no hydration step). If no data root resolves, an EMPTY
/// in-memory root is registered instead so `mods://` paths fail as not-found
/// rather than as an unknown source.
///
/// Web: a `MemoryAssetReader` over a fresh shared `Dir`, ALSO inserted as
/// [`ModsSourceDir`] so the startup hydrator can fill it from IndexedDB after
/// the source is built (the `Dir` is Arc-shared; late inserts are visible).
/// The memory source needs NO sandbox: the `Dir` resolves each path component
/// by hash-map lookup (a `..` name is just an absent key), never by filesystem
/// traversal, so an escaping request cannot leave it by construction.
pub fn register_mods_source(app: &mut bevy::app::App) {
    use bevy::asset::{io::AssetSourceBuilder, AssetApp};

    #[cfg(not(target_arch = "wasm32"))]
    {
        use bevy::asset::io::file::FileAssetReader;

        match backend::mods_root() {
            Some(root) => {
                app.register_asset_source(
                    MODS_SOURCE,
                    AssetSourceBuilder::new(move || {
                        Box::new(SandboxedAssetReader {
                            inner: FileAssetReader::new(root.clone()),
                        })
                    }),
                );
            }
            None => {
                bevy::log::warn!("mod cache: no data dir available; the mods:// source is empty");
                let dir = bevy::asset::io::memory::Dir::default();
                app.register_asset_source(
                    MODS_SOURCE,
                    AssetSourceBuilder::new(move || {
                        Box::new(bevy::asset::io::memory::MemoryAssetReader { root: dir.clone() })
                    }),
                );
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        use bevy::asset::io::memory::{Dir, MemoryAssetReader};

        let dir = Dir::default();
        let reader_dir = dir.clone();
        app.register_asset_source(
            MODS_SOURCE,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: reader_dir.clone(),
                })
            }),
        );
        app.insert_resource(ModsSourceDir(dir));
    }
}

/// Confines the native `mods://` source to its root (review 142906 R1.1): any
/// requested path with a non-Normal component (`..`, a root, a drive prefix,
/// `.`) is rejected as not-found before the wrapped reader sees it.
///
/// Threat and layering (verified in bevy_asset 0.19 source): `AssetPath::
/// resolve` PRESERVES an underflowing `..` (normalize_path, path.rs:692, per
/// RFC 1808), so a bundle manifest listing `../../x.content.ron` yields a
/// source-relative path that still starts with `..` - record validation alone
/// cannot stop a malicious MANIFEST. The FIRST guard against such a path is
/// bevy itself: `AssetPlugin::unapproved_path_mode` defaults to `Forbid` and
/// the server rejects `is_unapproved()` paths at load time (server/mod.rs:544);
/// the raw `FileAssetReader`, which would happily raw-join `..` onto its
/// root, never sees them. This sandbox is the READER-layer backstop so the
/// cache's containment does not hinge on that config staying at its default
/// (an `unapproved_path_mode: Allow` app, a `load_override` caller, or any
/// future direct-reader consumer would otherwise reopen the hole). Escaping
/// requests always retain a non-Normal component, so the check is exact.
#[cfg(not(target_arch = "wasm32"))]
struct SandboxedAssetReader<R: bevy::asset::io::AssetReader> {
    inner: R,
}

#[cfg(not(target_arch = "wasm32"))]
impl<R: bevy::asset::io::AssetReader> SandboxedAssetReader<R> {
    fn check(&self, path: &Path) -> Result<(), bevy::asset::io::AssetReaderError> {
        if has_only_normal_components(path) {
            Ok(())
        } else {
            bevy::log::warn!(
                "mod cache: rejected a mods:// request that could escape the cache root: '{}'",
                path.display()
            );
            Err(bevy::asset::io::AssetReaderError::NotFound(
                path.to_path_buf(),
            ))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<R: bevy::asset::io::AssetReader> bevy::asset::io::AssetReader for SandboxedAssetReader<R> {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<impl bevy::asset::io::Reader + 'a, bevy::asset::io::AssetReaderError> {
        self.check(path)?;
        self.inner.read(path).await
    }

    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<impl bevy::asset::io::Reader + 'a, bevy::asset::io::AssetReaderError> {
        self.check(path)?;
        self.inner.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<bevy::asset::io::PathStream>, bevy::asset::io::AssetReaderError> {
        self.check(path)?;
        self.inner.read_directory(path).await
    }

    async fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<bool, bevy::asset::io::AssetReaderError> {
        self.check(path)?;
        self.inner.is_directory(path).await
    }
}

/// The shared in-memory root behind the web `mods://` source. The startup
/// hydrator (`nova_assets`' wasm startup systems) reads every cached file out
/// of IndexedDB and `insert_asset`s it here before any `mods://` load is
/// kicked - a memory-source read of a not-yet-inserted path fails the load
/// permanently (the asset server does not retry).
#[cfg(target_arch = "wasm32")]
#[derive(bevy::prelude::Resource, Clone)]
pub struct ModsSourceDir(pub bevy::asset::io::memory::Dir);

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::{Path, PathBuf};

    use bevy::log::warn;

    use super::{is_safe_id, is_safe_rel_path, InstalledModRecord};

    /// `<data_root>`: `$NOVA_MOD_CACHE_ROOT` if set (the test/tooling override,
    /// see the module doc), else `dirs::data_dir()/nova-protocol`.
    ///
    /// The override is ABSOLUTIZED here, the single place it is read: a
    /// relative value would otherwise diverge between the `mods://` source
    /// (`FileAssetReader` joins relative roots onto bevy's base path, the
    /// executable dir) and these fs helpers (which join onto the CWD).
    fn data_root() -> Option<PathBuf> {
        match std::env::var_os("NOVA_MOD_CACHE_ROOT") {
            Some(root) => std::path::absolute(PathBuf::from(root)).ok(),
            None => dirs::data_dir().map(|d| d.join("nova-protocol")),
        }
    }

    /// The `mods://` source root: `<data_root>/mods`. `None` when no data root
    /// resolves (headless CI without HOME, e.g.).
    pub fn mods_root() -> Option<PathBuf> {
        data_root().map(|d| d.join("mods"))
    }

    /// The last-good portal catalog's store file (see the crate-level wrapper).
    pub fn portal_catalog_store_path() -> Option<PathBuf> {
        data_root().map(|d| d.join("portal_catalog.json"))
    }

    pub fn read_index() -> Option<Vec<InstalledModRecord>> {
        read_index_at(&data_root()?)
    }

    pub fn write_index(records: &[InstalledModRecord]) {
        let Some(root) = data_root() else {
            warn!("mod cache: no data dir available; the installed-mods index will not persist");
            return;
        };
        if let Err(e) = write_index_at(&root, records) {
            warn!("mod cache: could not write the installed-mods index: {e}");
        }
    }

    /// Read one cached mod file's bytes, or `None` if absent/unreadable.
    pub fn read_mod_file(id: &str, path: &str) -> Option<Vec<u8>> {
        read_mod_file_at(&data_root()?, id, path)
    }

    /// Store a downloaded mod's files under `<data_root>/mods/<id>/`.
    pub fn store_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> std::io::Result<()> {
        store_mod_files_at(&data_root().ok_or(no_data_dir())?, id, files)
    }

    /// Remove a mod's cached files (missing files are fine), pruning the
    /// emptied directories.
    pub fn remove_mod_files(id: &str, paths: &[String]) -> std::io::Result<()> {
        remove_mod_files_at(&data_root().ok_or(no_data_dir())?, id, paths)
    }

    /// Install a mod into the local cache from in-memory files: store the
    /// bytes, then upsert the index record (files first, index last - the same
    /// order a failed install must leave a readable state in). Used by tests
    /// and local tooling; the network installer (task 163508) composes the same
    /// primitives.
    pub fn install_local(
        id: &str,
        version: &str,
        bundle: &str,
        files: &[(String, Vec<u8>)],
    ) -> std::io::Result<()> {
        install_local_at(
            &data_root().ok_or(no_data_dir())?,
            id,
            version,
            bundle,
            files,
        )
    }

    /// Remove the whole `<data_root>/mods/<id>` tree (uninstall's file half).
    pub fn remove_mod(id: &str) -> std::io::Result<()> {
        remove_mod_at(&data_root().ok_or(no_data_dir())?, id)
    }

    fn no_data_dir() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir available")
    }

    fn bad_input(what: &str, value: &str) -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("mod cache: unsafe {what} '{value}'"),
        )
    }

    // The `*_at` functions below are the pure (root in) halves the unit tests
    // drive under a temp root. They RE-CHECK ids/paths at the fs boundary even
    // though the module-level public API already validated them: these are
    // callable directly (tests, the 163508 installer), so the fs layer must
    // not rely on the caller having gone through the wrappers.

    pub fn read_index_at(root: &Path) -> Option<Vec<InstalledModRecord>> {
        let bytes = std::fs::read(root.join("installed.mods.ron")).ok()?;
        ron::de::from_bytes::<Vec<InstalledModRecord>>(&bytes).ok()
    }

    pub fn write_index_at(root: &Path, records: &[InstalledModRecord]) -> std::io::Result<()> {
        std::fs::create_dir_all(root)?;
        let ron = ron::ser::to_string(records)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(root.join("installed.mods.ron"), ron)
    }

    pub fn read_mod_file_at(root: &Path, id: &str, path: &str) -> Option<Vec<u8>> {
        if !is_safe_id(id) || !is_safe_rel_path(path) {
            return None;
        }
        std::fs::read(root.join("mods").join(id).join(path)).ok()
    }

    pub fn store_mod_files_at(
        root: &Path,
        id: &str,
        files: &[(String, Vec<u8>)],
    ) -> std::io::Result<()> {
        if !is_safe_id(id) {
            return Err(bad_input("mod id", id));
        }
        let mod_dir = root.join("mods").join(id);
        for (path, bytes) in files {
            if !is_safe_rel_path(path) {
                return Err(bad_input("mod file path", path));
            }
            let file = mod_dir.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file, bytes)?;
        }
        Ok(())
    }

    pub fn remove_mod_files_at(root: &Path, id: &str, paths: &[String]) -> std::io::Result<()> {
        if !is_safe_id(id) {
            return Err(bad_input("mod id", id));
        }
        let mod_dir = root.join("mods").join(id);
        for path in paths {
            if !is_safe_rel_path(path) {
                return Err(bad_input("mod file path", path));
            }
            match std::fs::remove_file(mod_dir.join(path)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            // Prune now-empty subdirectories up to (not including) the mod dir.
            // `remove_dir` refuses non-empty dirs, so this is safely best-effort.
            let mut dir = mod_dir.join(path);
            while let Some(parent) = dir.parent().map(Path::to_path_buf) {
                if parent == mod_dir || std::fs::remove_dir(&parent).is_err() {
                    break;
                }
                dir = parent;
            }
        }
        // Drop the mod dir itself once emptied (best-effort, same rationale).
        let _ = std::fs::remove_dir(&mod_dir);
        Ok(())
    }

    pub fn install_local_at(
        root: &Path,
        id: &str,
        version: &str,
        bundle: &str,
        files: &[(String, Vec<u8>)],
    ) -> std::io::Result<()> {
        if !is_safe_rel_path(bundle) {
            return Err(bad_input("bundle path", bundle));
        }
        store_mod_files_at(root, id, files)?;
        let mut records = read_index_at(root).unwrap_or_default();
        let record = InstalledModRecord {
            id: id.to_string(),
            version: version.to_string(),
            bundle: bundle.to_string(),
        };
        match records.iter_mut().find(|r| r.id == id) {
            Some(existing) => *existing = record,
            None => records.push(record),
        }
        write_index_at(root, &records)
    }

    /// Remove the mod's whole cache directory - everything under
    /// `<root>/mods/<id>`, exact file list not required (also sweeps orphans
    /// an older install left behind). Missing dir is fine.
    pub fn remove_mod_at(root: &Path, id: &str) -> std::io::Result<()> {
        if !is_safe_id(id) {
            return Err(bad_input("mod id", id));
        }
        match std::fs::remove_dir_all(root.join("mods").join(id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    //! The web half: index in localStorage (small + sync, the mod_prefs split),
    //! file bytes in IndexedDB via the thin hand-rolled wrapper below.
    //! Hand-rolled over a crate (rexie et al) on purpose: the needed surface is
    //! four operations on one object store, and the repo already owns its two
    //! short storage impls (mod_prefs) - a third-party wasm-bindgen pin is a
    //! bigger liability than ~100 lines. Revisit if the surface grows.
    //!
    //! COMPILE-GATED, NOT RUNTIME-TESTED here: neither the local test runner
    //! nor PR/master CI compiles the wasm target (only the manual web deploy
    //! does), so this half is statically reviewed against web-sys 0.3; its
    //! first runtime exercise is task 163508's web testing.

    use bevy::log::warn;
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};

    use super::InstalledModRecord;

    /// The localStorage key for the downloaded-mods index.
    const INDEX_KEY: &str = "nova_protocol.installed_mods";
    /// The IndexedDB database / object store holding the cached file bytes,
    /// keyed `<id>/<path>`.
    const DB_NAME: &str = "nova-protocol";
    const STORE_NAME: &str = "mod-files";
    const DB_VERSION: u32 = 1;

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn read_index() -> Option<Vec<InstalledModRecord>> {
        let raw = storage()?.get_item(INDEX_KEY).ok()??;
        ron::de::from_str::<Vec<InstalledModRecord>>(&raw).ok()
    }

    pub fn write_index(records: &[InstalledModRecord]) {
        let Some(storage) = storage() else {
            warn!("mod cache: no localStorage; the installed-mods index will not persist");
            return;
        };
        match ron::ser::to_string(records) {
            Ok(s) => {
                if storage.set_item(INDEX_KEY, &s).is_err() {
                    warn!("mod cache: localStorage write failed; installed-mods index not saved");
                }
            }
            Err(e) => warn!("mod cache: could not encode the installed-mods index: {e}"),
        }
    }

    fn file_key(id: &str, path: &str) -> String {
        format!("{id}/{path}")
    }

    /// Read one cached mod file's bytes, or `None` if absent/unreadable.
    pub async fn read_mod_file(id: &str, path: &str) -> Option<Vec<u8>> {
        idb_get(&file_key(id, path)).await
    }

    /// Store a downloaded mod's files in the IndexedDB store.
    pub async fn store_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> Result<(), String> {
        for (path, bytes) in files {
            idb_put(&file_key(id, path), bytes).await?;
        }
        Ok(())
    }

    /// Remove a mod's cached files (missing keys are fine - IDB delete is
    /// idempotent).
    pub async fn remove_mod_files(id: &str, paths: &[String]) -> Result<(), String> {
        for path in paths {
            idb_delete(&file_key(id, path)).await?;
        }
        Ok(())
    }

    /// Store all of a mod's files in ONE readwrite transaction and resolve on
    /// the TRANSACTION settling - the R1.4-correct commit signal (request
    /// `success` fires before the transaction is durable, and a later abort
    /// rolls every put back). All-or-nothing by IDB's own transaction
    /// semantics: the installer writes the index record only after `Ok`.
    pub async fn commit_mod_files(id: &str, files: &[(String, Vec<u8>)]) -> Result<(), String> {
        let (db, store) = open_store(web_sys::IdbTransactionMode::Readwrite).await?;
        let tx = store.transaction();
        for (path, bytes) in files {
            let value: JsValue = js_sys::Uint8Array::from(bytes.as_slice()).into();
            if store
                .put_with_key(&value, &JsValue::from_str(&file_key(id, path)))
                .is_err()
            {
                // A synchronously-rejected put (bad key/value) never reaches
                // the store: abort so the earlier puts roll back too.
                let _ = tx.abort();
                db.close();
                return Err("IndexedDB put failed".to_string());
            }
        }
        let result = await_transaction(&tx).await;
        db.close();
        result
    }

    /// Remove every key under `<id>/` (uninstall's file half), returning the
    /// removed keys so the caller can also evict them from the in-memory
    /// `mods://` `Dir`.
    pub async fn remove_mod(id: &str) -> Result<Vec<String>, String> {
        let prefix = format!("{id}/");
        let keys: Vec<String> = idb_all_keys()
            .await?
            .into_iter()
            .filter(|k| k.starts_with(&prefix))
            .collect();
        for key in &keys {
            idb_delete(key).await?;
        }
        Ok(keys)
    }

    /// Await an `IdbTransaction` settling: resolve on `complete`, reject on
    /// `error`/`abort` - the only events that mean commit or rollback. Like
    /// [`await_request`], the one-shot closures hand their memory to JS
    /// (`once_into_js`): whichever of the three never fires leaks a few
    /// bytes, once per commit - see the leak rationale there.
    async fn await_transaction(tx: &web_sys::IdbTransaction) -> Result<(), String> {
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let on_complete = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = resolve.call0(&JsValue::UNDEFINED);
            });
            let reject_on_abort = reject.clone();
            let on_error = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("IndexedDB transaction failed"),
                );
            });
            let on_abort = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject_on_abort.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("IndexedDB transaction aborted"),
                );
            });
            tx.set_oncomplete(Some(on_complete.unchecked_ref()));
            tx.set_onerror(Some(on_error.unchecked_ref()));
            tx.set_onabort(Some(on_abort.unchecked_ref()));
        });
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map(|_| ())
            .map_err(|_| "IndexedDB transaction did not commit".to_string())
    }

    /// Every cached file as `(key, bytes)` with keys `<id>/<path>` - the
    /// startup hydrator dumps the whole store into the `mods://` memory `Dir`
    /// (whatever the index says: an orphaned file is harmless there, an absent
    /// one would fail its load). Unreadable store degrades to empty.
    pub async fn read_all_files() -> Vec<(String, Vec<u8>)> {
        let Ok(keys) = idb_all_keys().await else {
            return Vec::new();
        };
        let mut files = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(bytes) = idb_get(&key).await {
                files.push((key, bytes));
            }
        }
        files
    }

    /// Await an `IdbRequest` by bridging its success/error EVENTS into a
    /// `Promise` (IDB requests are event-based, not thenable). The one-shot
    /// closures hand their memory to JS (`once_into_js`), freed when invoked;
    /// the loser of the success/error pair leaks a few bytes - acceptable for
    /// this low-frequency surface.
    async fn await_request(request: web_sys::IdbRequest) -> Result<JsValue, String> {
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let ok_request = request.clone();
            let on_ok = Closure::once_into_js(move |_: web_sys::Event| {
                let value = ok_request.result().unwrap_or(JsValue::UNDEFINED);
                let _ = resolve.call1(&JsValue::UNDEFINED, &value);
            });
            let on_err = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("IndexedDB request failed"),
                );
            });
            request.set_onsuccess(Some(on_ok.unchecked_ref()));
            request.set_onerror(Some(on_err.unchecked_ref()));
        });
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|_| "IndexedDB request failed".to_string())
    }

    /// Open (creating/upgrading on first use) the cache database. Opened per
    /// operation and CLOSED by the caller after it - connections otherwise
    /// linger until GC (review 142906 R1.4). Simple over fast: the browser
    /// caches the connection handshake and this surface runs once at startup
    /// plus per install.
    ///
    /// The open request wires all four callbacks into one promise: success,
    /// error, `blocked` (an old connection in another tab holds a lower
    /// version open across a future `DB_VERSION` bump - it must surface as an
    /// error, not wedge the open forever), and `upgradeneeded` (creates the
    /// store on first use).
    async fn open_db() -> Result<web_sys::IdbDatabase, String> {
        let factory = web_sys::window()
            .ok_or("no window")?
            .indexed_db()
            .map_err(|_| "IndexedDB unavailable")?
            .ok_or("IndexedDB unavailable")?;
        let request = factory
            .open_with_u32(DB_NAME, DB_VERSION)
            .map_err(|_| "IndexedDB open failed")?;
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let ok_request = request.clone();
            let on_ok = Closure::once_into_js(move |_: web_sys::Event| {
                let value = ok_request.result().unwrap_or(JsValue::UNDEFINED);
                let _ = resolve.call1(&JsValue::UNDEFINED, &value);
            });
            let reject_on_error = reject.clone();
            let on_err = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject_on_error.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("IndexedDB open failed"),
                );
            });
            let on_blocked = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("IndexedDB open blocked by another connection"),
                );
            });
            let upgrade_request = request.clone();
            let on_upgrade = Closure::once_into_js(move |_: web_sys::Event| {
                if let Ok(result) = upgrade_request.result() {
                    let db: web_sys::IdbDatabase = result.unchecked_into();
                    if !db.object_store_names().contains(STORE_NAME) {
                        let _ = db.create_object_store(STORE_NAME);
                    }
                }
            });
            request.set_onsuccess(Some(on_ok.unchecked_ref()));
            request.set_onerror(Some(on_err.unchecked_ref()));
            request.set_onblocked(Some(on_blocked.unchecked_ref()));
            request.set_onupgradeneeded(Some(on_upgrade.unchecked_ref()));
        });
        let db = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|_| "IndexedDB open failed")?;
        Ok(db.unchecked_into())
    }

    /// One connection + one single-store transaction per operation. The caller
    /// MUST `db.close()` when done (close is safe mid-transaction: it only
    /// takes effect once the transaction settles).
    async fn open_store(
        mode: web_sys::IdbTransactionMode,
    ) -> Result<(web_sys::IdbDatabase, web_sys::IdbObjectStore), String> {
        let db = open_db().await?;
        let tx = db
            .transaction_with_str_and_mode(STORE_NAME, mode)
            .map_err(|_| "IndexedDB transaction failed")?;
        let store = tx
            .object_store(STORE_NAME)
            .map_err(|_| "IndexedDB store missing".to_string())?;
        Ok((db, store))
    }

    async fn idb_get(key: &str) -> Option<Vec<u8>> {
        let (db, store) = open_store(web_sys::IdbTransactionMode::Readonly)
            .await
            .ok()?;
        let value = match store.get(&JsValue::from_str(key)) {
            Ok(request) => await_request(request).await.ok(),
            Err(_) => None,
        };
        db.close();
        let value = value?;
        if value.is_undefined() || value.is_null() {
            return None;
        }
        Some(js_sys::Uint8Array::new(&value).to_vec())
    }

    /// NOTE: resolving on the request's `success` is NOT transaction commit -
    /// a put can still abort at commit time (e.g. quota exceeded). Fine for
    /// this cache's best-effort surface; the installer flow (task 163508) must
    /// await the TRANSACTION's `complete` event for its files-first-index-last
    /// discipline, not per-request success.
    async fn idb_put(key: &str, bytes: &[u8]) -> Result<(), String> {
        let (db, store) = open_store(web_sys::IdbTransactionMode::Readwrite).await?;
        let value: JsValue = js_sys::Uint8Array::from(bytes).into();
        let result = match store.put_with_key(&value, &JsValue::from_str(key)) {
            Ok(request) => await_request(request).await.map(|_| ()),
            Err(_) => Err("IndexedDB put failed".to_string()),
        };
        db.close();
        result
    }

    async fn idb_delete(key: &str) -> Result<(), String> {
        let (db, store) = open_store(web_sys::IdbTransactionMode::Readwrite).await?;
        let result = match store.delete(&JsValue::from_str(key)) {
            Ok(request) => await_request(request).await.map(|_| ()),
            Err(_) => Err("IndexedDB delete failed".to_string()),
        };
        db.close();
        result
    }

    async fn idb_all_keys() -> Result<Vec<String>, String> {
        let (db, store) = open_store(web_sys::IdbTransactionMode::Readonly).await?;
        let keys = match store.get_all_keys() {
            Ok(request) => await_request(request).await,
            Err(_) => Err("IndexedDB getAllKeys failed".to_string()),
        };
        db.close();
        Ok(js_sys::Array::from(&keys?)
            .iter()
            .filter_map(|k| k.as_string())
            .collect())
    }
}

// The native fs backend is unit-tested through its pure `*_at` functions under a
// temp root. The wasm half is cfg-guarded: PR/master CI compiles native only, so
// (as with mod_prefs) static review against web-sys 0.3 is its real guard until
// task 163508 exercises it in a browser.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{
        backend::{
            install_local_at, read_index_at, read_mod_file_at, remove_mod_at, remove_mod_files_at,
            store_mod_files_at, write_index_at,
        },
        InstalledModRecord,
    };

    fn record(id: &str, version: &str) -> InstalledModRecord {
        InstalledModRecord {
            id: id.to_string(),
            version: version.to_string(),
            bundle: format!("{id}.bundle.ron"),
        }
    }

    #[test]
    fn index_round_trips_under_a_temp_root() {
        let root = tempfile::tempdir().unwrap();
        let records = vec![record("pack_a", "1.0.0"), record("other", "0.2.0")];
        write_index_at(root.path(), &records).unwrap();
        assert_eq!(
            read_index_at(root.path()),
            Some(records),
            "the records round-trip through RON"
        );
    }

    #[test]
    fn missing_index_reads_none() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            read_index_at(root.path()),
            None,
            "no index file reads as no downloaded mods"
        );
    }

    #[test]
    fn corrupt_index_reads_none() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("installed.mods.ron"), b"not ron {{{").unwrap();
        assert_eq!(
            read_index_at(root.path()),
            None,
            "corrupt data reads as no downloaded mods (not a panic)"
        );
    }

    #[test]
    fn files_store_read_and_remove() {
        let root = tempfile::tempdir().unwrap();
        let files = vec![
            ("pack.bundle.ron".to_string(), b"bundle".to_vec()),
            ("scenarios/run.content.ron".to_string(), b"content".to_vec()),
        ];
        store_mod_files_at(root.path(), "pack", &files).unwrap();
        assert_eq!(
            read_mod_file_at(root.path(), "pack", "scenarios/run.content.ron"),
            Some(b"content".to_vec()),
            "nested files store under mods/<id>/<path>"
        );

        let paths: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
        remove_mod_files_at(root.path(), "pack", &paths).unwrap();
        assert_eq!(
            read_mod_file_at(root.path(), "pack", "pack.bundle.ron"),
            None,
            "removed files no longer read"
        );
        assert!(
            !root.path().join("mods").join("pack").exists(),
            "the emptied mod dir is pruned"
        );
    }

    /// `remove_mod_at` (the uninstall flow's file half) sweeps the WHOLE mod
    /// directory without needing the file list, leaves other mods alone, is
    /// idempotent on a missing dir, and still rejects unsafe ids. Deleting
    /// the backend fn is a compile error; weakening its id gate fails the
    /// last assertion.
    #[test]
    fn remove_mod_sweeps_the_whole_mod_dir() {
        let root = tempfile::tempdir().unwrap();
        let files = vec![
            ("pack.bundle.ron".to_string(), b"bundle".to_vec()),
            ("scenarios/run.content.ron".to_string(), b"content".to_vec()),
        ];
        store_mod_files_at(root.path(), "pack", &files).unwrap();
        store_mod_files_at(root.path(), "other", &files).unwrap();

        remove_mod_at(root.path(), "pack").unwrap();
        assert!(
            !root.path().join("mods").join("pack").exists(),
            "the whole mod dir is gone"
        );
        assert_eq!(
            read_mod_file_at(root.path(), "other", "pack.bundle.ron"),
            Some(b"bundle".to_vec()),
            "other mods' files are untouched"
        );
        remove_mod_at(root.path(), "pack").unwrap(); // missing dir is fine
        assert!(
            remove_mod_at(root.path(), "../other").is_err(),
            "an escaping id must be rejected"
        );
    }

    #[test]
    fn install_local_stores_files_and_upserts_the_record() {
        let root = tempfile::tempdir().unwrap();
        let files = vec![("pack.bundle.ron".to_string(), b"v1".to_vec())];
        install_local_at(root.path(), "pack", "1.0.0", "pack.bundle.ron", &files).unwrap();
        assert_eq!(
            read_index_at(root.path()),
            Some(vec![InstalledModRecord {
                id: "pack".to_string(),
                version: "1.0.0".to_string(),
                bundle: "pack.bundle.ron".to_string(),
            }]),
            "the record lands in the index"
        );

        // Re-installing the same id REPLACES its record (an update), never
        // duplicates it.
        install_local_at(root.path(), "pack", "2.0.0", "pack.bundle.ron", &files).unwrap();
        let records = read_index_at(root.path()).unwrap();
        assert_eq!(records.len(), 1, "same id upserts, not appends");
        assert_eq!(records[0].version, "2.0.0", "the newer version wins");
    }

    /// The sandbox pinned at the READER layer, where it is the ONLY guard: the
    /// raw `FileAssetReader` really serves an escaping `../` request (the
    /// vulnerability is real at this layer - bevy's `UnapprovedPathMode`
    /// load-time gate sits ABOVE the reader and is deliberately bypassed
    /// here), and `SandboxedAssetReader` over the same root refuses it as
    /// not-found. Deleting the sandbox fails the second half.
    #[test]
    fn sandboxed_reader_rejects_escapes_the_raw_reader_serves() {
        use bevy::asset::io::{file::FileAssetReader, AssetReader, AssetReaderError, Reader};

        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(root.path().join("decoy.txt"), b"outside the mods root").unwrap();
        let escape = std::path::Path::new("../decoy.txt");

        let raw = FileAssetReader::new(&mods);
        let raw_bytes = bevy::tasks::block_on(async {
            let mut reader = raw.read(escape).await.expect("the raw reader escapes");
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .expect("read the decoy");
            bytes
        });
        assert_eq!(
            raw_bytes, b"outside the mods root",
            "an unwrapped reader really serves the escaping request"
        );

        let sandboxed = super::SandboxedAssetReader {
            inner: FileAssetReader::new(&mods),
        };
        let denied = bevy::tasks::block_on(async {
            matches!(
                sandboxed.read(escape).await.map(|_| ()),
                Err(AssetReaderError::NotFound(_))
            )
        });
        assert!(
            denied,
            "the sandboxed reader must reject the escaping request as not-found"
        );
    }

    /// The SHARED public-API gate (`validate_file_op`) is cfg-independent: it
    /// runs before the platform dispatch, so passing here pins the exact
    /// validation the wasm backend gets too (review 142906 R1.1/R1.3) - and
    /// rejecting `/` in ids is what keeps the wasm `<id>/<path>` IndexedDB
    /// keys unambiguous.
    #[test]
    fn shared_public_api_gate_rejects_unsafe_ids_and_paths() {
        let ok: Vec<&str> = vec!["pack.bundle.ron", "scenarios/run.content.ron"];
        assert!(super::validate_file_op("pack", ok.iter().copied()).is_ok());
        assert!(
            super::validate_file_op("../pack", ok.iter().copied()).is_err(),
            "an escaping id is rejected before either backend runs"
        );
        assert!(
            super::validate_file_op("a/b", ok.iter().copied()).is_err(),
            "a nested id is rejected (also excludes wasm key ambiguity)"
        );
        assert!(
            super::validate_file_op("pack", ["../evil.ron"]).is_err(),
            "an escaping file path is rejected before either backend runs"
        );
        assert!(
            super::validate_file_op("pack", ["/etc/passwd"]).is_err(),
            "an absolute file path is rejected"
        );
    }

    #[test]
    fn escaping_paths_and_ids_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let evil_file = vec![("../evil.ron".to_string(), b"x".to_vec())];
        assert!(
            store_mod_files_at(root.path(), "pack", &evil_file).is_err(),
            "a .. component must not escape the cache root"
        );
        let file = vec![("ok.ron".to_string(), b"x".to_vec())];
        assert!(
            store_mod_files_at(root.path(), "../pack", &file).is_err(),
            "an escaping id must be rejected"
        );
        assert!(
            store_mod_files_at(root.path(), "a/b", &file).is_err(),
            "a nested id must be rejected (ids are single dir names)"
        );
        assert_eq!(
            read_mod_file_at(root.path(), "pack", "../../etc/passwd"),
            None,
            "reads are bounded the same way"
        );
    }
}
