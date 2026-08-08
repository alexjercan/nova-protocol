//! `fps_within_baseline`: a soft gate on REGRESSIONS only.
//!
//! An improvement is a PASS with the delta noted, never a warning - an
//! automated caller must not read a speedup as a regression flag.

use nova_probe::prelude::*;

use super::{Check, CheckStatus, NotApplicable, RunArtifacts};
use crate::evaluation::prelude::*;

/// Soft FPS gate: the worst same-label mean-frame-time delta against the
/// baseline may move this many percent before the check turns WARN. One
/// tunable on purpose; frame numbers on a shared host are noisy, so this is
/// a flag for the reviewer, never a hard failure.
pub const FPS_WARN_THRESHOLD_PCT: f64 = 10.0;

fn threshold() -> String {
    format!("regression <= {FPS_WARN_THRESHOLD_PCT}%")
}

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let no_input = |status, value: &str, detail: String| Check {
        name: "fps_within_baseline",
        status,
        value: value.into(),
        threshold: threshold(),
        detail,
        data: serde_json::Value::Null,
    };
    let runs = match artifacts.resolve(Capability::FrameTime, artifacts.runs.as_ref()) {
        Input::Present(runs) => runs,
        Input::NotDeclared(capability) => {
            return no_input(
                CheckStatus::NotApplicable(NotApplicable::NotDeclared(capability)),
                "not claimed",
                format!(
                    "the example wires no {} - it makes no frame-cost assertion, \
                     so there is nothing to compare",
                    capability.wiring()
                ),
            )
        }
        Input::NotArmed(capability) => {
            return no_input(
                CheckStatus::NotApplicable(NotApplicable::NotArmed(capability)),
                "not armed",
                format!(
                    "the example wires {} but this run did not arm the capture \
                     (pass --fps)",
                    capability.wiring()
                ),
            )
        }
        Input::ArmedButAbsent(capability) => {
            return no_input(
                CheckStatus::Fail,
                "armed and silent",
                format!(
                    "the example declares {} and probe armed the capture, but no \
                     frametime.csv was written",
                    capability.wiring()
                ),
            )
        }
        Input::Unknown(_) => {
            return no_input(
                CheckStatus::Skipped,
                "no capture",
                "frametime.csv not captured (arm NOVA_PERF)".into(),
            )
        }
    };
    // The operator's half: a baseline is an argument, not a capability.
    let Some(baseline) = &artifacts.baseline else {
        return no_input(
            CheckStatus::NotApplicable(NotApplicable::InputNotSupplied("--baseline")),
            "no baseline",
            "a delta needs a baseline capture: pass --baseline <dir>".into(),
        );
    };

    let mut worst_regression: Option<(String, f64)> = None;
    let mut best_note: Option<(String, f64)> = None;
    let mut matched = 0;
    for run in runs {
        if let Some(base) = baseline.iter().find(|b| b.label == run.label) {
            if base.stats.mean_ms > 0.0 {
                matched += 1;
                let delta = (run.stats.mean_ms - base.stats.mean_ms) / base.stats.mean_ms * 100.0;
                if delta > 0.0 {
                    if worst_regression.as_ref().is_none_or(|(_, w)| delta > *w) {
                        worst_regression = Some((run.label.clone(), delta));
                    }
                } else if best_note.as_ref().is_none_or(|(_, b)| delta < *b) {
                    best_note = Some((run.label.clone(), delta));
                }
            }
        }
    }

    if matched == 0 {
        return no_input(
            CheckStatus::NotApplicable(NotApplicable::InputNotComparable("baseline labels")),
            "no matching labels",
            "baseline shares no run labels with this capture (baselines are \
             only valid probe-run-vs-probe-run or sweep-vs-sweep)"
                .into(),
        );
    }

    match worst_regression {
        Some((label, delta)) if delta > FPS_WARN_THRESHOLD_PCT => Check {
            name: "fps_within_baseline",
            status: CheckStatus::Warn,
            value: format!("worst {label}: +{delta:.1}%"),
            threshold: threshold(),
            detail: "soft gate: frame numbers are host-noisy; reviewer judges \
                     (was the host quiet? is the delta consistent?)"
                .into(),
            data: serde_json::json!({ "label": label, "delta_pct": delta }),
        },
        Some((label, delta)) => Check {
            name: "fps_within_baseline",
            status: CheckStatus::Pass,
            value: format!("worst {label}: +{delta:.1}%"),
            threshold: threshold(),
            detail: "worst regression within the soft gate".into(),
            data: serde_json::json!({ "label": label, "delta_pct": delta }),
        },
        None => {
            let (label, delta) = best_note.expect("matched > 0 with no regressions");
            Check {
                name: "fps_within_baseline",
                status: CheckStatus::Pass,
                value: format!("improved; best {label}: {delta:.1}%"),
                threshold: threshold(),
                detail: "no label regressed against the baseline".into(),
                data: serde_json::json!({ "label": label, "delta_pct": delta }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{
        checks::{evaluate_checks, overall_verdict},
        fixtures::*,
    };

    const HEADER: &str =
        "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps";

    fn write_csv(dir: &std::path::Path, row: &str) {
        std::fs::write(dir.join("frametime.csv"), format!("{HEADER}\n{row}\n")).unwrap();
    }

    /// Baseline mean 100 ms, so a row's mean_ms IS its delta in percent.
    fn baseline_dir() -> std::path::PathBuf {
        let dir = scratch_run_dir();
        write_csv(
            &dir,
            "scene-high,100,100.0,90.0,120.0,99.0,110.0,115.0,120.0,10.0,8.7",
        );
        dir
    }

    /// An example that wires no frame-time capture makes no frame-cost
    /// assertion: the row is N/A and names the call that would make one.
    #[test]
    fn an_undeclared_frame_time_claim_is_not_applicable() {
        let dir = scratch_run_dir();
        let _ = std::fs::remove_file(dir.join("frametime.csv"));
        std::fs::write(
            dir.join("probe-contract.json"),
            ProbeContract::of([Capability::Timeline])
                .to_json()
                .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join("probe-run.json"),
            manifest_ok().to_json().to_string(),
        )
        .unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(
            c.status,
            CheckStatus::NotApplicable(NotApplicable::NotDeclared(Capability::FrameTime)),
            "{c:?}"
        );
        assert!(c.detail.contains("nova_frametime()"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fps_gate_warns_beyond_threshold_and_passes_within() {
        let base_dir = baseline_dir();
        let dir = scratch_run_dir();

        // +5% is within the soft gate.
        write_csv(
            &dir,
            "scene-high,100,105.0,90.0,120.0,99.0,110.0,115.0,120.0,9.5,8.7",
        );
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::Pass
        );

        // +25% warns.
        write_csv(
            &dir,
            "scene-high,100,125.0,90.0,150.0,120.0,140.0,145.0,150.0,8.0,6.9",
        );
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(c.status, CheckStatus::Warn, "{c:?}");
        assert!(c.value.contains("+25.0%"), "{c:?}");
        // A WARN alone never fails the run (soft gate).
        assert_eq!(overall_verdict(&checks), "WARN");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn an_improvement_passes_and_is_never_reported_as_a_regression() {
        let base_dir = baseline_dir();
        let dir = scratch_run_dir();
        // 50% FASTER: must PASS with the improvement noted, never WARN.
        write_csv(
            &dir,
            "scene-high,100,50.0,45.0,60.0,49.0,55.0,58.0,60.0,20.0,17.2",
        );
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(c.status, CheckStatus::Pass, "{c:?}");
        assert!(c.value.contains("improved"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn a_baseline_sharing_no_labels_is_not_applicable_rather_than_passing() {
        let base_dir = baseline_dir();
        let dir = scratch_run_dir();
        write_csv(
            &dir,
            "other-scene,100,50.0,45.0,60.0,49.0,55.0,58.0,60.0,20.0,17.2",
        );
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(
            c.status,
            CheckStatus::NotApplicable(NotApplicable::InputNotComparable("baseline labels")),
            "{c:?}"
        );
        assert!(c.value.contains("no matching labels"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&base_dir);
    }
}
