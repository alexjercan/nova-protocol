//! The unified run report: one run directory in, `report.html` +
//! `checks.json` out - the assembly point of the run-harness.
//!
//! A run directory holds whatever artifacts a run produced, each OPTIONAL:
//!
//! - `timeline.jsonl` - the run-timeline recorder's stream (states, scenario
//!   events, variables, markers, invariant entries, run bracket);
//! - `frametime.csv` - the clean pass's FPS stats (schema v1/v2);
//! - `trace.json` - the profiled pass's chrome trace;
//! - `run.log` - the run's captured stdout/stderr.
//!
//! Missing artifacts make their checks SKIPPED and their report sections
//! say why - the report never silently omits a dimension. The auto checks
//! produce a provisional OK/WARN/FAIL verdict (mirrored into `checks.json`
//! so an agent never parses HTML), and the report ends with the reviewer
//! checklist: the FINAL call is a human's or an agent's, not the tool's.
//!
//! NOTE: honesty rules the reviews settled on, and every check still holds
//! to them: invariant violations are counted PER NAME (a stuck entity
//! violates every frame); FPS regressions are WARN, not FAIL (noisy shared
//! hosts); profile shares are for RANKING and never summed into a pie
//! (parent and child spans overlap); a truncated timeline IS the crash
//! signal (the recorder flushes per entry).

mod artifacts;
mod checks;
#[cfg(test)]
mod fixtures;
mod html;
mod manifest;

pub use artifacts::{ArtifactFailure, Input, RunArtifacts};
pub use checks::{
    check_names, checks_json, evaluate_checks, measured_count, overall_verdict, print_checks,
    status_class, Check, CheckStatus, NotApplicable, FPS_WARN_THRESHOLD_PCT,
};
pub use html::render_run_report;
pub use manifest::{run_identity, PassRecord, RunManifest};
