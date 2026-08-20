//! `capture_simulated`: a capture window the game REFUSED is a failure, not a
//! statistic. Every reason the capture can refuse for lands here.
//!
//! The condition is detected in the game, by the capture itself - not inferred
//! here from the numbers. It has to be: each of these faults produces a mean
//! and a median in exactly the shape the repeat gate admits. One 4v4 window
//! spent 555 of its 900 frames on a paused result screen and reported a
//! believable 93 ms mean.
//!
//! An aborted capture writes no CSV row, so the row it would have contaminated
//! is simply absent; this check is what stops that absence from reading as a
//! quiet, slightly smaller set.
//!
//! The NAME is narrower than the check: it predates the other reasons and
//! renaming it would break every `checks.json` a stored baseline holds. Read
//! the `reason` field, never the name.

use nova_probe::prelude::*;

use super::{Check, CheckStatus, NotApplicable, RunArtifacts};
use crate::evaluation::prelude::*;

const THRESHOLD: &str = "every captured frame simulated";

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let row = |status, value: String, detail: String, data| Check {
        name: "capture_simulated",
        status,
        value,
        threshold: THRESHOLD.into(),
        detail,
        data,
    };

    let aborts: Vec<CaptureAbort> = artifacts
        .log
        .as_ref()
        .map(|log| log.lines().filter_map(parse_capture_abort_line).collect())
        .unwrap_or_default();

    if let Some(first) = aborts.first() {
        return row(
            CheckStatus::Fail,
            format!("{} capture(s) refused", aborts.len()),
            format!(
                "{} was refused {} frame(s) into its {} phase (reason: {}) - it wrote \
                 no stats, so the row it would have contributed is missing on purpose; \
                 the run log carries the reason in full",
                first.label, first.frame, first.phase, first.reason
            ),
            serde_json::json!({
                "aborted": aborts.len(),
                "captures": aborts.iter().map(|abort| serde_json::json!({
                    "label": abort.label,
                    "reason": abort.reason,
                    "phase": abort.phase,
                    "frame": abort.frame,
                    "warmup_frames": abort.window.0,
                    "capture_frames": abort.window.1,
                })).collect::<Vec<_>>(),
            }),
        );
    }

    // No abort line. Whether that is a PASS depends on whether anything was
    // captured at all: a run with no capture has nothing to say either way.
    match artifacts.resolve(Capability::FrameTime, artifacts.runs.as_ref()) {
        Input::Present(runs) => row(
            CheckStatus::Pass,
            format!("{} capture(s) refused: 0", runs.len()),
            "no capture refused its window".into(),
            serde_json::json!({ "aborted": 0, "captures": [] }),
        ),
        Input::NotDeclared(capability) => row(
            CheckStatus::NotApplicable(NotApplicable::NotDeclared(capability)),
            "not claimed".into(),
            format!(
                "the example wires no {} - there is no capture window to refuse",
                capability.wiring()
            ),
            serde_json::Value::Null,
        ),
        Input::NotArmed(capability) => row(
            CheckStatus::NotApplicable(NotApplicable::NotArmed(capability)),
            "not armed".into(),
            format!(
                "the example wires {} but this run did not arm the capture",
                capability.wiring()
            ),
            serde_json::Value::Null,
        ),
        // Armed and silent is `fps_within_baseline`'s failure to report, and
        // this check has no second opinion about it: the log carries no abort
        // line, so nothing here saw a refusal.
        Input::ArmedButAbsent(_) | Input::Unknown(_) => row(
            CheckStatus::Skipped,
            "no capture".into(),
            "no capture window ran, so none could be refused".into(),
            serde_json::Value::Null,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{checks::evaluate_checks, fixtures::*};

    /// Scan `log` as if it were the run's captured output.
    fn scan(log: &str) -> Check {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("run.log"), log).unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let c = check(&evaluate_checks(&artifacts), "capture_simulated").clone();
        let _ = std::fs::remove_dir_all(&dir);
        c
    }

    /// The whole point: the run FAILS and the row names the capture, the
    /// phase and the frame, so nobody has to read a mean to find out.
    #[test]
    fn a_capture_that_met_a_stopped_simulation_fails_and_names_it() {
        let c = scan(
            "INFO nova perf: armed\n\
             ERROR nova perf: label=wfc_arena#3 ABORTED reason=simulation_stopped \
             phase=capture frame=345 warmup=180 frames=900 - Time<Virtual> was stopped\n",
        );
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["aborted"], 1);
        assert_eq!(c.data["captures"][0]["label"], "wfc_arena#3");
        assert_eq!(c.data["captures"][0]["frame"], 345);
        assert!(c.detail.contains("wfc_arena#3"), "{c:?}");
    }

    /// Every capture of a repeat set is counted, not just the first.
    #[test]
    fn every_refused_capture_of_a_set_is_counted() {
        let c = scan(
            "ERROR nova perf: label=a#1 ABORTED reason=simulation_stopped phase=capture \
             frame=10 warmup=180 frames=900\n\
             ERROR nova perf: label=a#5 ABORTED reason=simulation_stopped phase=warmup \
             frame=3 warmup=180 frames=900\n",
        );
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["aborted"], 2);
        assert_eq!(c.data["captures"][1]["phase"], "warmup");
    }

    /// A run with a clean capture passes and says how many windows held.
    #[test]
    fn a_clean_capture_passes() {
        let dir = scratch_run_dir();
        std::fs::write(dir.join("run.log"), "INFO nova perf: wrote stats\n").unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let c = check(&evaluate_checks(&artifacts), "capture_simulated").clone();
        assert_eq!(c.status, CheckStatus::Pass, "{c:?}");
        assert_eq!(c.data["aborted"], 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
