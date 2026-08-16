//! Everything a run directory produced, loaded once: the timeline, the
//! frame-time stats, the trace, the log, and the manifest. Every artifact is
//! optional - a missing one becomes a SKIPPED check, never a silent omission.

/// Glob-import surface for a run's collected artifacts.
pub mod prelude {
    pub use super::{ArtifactFailure, Input, RunArtifacts};
}

use std::path::{Path, PathBuf};

use nova_probe::{
    capabilities::timeline::{parse_timeline, TimelineEvent},
    contract::{Capability, ProbeContract},
    stats::{parse_frametime_csv, PerfRun},
};

use super::manifest::RunManifest;
use crate::evaluation::profile::prelude::*;

/// A present-but-unloadable artifact. Never silently dropped: [`RunArtifacts::load`]
/// leaves the field `None` and records the reason here, and the
/// `artifacts_loadable` check turns it into a FAILED row so the run cannot
/// verdict OK on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFailure {
    /// The artifact's filename, as it sits in the run dir.
    pub name: String,
    /// Why it could not be read or parsed.
    pub reason: String,
}

/// Everything a run directory yielded (each artifact optional).
#[derive(Default)]
pub struct RunArtifacts {
    /// Parsed `timeline.jsonl`.
    pub timeline: Option<Vec<TimelineEvent>>,
    /// Parsed `frametime.csv`.
    pub runs: Option<Vec<PerfRun>>,
    /// Aggregated `trace.json` system costs.
    pub costs: Option<Vec<SystemCost>>,
    /// Raw `run.log` contents.
    pub log: Option<String>,
    /// Parsed baseline `frametime.csv` (from `--baseline`).
    pub baseline: Option<Vec<PerfRun>>,
    /// Parsed `probe-run.json` (present in probe-produced dirs).
    pub manifest: Option<RunManifest>,
    /// Parsed `probe-contract.json`: what the EXAMPLE claimed, by wiring.
    /// `None` for a dir that predates the contract and for a web run (no
    /// filesystem) - absent is NOT "declares nothing", see [`Input::Unknown`].
    pub contract: Option<ProbeContract>,
    /// Reload intervals per run label (from each `<label>.json` sidecar's
    /// `reload_ms`, written by looped captures) - excluded from the frame
    /// stats by the capture, shown as their own line.
    pub reloads: Vec<(String, Vec<f64>)>,
    /// Artifacts that were present and could not be loaded. Each one is a
    /// field above left `None` for a reason that is NOT "not captured".
    pub failures: Vec<ArtifactFailure>,
}

/// The per-artifact reader behind [`RunArtifacts::load`]: it owns the run dir
/// and the failure list, so a bad read or a bad parse degrades that ONE
/// artifact instead of discarding the other eleven.
struct Loader<'a> {
    dir: &'a Path,
    failures: Vec<ArtifactFailure>,
}

impl Loader<'_> {
    /// The raw contents of `name`, or `None` when it is absent (nothing to
    /// report) or unreadable (reported).
    fn read(&mut self, name: &str) -> Option<String> {
        let path = self.dir.join(name);
        if !path.exists() {
            return None;
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => Some(contents),
            Err(error) => {
                self.fail(name, format!("could not read: {error}"));
                None
            }
        }
    }

    /// `name` read and parsed, or `None` with the reason recorded.
    fn load<T>(&mut self, name: &str, parse: impl FnOnce(&str) -> Result<T, String>) -> Option<T> {
        let contents = self.read(name)?;
        match parse(&contents) {
            Ok(value) => Some(value),
            Err(reason) => {
                self.fail(name, reason);
                None
            }
        }
    }

    fn fail(&mut self, name: &str, reason: String) {
        self.failures.push(ArtifactFailure {
            name: name.to_string(),
            reason,
        });
    }
}

