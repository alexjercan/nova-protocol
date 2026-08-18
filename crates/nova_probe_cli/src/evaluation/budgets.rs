//! The recorded FRAME BUDGETS: what each profiling case is allowed to cost.
//!
//! One entry per case, keyed by the `frametime.csv` row LABEL, because that is
//! the only key both capture paths agree on: a plain `probe run <example>`
//! labels its row with the example name, and a `--scenario <id>` sweep cell
//! labels its row with the scenario id.
//!
//! ## Why a table and not a baseline
//!
//! `fps_within_baseline` needs a previous run to compare against, warns rather
//! than fails, and reads the MEAN. None of that catches the failure this table
//! exists for: a commit that makes one frame in twenty cost 200 ms lands green
//! against a baseline nobody captured, and its mean barely moves. A budget is
//! an absolute number a reviewer wrote down on purpose, and the WORST frame is
//! what it is written against, because every stutter this project has shipped
//! was a tail.
//!
//! ## Why each entry names its host, renderer and profile
//!
//! A flat ms budget applied to every run would permanently fail the software
//! raster floor and the web pass, which is why one was refused before. So an
//! entry records the conditions it was measured under and the check declines
//! to grade a row that does not match them - N/A, never a pass. An unmatched
//! row is an ungraded row, and the report says so.

/// Glob-import surface for the recorded frame budgets.
pub mod prelude {
    pub use super::{budget_for, FrameBudget, BUDGETS};
}

/// One case's recorded budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameBudget {
    /// The `frametime.csv` row label this budget grades.
    pub label: &'static str,
    /// What the case IS, for a reader who was not there when it was measured.
    pub case: &'static str,
    /// The literal command that reproduces it.
    pub command: &'static str,
    /// The worst single frame the case may cost, milliseconds. The gate.
    pub worst_ms: f64,
    /// The measured worst frame the budget was set from, milliseconds.
    pub measured_worst_ms: f64,
    /// The measured mean frame time, milliseconds. Context only - the mean is
    /// not gated, because a mean is what hid every stutter this project has
    /// had.
    pub measured_mean_ms: f64,
    /// Why `worst_ms` sits where it does, in one sentence.
    pub headroom: &'static str,
    /// Build profile of the capture the budget was measured from (`dev` or
    /// `release`). A row from the other profile is not comparable.
    pub profile: &'static str,
    /// wgpu backend the budget was measured on. A row from another backend -
    /// the lavapipe software floor, the web pass - is not comparable.
    pub backend: &'static str,
}

/// Every recorded budget, most expensive case first.
///
/// Numbers come from measured runs on one host; the measurement behind each,
/// and the cases deliberately ABSENT because they could not be made
/// repeatable, are in `tasks/20260818-221027/REPORT.md`.
///
/// Every entry sits at TWICE its measured worst frame. That is not caution, it
/// is what the samples allow: the worst frame of the same case on the same
/// commit varied by up to 2.2x run to run on this host, so a tighter gate
/// would fire on noise and get muted. What these catch is a DOUBLING of the
/// tail, which is the size of the regression this release exists to stop.
pub const BUDGETS: &[FrameBudget] = &[
    FrameBudget {
        label: "stress_torpedoes",
        case: "a thousand torpedoes under guidance at once (engine ceiling, not content)",
        command: "cargo run --features debug probe run stress_torpedoes",
        worst_ms: 1164.4,
        measured_worst_ms: 582.2,
        measured_mean_ms: 131.4,
        headroom: "2x the worst of 3 runs (536-582 ms)",
        profile: "dev",
        backend: "vulkan",
    },
    FrameBudget {
        label: "stress_one_structure",
        case: "one thousand-section hull (engine ceiling, not content)",
        command: "cargo run --features debug probe run stress_one_structure",
        worst_ms: 289.0,
        measured_worst_ms: 144.5,
        measured_mean_ms: 41.6,
        headroom: "2x the worst of 3 runs (132-145 ms)",
        profile: "dev",
        backend: "vulkan",
    },
    FrameBudget {
        label: "stress_many_structures",
        case: "a hundred ten-section hulls (engine ceiling, not content)",
        command: "cargo run --features debug probe run stress_many_structures",
        worst_ms: 289.2,
        measured_worst_ms: 144.6,
        measured_mean_ms: 29.9,
        headroom: "2x the worst of 3 runs (65-145 ms, the widest spread here)",
        profile: "dev",
        backend: "vulkan",
    },
    FrameBudget {
        label: "stress_bullets",
        case: "a thousand turret rounds in flight at once (engine ceiling, not content)",
        command: "cargo run --features debug probe run stress_bullets",
        worst_ms: 148.0,
        measured_worst_ms: 74.0,
        measured_mean_ms: 21.2,
        headroom: "2x the worst of 5 runs (36-74 ms)",
        profile: "dev",
        backend: "vulkan",
    },
    FrameBudget {
        label: "scene_baseline",
        case: "the asteroid_field sandbox as it ships, AT REST - nothing fires in it",
        command: "cargo run --features debug probe run scene_baseline",
        worst_ms: 86.6,
        measured_worst_ms: 43.3,
        measured_mean_ms: 22.5,
        headroom: "2x the worst of 4 runs (40-43 ms) on a QUIET host; a loaded \
                   box read 117 ms on the same binary",
        profile: "dev",
        backend: "vulkan",
    },
];

/// The budget recorded for a capture row label, if there is one.
pub fn budget_for(label: &str) -> Option<&'static FrameBudget> {
    BUDGETS.iter().find(|budget| budget.label == label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_budget_leaves_headroom_over_what_was_measured() {
        for budget in BUDGETS {
            assert!(
                budget.worst_ms > budget.measured_worst_ms,
                "{}: a budget at or under the measured worst frame fails on \
                 the run it was set from",
                budget.label
            );
            assert!(
                !budget.headroom.is_empty(),
                "{}: a budget without a justification is a round number \
                 somebody invented",
                budget.label
            );
            assert!(
                budget.command.contains("probe run"),
                "{}: an entry must carry the literal command that reproduces it",
                budget.label
            );
        }
    }

    #[test]
    fn one_budget_per_label() {
        let mut labels: Vec<&str> = BUDGETS.iter().map(|budget| budget.label).collect();
        labels.sort_unstable();
        let count = labels.len();
        labels.dedup();
        assert_eq!(count, labels.len(), "two budgets share a label");
    }

    #[test]
    fn lookup_is_by_label() {
        for budget in BUDGETS {
            assert_eq!(budget_for(budget.label), Some(budget));
        }
        assert_eq!(budget_for("no_such_case"), None);
    }
}
