//! `reached_playing`: every harnessed example's smoke contract is "reach
//! Playing and exit without panic".
//!
//! An app that exits cleanly while still Loading (graceful asset failure)
//! must not pass unnoticed.

use super::{timeline_skip_detail, Check, CheckStatus, RunArtifacts};

const THRESHOLD: &str = "a GameStates transition entered Playing";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(timeline) = artifacts.timeline.as_ref() else {
        return Check {
            name: "reached_playing",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: THRESHOLD.into(),
            detail: timeline_skip_detail(artifacts.manifest.as_ref()),
            data: serde_json::Value::Null,
        };
    };

    let entered = timeline.iter().find(|e| {
        e.kind == "state" && e.name == "GameStates" && e.data["entered"].as_str() == Some("Playing")
    });
    match entered {
        Some(entry) => Check {
            name: "reached_playing",
            status: CheckStatus::Pass,
            value: format!("Playing at frame {}", entry.frame),
            threshold: THRESHOLD.into(),
            detail: "the run reached gameplay".into(),
            data: serde_json::json!({ "frame": entry.frame }),
        },
        None => Check {
            name: "reached_playing",
            status: CheckStatus::Fail,
            value: "never entered Playing".into(),
            threshold: THRESHOLD.into(),
            detail: "the app ended while still loading/menu - the smoke contract \
                     (reach Playing) was not met"
                .into(),
            data: serde_json::json!({ "reached": false }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{checks::evaluate_checks, fixtures::*};

    #[test]
    fn reached_playing_fails_when_the_run_never_left_loading() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .filter(|l| !l.contains("\"entered\":\"Playing\""))
            .collect();
        // Keep the file bracket-consistent: drop one entry, patch run_end's
        // count down by one.
        let patched = kept.join("\n").replace("\"entries\":10", "\"entries\":9");
        std::fs::write(&path, patched).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "reached_playing").status, CheckStatus::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_healthy_fixture_names_the_frame_it_reached_playing() {
        let artifacts = RunArtifacts::load(&fixture(), None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "reached_playing");
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.data["frame"].is_number(), "{c:?}");
    }
}