impl RunArtifacts {
    /// Load whatever exists in `dir`. A present-but-unloadable artifact is
    /// never read as "not captured": its field stays `None` and the reason
    /// lands in [`failures`], which the `artifacts_loadable` check turns into
    /// a FAILED row. Absent files are simply `None`.
    ///
    /// Only the operator's own `--baseline` is still a hard error - it is an
    /// argument, not evidence the run produced, and a bad one must not
    /// degrade into a quietly missing comparison.
    ///
    /// [`failures`]: RunArtifacts::failures
    pub fn load(dir: &Path, baseline_dir: Option<&Path>) -> Result<Self, String> {
        let mut loader = Loader {
            dir,
            failures: Vec::new(),
        };
        let timeline = loader.load("timeline.jsonl", parse_timeline);
        let runs = loader.load("frametime.csv", parse_frametime_csv);
        let costs = loader.load("trace.json", aggregate_system_costs);
        // The game's logs: run.log (single run), fps-run.log (the fps pass is
        // a real game run too; its panics/errors gate), web-run.log (chromium's
        // output AND the game's - stats.rs parses `nova perf:` out of its
        // INFO:CONSOLE lines), plus run-<n>.log (sweep cells) in cell order.
        let mut log_parts: Vec<String> = Vec::new();
        for name in ["run.log", "fps-run.log", "web-run.log"] {
            if let Some(contents) = loader.read(name) {
                log_parts.push(contents);
            }
        }
        let mut cell_logs: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("run-") && n.ends_with(".log"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        cell_logs.sort();
        for path in cell_logs {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(contents) = loader.read(name) {
                log_parts.push(contents);
            }
        }
        let log = if log_parts.is_empty() {
            None
        } else {
            Some(log_parts.join("\n"))
        };
        let manifest = loader.load("probe-run.json", RunManifest::from_json);
        let contract = loader.load("probe-contract.json", ProbeContract::from_json);
        // Reload sidecars: each run label may have a <label>.json whose
        // reload_ms array records looped-capture reload intervals.
        let mut reloads: Vec<(String, Vec<f64>)> = Vec::new();
        if let Some(runs) = &runs {
            for run in runs {
                let Ok(contents) = std::fs::read_to_string(
                    dir.join(format!("{}.json", run.label.replace(['/', '\\'], "_"))),
                ) else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
                    continue;
                };
                let intervals: Vec<f64> = value
                    .get("reload_ms")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
                    .unwrap_or_default();
                if !intervals.is_empty() {
                    reloads.push((run.label.clone(), intervals));
                }
            }
        }
        let baseline = match baseline_dir {
            None => None,
            Some(base) => {
                let path = base.join("frametime.csv");
                let contents = std::fs::read_to_string(&path)
                    .map_err(|e| format!("baseline {}: {e}", path.display()))?;
                Some(parse_frametime_csv(&contents).map_err(|e| format!("baseline: {e}"))?)
            }
        };
        Ok(Self {
            timeline,
            runs,
            costs,
            log,
            baseline,
            manifest,
            contract,
            reloads,
            failures: loader.failures,
        })
    }

    /// What a check may conclude about the capability it needs, from the two
    /// halves of the handshake: the contract (what the example WIRED) and the
    /// manifest (what probe ARMED). See [`Input`].
    pub fn resolve<'a, T>(&self, capability: Capability, artifact: Option<&'a T>) -> Input<'a, T> {
        if self
            .contract
            .as_ref()
            .is_some_and(|c| !c.declares(capability))
        {
            return Input::NotDeclared(capability);
        }
        if self.armed(capability) == Some(false) {
            return Input::NotArmed(capability);
        }
        match artifact {
            Some(artifact) => Input::Present(artifact),
            // Only a run that both claimed the capability and was armed for
            // it owes an artifact. Anything less is unknowable, not a gap.
            None if self.contract.is_some() && self.armed(capability) == Some(true) => {
                Input::ArmedButAbsent(capability)
            }
            None => Input::Unknown(capability),
        }
    }

    /// The recorded failure for `name`, if that artifact was present and
    /// would not load. What lets a skip detail say "unloadable" instead of
    /// sending the reader after an env var that was set all along.
    pub(super) fn failure_for(&self, name: &str) -> Option<&ArtifactFailure> {
        self.failures.iter().find(|failure| failure.name == name)
    }

    /// Whether the dir yielded anything at all, loaded or failed. What tells
    /// "there was nothing to load" apart from "everything loaded".
    pub(super) fn any_present(&self) -> bool {
        !self.failures.is_empty()
            || self.timeline.is_some()
            || self.runs.is_some()
            || self.costs.is_some()
            || self.log.is_some()
            || self.manifest.is_some()
            || self.contract.is_some()
    }

    /// Whether probe armed `capability` for this run. `None` when there is no
    /// manifest to say.
    fn armed(&self, capability: Capability) -> Option<bool> {
        let manifest = self.manifest.as_ref()?;
        Some(match capability {
            Capability::Timeline => manifest.armed_timeline,
            Capability::Invariants => manifest.armed_invariants,
            Capability::FrameTime => manifest.armed_fps,
            // `probe run` does not arm snapshots: they are an on-demand
            // debugging artifact nothing grades, so the manifest has nothing
            // to say and "unknown" is the honest answer. A check that reads
            // one is what earns a manifest field.
            Capability::Snapshot => return None,
        })
    }
}

/// What a check can do with the capability it needs. Only [`Present`] grades
/// and only [`ArmedButAbsent`] fails; the rest are reasons the check does not
/// apply, each naming its capability so the row says WHY.
///
/// [`Present`]: Input::Present
/// [`ArmedButAbsent`]: Input::ArmedButAbsent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input<'a, T> {
    /// The example wires no such plugin: it makes no claim, so there is
    /// nothing to hold it to.
    NotDeclared(Capability),
    /// Wired, but this run did not arm it. Current frame-time runs do not
    /// produce this state; old manifests and deliberately stripped recorder
    /// surfaces can.
    NotArmed(Capability),
    /// Claimed, armed, and silent. The one state that is a failure.
    ArmedButAbsent(Capability),
    /// Nothing to resolve against: a run dir that predates the contract, a
    /// web run (no filesystem to write one), or a dir with no manifest. Not
    /// evidence of anything, in either direction.
    Unknown(Capability),
    /// The artifact is there. Grade it.
    Present(&'a T),
}
