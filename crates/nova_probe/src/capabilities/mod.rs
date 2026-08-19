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
//! - [`snapshot`] - the whole world's state on demand (ships, sections,
//!   fixtures, weapons, ordnance) -> `snapshot.jsonl`. The timeline says what
//!   happened; this says what the world LOOKS like.
//!
//! [`NovaProbePlugin`] bundles all four. It does not replace their
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
// The world-state snapshot writes a JSONL file, so it is native-only with the
// recorder and wasm gets the same shape of no-op stub.
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot;
#[cfg(target_arch = "wasm32")]
pub mod snapshot {
    //! Wasm stubs for the native-only world-state snapshot.
    use bevy::prelude::*;

    /// No-op on wasm (no filesystem for the JSONL sink).
    pub fn nova_snapshot() -> SnapshotPlugin {
        SnapshotPlugin
    }

    /// Inert wasm stand-in for the native snapshot plugin.
    pub struct SnapshotPlugin;

    impl SnapshotPlugin {
        /// No-op on wasm.
        pub fn out(self, _path: impl Into<std::path::PathBuf>) -> Self {
            self
        }

        /// No-op on wasm.
        pub fn at_frames(self, _frames: impl IntoIterator<Item = u32>) -> Self {
            self
        }
    }

    impl Plugin for SnapshotPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// No-op on wasm.
    pub fn probe_snapshot(_world: &mut World, _reason: &str) {}

    /// Glob-import surface for the wasm stubs; the same names the native
    /// module publishes, minus the serializer and the parser.
    pub mod prelude {
        pub use super::{nova_snapshot, probe_snapshot, SnapshotPlugin};
    }
}
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
    pub use super::{
        frametime::prelude::*, snapshot::prelude::*, timeline::prelude::*, NovaProbePlugin,
        CORRECTNESS_MODE, PROBE_MODE_ENV,
    };
}

/// Selects correctness-only probe wiring in child processes.
///
/// The host sets this to [`CORRECTNESS_MODE`] when it needs behavioral
/// evidence without frame-time or profiling passes.
pub const PROBE_MODE_ENV: &str = "NOVA_PROBE_MODE";

/// Value of [`PROBE_MODE_ENV`] that selects correctness-only wiring.
pub const CORRECTNESS_MODE: &str = "correctness";

/// Every capability at once - what "this binary is being probed" looks like
/// as one line.
///
/// Each capability is env-gated downstream, so an unarmed run pays nothing.
/// Builder methods retain the single bundle entry point for examples that
/// need custom invariants, a frame-time driver, or no frame-time claim.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use nova_probe::NovaProbePlugin;
/// # fn add(app: &mut App) {
/// app.add_plugins(NovaProbePlugin::default());
/// # }
/// ```
pub struct NovaProbePlugin {
    frametime: Option<FrameTimePlugin>,
    #[cfg(not(target_arch = "wasm32"))]
    invariants: InvariantsPlugin,
}

impl NovaProbePlugin {
    /// Do not declare or wire frame-time capture for this example.
    pub fn without_frametime(mut self) -> Self {
        self.frametime = None;
        self
    }

    /// Attach a per-frame driver to this example's frame-time capture.
    pub fn drive_frametime(
        mut self,
        driver: impl Fn(&mut World, u32) + Send + Sync + 'static,
    ) -> Self {
        self.frametime = self.frametime.map(|plugin| plugin.drive(driver));
        self
    }

    /// Hold this example's frame-time capture until `ready` holds (see
    /// [`FrameTimePlugin::ready_when`]).
    pub fn ready_frametime(
        mut self,
        ready: impl Fn(&World) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.frametime = self.frametime.map(|plugin| plugin.ready_when(ready));
        self
    }

    /// Register scenario variables that must never decrease.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn monotonic<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.invariants = self.invariants.monotonic(keys);
        self
    }
}

impl Default for NovaProbePlugin {
    fn default() -> Self {
        Self {
            frametime: Some(nova_frametime()),
            #[cfg(not(target_arch = "wasm32"))]
            invariants: nova_invariants(),
        }
    }
}

impl Plugin for NovaProbePlugin {
    fn build(&self, app: &mut App) {
        let correctness_only = std::env::var(PROBE_MODE_ENV).as_deref() == Ok(CORRECTNESS_MODE);
        if !correctness_only {
            if let Some(plugin) = &self.frametime {
                app.add_plugins(plugin.clone());
            }
        }
        app.add_plugins(nova_timeline());
        app.add_plugins(nova_snapshot());
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(self.invariants.clone());
    }
}
