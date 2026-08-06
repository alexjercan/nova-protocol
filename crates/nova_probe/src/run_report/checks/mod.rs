//! The automatic checks over a run: the OK/WARN/FAIL rows, the provisional
//! verdict, and the `checks.json` mirror an agent reads instead of the HTML.
//!
//! One check per module, each exposing `evaluate(&RunArtifacts) -> Check` and
//! owning its own tests. This file holds only what is SHARED: the row type,
//! the status enum, the skip-detail wording, the verdict fold, and the
//! [`CHECKS`] table that names them in report order.

mod fps_within_baseline;
mod invariants_held;
mod log_clean;
mod process_exit;
mod reached_playing;
mod run_completed;

pub use fps_within_baseline::FPS_WARN_THRESHOLD_PCT;
pub(super) use invariants_held::violations_by_name;

use super::{artifacts::RunArtifacts, manifest::RunManifest};

/// One verdict row.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// Stable check id (`run_completed`, `invariants_held`, ...).
    pub name: &'static str,
    /// The row's outcome.
    pub status: CheckStatus,
    /// The measured value, human-readable.
    pub value: String,
    /// The gate it was held against.
    pub threshold: String,
    /// One sentence of context (why skipped, what failed).
    pub detail: String,
    /// Structured payload for machine consumers (counts, deltas) - the
    /// prose fields are for humans, this is for agents.
    pub data: serde_json::Value,
}

/// Check outcome. `Warn` never fails the run (soft gates); `Skipped` means
/// the input artifact was not captured, not that the property held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The property held.
    Pass,
    /// A soft-gate concern; never fails the run.
    Warn,
    /// The property was violated; fails the run.
    Fail,
    /// The input artifact was not captured, so the check could not run.
    Skipped,
}

impl CheckStatus {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skipped => "SKIPPED",
        }
    }
}

/// Every check, in report order. Adding one is adding a module and a line
/// here - the aggregation holds no per-check knowledge.
const CHECKS: &[fn(&RunArtifacts) -> Check] = &[
    process_exit::evaluate,
    run_completed::evaluate,
    reached_playing::evaluate,
    invariants_held::evaluate,
    fps_within_baseline::evaluate,
    log_clean::evaluate,
];

/// Evaluate every auto check against the loaded artifacts.
pub fn evaluate_checks(artifacts: &RunArtifacts) -> Vec<Check> {
    CHECKS.iter().map(|evaluate| evaluate(artifacts)).collect()
}

/// How many checks actually measured something (not SKIPPED).
pub fn measured_count(checks: &[Check]) -> usize {
    checks
        .iter()
        .filter(|c| c.status != CheckStatus::Skipped)
        .count()
}

/// The provisional overall verdict: FAIL if any hard check failed, WARN if
/// anything warned, NO_DATA when NOTHING was measured (a dir with zero
/// evidence must not read as a passing run), OK otherwise. OK is always
/// OK-with-coverage: consumers read `measured_count` alongside it - an OK
/// with run_completed/invariants_held SKIPPED only proves the example's own
/// assertions (its exit status), not the recorded run. The reviewer owns
/// the final call either way.
pub fn overall_verdict(checks: &[Check]) -> &'static str {
    if measured_count(checks) == 0 {
        "NO_DATA"
    } else if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        "FAIL"
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        "WARN"
    } else {
        "OK"
    }
}

/// Why a timeline-fed check has no input. "Not captured" means different
/// things depending on whether probe ARMED the surface: an armed-but-silent
/// run is a WIRING gap in the example, and saying "arm NOVA_PERF_TIMELINE"
/// there sends the reader after the wrong thing.
fn timeline_skip_detail(manifest: Option<&RunManifest>) -> String {
    match manifest {
        Some(m) if m.armed_timeline => format!(
            "probe armed the recorder but {} is not wired with nova_probe::nova_timeline()",
            m.example
        ),
        _ => "timeline.jsonl not captured (arm NOVA_PERF_TIMELINE)".into(),
    }
}

/// Print the verdict rows to stdout (shared by the probe and run_report
/// bins, so the two never drift).
pub fn print_checks(checks: &[Check]) {
    for check in checks {
        println!(
            "  {:22} {:8} {}",
            check.name,
            check.status.as_str(),
            check.value
        );
    }
}

/// The machine-readable mirror of the verdict rows, plus the run's
/// identity (from the manifest) and the measured-coverage figure - an
/// agent reads verdict AND measured, never verdict alone.
pub fn checks_json(checks: &[Check], manifest: Option<&RunManifest>) -> serde_json::Value {
    serde_json::json!({
        "verdict": overall_verdict(checks),
        "measured": format!("{}/{}", measured_count(checks), checks.len()),
        "reviewer_confirmation_required": true,
        "run": manifest.map(RunManifest::to_json),
        "checks": checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "status": c.status.as_str(),
            "value": c.value,
            "threshold": c.threshold,
            "detail": c.detail,
            "data": c.data,
        })).collect::<Vec<_>>(),
        "generated_by": "nova_probe run_report",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{
        fixtures::*,
        manifest::{PassRecord, RunManifest},
    };

    #[test]
    fn healthy_fixture_passes_every_present_check() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "run_completed").status, CheckStatus::Pass);
        assert_eq!(check(&checks, "reached_playing").status, CheckStatus::Pass);
        assert_eq!(check(&checks, "invariants_held").status, CheckStatus::Pass);
        // No manifest in the fixture -> exit status unknowable.
        assert_eq!(check(&checks, "process_exit").status, CheckStatus::Skipped);
        // No baseline passed -> FPS check skipped even though runs exist.
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::Skipped
        );
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Pass);
        assert_eq!(overall_verdict(&checks), "OK");
        assert_eq!(measured_count(&checks), 4);
    }

    #[test]
    fn zero_evidence_is_no_data_never_ok() {
        let dir = scratch_run_dir();
        for name in ["timeline.jsonl", "frametime.csv", "trace.json", "run.log"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert!(checks.iter().all(|c| c.status == CheckStatus::Skipped));
        assert_eq!(measured_count(&checks), 0);
        // A dir with zero evidence must never read as a passing run.
        assert_eq!(overall_verdict(&checks), "NO_DATA");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn checks_json_mirrors_rows_with_coverage_and_run_identity() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000000,
            git_sha: "abc123".into(),
            full_git_sha: "abc123def".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: false,
            passes: vec![PassRecord {
                name: "clean".into(),
                success: true,
                timed_out: false,
            }],
        };
        let json = checks_json(&checks, Some(&manifest));
        assert_eq!(json["verdict"], "OK");
        assert_eq!(json["measured"], "4/6");
        assert_eq!(json["reviewer_confirmation_required"], true);
        assert_eq!(json["run"]["example"], "playable");
        assert_eq!(json["run"]["passes"][0]["name"], "clean");
        assert_eq!(json["checks"].as_array().unwrap().len(), checks.len());
        // process_exit leads the rows and carries structured data.
        assert_eq!(json["checks"][0]["name"], "process_exit");
        let inv = json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "invariants_held")
            .unwrap();
        assert_eq!(inv["data"]["violations"], 0);
    }

    #[test]
    fn the_checks_table_is_the_only_roster() {
        let artifacts = RunArtifacts::default();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(checks.len(), CHECKS.len());
        let mut names: Vec<&str> = checks.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CHECKS.len(), "two checks share a name");
    }
}
