//! The automatic checks over a run: the OK/WARN/FAIL rows, the provisional
//! verdict, and the `checks.json` mirror an agent reads instead of the HTML.
//!
//! One check per module, each exposing `evaluate(&RunArtifacts) -> Check` and
//! owning its own tests. This file holds only what is SHARED: the row type,
//! the status enum, the skip-detail wording, the verdict fold, and the
//! `CHECKS` table that names them in report order.

/// Glob-import surface for the check roster and its verdicts.
pub mod prelude {
    pub use super::{
        check_names, checks_json, evaluate_checks, measured_count, overall_verdict, print_checks,
        status_class, Check, CheckStatus, NotApplicable, FPS_WARN_THRESHOLD_PCT,
    };
}

mod artifacts_loadable;
mod fps_within_baseline;
mod invariants_held;
mod log_clean;
mod process_exit;
mod reached_playing;
mod run_completed;

pub use fps_within_baseline::FPS_WARN_THRESHOLD_PCT;
pub(crate) use invariants_held::violations_by_name;
use nova_probe::prelude::*;

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

/// Check outcome. `Warn` never fails the run (soft gates). Neither
/// `Skipped` nor `NotApplicable` means the property held - the first is
/// "unknowable here", the second is "there was nothing to hold it to".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// The property held.
    Pass,
    /// A soft-gate concern; never fails the run.
    Warn,
    /// The property was violated; fails the run.
    Fail,
    /// The input artifact was not captured and nothing says why: a run dir
    /// that predates the contract, a web run, a dir with no manifest.
    Skipped,
    /// The check does not apply to this run, with the reason as a VALUE.
    NotApplicable(NotApplicable),
}

/// Why a check does not apply. The two capability variants are the two
/// halves of the coverage handshake; the two input variants are the
/// operator's side of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotApplicable {
    /// The example wires no such plugin - it makes no claim.
    NotDeclared(Capability),
    /// Wired, but this run did not arm it.
    NotArmed(Capability),
    /// An operator-supplied input was not given (`--baseline`).
    InputNotSupplied(&'static str),
    /// An input WAS supplied but cannot be compared (a baseline sharing no
    /// run labels). Never a pass.
    InputNotComparable(&'static str),
}

impl CheckStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skipped => "SKIPPED",
            CheckStatus::NotApplicable(_) => "N/A",
        }
    }

    /// Whether the check produced a judgement about the property. Neither
    /// SKIPPED nor N/A does.
    pub fn graded(self) -> bool {
        matches!(
            self,
            CheckStatus::Pass | CheckStatus::Warn | CheckStatus::Fail
        )
    }
}

/// The CSS/class token for a status string as it appears in `checks.json`.
/// Separate from the wire word because `N/A` is not a legal class name.
pub fn status_class(status: &str) -> &'static str {
    match status {
        "PASS" => "pass",
        "WARN" => "warn",
        "FAIL" => "fail",
        "SKIPPED" => "skipped",
        "N/A" => "na",
        _ => "unknown",
    }
}

/// Every check, in report order: its name, the capability it grades (`None`
/// for the three that need no plugin - probe owns the exit status, the
/// captured stdio and the artifacts themselves for every run), and its
/// evaluator. Adding a check is
/// adding a module and a row here - the aggregation holds no per-check
/// knowledge.
const CHECKS: &[(&str, Option<Capability>, fn(&RunArtifacts) -> Check)] = &[
    ("process_exit", None, process_exit::evaluate),
    (
        "run_completed",
        Some(Capability::Timeline),
        run_completed::evaluate,
    ),
    (
        "reached_playing",
        Some(Capability::Timeline),
        reached_playing::evaluate,
    ),
    (
        "invariants_held",
        Some(Capability::Invariants),
        invariants_held::evaluate,
    ),
    (
        "fps_within_baseline",
        Some(Capability::FrameTime),
        fps_within_baseline::evaluate,
    ),
    ("log_clean", None, log_clean::evaluate),
    // Last because it grades the EVIDENCE rather than the run: when it fails,
    // it is the reason the rows above read the way they do.
    ("artifacts_loadable", None, artifacts_loadable::evaluate),
];

/// Every check's name, in report order. The aggregate table renders one
/// column per entry, so a check added to `CHECKS` appears there too rather
/// than needing a second list that can silently fall behind.
pub fn check_names() -> impl Iterator<Item = &'static str> {
    CHECKS.iter().map(|(name, _, _)| *name)
}

/// Evaluate every auto check against the loaded artifacts.
pub fn evaluate_checks(artifacts: &RunArtifacts) -> Vec<Check> {
    CHECKS
        .iter()
        .map(|(_, _, evaluate)| evaluate(artifacts))
        .collect()
}

/// The capability a check grades, by name - `None` both for a check that
/// needs no plugin and for a name the roster does not know.
fn capability_of(name: &str) -> Option<Capability> {
    CHECKS
        .iter()
        .find(|(check, _, _)| *check == name)
        .and_then(|(_, capability, _)| *capability)
}

