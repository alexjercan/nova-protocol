//! `frame_within_budget`: the HARD gate on the worst frame of a recorded case.
//!
//! The one check that fails without a baseline and without a reviewer. It
//! grades `max_ms` - the single slowest frame of the capture window - against
//! the number written down in [`budgets`](crate::evaluation::budgets), because
//! a stutter is a tail and a mean hides it.
//!
//! A row with no recorded budget is N/A, not a pass: most examples are not
//! profiling cases and must not be graded as though they were.

use nova_probe::prelude::*;

use super::{Check, CheckStatus, NotApplicable, RunArtifacts};
use crate::evaluation::prelude::*;

/// The row this check graded, with its budget.
struct Graded<'a> {
    run: &'a PerfRun,
    budget: &'static FrameBudget,
}

/// The rows that carry a recorded budget AND were captured under the
/// conditions it was recorded for.
fn gradeable(runs: &[PerfRun]) -> Vec<Graded<'_>> {
    runs.iter()
        .filter_map(|run| {
            let budget = budget_for(&run.label)?;
            // A dev capture against a release budget, or the lavapipe floor
            // against a GPU budget, is not a comparison. Refuse it here rather
            // than fail a run for being measured somewhere else.
            let comparable =
                run.meta.profile == budget.profile && run.meta.backend == budget.backend;
            comparable.then_some(Graded { run, budget })
        })
        .collect()
}

