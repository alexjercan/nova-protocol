//! `artifacts_loadable`: every artifact the run dir held could be read and
//! parsed.
//!
//! This is the row that keeps [`RunArtifacts::load`]'s promise now that it no
//! longer aborts the whole report on one bad file. A corrupt-but-present
//! artifact used to `?`-propagate out of `finish_report`, so `report.html` and
//! `checks.json` were never written - AFTER `clean_out_dir` had deleted the
//! previous ones. The evidence is now isolated per artifact and the failure is
//! reported here instead of deleting the report that would have shown it.
//!
//! [`RunArtifacts::load`]: crate::evaluation::RunArtifacts::load

use super::{Check, CheckStatus, RunArtifacts};

const THRESHOLD: &str = "every present artifact parses";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    if artifacts.failures.is_empty() {
        let status = if artifacts.any_present() {
            CheckStatus::Pass
        } else {
            // Nothing was captured, so nothing was loadable or otherwise.
            // Passing here would turn a zero-evidence dir into a graded run.
            CheckStatus::Skipped
        };
        return Check {
            name: "artifacts_loadable",
            status,
            value: "0 unloadable".into(),
            threshold: THRESHOLD.into(),
            detail: match status {
                CheckStatus::Pass => "every artifact present in the run dir parsed".into(),
                _ => "the run dir held no artifacts to load".into(),
            },
            data: serde_json::json!({ "unloadable": 0 }),
        };
    }
    Check {
        name: "artifacts_loadable",
        status: CheckStatus::Fail,
        value: format!("{} unloadable artifact(s)", artifacts.failures.len()),
        threshold: THRESHOLD.into(),
        detail: format!(
            "first: {}: {}",
            artifacts.failures[0].name, artifacts.failures[0].reason
        ),
        data: serde_json::json!({
            "unloadable": artifacts.failures.len(),
            "artifacts": artifacts.failures.iter().map(|f| serde_json::json!({
                "name": f.name,
                "reason": f.reason,
            })).collect::<Vec<_>>(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use nova_probe::prelude::*;

    use super::*;
    use crate::evaluation::{
        checks::{evaluate_checks, timeline_skip_detail},
        fixtures::*,
    };

    /// The failure this check exists for: a truncated trace still produces a
    /// report, and the run cannot verdict OK on it.
    #[test]
    fn a_corrupt_artifact_fails_the_run_instead_of_killing_the_report() {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("trace.json"), "{\"traceEvents\": [").unwrap();
        let artifacts = RunArtifacts::load(&dir, None).expect("a corrupt artifact still loads");
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "artifacts_loadable");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["unloadable"], 1, "{c:?}");
        assert_eq!(c.data["artifacts"][0]["name"], "trace.json");
        // The other eleven artifacts survive one bad file.
        assert!(artifacts.timeline.is_some(), "timeline was discarded");
        assert!(artifacts.runs.is_some(), "frametime was discarded");
        assert_eq!(
            crate::evaluation::overall_verdict(&checks),
            "FAIL",
            "a run with unreadable evidence must not verdict OK"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A torn timeline is the corrupt artifact that matters most: it is the
    /// input three checks grade on. The claim it feeds must FAIL - the run
    /// armed and declared a recorder - rather than skip as "not captured",
    /// and the skip wording must send the reader at the file, not at an env
    /// var that was set all along.
    #[test]
    fn a_torn_timeline_fails_the_claim_it_feeds_rather_than_skipping_it() {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("timeline.jsonl"), "{\"kind\": \"stat\n").unwrap();
        write_contract(&dir, [Capability::Timeline]);
        write_manifest(&dir, &manifest_ok());

        let artifacts = RunArtifacts::load(&dir, None).expect("a torn timeline still loads");
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "artifacts_loadable");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["artifacts"][0]["name"], "timeline.jsonl", "{c:?}");
        for name in ["reached_playing", "run_completed"] {
            let c = check(&checks, name);
            assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        }
        assert!(
            timeline_skip_detail(&artifacts).contains("unloadable"),
            "the skip wording still blames the arming"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A log probe cannot even decode. `log_clean` has no input, so the run
    /// stays ungraded on its output - and it must say so out loud instead of
    /// reading as a quiet run.
    #[test]
    fn a_non_utf8_log_is_reported_rather_than_read_as_a_clean_run() {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("run.log"), [0x49, 0x4e, 0x46, 0x4f, 0xff, 0x0a]).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).expect("an undecodable log still loads");
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "artifacts_loadable");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["artifacts"][0]["name"], "run.log", "{c:?}");
        assert!(
            artifacts.log.is_none(),
            "an undecodable log is not evidence"
        );
        assert_eq!(
            crate::evaluation::overall_verdict(&checks),
            "FAIL",
            "a run whose log could not be read must not verdict OK"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_healthy_run_passes_and_an_empty_dir_skips() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        assert_eq!(
            check(&evaluate_checks(&artifacts), "artifacts_loadable").status,
            CheckStatus::Pass
        );
        assert_eq!(
            check(
                &evaluate_checks(&RunArtifacts::default()),
                "artifacts_loadable"
            )
            .status,
            CheckStatus::Skipped
        );
    }
}
