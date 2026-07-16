//! Parity guard for the built-in content files (tasks 20260714-150508,
//! 20260716-155823): the committed `assets/base/**/*.content.ron` must match
//! their builders byte for byte.
//!
//! The config builders (`build_section_catalog` / `build_scenarios`) are the
//! SINGLE definition of each built-in; at runtime `register_bundles` loads the
//! committed RON (via the base bundle) and routes each item into
//! `GameSections` / `GameScenarios`. The `gen_content` bin is the one writer
//! of those files; this test is assert-only - a MISSING file fails like a
//! drifted one, so `cargo test` never mutates the assets tree.
//!
//! - `assets/base/sections/base.content.ron` = one `Vec<Content>` of `Section((..))`.
//! - `assets/base/scenarios/<id>.content.ron` = a `Vec<Content>` with one `Scenario((..))`.
//!
//! A second guard pins the UNIFORMITY invariant (every base content file is
//! builder-backed, per 20260716-155816): `base.bundle.ron` must ship exactly
//! the generated file set, so a hand-written file cannot hide in the bundle.

use std::{collections::BTreeSet, path::PathBuf};

use nova_assets::scenario_generation::content_files;
use nova_mod_format::BundleManifest;

/// The one regeneration path, named by every failure in this file.
const REGEN: &str = "run `cargo run -p nova_assets --bin gen_content` and commit the result";

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

/// `base.bundle.ron`'s content list and the generator's file map must be the
/// SAME set (paths in the bundle are relative to the bundle's directory,
/// `assets/base/`). Catches both directions: a generated file the bundle
/// forgot to ship, and a hand-added bundle entry no builder backs.
#[test]
fn base_bundle_ships_exactly_the_generated_files() {
    let bundle_path = assets_dir().join("base/base.bundle.ron");
    let manifest: BundleManifest = ron::de::from_str(
        &std::fs::read_to_string(&bundle_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", bundle_path.display())),
    )
    .expect("base.bundle.ron parses as a BundleManifest");

    let shipped: BTreeSet<String> = manifest.content.into_iter().collect();
    let generated: BTreeSet<String> = content_files()
        .into_iter()
        .map(|(rel, _)| {
            rel.strip_prefix("base/")
                .expect("generated files live under assets/base/")
                .to_string()
        })
        .collect();
    assert_eq!(
        shipped, generated,
        "base.bundle.ron and the generator disagree about the base content set; \
         {REGEN} and align the bundle's content list"
    );
}
