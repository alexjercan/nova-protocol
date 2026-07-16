//! Pure serde types for the Nova Protocol mod formats - the ENGINE-FREE half of
//! the modding data model, split out of `nova_modding` (task 20260715-142900) so
//! the portal generator (`nova_portal_gen`) builds in seconds without bevy.
//!
//! Three format families live here:
//! - the on-disk bundle manifest (`*.bundle.ron`): [`BundleManifest`] + the mod's
//!   [`ModMeta`] self-description;
//! - the installed catalog (`mods.catalog.ron`): [`CatalogManifest`] of thin
//!   [`ModEntry`] declarations;
//! - the PORTAL wire schema (`catalog.json`, JSON): [`PortalCatalog`] /
//!   [`PortalEntry`] / [`PortalFile`] - what the static mod portal serves and the
//!   game fetches when browsing Explore.
//!
//! `nova_modding` re-exports the RON types, so game code keeps importing them
//! from there; the asset LOADERS (which need bevy) stay in `nova_modding`.

use serde::{Deserialize, Serialize};

/// Pure mod-dependency resolution (topological order, transitive closure,
/// dependents) over an id-keyed graph. Engine-free; shared by the merge, the
/// menu and the portal. Task 20260715-142931.
pub mod deps;

/// The portal catalog schema version THIS build writes and reads. Bump on any
/// breaking change to the wire types below; the game checks it before trusting
/// a fetched catalog, so old clients fail loud instead of misparsing.
pub const PORTAL_SCHEMA_VERSION: u32 = 1;

/// A mod's SELF-DESCRIPTION, authored in its `*.bundle.ron` manifest - the
/// Factorio `info.json` analog and the single source of truth for mod metadata
/// (the menu list, the details panel, and the mod portal all read it).
///
/// Every field is optional (serde-defaulted) so a bare `(content: [...])`
/// manifest stays valid. Conventions: `version` is an opaque semver-ish string
/// (empty = unversioned; the base game leaves it empty, the GAME version is
/// authoritative there; the PORTAL requires it non-empty to publish);
/// `dependencies` lists mod ids - `base` is an IMPLICIT dependency and is not
/// declared (resolved by [`deps`]: install pulls them, enable auto-enables them,
/// merge order is topological); `icon`/`screenshots` are paths
/// relative to the bundle's directory (RON `Option` syntax:
/// `icon: Some("icon.png")`), reserved for the portal and the details panel.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModMeta {
    /// Display name for the mods list (falls back to the catalog id when empty).
    #[serde(default)]
    pub name: String,
    /// One-line description for the mods list / details panel.
    #[serde(default)]
    pub description: String,
    /// Author credit, shown in the details panel.
    #[serde(default)]
    pub author: String,
    /// Opaque version string (semver-ish); empty = unversioned.
    #[serde(default)]
    pub version: String,
    /// Ids of mods this one needs (`base` is implicit, never listed). Resolved
    /// by [`deps`]: installing pulls missing ones, enabling auto-enables them,
    /// and merge order is dependency-topological. Ids only - no version ranges.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Icon image path, relative to the bundle directory (reserved).
    #[serde(default)]
    pub icon: Option<String>,
    /// Screenshot image paths, relative to the bundle directory (reserved).
    #[serde(default)]
    pub screenshots: Vec<String>,
}

/// The on-disk `*.bundle.ron` manifest: the list of content files a bundle
/// folder packages, as paths RELATIVE to the manifest file's own directory,
/// plus the mod's [`ModMeta`] self-description.
///
/// A bundle is a DIRECTORY plus this manifest - the manifest, not directory
/// enumeration, is what makes bundles wasm-safe (`load_folder` is broken on the
/// web target, so a bundle can never rely on listing its directory).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Content-file paths, relative to the `bundle.ron` file's directory (e.g.
    /// `"sections/base.content.ron"`, `"scenarios/shakedown_run.content.ron"`).
    pub content: Vec<String>,
    /// The mod's self-description; defaulted so meta-less manifests stay valid.
    #[serde(default)]
    pub meta: ModMeta,
}

