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

/// Glob-import surface for the whole evaluation half: the catalog a run was
/// resolved from, its manifest and artifacts, the trace aggregation and the
/// checks that grade them.
///
/// `overall_verdict` is deliberately NOT re-exported at the crate root beside
/// [`report::aggregate`](crate::report::aggregate)'s function of the same name
/// - one grades a run's checks, the other grades a sweep's rows.
pub mod prelude {
    pub use super::{
        artifacts::prelude::*, catalog::prelude::*, checks::prelude::*, manifest::prelude::*,
        profile::prelude::*,
    };
}

pub use prelude::*;
