//! Generator + parity guard for the section-prototype catalog (task
//! 20260714-113408: the code-built section catalog is now a RON data file).
//!
//! `build_sections` (via `build_section_catalog`) is the SINGLE definition of the
//! catalog; at runtime `register_sections` loads the committed
//! `assets/sections/base.sections.ron` into `GameSections`, and the builder is only
//! exercised here. This test rebuilds the catalog with PATH-based mesh refs (what
//! production loads them from), serializes it with the same deterministic
//! `PrettyConfig` the scenarios use, and compares to the committed file.
//!
//! The first run (before the file exists) WRITES it, so `cargo test` is the tool
//! that produces the data file; every run after that asserts the file on disk still
//! matches the builder, catching drift in either direction.

use std::path::PathBuf;

use nova_assets::scenario_generation::{build_section_catalog, pretty_config};

/// The workspace `assets/sections` dir (created on the first run).
fn sections_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sections")
}

#[test]
fn built_in_section_catalog_matches_committed_ron() {
    let dir = sections_dir();
    let path = dir.join("base.sections.ron");

    let catalog = build_section_catalog();
    let generated =
        ron::ser::to_string_pretty(&catalog, pretty_config()).expect("serialize section catalog");
    // A trailing newline keeps the file POSIX-clean and matches the scenarios.
    let generated = format!("{generated}\n");

    if !path.exists() {
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|err| panic!("create {}: {err}", dir.display()));
        std::fs::write(&path, &generated)
            .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
        // Do not fail the run that creates the file; the next run guards it.
        return;
    }

    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    assert_eq!(
        committed,
        generated,
        "committed {} has drifted from build_sections; regenerate it (delete the file and \
         re-run this test) - diff the two to see the change",
        path.display()
    );
}