/// One INSTALLED mod's catalog DECLARATION: identity + where to find it +
/// deployment flags. A thin pointer, not metadata - the mod's self-description
/// (name, author, version, ...) lives in its own bundle's [`ModMeta`].
///
/// `id` is what `register_bundles` keys enable/disable on and the merge-overlay
/// namespace. `bundle` is the mod's `*.bundle.ron` manifest path, RELATIVE to the
/// asset root (the catalog lives at the root). `base` marks the base game's own
/// entry - enabled by default and (in the UI) locked on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModEntry {
    /// Stable id - the enable/disable key and the merge-overlay namespace.
    pub id: String,
    /// The mod's `*.bundle.ron` manifest path, asset-root-relative.
    pub bundle: String,
    /// True for the base game's entry: enabled by default, locked on in the UI.
    #[serde(default)]
    pub base: bool,
    /// True for dev/tooling mods: omitted from the player-facing mods list, but
    /// still installed - the bundle loads and the mod is enableable by id from
    /// code. No shipped mod uses it right now; the semantics are pinned by
    /// nova_assets' synthetic-catalog tests.
    #[serde(default)]
    pub hidden: bool,
}

/// The on-disk `mods.catalog.ron`: every INSTALLED mod, in load order (base first).
///
/// This is the wasm-safe source of truth for what is installed - a manifest, never
/// directory enumeration (`load_folder` is broken on the web target). Which entries
/// are ENABLED is a separate runtime concern (`nova_assets::EnabledMods`), not baked
/// into this read-only asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogManifest {
    /// Installed mods, in load order. `base` should come first so it merges first.
    pub mods: Vec<ModEntry>,
}

/// One file of a published portal mod: its path within the mod directory plus
/// the size and sha256 the client verifies after download.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalFile {
    /// Path relative to the mod's version directory (mirrors the source layout).
    pub path: String,
    /// File size in bytes (also the download-size display and a cheap pre-check).
    pub size: u64,
    /// Lowercase hex sha256 of the file's bytes.
    pub sha256: String,
}

/// One published mod in the portal catalog: identity + self-description + the
/// complete, generated file list (the client never enumerates directories - the
/// generator did, at publish time).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortalEntry {
    /// Stable mod id (the webmods/ source directory name).
    pub id: String,
    /// The published version (from the bundle meta; non-empty - the generator
    /// rejects unversioned mods). Files live under `<id>/<version>/`.
    pub version: String,
    /// The bundle manifest's path within the mod directory (identifies which of
    /// `files` is the entry point).
    pub bundle: String,
    /// The mod's self-description, copied from its bundle meta at publish time.
    pub meta: ModMeta,
    /// Every file of the mod, with sizes + hashes, sorted by path.
    pub files: Vec<PortalFile>,
    /// Sum of `files[].size` (download-size display without iterating).
    pub total_size: u64,
}

