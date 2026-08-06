//! `invariants_held`: the engine-guaranteed bounds the run asserted every
//! frame.
//!
//! The summary entry carries the tally; per-name counts ride in detail AND
//! data - a stuck entity violates every frame, so a raw total says nothing
//! about how many things are wrong.

use std::collections::BTreeMap;

use super::{timeline_skip_detail, Check, CheckStatus, RunArtifacts};
use crate::recorder::TimelineEvent;

const THRESHOLD: &str = "0 violations";

/// Count invariant violations per name off the timeline (per-name counts,
/// not raw totals: one stuck entity violates every frame).
pub(in crate::run_report) fn violations_by_name(
    timeline: &[TimelineEvent],
) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for entry in timeline.iter().filter(|e| e.kind == "invariant") {
        *counts.entry(entry.name.clone()).or_insert(0) += 1;
    }
    counts
}

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let Some(timeline) = artifacts.timeline.as_ref() else {
        return Check {
            name: "invariants_held",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: THRESHOLD.into(),
            detail: timeline_skip_detail(artifacts.manifest.as_ref()),
            data: serde_json::Value::Null,
        };
    };

    let summary = timeline
        .iter()
        .rev()
        .find(|e| e.kind == "invariant_summary");
    let by_name = violations_by_name(timeline);

    // No summary AND no violation entries: the checks never ran at all.
    // That is a coverage gap, not a clean bill of health.
    if summary.is_none() && by_name.is_empty() {
        return Check {
            name: "invariants_held",
            status: CheckStatus::Skipped,
            value: "no invariant entries".into(),
            threshold: THRESHOLD.into(),
            detail: match artifacts.manifest.as_ref() {
                Some(m) if m.armed_invariants => format!(
                    "probe armed the checks but {} is not wired with \
                     nova_probe::nova_invariants()",
                    m.example
                ),
                _ => "invariants not armed (arm NOVA_PERF_INVARIANTS)".into(),
            },
            data: serde_json::Value::Null,
        };
    }

    let violations = summary
        .map(|s| s.data["violations"].as_u64().unwrap_or(0))
        .unwrap_or_else(|| by_name.values().sum());
    let checks_run = summary
        .map(|s| s.data["checks"].as_u64().unwrap_or(0))
        .unwrap_or(0);
    let counts = serde_json::json!({
        "violations": violations,
        "checked_frames": checks_run,
        "by_name": by_name,
    });

    if violations == 0 {
        Check {
            name: "invariants_held",
            status: CheckStatus::Pass,
            value: format!("0 violations over {checks_run} checked frames"),
            threshold: THRESHOLD.into(),
            detail: "every engine-guaranteed bound held".into(),
            data: counts,
        }
    } else {
        let names: Vec<String> = by_name
            .iter()
            .map(|(name, n)| format!("{name} x{n}"))
            .collect();
        Check {
            name: "invariants_held",
            status: CheckStatus::Fail,
            value: format!("{violations} violation entries"),
            threshold: THRESHOLD.into(),
            detail: format!(
                "by name (a persisting violation repeats per frame): {}",
                names.join(", ")
            ),
            data: counts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_report::{checks::evaluate_checks, fixtures::*};

    #[test]
    fn violations_fail_invariants_with_per_name_counts() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        let mut contents = std::fs::read_to_string(&path).unwrap();
        // Two violations of one name + a summary reporting them: the check
        // fails and the detail carries the per-name count.
        let violation = r#"{"t_real":3.0,"frame":90,"scenario_elapsed":null,"kind":"invariant","name":"health_bounds","data":{"current":-1.0}}"#;
        // Anchor on the SUMMARY line's unique frame, not the shared t_real
        // prefix (run_end shares t_real=4.0 - the first version doubled the
        // insertion and planted x4).
        contents = contents.replace(
            "{\"t_real\":4.0,\"frame\":118",
            &format!("{violation}\n{violation}\n{{\"t_real\":4.0,\"frame\":118"),
        );
        contents = contents.replace(
            "\"data\":{\"checks\":120,\"violations\":0}",
            "\"data\":{\"checks\":120,\"violations\":2}",
        );
        std::fs::write(&path, contents).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "invariants_held");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("health_bounds x2"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn armed_but_unwired_invariants_skip_naming_the_wiring() {
        let artifacts = RunArtifacts {
            manifest: Some(manifest_ok()),
            timeline: Some(Vec::new()),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "invariants_held");
        assert_eq!(c.status, CheckStatus::Skipped);
        assert!(c.detail.contains("nova_invariants()"), "{c:?}");
    }
}
