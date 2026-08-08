//! `reached_playing`: every harnessed example's smoke contract is "reach
//! Playing and exit without panic".
//!
//! An app that exits cleanly while still Loading (graceful asset failure)
//! must not pass unnoticed.

use nova_probe::prelude::*;

use super::{timeline_skip_detail, Check, CheckStatus, NotApplicable, RunArtifacts};
use crate::evaluation::prelude::*;

const THRESHOLD: &str = "a GameStates transition entered Playing";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let no_input = |status, value: &str, detail: String| Check {
        name: "reached_playing",
        status,
        value: value.into(),
        threshold: THRESHOLD.into(),
        detail,
        data: serde_json::Value::Null,
    };
    let timeline = match artifacts.resolve(Capability::Timeline, artifacts.timeline.as_ref()) {
        Input::Present(timeline) => timeline,
        Input::NotDeclared(capability) => {
            return no_input(
                CheckStatus::NotApplicable(NotApplicable::NotDeclared(capability)),
                "not claimed",
                format!(
                    "the example wires no {} - it makes no timeline claim, so \
                     reaching Playing is not observable",
                    capability.wiring()
                ),
            )
        }
        Input::NotArmed(capability) => {
            return no_input(
                CheckStatus::NotApplicable(NotApplicable::NotArmed(capability)),
                "not armed",
                format!(
                    "the example wires {} but this run did not arm it (see the \
                     manifest's armed flags)",
                    capability.wiring()
                ),
            )
        }
        // Claimed it, was armed for it, wrote nothing: the one state the
        // contract turns from a shrug into a failure.
        Input::ArmedButAbsent(capability) => {
            return no_input(
                CheckStatus::Fail,
                "armed and silent",
                format!(
                    "the example declares {} and probe armed it, but no \
                     timeline.jsonl was written - the run recorded nothing",
                    capability.wiring()
                ),
            )
        }
        Input::Unknown(_) => {
            return no_input(
                CheckStatus::Skipped,
                "no timeline",
                timeline_skip_detail(artifacts),
            )
        }
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
    use crate::evaluation::{checks::evaluate_checks, fixtures::*, manifest::RunManifest};

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

    /// The green-and-wrong case the contract exists to kill: probe armed the
    /// recorder, the example claims one, and nothing was written.
    #[test]
    fn an_armed_and_silent_timeline_fails_instead_of_skipping() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("timeline.jsonl")).unwrap();
        write_contract(&dir, [Capability::Timeline]);
        write_manifest(&dir, &manifest_ok());

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "reached_playing");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert!(c.detail.contains("nova_timeline()"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An example that wires no recorder is not judged on one - and the row
    /// names the call it would take to make the claim.
    #[test]
    fn an_undeclared_timeline_is_not_applicable_and_names_the_missing_wiring() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("timeline.jsonl")).unwrap();
        write_contract(&dir, [Capability::FrameTime]);
        write_manifest(&dir, &manifest_ok());

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "reached_playing");
        assert_eq!(
            c.status,
            CheckStatus::NotApplicable(NotApplicable::NotDeclared(Capability::Timeline)),
            "{c:?}"
        );
        assert!(c.detail.contains("nova_timeline()"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A declared capability this run did not arm is not a gap either. Sweep
    /// cells deliberately strip the recorder surfaces.
    #[test]
    fn a_declared_but_unarmed_timeline_is_not_applicable_rather_than_failing() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("timeline.jsonl")).unwrap();
        write_contract(&dir, [Capability::Timeline]);
        let manifest = RunManifest {
            armed_timeline: false,
            ..manifest_ok()
        };
        write_manifest(&dir, &manifest);

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "reached_playing");
        assert_eq!(
            c.status,
            CheckStatus::NotApplicable(NotApplicable::NotArmed(Capability::Timeline)),
            "{c:?}"
        );
        assert_eq!(c.value, "not armed", "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A run dir that predates the contract keeps its old reading: absent is
    /// unknowable, never "declares nothing".
    #[test]
    fn a_dir_without_a_contract_still_skips_not_fails() {
        let dir = scratch_run_dir();
        std::fs::remove_file(dir.join("timeline.jsonl")).unwrap();
        write_manifest(&dir, &manifest_ok());

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(
            check(&checks, "reached_playing").status,
            CheckStatus::Skipped
        );
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
