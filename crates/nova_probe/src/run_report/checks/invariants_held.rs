//! `invariants_held`: the engine-guaranteed bounds the run asserted every
//! frame.
//!
//! The summary entry carries the tally; per-name counts ride in detail AND
//! data - a stuck entity violates every frame, so a raw total says nothing
//! about how many things are wrong.

use std::collections::BTreeMap;

use super::{timeline_skip_detail, Check, CheckStatus, NotApplicable, RunArtifacts};
use crate::{contract::Capability, recorder::TimelineEvent, run_report::artifacts::Input};

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
    let no_input = |status, value: &str, detail: String| Check {
        name: "invariants_held",
        status,
        value: value.into(),
        threshold: THRESHOLD.into(),
        detail,
        data: serde_json::Value::Null,
    };
    // The invariant evidence rides IN the timeline, so the artifact this
    // capability owes is not the file - it is invariant entries inside it. A
    // timeline with none is exactly as silent as no timeline at all.
    let evidence = artifacts.timeline.as_ref().filter(|timeline| {
        timeline
            .iter()
            .any(|e| e.kind == "invariant" || e.kind == "invariant_summary")
    });
    let timeline = match artifacts.resolve(Capability::Invariants, evidence) {
        Input::Present(timeline) => timeline,
        Input::NotDeclared(capability) => {
            return no_input(
                CheckStatus::NotApplicable(NotApplicable::NotDeclared(capability)),
                "not claimed",
                format!(
                    "the example wires no {} - it asserts no engine-guaranteed \
                     bounds, so there are none to hold",
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
        // Claimed, armed, and no invariant entry in the timeline: the checks
        // never ran. A coverage gap, not a clean bill of health.
        Input::ArmedButAbsent(capability) => {
            return no_input(
                CheckStatus::Fail,
                "no invariant entries",
                format!(
                    "the example declares {} and probe armed it, but the run \
                     recorded no invariant entries - nothing was checked",
                    capability.wiring()
                ),
            )
        }
        Input::Unknown(_) => {
            return no_input(
                CheckStatus::Skipped,
                "no invariant entries",
                // No contract to resolve against, so the manifest's armed
                // flag is the only clue - the pre-contract wording, kept.
                match artifacts.manifest.as_ref() {
                    Some(m) if m.armed_invariants => format!(
                        "probe armed the checks but {} is not wired with \
                         nova_probe::nova_invariants()",
                        m.example
                    ),
                    _ if artifacts.timeline.is_none() => {
                        timeline_skip_detail(artifacts.manifest.as_ref())
                    }
                    _ => "invariants not armed (arm NOVA_PERF_INVARIANTS)".into(),
                },
            );
        }
    };

    let summary = timeline
        .iter()
        .rev()
        .find(|e| e.kind == "invariant_summary");
    let by_name = violations_by_name(timeline);

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
    use crate::{
        contract::ProbeContract,
        run_report::{checks::evaluate_checks, fixtures::*},
    };

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

    /// Declared, armed, and the timeline holds not one invariant entry: the
    /// checks never ran, which is a gap and not a clean bill of health.
    #[test]
    fn an_armed_and_silent_invariants_claim_fails_instead_of_skipping() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .filter(|l| !l.contains("\"invariant"))
            .collect();
        std::fs::write(&path, kept.join("\n")).unwrap();
        std::fs::write(
            dir.join("probe-contract.json"),
            ProbeContract::of([Capability::Invariants])
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
        let c = check(&checks, "invariants_held");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert!(c.detail.contains("nova_invariants()"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An example that asserts no bounds is not judged on them - and the row
    /// is N/A, so it never counts toward coverage.
    #[test]
    fn an_undeclared_invariants_claim_is_not_applicable() {
        let dir = scratch_run_dir();
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
        let c = check(&checks, "invariants_held");
        assert_eq!(
            c.status,
            CheckStatus::NotApplicable(NotApplicable::NotDeclared(Capability::Invariants)),
            "{c:?}"
        );
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
