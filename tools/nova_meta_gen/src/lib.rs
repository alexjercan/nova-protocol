//! Default `.meta` sidecar generator for the web build.
//!
//! # Why this exists
//!
//! The game ships `AssetMetaCheck::Always` (see `nova_core::assets_plugin`),
//! which makes Bevy fetch a `<path>.meta` sidecar for EVERY asset. On a server
//! that returns a real 404 for a missing file, Bevy falls back to the loader's
//! default meta and all is well. But `trunk serve` (and any SPA
//! history-fallback server) answers a missing `.meta` with `200 OK` and an
//! HTML body; Bevy then tries to parse that HTML as RON and fails with
//! `Failed to deserialize meta ... ExpectedNamedStructLike("AssetMetaMinimal")`,
//! and the asset never loads. See
//! the `asset-meta-always-web-cost` lesson (LESSONS.md, repo root) for the trace.
//!
//! The fix is to make sure every asset actually HAS a `.meta`, so the server
//! never has to serve a missing one. This tool writes the default `.meta` for
//! every asset that lacks one, using Bevy's own
//! [`AssetServer::write_default_loader_meta_file_for_path`] so each sidecar
//! names the correct loader (picked by extension) with that loader's default
//! settings. Hand-authored sidecars (e.g. the cubemap `array_layout` metas) are
//! never overwritten - the Bevy API returns `MetaAlreadyExists` and we skip.
//!
//! # Headless and GPU-free
//!
//! We register exactly the loaders the game's assets use, the same way the
//! headless asset tests do (manual `register_asset_loader` with
//! `CompressedImageFormats::NONE`), and we NEVER add `RenderPlugin` or call
//! `App::run`. `default_meta()` only reads each loader's `Settings::default()`,
//! so the loader instances' runtime field values are irrelevant here. This lets
//! the tool run in CI (the deploy job) with no GPU.

#![warn(missing_docs)]

use std::path::{Path, PathBuf};

use bevy::{
    app::App,
    asset::{AssetApp, AssetPlugin, AssetServer},
    audio::AudioLoader,
    gltf::GltfPlugin,
    image::{CompressedImageFormats, Image, ImageLoader},
    shader::{Shader, ShaderLoader},
    tasks::block_on,
    MinimalPlugins,
};
use nova_modding::NovaModdingPlugin;

/// What happened for a single asset file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A fresh default `.meta` was written next to the asset.
    Written,
    /// A `.meta` already existed (hand-authored or from a previous run); kept.
    AlreadyExists,
    /// No registered loader claims this extension (e.g. `.md`); nothing to do.
    NoLoader,
}

/// Summary of a generation pass.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Count of fresh default `.meta` sidecars written.
    pub written: usize,
    /// Count of assets that already had a `.meta` and were left untouched.
    pub already_exists: usize,
    /// Count of assets no registered loader claims (skipped).
    pub no_loader: usize,
}

/// Build the headless, GPU-free generator app for the asset source rooted at
/// `assets_dir`. The default asset source's reader AND writer are rooted here,
/// so [`AssetServer::write_default_loader_meta_file_for_path`] writes sidecars
/// back into `assets_dir`.
pub fn build_app(assets_dir: &str) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: assets_dir.to_string(),
            ..Default::default()
        },
    ));

    // Built-in loaders, registered by hand (no render device needed). The
    // compressed-format set does not affect `default_meta()` output.
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE));
    app.init_asset::<Shader>();
    app.register_asset_loader(ShaderLoader);
    app.register_asset_loader(AudioLoader);

    // GltfPlugin registers the `gltf`/`glb` loader in its `finish()`, falling
    // back to `CompressedImageFormats::NONE` when no `RenderPlugin` supplied a
    // device. It only touches Gltf asset types, so it is safe without the rest
    // of DefaultPlugins.
    app.add_plugins(GltfPlugin::default());

    // The three custom RON loaders: `content.ron`, `bundle.ron`, `catalog.ron`.
    app.add_plugins(NovaModdingPlugin);

    // `finish()` runs GltfPlugin's loader registration; `cleanup()` keeps the
    // App in a consistent post-startup state. Neither runs the schedule.
    app.finish();
    app.cleanup();
    app
}

/// Recursively collect asset files under `root`, relative to `root`, skipping
/// existing `.meta` sidecars (they are handled by the write API's own
/// "already exists" check, and we never want to generate a meta for a meta).
fn collect_asset_paths(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "meta") {
                continue;
            } else if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Classify a single write attempt. Returns `Err` only for genuine failures
/// (I/O, write errors); "no loader" and "already exists" are normal [`Outcome`]s.
fn generate_one(server: &AssetServer, rel: &Path) -> Result<Outcome, String> {
    use bevy::asset::{io::AssetSourceId, AssetPath};

    // Explicit default source so a filename that happens to contain "://" is
    // never misread as a source scheme.
    let asset_path = AssetPath::from_path(rel).with_source(AssetSourceId::Default);
    match block_on(server.write_default_loader_meta_file_for_path(asset_path)) {
        Ok(()) => Ok(Outcome::Written),
        Err(bevy::asset::WriteDefaultMetaError::MetaAlreadyExists) => Ok(Outcome::AlreadyExists),
        Err(bevy::asset::WriteDefaultMetaError::MissingAssetLoader(_)) => Ok(Outcome::NoLoader),
        Err(e) => Err(format!("{}: {e}", rel.display())),
    }
}

/// Generate default `.meta` sidecars for every asset under `assets_dir` that
/// lacks one. Returns a [`Summary`]; per-file loader/existence results are
/// reported through `on_outcome` (use it to log). Errors from individual files
/// are collected and returned so the caller can decide whether to fail.
pub fn generate(
    server: &AssetServer,
    assets_dir: &str,
    mut on_outcome: impl FnMut(&Path, &Outcome),
) -> Result<Summary, Vec<String>> {
    let root = Path::new(assets_dir);
    let paths = collect_asset_paths(root).map_err(|e| vec![format!("scan {assets_dir}: {e}")])?;

    let mut summary = Summary::default();
    let mut errors = Vec::new();
    for rel in &paths {
        match generate_one(server, rel) {
            Ok(outcome) => {
                match outcome {
                    Outcome::Written => summary.written += 1,
                    Outcome::AlreadyExists => summary.already_exists += 1,
                    Outcome::NoLoader => summary.no_loader += 1,
                }
                on_outcome(rel, &outcome);
            }
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(summary)
    } else {
        Err(errors)
    }
}