/// The portal's `catalog.json`: everything the static mod portal serves,
/// generated by `nova_portal_gen` - never hand-maintained. JSON (not RON) so the
/// TypeScript site and a future server API can produce/consume the same shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortalCatalog {
    /// Wire-schema version ([`PORTAL_SCHEMA_VERSION`]); clients reject unknown.
    pub schema_version: u32,
    /// Published mods, sorted by id.
    pub entries: Vec<PortalEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A meta-less `(content: [...])` manifest still decodes (back-compat pin:
    /// every field of [`ModMeta`] is serde-defaulted), and a manifest WITH a meta
    /// block decodes every field.
    #[test]
    fn bundle_manifest_ron_decodes() {
        // Meta-less body (the pre-142849 format) -> ModMeta::default().
        let ron = r#"(content: [
            "sections/base.content.ron",
            "scenarios/demo.content.ron",
        ])"#;

        let manifest: BundleManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("bundle manifest should decode");
        assert_eq!(
            manifest.content,
            vec![
                "sections/base.content.ron".to_string(),
                "scenarios/demo.content.ron".to_string(),
            ]
        );
        assert!(
            manifest.meta.name.is_empty() && manifest.meta.dependencies.is_empty(),
            "a meta-less manifest defaults to an empty ModMeta"
        );

        // Full meta block -> every field decodes.
        let ron = r#"(
            content: ["mod.content.ron"],
            meta: (
                name: "Demo Mod",
                description: "a demo",
                author: "someone",
                version: "1.2.3",
                dependencies: ["other-mod"],
                icon: Some("icon.png"),
                screenshots: ["shots/a.png"],
            ),
        )"#;
        let manifest: BundleManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("meta manifest should decode");
        assert_eq!(manifest.meta.name, "Demo Mod");
        assert_eq!(manifest.meta.description, "a demo");
        assert_eq!(manifest.meta.author, "someone");
        assert_eq!(manifest.meta.version, "1.2.3");
        assert_eq!(manifest.meta.dependencies, vec!["other-mod".to_string()]);
        assert_eq!(manifest.meta.icon.as_deref(), Some("icon.png"));
        assert_eq!(manifest.meta.screenshots, vec!["shots/a.png".to_string()]);
    }

    /// A `mods.catalog.ron` body decodes into a [`CatalogManifest`] carrying the
    /// installed mods' thin declarations in order, with `base` and `hidden`
    /// defaulting to false when omitted. (The actual load of each bundle into an
    /// `InstalledCatalog` is exercised by the `nova_assets` integration test on
    /// the real asset server.)
    #[test]
    fn catalog_manifest_ron_decodes() {
        let ron = r#"(mods: [
            (id: "base", bundle: "base/base.bundle.ron", base: true),
            (id: "demo", bundle: "mods/demo/demo.bundle.ron"),
            (id: "reel", bundle: "mods/reel/reel.bundle.ron", hidden: true),
        ])"#;
        let manifest: CatalogManifest =
            ron::de::from_bytes(ron.as_bytes()).expect("catalog should decode");
        assert_eq!(manifest.mods.len(), 3);
        assert_eq!(manifest.mods[0].id, "base");
        assert!(manifest.mods[0].base, "base flag decodes");
        assert!(!manifest.mods[0].hidden, "hidden defaults to false");
        assert_eq!(manifest.mods[1].id, "demo");
        assert!(
            !manifest.mods[1].base,
            "base defaults to false when omitted"
        );
        assert_eq!(manifest.mods[1].bundle, "mods/demo/demo.bundle.ron");
        assert!(manifest.mods[2].hidden, "hidden flag decodes");
        assert!(!manifest.mods[2].base);
    }

    /// The portal wire schema round-trips through JSON byte-identically enough
    /// to trust between the generator and the game: every field survives, and
    /// the schema-version constant is what the catalog carries.
    #[test]
    fn portal_catalog_json_round_trips() {
        let catalog = PortalCatalog {
            schema_version: PORTAL_SCHEMA_VERSION,
            entries: vec![PortalEntry {
                id: "fixture_pack".to_string(),
                version: "1.0.0".to_string(),
                bundle: "fixture_pack.bundle.ron".to_string(),
                meta: ModMeta {
                    name: "Fixture Pack".to_string(),
                    description: "a synthetic fixture pack".to_string(),
                    author: "someone".to_string(),
                    version: "1.0.0".to_string(),
                    dependencies: vec![],
                    icon: None,
                    screenshots: vec![],
                },
                files: vec![PortalFile {
                    path: "fixture_pack.bundle.ron".to_string(),
                    size: 123,
                    sha256: "ab".repeat(32),
                }],
                total_size: 123,
            }],
        };
        let json = serde_json::to_string_pretty(&catalog).expect("serialize");
        let back: PortalCatalog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema_version, PORTAL_SCHEMA_VERSION);
        assert_eq!(back.entries.len(), 1);
        let entry = &back.entries[0];
        assert_eq!(entry.id, "fixture_pack");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.bundle, "fixture_pack.bundle.ron");
        assert_eq!(entry.meta.name, "Fixture Pack");
        assert_eq!(entry.files, catalog.entries[0].files);
        assert_eq!(entry.total_size, 123);
    }
}
