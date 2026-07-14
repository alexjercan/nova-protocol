//! Generator + parity guard for the built-in content files (task 20260714-150508:
//! the section catalog and the four built-in scenarios are now UNIFORM
//! `*.content.ron` files - a RON `Vec<Content>` where each item carries its kind).
//!
//! The config builders (`build_section_catalog` / `build_scenarios`) are the
//! SINGLE definition of each built-in; at runtime `register_bundles` loads the
//! committed `assets/base/**/*.content.ron` (via the base bundle) and routes each
//! item into `GameSections` /
//! `GameScenarios`, and the builders are only exercised here. This test rebuilds
//! each file's `Vec<Content>` with PATH-based asset refs (exactly what production
//! loads them from), serializes it with the deterministic `PrettyConfig`, and
//! compares to the committed file.
//!
//! - `assets/base/sections/base.content.ron` = one `Vec<Content>` of `Section((..))`.
//! - `assets/base/scenarios/<id>.content.ron` = a `Vec<Content>` with one `Scenario((..))`.
//!
//! The first run (before a file exists) WRITES it, so `cargo test` is the tool
//! that produces the data files; every run after that asserts the file on disk
//! still matches the builder, catching drift in either direction. (The
//! hand-migrated `demo.content.ron` has no builder and is not guarded here - the
//! `demo_scenario` integration test exercises it instead.)

use std::path::PathBuf;

use nova_assets::scenario_generation::{
    build_scenario_contents, build_section_content, pretty_config,
};
use nova_modding::prelude::Content;

/// The workspace `assets` dir (tests run with the crate root as cwd).
fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// Serialize a `Vec<Content>` the way production authored the committed file:
/// the deterministic pretty config plus a trailing newline (POSIX-clean).
fn serialize(content: &[Content]) -> String {
    let body = ron::ser::to_string_pretty(&content.to_vec(), pretty_config())
        .expect("serialize content Vec");
    format!("{body}\n")
}

/// Assert `path` on disk equals `generated`, writing it (and skipping the
/// assertion) on the first run when the file does not yet exist.
fn guard(path: &std::path::Path, generated: &str) {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
        }
        std::fs::write(path, generated)
            .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
        // Do not fail the run that creates the file; the next run guards it.
        return;
    }

    let committed = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    assert_eq!(
        committed,
        generated,
        "committed {} has drifted from its builder; regenerate it (delete the file and \
         re-run this test) - diff the two to see the change",
        path.display()
    );
}

#[test]
fn built_in_section_content_matches_committed_ron() {
    let path = assets_dir().join("base/sections/base.content.ron");
    let generated = serialize(&build_section_content());
    guard(&path, &generated);
}

#[test]
fn built_in_scenario_content_matches_committed_ron() {
    let dir = assets_dir().join("base/scenarios");
    for (id, content) in build_scenario_contents() {
        let path = dir.join(format!("{id}.content.ron"));
        let generated = serialize(&content);
        guard(&path, &generated);
    }
}
