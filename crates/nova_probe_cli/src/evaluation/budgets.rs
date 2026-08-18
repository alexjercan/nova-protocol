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

/// Every recorded budget, in the ranked order of the profiling report.
///
/// Numbers come from measured runs on one host; see
/// `tasks/20260818-221027/REPORT.md` for the measurement behind each and for
/// the cases that are deliberately ABSENT from this table because they could
/// not be made repeatable.
pub const BUDGETS: &[FrameBudget] = &[];

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
