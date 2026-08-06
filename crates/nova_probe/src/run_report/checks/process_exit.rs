//! `process_exit`: the children's actual outcomes, from the manifest.
//!
//! ALL primary passes count (a sweep runs one clean pass per matrix cell; the
//! web platform runs a web pass), and the worst outcome wins. For harnessed
//! examples this is real correctness evidence on its own: their autopilot
//! assertions panic on failure. SKIPPED only for foreign dirs. The
//! profiled/samply passes are auxiliary and excluded - their failures degrade
//! to missing artifacts by design.

use super::{Check, CheckStatus, RunArtifacts};
use crate::run_report::manifest::PassRecord;

const THRESHOLD: &str = "every primary pass exits success, untimed";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(manifest) = artifacts.manifest.as_ref() else {
        return Check {
            name: "process_exit",
            status: CheckStatus::Skipped,
            value: "no manifest".into(),
            threshold: THRESHOLD.into(),
            detail: "no probe-run.json - this dir was not produced by probe run".into(),
            data: serde_json::Value::Null,
        };
    };

    let primary: Vec<&PassRecord> = manifest
        .passes
        .iter()
        .filter(|p| p.name.starts_with("clean") || p.name == "web" || p.name == "fps")
        .collect();
    let failed: Vec<&&PassRecord> = primary
        .iter()
        .filter(|p| !p.success || p.timed_out)
        .collect();
    let data = serde_json::json!({
        "primary_passes": primary.len(),
        "failed": failed.iter().map(|p| serde_json::json!({
            "name": p.name, "timed_out": p.timed_out,
        })).collect::<Vec<_>>(),
    });

    if primary.is_empty() {
        Check {
            name: "process_exit",
            status: CheckStatus::Skipped,
            value: "no primary passes recorded".into(),
            threshold: THRESHOLD.into(),
            detail: "the manifest lists no clean/web passes".into(),
            data,
        }
    } else if failed.is_empty() {
        Check {
            name: "process_exit",
            status: CheckStatus::Pass,
            value: format!("{} pass(es), all clean exits", primary.len()),
            threshold: THRESHOLD.into(),
            detail: "every run's own assertions held".into(),
            data,
        }
    } else {
        let names: Vec<String> = failed
            .iter()
            .map(|p| {
                format!(
                    "{}{}",
                    p.name,
                    if p.timed_out { " (timed out)" } else { "" }
                )
            })
            .collect();
        Check {
            name: "process_exit",
            status: CheckStatus::Fail,
            value: format!("{}/{} pass(es) failed", failed.len(), primary.len()),
            threshold: THRESHOLD.into(),
            detail: format!("failed: {} - read the matching log", names.join(", ")),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{
        checks::{evaluate_checks, overall_verdict},
        fixtures::*,
        manifest::RunManifest,
    };

    #[test]
    fn a_timed_out_primary_pass_fails_and_names_itself() {
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000123,
            git_sha: "abc123".into(),
            full_git_sha: "abc123def".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: true,
            fps_skipped: Some("narrative scenario".into()),
            passes: vec![
                PassRecord {
                    name: "clean".into(),
                    success: false,
                    timed_out: true,
                },
                // Auxiliary: never counted, even when it succeeds.
                PassRecord {
                    name: "profiled".into(),
                    success: true,
                    timed_out: false,
                },
            ],
        };
        let artifacts = RunArtifacts {
            manifest: Some(manifest),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "process_exit");
        assert_eq!(c.status, CheckStatus::Fail);
        // The all-passes shape: the count in value, the names + timeout
        // markers in detail/data.
        assert!(c.value.contains("1/1 pass(es) failed"), "{c:?}");
        assert!(c.detail.contains("clean (timed out)"), "{c:?}");
        assert_eq!(c.data["failed"][0]["timed_out"], true);
        // ...and the verdict is FAIL even though everything else skipped
        // (a hung run must produce a failing report).
        assert_eq!(overall_verdict(&checks), "FAIL");
    }

    #[test]
    fn a_failed_non_timeout_exit_fails_too() {
        let artifacts = RunArtifacts {
            manifest: Some(RunManifest {
                passes: vec![PassRecord {
                    name: "clean".into(),
                    success: false,
                    timed_out: false,
                }],
                ..manifest_ok()
            }),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "process_exit").status, CheckStatus::Fail);
    }

    #[test]
    fn a_foreign_dir_skips_rather_than_passing() {
        let checks = evaluate_checks(&RunArtifacts::default());
        let c = check(&checks, "process_exit");
        assert_eq!(c.status, CheckStatus::Skipped);
        assert!(c.detail.contains("probe-run.json"), "{c:?}");
    }
}
