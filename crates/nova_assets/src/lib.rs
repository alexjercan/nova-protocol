//! `nova_assets` loads the game's assets and assembles its content. It sets up
//! `bevy_asset_loader` (glTF, textures, shaders, sounds), registers the
//! built-in sections and scenarios, and performs the MOD MERGE - resolving the
//! installed-mods catalog and `EnabledMods` into the active section/scenario
//! set. The base content it registers is GENERATED from Rust builders (edit the
//! builder, not the committed `*.content.ron`); those builders and the
//! `content` CLI that writes and lints them live in `nova_authoring`, which the
//! game never links.
//!
//! Static assets PRELOAD through `bevy_asset_loader` collections: the UI font in
//! the `Boot` [`BootAssets`] collection (published as [`nova_ui::font::UiFont`]),
//! and textures, meshes, the shared HUD art and the UI SFX in the `Loading`
//! [`GameAssets`] collection - so everything is load-gated before gameplay
//! starts rather than fetched lazily at first use. Scenario-authored `AssetRef`s,
//! downloaded `mods://` bundles and OPTIONAL cataloged bundles are the DYNAMIC
//! exceptions and stay on the asset server by design: the first two have paths
//! that vary at runtime, and the third is what [`safe_mode`](crate::safe_mode)
//! exists for - only the base game is mandatory, so a broken optional mod is
//! disabled and reported rather than failing the collection the boot waits on.
#![warn(missing_docs)]

mod collections;
mod merge;
mod mod_set;
mod plugin;
mod reload;
mod safe_mode;

#[cfg(not(target_arch = "wasm32"))]
pub mod loose;
pub mod mod_cache;
pub mod mod_prefs;
pub mod mod_refs;
pub mod persist;
pub mod portal;
pub mod storage;

// The six private modules have no path of their own, so the crate root is
// where their preludes surface. The public modules are reachable as
// `nova_assets::<module>::prelude::*` and are re-exported here only for the
// names the crate's own consumers glob.
pub use collections::prelude::*;
pub use merge::prelude::*;
pub use mod_set::prelude::*;
pub use plugin::prelude::*;
pub use reload::prelude::*;
pub use safe_mode::prelude::*;

/// Glob-import surface: `use nova_assets::prelude::*` brings the loaded-asset
/// resources, the mod-set/portal types, and [`GameAssetsPlugin`] into scope.
pub mod prelude {
    pub use nova_mod_format::{PortalCatalog, PortalEntry};
    pub use nova_modding::prelude::ModMeta;

    pub use super::{
        portal::{
            FetchPortalCatalog, InstallJobs, InstallPortalMod, InstallStatus, PendingRemovals,
            PortalConfig, PortalFetchTimeout, RemoteCatalog, RemoteCatalogState,
            UninstallPortalMod,
        },
        DisabledMod, DownloadedMod, DownloadedMods, EnabledMods, FatalAssetFailure, GameAssets,
        GameAssetsPlugin, GameAssetsStates, ModCatalog, ModInfo, ModQuarantine, OptionalBundle,
        OptionalBundles, ReloadContent, RELOAD_KEY,
    };
}

/// The production `register_bundles` system, re-exported for the crate's
/// integration tests (which drive the RON modding pipeline end to end: load the
/// base bundle + its content files and route their items into `GameSections` /
/// `GameScenarios`). Not part of the public API.
#[doc(hidden)]
pub use crate::register_bundles as register_bundles_for_test;