pub(super) fn evaluate(artifacts: &RunArtifacts) -> Check {
    let row = |status, value: String, detail: String, data| Check {
        name: "frame_within_budget",
        status,
        value,
        threshold: "worst frame <= recorded budget".into(),
        detail,
        data,
    };
    let runs = match artifacts.resolve(Capability::FrameTime, artifacts.runs.as_ref()) {
        Input::Present(runs) => runs,
        Input::NotDeclared(capability) => {
            return row(
                CheckStatus::NotApplicable(NotApplicable::NotDeclared(capability)),
                "not claimed".into(),
                format!(
                    "the example wires no {} - it captures no frames, so there \
                     is no worst frame to hold to a budget",
                    capability.wiring()
                ),
                serde_json::Value::Null,
            )
        }
        Input::NotArmed(capability) => {
            return row(
                CheckStatus::NotApplicable(NotApplicable::NotArmed(capability)),
                "not armed".into(),
                format!(
                    "the example wires {} but this run did not arm the capture",
                    capability.wiring()
                ),
                serde_json::Value::Null,
            )
        }
        Input::ArmedButAbsent(capability) => {
            return row(
                CheckStatus::Fail,
                "armed and silent".into(),
                format!(
                    "the example declares {} and probe armed the capture, but no \
                     frametime.csv was written",
                    capability.wiring()
                ),
                serde_json::Value::Null,
            )
        }
        Input::Unknown(_) => {
            return row(
                CheckStatus::Skipped,
                "no capture".into(),
                "frametime.csv not captured (arm NOVA_PERF)".into(),
                serde_json::Value::Null,
            )
        }
    };

    let graded = gradeable(runs);
    if graded.is_empty() {
        let labels: Vec<&str> = runs.iter().map(|run| run.label.as_str()).collect();
        let recorded = BUDGETS.iter().any(|b| labels.contains(&b.label));
        return row(
            CheckStatus::NotApplicable(NotApplicable::InputNotComparable(if recorded {
                "capture conditions"
            } else {
                "recorded budget"
            })),
            if recorded {
                "budget not comparable".into()
            } else {
                "no budget".into()
            },
            if recorded {
                "a budget is recorded for this label but under another build \
                 profile or graphics backend; an absolute frame budget does \
                 not carry across those"
                    .into()
            } else {
                format!(
                    "no frame budget is recorded for {} - it is not a \
                     profiling case (crates/nova_probe_cli/src/evaluation/budgets.rs)",
                    labels.join(", ")
                )
            },
            serde_json::json!({ "labels": labels }),
        );
    }

    // The worst OVERSHOOT decides the row: a sweep can carry several graded
    // labels, and the run is only as good as its worst case.
    let worst = graded
        .iter()
        .max_by(|a, b| {
            let over = |g: &Graded| g.run.stats.max_ms - g.budget.worst_ms;
            over(a)
                .partial_cmp(&over(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("graded is not empty");
    let stats = &worst.run.stats;
    let budget = worst.budget;
    let data = serde_json::json!({
        "label": worst.run.label,
        "case": budget.case,
        "worst_ms": stats.max_ms,
        "budget_ms": budget.worst_ms,
        "mean_ms": stats.mean_ms,
        "p99_ms": stats.p99_ms,
        "command": budget.command,
        "graded": graded.len(),
    });
    let value = format!(
        "worst frame {:.1} ms of {:.1} ({})",
        stats.max_ms, budget.worst_ms, worst.run.label
    );
    if stats.max_ms > budget.worst_ms {
        return row(
            CheckStatus::Fail,
            value,
            format!(
                "{} blew its recorded budget by {:.1} ms (mean {:.1} ms, p99 \
                 {:.1} ms). The budget was set from {:.1} ms: {}. Re-run with \
                 `{}`.",
                budget.case,
                stats.max_ms - budget.worst_ms,
                stats.mean_ms,
                stats.p99_ms,
                budget.measured_worst_ms,
                budget.headroom,
                budget.command,
            ),
            data,
        );
    }
    row(
        CheckStatus::Pass,
        value,
        format!(
            "{} held its worst frame (mean {:.1} ms, p99 {:.1} ms; budget set \
             from {:.1} ms: {})",
            budget.case, stats.mean_ms, stats.p99_ms, budget.measured_worst_ms, budget.headroom,
        ),
        data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{
        checks::{evaluate_checks, overall_verdict},
        fixtures::*,
    };

    const HEADER: &str = "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,\
         mean_fps,one_pct_low_fps,backend,adapter,resolution,quality,git_sha,host,profile";

    /// One capture row, with `max_ms` and the conditions under the caller's
    /// control - everything else held at a plausible constant.
    fn write_row(dir: &std::path::Path, label: &str, max_ms: f64, profile: &str, backend: &str) {
        std::fs::write(
            dir.join("frametime.csv"),
            format!(
                "{HEADER}\n{label},900,8.0,5.0,{max_ms},7.5,12.0,20.0,{max_ms},125.0,50.0,\
                 {backend},Test Adapter,1280x720,default,abc123,devbox,{profile}\n"
            ),
        )
        .unwrap();
    }

    fn graded_row(dir: &std::path::Path) -> Check {
        write_contract(dir, [Capability::FrameTime]);
        let mut manifest = manifest_ok();
        manifest.armed_fps = true;
        write_manifest(dir, &manifest);
        let artifacts = RunArtifacts::load(dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        check(&checks, "frame_within_budget").clone()
    }

    /// A label nobody recorded a budget for is NOT graded. Most examples are
    /// not profiling cases and a pass here would be a claim about them.
    #[test]
    fn an_unbudgeted_label_is_not_applicable_never_a_pass() {
        let dir = scratch_run_dir();
        write_row(&dir, "some_other_example", 900.0, "dev", "vulkan");
        let check = graded_row(&dir);
        assert_eq!(
            check.status,
            CheckStatus::NotApplicable(NotApplicable::InputNotComparable("recorded budget")),
            "{check:?}"
        );
        assert!(check.detail.contains("budgets.rs"), "{check:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every recorded budget grades a row captured under its own conditions,
    /// and declines one captured under any other. The table's own numbers are
    /// the fixture, so re-measuring a case cannot leave this proving nothing.
    #[test]
    fn a_recorded_budget_grades_only_its_own_capture_conditions() {
        for budget in BUDGETS {
            let dir = scratch_run_dir();

            // Held: the measured worst frame is inside the budget by
            // construction (budgets.rs pins that), so it passes.
            write_row(
                &dir,
                budget.label,
                budget.measured_worst_ms,
                budget.profile,
                budget.backend,
            );
            let check = graded_row(&dir);
            assert_eq!(
                check.status,
                CheckStatus::Pass,
                "{}: {check:?}",
                budget.label
            );

            // Blown: one millisecond past the budget fails.
            write_row(
                &dir,
                budget.label,
                budget.worst_ms + 1.0,
                budget.profile,
                budget.backend,
            );
            let check = graded_row(&dir);
            assert_eq!(
                check.status,
                CheckStatus::Fail,
                "{}: {check:?}",
                budget.label
            );
            assert!(check.detail.contains(budget.command), "{check:?}");

            // The same blown number on the software floor is NOT graded: an
            // absolute budget does not carry across backends.
            write_row(
                &dir,
                budget.label,
                budget.worst_ms + 1.0,
                budget.profile,
                "llvmpipe",
            );
            let check = graded_row(&dir);
            assert_eq!(
                check.status,
                CheckStatus::NotApplicable(NotApplicable::InputNotComparable("capture conditions")),
                "{}: {check:?}",
                budget.label
            );

            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// A budget FAILURE fails the run outright - the difference between this
    /// check and the soft `fps_within_baseline` gate beside it.
    #[test]
    fn a_blown_budget_fails_the_verdict_not_merely_warns_it() {
        let Some(budget) = BUDGETS.first() else {
            return;
        };
        let dir = scratch_run_dir();
        write_row(
            &dir,
            budget.label,
            budget.worst_ms * 10.0,
            budget.profile,
            budget.backend,
        );
        write_contract(&dir, [Capability::FrameTime]);
        let mut manifest = manifest_ok();
        manifest.armed_fps = true;
        write_manifest(&dir, &manifest);
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(overall_verdict(&checks), "FAIL");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An example that captures no frames makes no budget claim.
    #[test]
    fn undeclared_frame_time_is_not_applicable() {
        let dir = scratch_run_dir();
        let _ = std::fs::remove_file(dir.join("frametime.csv"));
        std::fs::write(
            dir.join("probe-contract.json"),
            ProbeContract::of([Capability::Timeline])
                .to_json()
                .to_string(),
        )
        .unwrap();
        write_manifest(&dir, &manifest_ok());
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let check = check(&checks, "frame_within_budget");
        assert_eq!(
            check.status,
            CheckStatus::NotApplicable(NotApplicable::NotDeclared(Capability::FrameTime)),
            "{check:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
