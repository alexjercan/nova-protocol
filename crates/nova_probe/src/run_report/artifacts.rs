//! Everything a run directory produced, loaded once: the timeline, the
//! frame-time stats, the trace, the log, and the manifest. Every artifact is
//! optional - a missing one becomes a SKIPPED check, never a silent omission.

use std::path::{Path, PathBuf};

use super::manifest::RunManifest;
use crate::{
    contract::{Capability, ProbeContract},
    profile::{aggregate_system_costs, SystemCost},
    recorder::{parse_timeline, TimelineEvent},
    stats::{parse_frametime_csv, PerfRun},
};

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
}

impl RunArtifacts {
    /// Load whatever exists in `dir`. Unreadable-but-present artifacts are
    /// hard errors (a corrupt file must not read as "not captured");
    /// absent files are simply `None`.
    pub fn load(dir: &Path, baseline_dir: Option<&Path>) -> Result<Self, String> {
        let read_opt = |name: &str| -> Result<Option<String>, String> {
            let path = dir.join(name);
            if !path.exists() {
                return Ok(None);
            }
            std::fs::read_to_string(&path)
                .map(Some)
                .map_err(|e| format!("could not read {}: {e}", path.display()))
        };
        let timeline = read_opt("timeline.jsonl")?
            .map(|s| parse_timeline(&s).map_err(|e| format!("timeline.jsonl: {e}")))
            .transpose()?;
        let runs = read_opt("frametime.csv")?
            .map(|s| parse_frametime_csv(&s).map_err(|e| format!("frametime.csv: {e}")))
            .transpose()?;
        let costs = read_opt("trace.json")?
            .map(|s| aggregate_system_costs(&s).map_err(|e| format!("trace.json: {e}")))
            .transpose()?;
        // The game's logs: run.log (single run) plus run-<n>.log (sweep
        // cells), concatenated in cell order. web-run.log stays OUT - it is
        // chromium's own output, not the game's.
        let mut log_parts: Vec<String> = Vec::new();
        if let Some(main_log) = read_opt("run.log")? {
            log_parts.push(main_log);
        }
        // The fps pass is a real game run too; its panics/errors gate.
        if let Some(fps_log) = read_opt("fps-run.log")? {
            log_parts.push(fps_log);
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
            log_parts.push(
                std::fs::read_to_string(&path)
                    .map_err(|e| format!("could not read {}: {e}", path.display()))?,
            );
        }
        let log = if log_parts.is_empty() {
            None
        } else {
            Some(log_parts.join("\n"))
        };
        let manifest = read_opt("probe-run.json")?
            .map(|s| RunManifest::from_json(&s))
            .transpose()?;
        let contract = read_opt("probe-contract.json")?
            .map(|s| ProbeContract::from_json(&s))
            .transpose()?;
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

    /// Whether probe armed `capability` for this run. `None` when there is no
    /// manifest to say.
    fn armed(&self, capability: Capability) -> Option<bool> {
        let manifest = self.manifest.as_ref()?;
        Some(match capability {
            Capability::Timeline => manifest.armed_timeline,
            Capability::Invariants => manifest.armed_invariants,
            Capability::FrameTime => manifest.armed_fps,
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
    /// Wired, but this run did not arm it - no `--fps`, or a sweep cell that
    /// strips the recorder on purpose.
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
