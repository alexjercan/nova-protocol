//! Shared test fixtures: the checked-in mini run dir, a scratch copy of
//! it for mutation tests, and a healthy manifest.

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use super::{
    checks::Check,
    manifest::{PassRecord, RunManifest},
};

pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/run-mini")
}

/// A scratch run dir seeded from the fixture, for mutation tests.
pub fn scratch_run_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("nova_run_report_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(fixture()).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
    }
    dir
}

pub fn check<'a>(checks: &'a [Check], name: &str) -> &'a Check {
    checks.iter().find(|c| c.name == name).unwrap()
}

pub fn manifest_ok() -> RunManifest {
    RunManifest {
        example: "controller_section".into(),
        started_unix: 1,
        git_sha: "abc".into(),
        full_git_sha: "abcdef".into(),
        host: "h".into(),
        armed_timeline: true,
        armed_invariants: true,
        armed_fps: false,
        passes: vec![PassRecord {
            name: "clean".into(),
            success: true,
            timed_out: false,
        }],
    }
}
