//! The capabilities: one module per kind of evidence an example can collect
//! about its own run. Each is a Bevy plugin the example wires, each writes one
//! artifact into the run directory, and each declares itself into the run's
//! [`ProbeContract`] so `nova_probe_cli` knows what the run OWED before it
//! grades what it got.
//!
//! - [`frametime`] - wall-clock frame deltas over a fixed window ->
//!   `frametime.csv` + `<label>.json`.
//! - [`timeline`] - states, scenario events, variables and markers, in order
//!   -> `timeline.jsonl`.
//! - [`invariants`] - engine-guaranteed bounds asserted every frame, riding
//!   the timeline sink.
//!
//! [`NovaProbePlugin`] bundles all three. It does not replace their
//! per-example configuration: an example that needs a driver or a custom
//! output path still wires that capability itself.

use bevy::prelude::*;

use crate::prelude::*;

pub mod frametime;
// Continuous invariant checks ride the recorder's timeline sink, so they are
// native-only with it (nothing wasm-side references them - the examples that
// wire them never build for wasm).
#[cfg(not(target_arch = "wasm32"))]
pub mod invariants;
// The run-timeline recorder writes a JSONL file; the browser has no
// filesystem, so the module is native-only and wasm gets no-op stubs with the
// same signatures, so cross-target callers compile.
#[cfg(not(target_arch = "wasm32"))]
pub mod timeline;
#[cfg(target_arch = "wasm32")]
pub mod timeline {
    //! Wasm stubs for the native-only run-timeline recorder.
    use bevy::prelude::*;

    /// No-op on wasm (no filesystem for the JSONL sink).
    pub fn nova_timeline() -> RunRecorderPlugin {
        RunRecorderPlugin
    }

    /// Inert wasm stand-in for the native recorder plugin.
    pub struct RunRecorderPlugin;

    impl RunRecorderPlugin {
        /// No-op on wasm.
        pub fn out(self, _path: impl Into<std::path::PathBuf>) -> Self {
            self
        }
    }

    impl Plugin for RunRecorderPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// No-op on wasm.
    pub fn probe_marker(_world: &mut World, _name: &str, _data: serde_json::Value) {}

    /// Glob-import surface for the wasm stubs; the same names the native
    /// module publishes, minus the ones that read a file back.
    pub mod prelude {
        pub use super::{nova_timeline, probe_marker, RunRecorderPlugin};
    }
}

/// Glob-import surface for every capability, plus the bundle that wires them
/// all. `invariants` is native-only, so it is absent on wasm.
pub mod prelude {
    #[cfg(not(target_arch = "wasm32"))]
    pub use super::invariants::prelude::*;
    pub use super::{frametime::prelude::*, timeline::prelude::*, NovaProbePlugin};
}

/// Every capability at once - what "this binary is being probed" looks like
/// as one line.
///
/// Each field is env-gated downstream exactly as the individual plugin is, so
/// an unarmed run pays nothing. Wire a capability directly instead when it
/// needs configuration ([`FrameTimePlugin::drive`], a custom timeline path);
/// this bundles the defaults, it does not replace them.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use nova_probe::NovaProbePlugin;
/// # fn add(app: &mut App) {
/// app.add_plugins(NovaProbePlugin::default());
/// # }
/// ```
pub struct NovaProbePlugin {
    /// Wire the frame-time capture.
    pub frametime: bool,
    /// Wire the run-timeline recorder.
    pub timeline: bool,
    /// Wire the continuous invariant checks. Requires `timeline`: the
    /// invariant entries ride that sink.
    pub invariants: bool,
}

impl Default for NovaProbePlugin {
    fn default() -> Self {
        Self {
            frametime: true,
            timeline: true,
            invariants: true,
        }
    }
}

impl Plugin for NovaProbePlugin {
    fn build(&self, app: &mut App) {
        if self.frametime {
            app.add_plugins(nova_frametime());
        }
        if self.timeline {
            app.add_plugins(nova_timeline());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.invariants {
            app.add_plugins(nova_invariants());
        }
    }
}
