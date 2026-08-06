//! `run_completed`: the timeline must CLOSE, and the bracket's own entry
//! count must match what is actually on disk.
//!
//! A swallowed write (full disk) otherwise goes unnoticed. Flush-per-entry
//! means a panicked/killed run leaves a bracket-less file: that IS the crash
//! signal, by design.

use super::{timeline_skip_detail, Check, CheckStatus, RunArtifacts};

const THRESHOLD: &str = "run_end present + AppExit Success + entry count consistent";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(timeline) = artifacts.timeline.as_ref() else {
        return Check {
            name: "run_completed",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: THRESHOLD.into(),
            detail: timeline_skip_detail(artifacts.manifest.as_ref()),
            data: serde_json::Value::Null,
        };
    };

    match timeline.iter().rev().find(|e| e.kind == "run_end") {
        Some(end) if end.data["exit"].as_str().unwrap_or("").contains("Success") => {
            let written = end.data["entries"].as_u64().unwrap_or(0);
            let on_disk = (timeline.len() as u64).saturating_sub(1);
            if written != on_disk {
                Check {
                    name: "run_completed",
                    status: CheckStatus::Fail,
                    value: format!("{written} written vs {on_disk} on disk"),
                    threshold: THRESHOLD.into(),
                    detail: "the recorder wrote entries the file does not hold (full \
                             disk / IO errors were warned but swallowed)"
                        .into(),
                    data: serde_json::json!({ "written": written, "on_disk": on_disk }),
                }
            } else {
                Check {
                    name: "run_completed",
                    status: CheckStatus::Pass,
                    value: format!("run_end at frame {}", end.frame),
                    threshold: THRESHOLD.into(),
                    detail: "the run closed its bracket cleanly".into(),
                    data: serde_json::json!({ "end_frame": end.frame, "entries": written }),
                }
            }
        }
        Some(end) => Check {
            name: "run_completed",
            status: CheckStatus::Fail,
            value: format!("exit: {}", end.data["exit"]),
            threshold: THRESHOLD.into(),
            detail: "the run ended with a non-success exit".into(),
            data: serde_json::json!({ "exit": end.data["exit"] }),
        },
        None => Check {
            name: "run_completed",
            status: CheckStatus::Fail,
            value: "timeline truncated (no run_end)".into(),
            threshold: THRESHOLD.into(),
            detail: "flush-per-entry means truncation = the run died mid-flight".into(),
            data: serde_json::json!({ "truncated": true }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{
        checks::{evaluate_checks, measured_count, overall_verdict},
        fixtures::*,
    };

    #[test]
    fn truncated_timeline_fails_run_completed() {
        let dir = scratch_run_dir();
        // Drop the run_end line: flush-per-entry semantics say truncation
        // is the crash signal.
        let path = dir.join("timeline.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .filter(|l| !l.contains("\"run_end\""))
            .collect();
        std::fs::write(&path, kept.join("\n")).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.value.contains("truncated"), "{c:?}");
        assert_eq!(overall_verdict(&checks), "FAIL");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn swallowed_writes_fail_the_entry_cross_check() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        // Claim more entries than the file holds: ENOSPC's signature.
        let contents = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"entries\":10", "\"entries\":14");
        std::fs::write(&path, contents).unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.value.contains("14 written vs 10 on disk"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn armed_but_unwired_skip_details_name_the_wiring_not_the_env() {
        // The live repro's misdirection: probe DID arm the env; the detail
        // must say "not wired", not "arm NOVA_PERF_TIMELINE".
        let artifacts = RunArtifacts {
            manifest: Some(manifest_ok()),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Skipped);
        assert!(c.detail.contains("not wired with"), "{}", c.detail);
        assert!(c.detail.contains("controller_section"), "{}", c.detail);
        // And the verdict is OK-with-coverage (process_exit measured PASS),
        // not NO_DATA and not the old evidence-free OK.
        assert_eq!(overall_verdict(&checks), "OK");
        assert_eq!(measured_count(&checks), 1);
    }

    #[test]
    fn an_unarmed_run_is_told_to_arm_the_recorder() {
        let checks = evaluate_checks(&RunArtifacts::default());
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Skipped);
        assert!(c.detail.contains("NOVA_PERF_TIMELINE"), "{}", c.detail);
    }
}
