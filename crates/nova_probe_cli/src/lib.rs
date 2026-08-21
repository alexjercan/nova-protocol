//! nova_probe_cli: the HOST half of the run-harness. It spawns a cataloged
//! example as a child process, collects whatever artifacts that run wrote,
//! grades them against the check roster and renders the reports.
//!
//! The other half is `nova_probe`, which links into the game and writes those
//! artifacts. The two never share a process: the filesystem and the
//! `NOVA_PROBE_*` / autopilot env vars are the IPC, which is why this crate is
//! host-only with no wasm build at all - the split IS the `cfg`.
//!
//! The pipeline reads left to right, and so does the module tree:
//!
//! `collect evidence` (nova_probe, in the child) -> [`evaluation`] ->
//! [`report`].
//!
//! - [`native`] - the driver: command line, spec resolution, paths, child-run
//!   environments and supervision (including
//!   [`profile_sandbox`](native::profile_sandbox), the profile state a child
//!   run is given so it never reads or writes the operator's own mods and
//!   settings), the single run, the web pass, the sweep.
//! - [`evaluation`] - what the run left behind and what it is worth: the
//!   catalog it was resolved from, its manifest, its artifacts, the chrome
//!   trace, and the checks that grade them.
//! - [`report`] - the HTML and JSON that evaluation turns into: the per-run
//!   report and the multi-run status index.
#![warn(missing_docs)]

pub mod evaluation;
pub mod native;
pub mod report;

/// The whole host half in one glob: evaluation's names and the report
/// renderers.
///
/// Each module owns its own `prelude`, so publishing a new name is a one-file
/// edit rather than an edit here as well. [`native`] is absent on purpose - it
/// is the driver; the game binary's `probe` subcommand calls
/// [`native::main`] directly and nothing else imports from it.
///
/// `report`'s sweep-level `overall_verdict` arrives as `aggregate_verdict`:
/// [`evaluation`] has a function of the same name that grades one run's
/// checks, and a glob cannot carry both.
pub mod prelude {
    pub use crate::{
        evaluation::prelude::*,
        report::prelude::{
            index_json, overall_verdict as aggregate_verdict, render_index, render_run_report,
            verdict_severity, AllManifest, AllRow,
        },
    };
}

pub use prelude::*;
