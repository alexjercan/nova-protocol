//! Evidence in, verdict out: one run directory becomes a graded roster of
//! checks. Everything downstream of this module renders; nothing downstream
//! decides.
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

pub mod artifacts;
pub mod catalog;
pub mod checks;
pub mod manifest;
pub mod profile;

#[cfg(test)]
pub(crate) mod fixtures;

pub use artifacts::{ArtifactFailure, Input, RunArtifacts};
pub use catalog::{categories, load_example_catalog, parse_example_catalog, CatalogExample};
pub use checks::{
    check_names, checks_json, evaluate_checks, measured_count, overall_verdict, print_checks,
    status_class, Check, CheckStatus, NotApplicable, FPS_WARN_THRESHOLD_PCT,
};
pub use manifest::{run_identity, PassRecord, RunManifest};
pub use profile::{aggregate_system_costs, render_top_table, SystemCost};
