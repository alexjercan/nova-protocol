//! Generator + parity guard for the four built-in scenarios (task
//! 20260525-133028 follow-up: the code-built built-ins are now RON data files).
//!
//! Each built-in's config builder is the SINGLE definition of that scenario; at
//! runtime `register_scenario` loads the committed `assets/scenarios/*.ron`,
//! and the builder is only exercised here. This test rebuilds each config with
//! PATH-based asset refs (cubemap/texture paths and path-based section meshes -
//! exactly what production loads them from), serializes it with a deterministic
//! `PrettyConfig`, and compares the result to the committed file.
//!
//! The first run (before the files exist) WRITES them, so `cargo test` is the
//! tool that produces the data files; every run after that asserts the file on
//! disk still matches the builder, catching drift in either direction.

use std::path::PathBuf;

use nova_assets::scenario_generation::{build_scenarios, pretty_config};

/// The workspace `assets/scenarios` dir (tests run with the crate root as cwd).
fn scenarios_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/scenarios")
        .canonicalize()
        .expect("assets/scenarios dir exists")
}

#[test]
fn built_in_scenarios_match_their_committed_ron() {
    let dir = scenarios_dir();

    for scenario in build_scenarios() {
        let path = dir.join(format!("{}.scenario.ron", scenario.id));
        let generated = ron::ser::to_string_pretty(&scenario, pretty_config())
            .unwrap_or_else(|err| panic!("serialize {}: {err}", scenario.id));
        // A trailing newline keeps the files POSIX-clean and matches the demo.
        let generated = format!("{generated}\n");

        if !path.exists() {
            std::fs::write(&path, &generated)
                .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
            // Do not fail the run that creates the file; the next run guards it.
            continue;
        }

        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        assert_eq!(
            committed,
            generated,
            "committed {} has drifted from its builder; regenerate it (delete the file and \
             re-run this test, or write the generated string) - diff the two to see the change",
            path.display()
        );
    }
}