/// How many checks produced a judgement. SKIPPED and N/A are not
/// judgements.
pub fn measured_count(checks: &[Check]) -> usize {
    checks.iter().filter(|c| c.status.graded()).count()
}

/// The provisional overall verdict.
///
/// | Condition | Verdict |
/// | --- | --- |
/// | any hard check failed | FAIL |
/// | nothing was graded at all | NO_DATA |
/// | no CLAIM was graded | UNPROBEABLE |
/// | anything warned | WARN |
/// | otherwise | OK |
///
/// UNPROBEABLE names the sanctioned opt-out: an example that wires no probe
/// plugin exits cleanly with a clean log, so `process_exit` and `log_clean`
/// pass and nothing else is claimed. Not wiring IS the opt-out signal (there
/// is no skip list), so the verdict passes the exit gates - but it is kept
/// distinct from OK because two passing rows about the process say nothing
/// about the RUN. The smoke checks still hard-gate: a hang (timeout) or a
/// dirty log makes this FAIL, not UNPROBEABLE.
///
/// OK stays OK-with-coverage: consumers read `measured_count` alongside it,
/// and the reviewer owns the final call either way.
pub fn overall_verdict(checks: &[Check]) -> &'static str {
    let graded_a_claim = checks
        .iter()
        .any(|c| c.status.graded() && capability_of(c.name).is_some());
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        "FAIL"
    } else if measured_count(checks) == 0 {
        "NO_DATA"
    } else if !graded_a_claim {
        "UNPROBEABLE"
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        "WARN"
    } else {
        "OK"
    }
}

/// Why a timeline-fed check has no input. "Not captured" means different
/// things depending on whether probe ARMED the surface: an armed-but-silent
/// run is a WIRING gap in the example, and saying "arm NOVA_PERF_TIMELINE"
/// there sends the reader after the wrong thing. A timeline that WAS captured
/// and would not parse is a third thing again, and the loudest of the three.
fn timeline_skip_detail(artifacts: &RunArtifacts) -> String {
    if let Some(failure) = artifacts.failure_for("timeline.jsonl") {
        return format!(
            "timeline.jsonl is present but unloadable: {}",
            failure.reason
        );
    }
    match artifacts.manifest.as_ref() {
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
    use crate::evaluation::{
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
        // No baseline passed -> the delta does not apply even though the
        // capture exists. The operator's input is missing, not the claim.
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::NotApplicable(NotApplicable::InputNotSupplied("--baseline"))
        );
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Pass);
        assert_eq!(
            check(&checks, "artifacts_loadable").status,
            CheckStatus::Pass
        );
        assert_eq!(overall_verdict(&checks), "OK");
        assert_eq!(measured_count(&checks), 5);
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

    /// The green-and-wrong verdict this refactor exists to kill: an example
    /// that wires nothing exits cleanly with a clean log, so two rows PASS
    /// and four are N/A. The old fold printed OK at `measured 2/6`.
    #[test]
    fn an_example_that_claims_nothing_is_unprobeable_never_ok() {
        let dir = scratch_run_dir();
        for name in ["timeline.jsonl", "frametime.csv"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
        std::fs::write(
            dir.join("probe-contract.json"),
            ProbeContract::default().to_json().to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join("probe-run.json"),
            manifest_ok().to_json().to_string(),
        )
        .unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "process_exit").status, CheckStatus::Pass);
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Pass);
        // The contract and the manifest are themselves loadable artifacts.
        assert_eq!(
            check(&checks, "artifacts_loadable").status,
            CheckStatus::Pass
        );
        assert_eq!(measured_count(&checks), 3);
        assert_eq!(overall_verdict(&checks), "UNPROBEABLE");
        for name in ["run_completed", "reached_playing", "invariants_held"] {
            let c = check(&checks, name);
            assert_eq!(c.status.as_str(), "N/A", "{c:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// N/A is not measured and never counts as held - the two statuses that
    /// mean "no judgement" must both stay out of the coverage figure.
    #[test]
    fn neither_skipped_nor_not_applicable_counts_as_measured() {
        let graded = [CheckStatus::Pass, CheckStatus::Warn, CheckStatus::Fail];
        assert!(graded.iter().all(|s| s.graded()));
        assert!(!CheckStatus::Skipped.graded());
        assert!(
            !CheckStatus::NotApplicable(NotApplicable::NotDeclared(Capability::Timeline)).graded()
        );
    }

    /// Every roster row's name must match the name its evaluator stamps on
    /// the Check, or `capability_of` silently returns None and the fold
    /// stops seeing claims.
    #[test]
    fn the_roster_names_match_the_checks_they_produce() {
        let checks = evaluate_checks(&RunArtifacts::default());
        for ((name, _, _), check) in CHECKS.iter().zip(&checks) {
            assert_eq!(*name, check.name, "roster name drifted from its check");
        }
        assert_eq!(
            capability_of("reached_playing"),
            Some(Capability::Timeline),
            "a capability-backed check must be visible to the fold"
        );
        assert_eq!(capability_of("process_exit"), None);
        assert_eq!(capability_of("not_a_check"), None);
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
        assert_eq!(json["measured"], "5/7");
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
