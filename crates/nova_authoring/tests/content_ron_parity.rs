//! Parity guard for the built-in content files: the committed
//! `assets/base/**/*.content.ron` and `assets/mods/nova_protocol/**/*.content.ron`
//! must match their builders byte for byte.
//!
//! The config builders (`build_section_catalog` / `build_scenarios` /
//! `build_story_scenarios`) are the SINGLE definition of each built-in; at
//! runtime `register_bundles` loads the committed RON (via each bundle) and
//! routes each item into `GameSections` / `GameScenarios`. The `content` CLI's
//! `gen` subcommand is the one writer of those files; this test is assert-only
//! - a MISSING file fails like a drifted one, so `cargo test` never mutates
//! the assets tree.
//!
//! - `assets/base/sections/base.content.ron` = one `Vec<Content>` of `Section((..))`.
//! - `assets/base/scenarios/<id>.content.ron` = a `Vec<Content>` with one `Scenario((..))`.
//! - `assets/mods/nova_protocol/scenarios/<id>.content.ron` = the same, for the story.
//!
//! A second guard pins the UNIFORMITY invariant (every generated bundle's
//! content file is builder-backed): each bundle must ship exactly the
//! generated file set under its directory, so a hand-written file cannot hide
//! in the bundle.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use nova_authoring::generation::{content_files, STORY_MOD_DIR};
use nova_mod_format::BundleManifest;

/// The one regeneration path, named by every failure in this file.
const REGEN: &str = "run `cargo run content gen` and commit the result";

/// The workspace `assets` dir (tests run with the crate root as cwd).
fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

#[test]
fn committed_content_matches_builders() {
    for (rel, generated) in content_files() {
        let path = assets_dir().join(&rel);
        assert!(path.exists(), "{} is missing; {REGEN}", path.display());
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        assert_eq!(
            committed,
            generated,
            "committed {} has drifted from its builder; {REGEN} - diff the two to see the change",
            path.display()
        );
    }
}

/// The generated bundles: each bundle directory (assets-root-relative) and
/// its manifest path.
fn generated_bundles() -> [(&'static str, String); 2] {
    [
        ("base", "base/base.bundle.ron".to_string()),
        (
            STORY_MOD_DIR,
            format!("{STORY_MOD_DIR}/nova_protocol.bundle.ron"),
        ),
    ]
}

/// Each generated bundle's content list and the generator's file map under
/// its directory must be the SAME set (paths in a bundle are relative to the
/// bundle's directory). Catches both directions: a generated file the bundle
/// forgot to ship, and a hand-added bundle entry no builder backs.
#[test]
fn every_generated_bundle_ships_exactly_its_generated_files() {
    let mut generated: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (rel, _) in content_files() {
        let (dir, _) = generated_bundles()
            .into_iter()
            .find(|(dir, _)| rel.starts_with(&format!("{dir}/")))
            .unwrap_or_else(|| panic!("generated file {rel} belongs to no generated bundle"));
        let inside = rel[dir.len() + 1..].to_string();
        generated.entry(dir).or_default().insert(inside);
    }

    for (dir, manifest_rel) in generated_bundles() {
        let bundle_path = assets_dir().join(&manifest_rel);
        let manifest: BundleManifest = ron::de::from_str(
            &std::fs::read_to_string(&bundle_path)
                .unwrap_or_else(|err| panic!("read {}: {err}", bundle_path.display())),
        )
        .unwrap_or_else(|err| panic!("{manifest_rel} parses as a BundleManifest: {err}"));

        let shipped: BTreeSet<String> = manifest.content.into_iter().collect();
        let expected = generated.remove(dir).unwrap_or_default();
        assert_eq!(
            shipped, expected,
            "{manifest_rel} and the generator disagree about the {dir} content set; \
             {REGEN} and align the bundle's content list"
        );
    }
}
