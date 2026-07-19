//! The unified run report: one run directory in, `report.html` +
//! `checks.json` out - the assembly point of the run-harness (spike
//! tasks/20260719-112011/SPIKE.md, task 20260719-112304; absorbs the
//! perf-report task 20260718-152230 as its FPS section).
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
//! Honesty rules inherited from the family's reviews: invariant violations
//! are counted PER NAME (a stuck entity violates every frame - T3);
//! FPS regressions are WARN, not FAIL (noisy shared hosts - spike m4/m5);
//! profile shares are for RANKING and never summed into a pie (parent and
//! child spans overlap - T4 R1.2); a truncated timeline IS the crash
//! signal (flush-per-entry made it so - T2).

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    profile::{aggregate_system_costs, SystemCost},
    recorder::{parse_timeline, TimelineEvent},
    report::{escape, render_chart, render_table, STYLE},
    stats::{parse_frametime_csv, PerfRun},
};

/// Soft FPS gate: the worst same-label mean-frame-time delta against the
/// baseline may move this many percent before the check turns WARN. One
/// tunable on purpose; frame numbers on a shared host are noisy, so this is
/// a flag for the reviewer, never a hard failure.
pub const FPS_WARN_THRESHOLD_PCT: f64 = 10.0;

/// The manifest `probe run` writes (`probe-run.json`): what was executed,
/// with what outcome, producing which artifacts. The report treats it as
/// the run's identity - `process_exit` reads it, skip details use its
/// `armed` flags to distinguish "not armed" from "armed but the example is
/// not wired", and `probe report` (consolidation task) will refuse dirs
/// without one.
#[derive(Debug, Clone, PartialEq)]
pub struct RunManifest {
    /// The example that was run.
    pub example: String,
    /// Unix seconds when the run started.
    pub started_unix: u64,
    /// Short git SHA + host, same resolvers as the capture metadata.
    pub git_sha: String,
    pub host: String,
    /// Which capture surfaces probe armed (timeline/invariants always; fps
    /// only with --fps).
    pub armed_timeline: bool,
    pub armed_invariants: bool,
    pub armed_fps: bool,
    /// Per-pass outcomes, in execution order.
    pub passes: Vec<PassRecord>,
}

/// One executed pass and how it ended.
#[derive(Debug, Clone, PartialEq)]
pub struct PassRecord {
    /// `clean`, `profiled`, `samply`.
    pub name: String,
    /// The child exited successfully (false also when timed out).
    pub success: bool,
    /// The child was killed by the runner's timeout.
    pub timed_out: bool,
}

/// The run's identity pair (short git SHA, host tag) via the same
/// resolvers the capture metadata uses - for the probe bin's manifest.
pub fn run_identity() -> (String, String) {
    (
        crate::capture::resolve_git_sha(),
        crate::capture::resolve_host(),
    )
}

impl RunManifest {
    /// Serialize for `probe-run.json`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "example": self.example,
            "started_unix": self.started_unix,
            "git_sha": self.git_sha,
            "host": self.host,
            "armed": {
                "timeline": self.armed_timeline,
                "invariants": self.armed_invariants,
                "fps": self.armed_fps,
            },
            "passes": self.passes.iter().map(|p| serde_json::json!({
                "name": p.name, "success": p.success, "timed_out": p.timed_out,
            })).collect::<Vec<_>>(),
        })
    }

    /// Parse `probe-run.json`. Loud on malformed content - a corrupt
    /// manifest must not read as "no manifest".
    pub fn from_json(contents: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(contents).map_err(|e| format!("probe-run.json: {e}"))?;
        let s = |k: &str| -> Result<String, String> {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("probe-run.json: missing {k}"))
        };
        let armed = |k: &str| v["armed"].get(k).and_then(|x| x.as_bool()).unwrap_or(false);
        let passes = v
            .get("passes")
            .and_then(|p| p.as_array())
            .ok_or("probe-run.json: missing passes")?
            .iter()
            .map(|p| {
                Ok(PassRecord {
                    name: p
                        .get("name")
                        .and_then(|x| x.as_str())
                        .ok_or("probe-run.json: pass missing name")?
                        .to_string(),
                    success: p.get("success").and_then(|x| x.as_bool()).unwrap_or(false),
                    timed_out: p
                        .get("timed_out")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            example: s("example")?,
            started_unix: v.get("started_unix").and_then(|x| x.as_u64()).unwrap_or(0),
            git_sha: s("git_sha")?,
            host: s("host")?,
            armed_timeline: armed("timeline"),
            armed_invariants: armed("invariants"),
            armed_fps: armed("fps"),
            passes,
        })
    }
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
    /// Reload intervals per run label (from each `<label>.json` sidecar's
    /// `reload_ms`, written by looped captures) - excluded from the frame
    /// stats by the capture, shown as their own line (task 20260720-000616).
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
            reloads,
        })
    }
}

/// One verdict row.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// Stable check id (`run_completed`, `invariants_held`, ...).
    pub name: &'static str,
    /// The row's outcome.
    pub status: CheckStatus,
    /// The measured value, human-readable.
    pub value: String,
    /// The gate it was held against.
    pub threshold: String,
    /// One sentence of context (why skipped, what failed).
    pub detail: String,
    /// Structured payload for machine consumers (counts, deltas) - the
    /// prose fields are for humans, this is for agents.
    pub data: serde_json::Value,
}

/// Check outcome. `Warn` never fails the run (soft gates); `Skipped` means
/// the input artifact was not captured, not that the property held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skipped,
}

impl CheckStatus {
    fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skipped => "SKIPPED",
        }
    }
}

/// How many checks actually measured something (not SKIPPED).
pub fn measured_count(checks: &[Check]) -> usize {
    checks
        .iter()
        .filter(|c| c.status != CheckStatus::Skipped)
        .count()
}

/// The provisional overall verdict: FAIL if any hard check failed, WARN if
/// anything warned, NO_DATA when NOTHING was measured (a dir with zero
/// evidence must not read as a passing run), OK otherwise. OK is always
/// OK-with-coverage: consumers read `measured_count` alongside it - an OK
/// with run_completed/invariants_held SKIPPED only proves the example's own
/// assertions (its exit status), not the recorded run. The reviewer owns
/// the final call either way.
pub fn overall_verdict(checks: &[Check]) -> &'static str {
    if measured_count(checks) == 0 {
        "NO_DATA"
    } else if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        "FAIL"
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        "WARN"
    } else {
        "OK"
    }
}

/// Drop ANSI escape sequences (color codes) from a log line, so the log
/// scan sees the same text regardless of whether the run's output went to a
/// TTY or a file.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI sequence: ESC [ ... final byte in @-~
            if chars.peek() == Some(&'[') {
                chars.next();
                for f in chars.by_ref() {
                    if ('@'..='~').contains(&f) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Count invariant violations per name off the timeline (per-name counts,
/// not raw totals: one stuck entity violates every frame).
fn violations_by_name(timeline: &[TimelineEvent]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for entry in timeline.iter().filter(|e| e.kind == "invariant") {
        *counts.entry(entry.name.clone()).or_insert(0) += 1;
    }
    counts
}

/// Evaluate every auto check against the loaded artifacts.
pub fn evaluate_checks(artifacts: &RunArtifacts) -> Vec<Check> {
    let mut checks = Vec::new();
    let manifest = artifacts.manifest.as_ref();

    // process_exit: the children's actual outcomes, from the manifest -
    // ALL primary passes count (a sweep runs one clean pass per matrix
    // cell; the web platform runs a web pass), and the worst outcome wins.
    // For harnessed examples this is real correctness evidence on its own:
    // their autopilot assertions panic on failure. SKIPPED only for
    // foreign dirs. The profiled/samply passes are auxiliary and excluded
    // (their failures degrade to missing artifacts by design).
    checks.push(match manifest {
        None => Check {
            name: "process_exit",
            status: CheckStatus::Skipped,
            value: "no manifest".into(),
            threshold: "every primary pass exits success, untimed".into(),
            detail: "no probe-run.json - this dir was not produced by probe run".into(),
            data: serde_json::Value::Null,
        },
        Some(m) => {
            let primary: Vec<&PassRecord> = m
                .passes
                .iter()
                .filter(|p| p.name.starts_with("clean") || p.name == "web" || p.name == "fps")
                .collect();
            let failed: Vec<&&PassRecord> = primary
                .iter()
                .filter(|p| !p.success || p.timed_out)
                .collect();
            let data = serde_json::json!({
                "primary_passes": primary.len(),
                "failed": failed.iter().map(|p| serde_json::json!({
                    "name": p.name, "timed_out": p.timed_out,
                })).collect::<Vec<_>>(),
            });
            if primary.is_empty() {
                Check {
                    name: "process_exit",
                    status: CheckStatus::Skipped,
                    value: "no primary passes recorded".into(),
                    threshold: "every primary pass exits success, untimed".into(),
                    detail: "the manifest lists no clean/web passes".into(),
                    data,
                }
            } else if failed.is_empty() {
                Check {
                    name: "process_exit",
                    status: CheckStatus::Pass,
                    value: format!("{} pass(es), all clean exits", primary.len()),
                    threshold: "every primary pass exits success, untimed".into(),
                    detail: "every run's own assertions held".into(),
                    data,
                }
            } else {
                let names: Vec<String> = failed
                    .iter()
                    .map(|p| {
                        format!(
                            "{}{}",
                            p.name,
                            if p.timed_out { " (timed out)" } else { "" }
                        )
                    })
                    .collect();
                Check {
                    name: "process_exit",
                    status: CheckStatus::Fail,
                    value: format!("{}/{} pass(es) failed", failed.len(), primary.len()),
                    threshold: "every primary pass exits success, untimed".into(),
                    detail: format!("failed: {} - read the matching log", names.join(", ")),
                    data,
                }
            }
        }
    });

    // Skip-detail helper: "not captured" means different things depending
    // on whether probe ARMED the surface (finding 4's misdirection).
    let timeline_skip_detail = || -> String {
        match manifest {
            Some(m) if m.armed_timeline => format!(
                "probe armed the recorder but {} is not wired with nova_probe::nova_timeline()",
                m.example
            ),
            _ => "timeline.jsonl not captured (arm NOVA_PERF_TIMELINE)".into(),
        }
    };

    // run_completed: the timeline must CLOSE, and the bracket's own entry
    // count must match what is actually on disk (a swallowed write - full
    // disk - otherwise goes unnoticed). Flush-per-entry means a
    // panicked/killed run leaves a bracket-less file: that IS the crash
    // signal, by design.
    checks.push(match &artifacts.timeline {
        None => Check {
            name: "run_completed",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: "run_end present + AppExit Success + entry count consistent".into(),
            detail: timeline_skip_detail(),
            data: serde_json::Value::Null,
        },
        Some(timeline) => {
            let end = timeline.iter().rev().find(|e| e.kind == "run_end");
            match end {
                Some(end) if end.data["exit"].as_str().unwrap_or("").contains("Success") => {
                    let written = end.data["entries"].as_u64().unwrap_or(0);
                    let on_disk = (timeline.len() as u64).saturating_sub(1);
                    if written != on_disk {
                        Check {
                            name: "run_completed",
                            status: CheckStatus::Fail,
                            value: format!("{written} written vs {on_disk} on disk"),
                            threshold: "run_end present + AppExit Success + entry count consistent"
                                .into(),
                            detail: "the recorder wrote entries the file does not hold (full \
                                     disk / IO errors were warned but swallowed)"
                                .into(),
                            data: serde_json::json!({ "written": written, "on_disk": on_disk }),
                        }
                    } else {
                        Check {
                            name: "run_completed",
                            status: CheckStatus::Pass,
                            value: format!("run_end at frame {}", end.frame),
                            threshold: "run_end present + AppExit Success + entry count consistent"
                                .into(),
                            detail: "the run closed its bracket cleanly".into(),
                            data: serde_json::json!({ "end_frame": end.frame, "entries": written }),
                        }
                    }
                }
                Some(end) => Check {
                    name: "run_completed",
                    status: CheckStatus::Fail,
                    value: format!("exit: {}", end.data["exit"]),
                    threshold: "run_end present + AppExit Success + entry count consistent".into(),
                    detail: "the run ended with a non-success exit".into(),
                    data: serde_json::json!({ "exit": end.data["exit"] }),
                },
                None => Check {
                    name: "run_completed",
                    status: CheckStatus::Fail,
                    value: "timeline truncated (no run_end)".into(),
                    threshold: "run_end present + AppExit Success + entry count consistent".into(),
                    detail: "flush-per-entry means truncation = the run died mid-flight".into(),
                    data: serde_json::json!({ "truncated": true }),
                },
            }
        }
    });

    // reached_playing: every harnessed example's smoke contract is "reach
    // Playing and exit without panic" - an app that exits cleanly while
    // still Loading (graceful asset failure) must not pass unnoticed.
    checks.push(match &artifacts.timeline {
        None => Check {
            name: "reached_playing",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: "a GameStates transition entered Playing".into(),
            detail: timeline_skip_detail(),
            data: serde_json::Value::Null,
        },
        Some(timeline) => {
            let entered = timeline.iter().find(|e| {
                e.kind == "state"
                    && e.name == "GameStates"
                    && e.data["entered"].as_str() == Some("Playing")
            });
            match entered {
                Some(entry) => Check {
                    name: "reached_playing",
                    status: CheckStatus::Pass,
                    value: format!("Playing at frame {}", entry.frame),
                    threshold: "a GameStates transition entered Playing".into(),
                    detail: "the run reached gameplay".into(),
                    data: serde_json::json!({ "frame": entry.frame }),
                },
                None => Check {
                    name: "reached_playing",
                    status: CheckStatus::Fail,
                    value: "never entered Playing".into(),
                    threshold: "a GameStates transition entered Playing".into(),
                    detail: "the app ended while still loading/menu - the smoke contract \
                             (reach Playing) was not met"
                        .into(),
                    data: serde_json::json!({ "reached": false }),
                },
            }
        }
    });

    // invariants_held: the summary entry carries the tally; per-name counts
    // ride in detail AND data.
    checks.push(match &artifacts.timeline {
        None => Check {
            name: "invariants_held",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: "0 violations".into(),
            detail: timeline_skip_detail(),
            data: serde_json::Value::Null,
        },
        Some(timeline) => {
            let summary = timeline
                .iter()
                .rev()
                .find(|e| e.kind == "invariant_summary");
            let by_name = violations_by_name(timeline);
            match summary {
                None if by_name.is_empty() => Check {
                    name: "invariants_held",
                    status: CheckStatus::Skipped,
                    value: "no invariant entries".into(),
                    threshold: "0 violations".into(),
                    detail: match manifest {
                        Some(m) if m.armed_invariants => format!(
                            "probe armed the checks but {} is not wired with \
                             nova_probe::nova_invariants()",
                            m.example
                        ),
                        _ => "invariants not armed (arm NOVA_PERF_INVARIANTS)".into(),
                    },
                    data: serde_json::Value::Null,
                },
                summary => {
                    let violations = summary
                        .map(|s| s.data["violations"].as_u64().unwrap_or(0))
                        .unwrap_or_else(|| by_name.values().sum());
                    let checks_run = summary
                        .map(|s| s.data["checks"].as_u64().unwrap_or(0))
                        .unwrap_or(0);
                    let counts = serde_json::json!({
                        "violations": violations,
                        "checked_frames": checks_run,
                        "by_name": by_name,
                    });
                    if violations == 0 {
                        Check {
                            name: "invariants_held",
                            status: CheckStatus::Pass,
                            value: format!("0 violations over {checks_run} checked frames"),
                            threshold: "0 violations".into(),
                            detail: "every engine-guaranteed bound held".into(),
                            data: counts,
                        }
                    } else {
                        let names: Vec<String> = by_name
                            .iter()
                            .map(|(name, n)| format!("{name} x{n}"))
                            .collect();
                        Check {
                            name: "invariants_held",
                            status: CheckStatus::Fail,
                            value: format!("{violations} violation entries"),
                            threshold: "0 violations".into(),
                            detail: format!(
                                "by name (a persisting violation repeats per frame): {}",
                                names.join(", ")
                            ),
                            data: counts,
                        }
                    }
                }
            }
        }
    });

    // fps_within_baseline: a soft gate on REGRESSIONS only - an improvement
    // is a PASS with the delta noted, never a warning (an automated caller
    // must not read a speedup as a regression flag).
    checks.push(match (&artifacts.runs, &artifacts.baseline) {
        (Some(runs), Some(baseline)) => {
            let mut worst_regression: Option<(String, f64)> = None;
            let mut best_note: Option<(String, f64)> = None;
            let mut matched = 0;
            for run in runs {
                if let Some(base) = baseline.iter().find(|b| b.label == run.label) {
                    if base.stats.mean_ms > 0.0 {
                        matched += 1;
                        let delta =
                            (run.stats.mean_ms - base.stats.mean_ms) / base.stats.mean_ms * 100.0;
                        if delta > 0.0 {
                            if worst_regression.as_ref().is_none_or(|(_, w)| delta > *w) {
                                worst_regression = Some((run.label.clone(), delta));
                            }
                        } else if best_note.as_ref().is_none_or(|(_, b)| delta < *b) {
                            best_note = Some((run.label.clone(), delta));
                        }
                    }
                }
            }
            if matched == 0 {
                Check {
                    name: "fps_within_baseline",
                    status: CheckStatus::Skipped,
                    value: "no matching labels".into(),
                    threshold: format!("regression <= {FPS_WARN_THRESHOLD_PCT}%"),
                    detail: "baseline shares no run labels with this capture (baselines are \
                             only valid probe-run-vs-probe-run or sweep-vs-sweep)"
                        .into(),
                    data: serde_json::Value::Null,
                }
            } else {
                match worst_regression {
                    Some((label, delta)) if delta > FPS_WARN_THRESHOLD_PCT => Check {
                        name: "fps_within_baseline",
                        status: CheckStatus::Warn,
                        value: format!("worst {label}: +{delta:.1}%"),
                        threshold: format!("regression <= {FPS_WARN_THRESHOLD_PCT}%"),
                        detail: "soft gate: frame numbers are host-noisy; reviewer judges \
                                 (was the host quiet? is the delta consistent?)"
                            .into(),
                        data: serde_json::json!({ "label": label, "delta_pct": delta }),
                    },
                    Some((label, delta)) => Check {
                        name: "fps_within_baseline",
                        status: CheckStatus::Pass,
                        value: format!("worst {label}: +{delta:.1}%"),
                        threshold: format!("regression <= {FPS_WARN_THRESHOLD_PCT}%"),
                        detail: "worst regression within the soft gate".into(),
                        data: serde_json::json!({ "label": label, "delta_pct": delta }),
                    },
                    None => {
                        let (label, delta) = best_note.expect("matched > 0 with no regressions");
                        Check {
                            name: "fps_within_baseline",
                            status: CheckStatus::Pass,
                            value: format!("improved; best {label}: {delta:.1}%"),
                            threshold: format!("regression <= {FPS_WARN_THRESHOLD_PCT}%"),
                            detail: "no label regressed against the baseline".into(),
                            data: serde_json::json!({ "label": label, "delta_pct": delta }),
                        }
                    }
                }
            }
        }
        _ => Check {
            name: "fps_within_baseline",
            status: CheckStatus::Skipped,
            value: "missing capture or baseline".into(),
            threshold: format!("regression <= {FPS_WARN_THRESHOLD_PCT}%"),
            detail: "needs both frametime.csv and --baseline <dir>".into(),
            data: serde_json::Value::Null,
        },
    });

    // log_clean: panics and ERROR-level lines are hard failures. The level
    // token is matched as a whole word after ANSI stripping (a substring
    // match missed line-initial ERROR and false-positived on payloads).
    checks.push(match &artifacts.log {
        None => Check {
            name: "log_clean",
            status: CheckStatus::Skipped,
            value: "no run.log".into(),
            threshold: "no panics / ERROR lines".into(),
            detail: "log not captured alongside the run".into(),
            data: serde_json::Value::Null,
        },
        Some(log) => {
            let cleaned: Vec<String> = log.lines().map(strip_ansi).collect();
            let offending: Vec<&String> = cleaned
                .iter()
                .filter(|line| {
                    line.contains("panicked at")
                        || line.split_whitespace().any(|token| token == "ERROR")
                })
                .collect();
            if offending.is_empty() {
                Check {
                    name: "log_clean",
                    status: CheckStatus::Pass,
                    value: "0 panic/ERROR lines".into(),
                    threshold: "no panics / ERROR lines".into(),
                    detail: "log scanned clean".into(),
                    data: serde_json::json!({ "offending": 0 }),
                }
            } else {
                Check {
                    name: "log_clean",
                    status: CheckStatus::Fail,
                    value: format!("{} offending line(s)", offending.len()),
                    threshold: "no panics / ERROR lines".into(),
                    detail: format!(
                        "first: {}",
                        offending[0].chars().take(160).collect::<String>()
                    ),
                    data: serde_json::json!({
                        "offending": offending.len(),
                        "sample": offending.iter().take(5).map(|s| s.chars().take(160).collect::<String>()).collect::<Vec<_>>(),
                    }),
                }
            }
        }
    });

    checks
}

/// Print the verdict rows to stdout (shared by the probe and run_report
/// bins, so the two never drift).
pub fn print_checks(checks: &[Check]) {
    for check in checks {
        println!(
            "  {:22} {:8} {}",
            check.name,
            check.status.as_str(),
            check.value
        );
    }
}

/// The machine-readable mirror of the verdict rows, plus the run's
/// identity (from the manifest) and the measured-coverage figure - an
/// agent reads verdict AND measured, never verdict alone.
pub fn checks_json(checks: &[Check], manifest: Option<&RunManifest>) -> serde_json::Value {
    serde_json::json!({
        "verdict": overall_verdict(checks),
        "measured": format!("{}/{}", measured_count(checks), checks.len()),
        "reviewer_confirmation_required": true,
        "run": manifest.map(RunManifest::to_json),
        "checks": checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "status": c.status.as_str(),
            "value": c.value,
            "threshold": c.threshold,
            "detail": c.detail,
            "data": c.data,
        })).collect::<Vec<_>>(),
        "generated_by": "nova_probe run_report",
    })
}

/// Timeline entries worth a table row (the per-frame onupdate pulse is
/// frame-rate noise by design; it collapses into a count).
fn meaningful(timeline: &[TimelineEvent]) -> Vec<&TimelineEvent> {
    timeline
        .iter()
        .filter(|e| !(e.kind == "scenario_event" && e.name == "onupdate"))
        .collect()
}

/// Render the whole self-contained report.
pub fn render_run_report(dir: &Path, artifacts: &RunArtifacts, checks: &[Check]) -> String {
    let verdict = overall_verdict(checks);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!("<title>nova run report - {verdict}</title>\n"));
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n<h1>Run report</h1>\n");
    html.push_str(&format!(
        "<p class=\"meta\">Run dir <code>{}</code></p>\n",
        escape(&dir.display().to_string())
    ));

    // 1. Verdict banner (the CSS class for NO_DATA reuses the warn tint).
    let banner_class = match verdict {
        "OK" => "ok",
        "FAIL" => "fail",
        _ => "warn",
    };
    html.push_str(&format!(
        "<div class=\"banner {banner_class}\">Provisional verdict: {verdict} \
         ({} of {} checks measured)\
         <span class=\"confirm\">Auto checks only - a reviewer (human or agent) \
         must confirm via the checklist at the bottom. SKIPPED = not measured, \
         never held.</span></div>\n",
        measured_count(checks),
        checks.len(),
    ));
    html.push_str("<table>\n<thead><tr><th>check</th><th>status</th><th>value</th><th>threshold</th><th>detail</th></tr></thead>\n<tbody>\n");
    for check in checks {
        html.push_str(&format!(
            "<tr><td>{}</td><td class=\"status-{}\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            check.name,
            check.status.as_str().to_lowercase(),
            check.status.as_str(),
            escape(&check.value),
            escape(&check.threshold),
            escape(&check.detail),
        ));
    }
    html.push_str("</tbody>\n</table>\n");

    // 2. Run summary (manifest identity first, then timeline detail).
    html.push_str("<h2>Run summary</h2>\n");
    if let Some(manifest) = &artifacts.manifest {
        let passes: Vec<String> = manifest
            .passes
            .iter()
            .map(|p| {
                format!(
                    "{}{}",
                    p.name,
                    if p.timed_out {
                        " (TIMED OUT)"
                    } else if !p.success {
                        " (failed)"
                    } else {
                        ""
                    }
                )
            })
            .collect();
        html.push_str(&format!(
            "<p><code>{}</code> via probe run (started unix {}), git <code>{}</code> \
             on <code>{}</code>; passes: {}.</p>\n",
            escape(&manifest.example),
            manifest.started_unix,
            escape(&manifest.git_sha),
            escape(&manifest.host),
            escape(&passes.join(", ")),
        ));
    }
    match &artifacts.timeline {
        Some(timeline) => {
            if let Some(start) = timeline.iter().find(|e| e.kind == "run_start") {
                html.push_str(&format!(
                    "<p>git <code>{}</code> on <code>{}</code>; {} timeline entries",
                    escape(start.data["git_sha"].as_str().unwrap_or("unknown")),
                    escape(start.data["host"].as_str().unwrap_or("unknown")),
                    timeline.len(),
                ));
                if let Some(end) = timeline.iter().rev().find(|e| e.kind == "run_end") {
                    html.push_str(&format!(
                        "; ended frame {} at t={:.1} s",
                        end.frame, end.t_real
                    ));
                }
                html.push_str(".</p>\n");
            }
        }
        None => html.push_str("<p>(no timeline captured)</p>\n"),
    }

    // 3. Correctness.
    html.push_str("<h2>Correctness</h2>\n");
    match &artifacts.timeline {
        None => html.push_str(
            "<p>No timeline captured - arm <code>NOVA_PERF_TIMELINE</code> to record \
             states, scenario events, variables and script beats.</p>\n",
        ),
        Some(timeline) => {
            let by_name = violations_by_name(timeline);
            if by_name.is_empty() {
                html.push_str("<p>No invariant violations recorded.</p>\n");
            } else {
                html.push_str(
                    "<p>Invariant violations by name (a persisting violation repeats \
                     every frame):</p>\n<ul>\n",
                );
                for (name, count) in &by_name {
                    html.push_str(&format!(
                        "<li><code>{}</code> x{count}</li>\n",
                        escape(name)
                    ));
                }
                html.push_str("</ul>\n");
            }
            let rows = meaningful(timeline);
            let pulses = timeline.len() - rows.len();
            html.push_str(&format!(
                "<p>{} meaningful entries ({} per-frame onupdate pulses collapsed):</p>\n",
                rows.len(),
                pulses
            ));
            html.push_str(
                "<details open><summary>run timeline (meaningful entries)</summary>\n\
                 <table>\n<thead><tr><th>frame</th><th>t</th><th>scenario t</th>\
                 <th>kind</th><th>name</th><th>data</th></tr></thead>\n<tbody>\n",
            );
            const MAX_ROWS: usize = 200;
            for entry in rows.iter().take(MAX_ROWS) {
                html.push_str(&format!(
                    "<tr><td class=\"num\">{}</td><td class=\"num\">{:.2}</td>\
                     <td class=\"num\">{}</td><td>{}</td><td>{}</td><td><code>{}</code></td></tr>\n",
                    entry.frame,
                    entry.t_real,
                    entry
                        .scenario_elapsed
                        .map(|s| format!("{s:.2}"))
                        .unwrap_or_else(|| "-".into()),
                    escape(&entry.kind),
                    escape(&entry.name),
                    escape(&entry.data.to_string().chars().take(120).collect::<String>()),
                ));
            }
            html.push_str("</tbody>\n</table>\n");
            if rows.len() > MAX_ROWS {
                html.push_str(&format!(
                    "<p>({} more meaningful entries in <code>timeline.jsonl</code>)</p>\n",
                    rows.len() - MAX_ROWS
                ));
            }
            html.push_str("</details>\n");
        }
    }

    // 4. Performance (the absorbed perf_report as a section).
    html.push_str("<h2>Performance</h2>\n");
    match &artifacts.runs {
        None => html.push_str(
            "<p>No frame-time capture in this run dir - probe run --fps (a \
             wired example) or the sweep matrix flags produce frametime.csv.</p>\n",
        ),
        Some(runs) => {
            html.push_str(&render_chart(runs));
            let baseline_map = artifacts
                .baseline
                .as_ref()
                .map(|base| base.iter().map(|run| (run.label.as_str(), run)).collect())
                .unwrap_or_default();
            html.push_str(&render_table(
                runs,
                "run",
                &baseline_map,
                artifacts.baseline.is_some(),
            ));
            // Looped captures: scene reloads are EXCLUDED from the stats
            // above (their count is host-speed-dependent) and reported as
            // their own number - scene-loading cost stays visible instead
            // of smearing someone else's percentile tail.
            for (label, intervals) in &artifacts.reloads {
                let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
                let max = intervals.iter().cloned().fold(0.0_f64, f64::max);
                html.push_str(&format!(
                    "<p class=\"note\">{}: {} scene reload(s) during the looped \
                     capture - mean {:.1} ms, max {:.1} ms - excluded from the \
                     stats above.</p>\n",
                    crate::report::escape(label),
                    intervals.len(),
                    mean,
                    max
                ));
            }
        }
    }

    // 5. Profile (top-N; ranking only - see the module docs).
    html.push_str("<h2>Profile</h2>\n");
    match &artifacts.costs {
        None => html.push_str(
            "<p>No trace in this run dir - probe run --profile produces \
             trace.json; open it in Perfetto for the deep dive.</p>\n",
        ),
        Some(costs) => {
            html.push_str(
                "<p>Top systems by total span time - traced-run numbers RANK \
                 systems; they never compare against the clean pass, and \
                 shares overlap (parent and child spans both count), so they \
                 must not be summed.</p>\n",
            );
            html.push_str(
                "<table>\n<thead><tr><th>#</th><th>system</th><th>calls</th>\
                 <th>total ms</th><th>mean ms/call</th><th>share</th></tr></thead>\n<tbody>\n",
            );
            for (i, cost) in costs.iter().take(15).enumerate() {
                html.push_str(&format!(
                    "<tr><td class=\"num\">{}</td><td><code>{}</code></td>\
                     <td class=\"num\">{}</td><td class=\"num\">{:.2}</td>\
                     <td class=\"num\">{:.4}</td><td class=\"num\">{:.1}%</td></tr>\n",
                    i + 1,
                    escape(&cost.name),
                    cost.calls,
                    cost.total_ms,
                    cost.mean_ms_per_call,
                    cost.share_pct,
                ));
            }
            html.push_str("</tbody>\n</table>\n");
        }
    }

    // 6. Log tail (collapsible).
    html.push_str("<h2>Log</h2>\n");
    match &artifacts.log {
        None => html.push_str("<p>No run.log captured.</p>\n"),
        Some(log) => {
            let lines: Vec<&str> = log.lines().collect();
            let tail: Vec<&str> = lines.iter().rev().take(60).rev().copied().collect();
            html.push_str(&format!(
                "<details><summary>last {} of {} log lines</summary>\n<pre>{}</pre>\n</details>\n",
                tail.len(),
                lines.len(),
                escape(&tail.join("\n")),
            ));
        }
    }

    // 7. Reviewer checklist.
    html.push_str(
        "<h2>What to check (reviewer)</h2>\n<ol class=\"checklist\">\n\
         <li>Does the verdict banner match your reading of the rows? A SKIPPED \
         check means NOT MEASURED, never \"held\".</li>\n\
         <li>If <code>fps_within_baseline</code> is WARN: was the host quiet? Is the \
         delta consistent across labels, or one noisy row?</li>\n\
         <li>Scan the timeline: do the script beats and scenario events tell the \
         story this run was supposed to tell? Anything unexpected between them?</li>\n\
         <li>If invariants fired: which name, which frame - open timeline.jsonl \
         at that frame for the surrounding events.</li>\n\
         <li>If a system jumped in the profile table: open trace.json in Perfetto \
         (and the samply profile if captured) before concluding.</li>\n\
         </ol>\n\
         <p class=\"oknok\">Reviewer verdict: OK / NOT OK (delete one) - \
         reasoning:</p>\n",
    );

    html.push_str(
        "<footer>Generated by <code>nova_probe run_report</code>; \
         machine-readable mirror in <code>checks.json</code>.</footer>\n",
    );
    html.push_str("</body>\n</html>\n");
    html
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU32, Ordering},
    };

    use super::*;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/run-mini")
    }

    /// A scratch run dir seeded from the fixture, for mutation tests.
    fn scratch_run_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("nova_run_report_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for entry in std::fs::read_dir(fixture()).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), dir.join(entry.file_name())).unwrap();
        }
        dir
    }

    fn check<'a>(checks: &'a [Check], name: &str) -> &'a Check {
        checks.iter().find(|c| c.name == name).unwrap()
    }

    #[test]
    fn healthy_fixture_passes_every_present_check() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "run_completed").status, CheckStatus::Pass);
        assert_eq!(check(&checks, "reached_playing").status, CheckStatus::Pass);
        assert_eq!(check(&checks, "invariants_held").status, CheckStatus::Pass);
        // No manifest in the fixture -> exit status unknowable.
        assert_eq!(check(&checks, "process_exit").status, CheckStatus::Skipped);
        // No baseline passed -> FPS check skipped even though runs exist.
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::Skipped
        );
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Pass);
        assert_eq!(overall_verdict(&checks), "OK");
        assert_eq!(measured_count(&checks), 4);
    }

    #[test]
    fn truncated_timeline_fails_run_completed() {
        let dir = scratch_run_dir();
        // Drop the run_end line: flush-per-entry semantics say truncation
        // is the crash signal.
        let path = dir.join("timeline.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .filter(|l| !l.contains("\"run_end\""))
            .collect();
        std::fs::write(&path, kept.join("\n")).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.value.contains("truncated"), "{c:?}");
        assert_eq!(overall_verdict(&checks), "FAIL");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn violations_fail_invariants_with_per_name_counts() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        let mut contents = std::fs::read_to_string(&path).unwrap();
        // Two violations of one name + a summary reporting them: the check
        // fails and the detail carries the per-name count.
        let violation = r#"{"t_real":3.0,"frame":90,"scenario_elapsed":null,"kind":"invariant","name":"health_bounds","data":{"current":-1.0}}"#;
        // Anchor on the SUMMARY line's unique frame, not the shared t_real
        // prefix (run_end shares t_real=4.0 - the first version doubled the
        // insertion and planted x4).
        contents = contents.replace(
            "{\"t_real\":4.0,\"frame\":118",
            &format!("{violation}\n{violation}\n{{\"t_real\":4.0,\"frame\":118"),
        );
        contents = contents.replace(
            "\"data\":{\"checks\":120,\"violations\":0}",
            "\"data\":{\"checks\":120,\"violations\":2}",
        );
        std::fs::write(&path, contents).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "invariants_held");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("health_bounds x2"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fps_gate_warns_beyond_threshold_and_passes_within() {
        let base_dir = scratch_run_dir();
        let dir = scratch_run_dir();
        // Baseline mean 100 ms; +5% passes, +25% warns (soft).
        std::fs::write(
            base_dir.join("frametime.csv"),
            "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\nscene-high,100,100.0,90.0,120.0,99.0,110.0,115.0,120.0,10.0,8.7\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("frametime.csv"),
            "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\nscene-high,100,105.0,90.0,120.0,99.0,110.0,115.0,120.0,9.5,8.7\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::Pass
        );

        std::fs::write(
            dir.join("frametime.csv"),
            "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\nscene-high,100,125.0,90.0,150.0,120.0,140.0,145.0,150.0,8.0,6.9\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(c.status, CheckStatus::Warn, "{c:?}");
        assert!(c.value.contains("+25.0%"), "{c:?}");
        // A WARN alone never fails the run (soft gate).
        assert_eq!(overall_verdict(&checks), "WARN");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn ansi_colored_error_line_still_fails_log_clean() {
        let dir = scratch_run_dir();
        // A TTY-captured bevy log wraps the level in color codes; the scan
        // must see through them.
        std::fs::write(
            dir.join("run.log"),
            "ok line\n2026-07-19T10:00:00Z \u{1b}[31m ERROR \u{1b}[0m something broke\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn planted_panic_fails_log_clean() {
        let dir = scratch_run_dir();
        std::fs::write(
            dir.join("run.log"),
            "INFO fine\nthread 'main' panicked at src/x.rs:1:1:\nboom\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zero_evidence_is_no_data_never_ok() {
        let dir = scratch_run_dir();
        for name in ["timeline.jsonl", "frametime.csv", "trace.json", "run.log"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert!(checks.iter().all(|c| c.status == CheckStatus::Skipped));
        assert_eq!(measured_count(&checks), 0);
        // A dir with zero evidence must never read as a passing run
        // (finding 4: the live repro said OK over nothing).
        assert_eq!(overall_verdict(&checks), "NO_DATA");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_html_carries_every_section_and_skip_reasons() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        let html = render_run_report(&fixture(), &artifacts, &checks);

        for marker in [
            "Provisional verdict: OK",
            "reviewer (human or agent)",
            "<h2>Run summary</h2>",
            "<h2>Correctness</h2>",
            "onupdate pulses collapsed",
            "<h2>Performance</h2>",
            "<h2>Profile</h2>",
            "must not be summed",
            "<h2>Log</h2>",
            "What to check (reviewer)",
            "Reviewer verdict: OK / NOT OK",
        ] {
            assert!(html.contains(marker), "missing {marker:?}");
        }
        // Self-contained.
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("src=\"http"));
    }

    #[test]
    fn checks_json_mirrors_rows_with_coverage_and_run_identity() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000000,
            git_sha: "abc123".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: false,
            passes: vec![PassRecord {
                name: "clean".into(),
                success: true,
                timed_out: false,
            }],
        };
        let json = checks_json(&checks, Some(&manifest));
        assert_eq!(json["verdict"], "OK");
        assert_eq!(json["measured"], "4/6");
        assert_eq!(json["reviewer_confirmation_required"], true);
        assert_eq!(json["run"]["example"], "playable");
        assert_eq!(json["run"]["passes"][0]["name"], "clean");
        assert_eq!(json["checks"].as_array().unwrap().len(), checks.len());
        // process_exit leads the rows and carries structured data.
        assert_eq!(json["checks"][0]["name"], "process_exit");
        let inv = json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "invariants_held")
            .unwrap();
        assert_eq!(inv["data"]["violations"], 0);
    }

    #[test]
    fn manifest_round_trips_and_drives_process_exit() {
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000123,
            git_sha: "abc123".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: true,
            passes: vec![
                PassRecord {
                    name: "clean".into(),
                    success: false,
                    timed_out: true,
                },
                PassRecord {
                    name: "profiled".into(),
                    success: true,
                    timed_out: false,
                },
            ],
        };
        let parsed = RunManifest::from_json(&manifest.to_json().to_string()).expect("round-trips");
        assert_eq!(parsed, manifest);

        // A timed-out clean pass is a process_exit FAIL...
        let artifacts = RunArtifacts {
            reloads: Vec::new(),
            manifest: Some(parsed),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "process_exit");
        assert_eq!(c.status, CheckStatus::Fail);
        // The all-passes shape: the count in value, the names + timeout
        // markers in detail/data.
        assert!(c.value.contains("1/1 pass(es) failed"), "{c:?}");
        assert!(c.detail.contains("clean (timed out)"), "{c:?}");
        assert_eq!(c.data["failed"][0]["timed_out"], true);
        // ...and the verdict is FAIL even though everything else skipped
        // (a hung run must produce a failing report, finding 2).
        assert_eq!(overall_verdict(&checks), "FAIL");

        // A failed (non-timeout) exit also fails.
        let artifacts = RunArtifacts {
            reloads: Vec::new(),
            manifest: Some(RunManifest {
                passes: vec![PassRecord {
                    name: "clean".into(),
                    success: false,
                    timed_out: false,
                }],
                ..manifest_ok()
            }),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "process_exit").status, CheckStatus::Fail);
    }

    fn manifest_ok() -> RunManifest {
        RunManifest {
            example: "controller_section".into(),
            started_unix: 1,
            git_sha: "abc".into(),
            host: "h".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: false,
            passes: vec![PassRecord {
                name: "clean".into(),
                success: true,
                timed_out: false,
            }],
        }
    }

    #[test]
    fn armed_but_unwired_skip_details_name_the_wiring_not_the_env() {
        // The live repro's misdirection: probe DID arm the env; the detail
        // must say "not wired", not "arm NOVA_PERF_TIMELINE".
        let artifacts = RunArtifacts {
            reloads: Vec::new(),
            manifest: Some(manifest_ok()),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Skipped);
        assert!(c.detail.contains("not wired with"), "{}", c.detail);
        assert!(c.detail.contains("controller_section"), "{}", c.detail);
        // And the verdict is OK-with-coverage (process_exit measured PASS),
        // not NO_DATA and not the old evidence-free OK.
        assert_eq!(overall_verdict(&checks), "OK");
        assert_eq!(measured_count(&checks), 1);
    }

    #[test]
    fn reached_playing_fails_when_the_run_never_left_loading() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        let contents = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = contents
            .lines()
            .filter(|l| !l.contains("\"entered\":\"Playing\""))
            .collect();
        // Keep the file bracket-consistent: drop one entry, patch run_end's
        // count down by one.
        let patched = kept.join("\n").replace("\"entries\":10", "\"entries\":9");
        std::fs::write(&path, patched).unwrap();

        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert_eq!(check(&checks, "reached_playing").status, CheckStatus::Fail);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn swallowed_writes_fail_the_entry_cross_check() {
        let dir = scratch_run_dir();
        let path = dir.join("timeline.jsonl");
        // Claim more entries than the file holds: ENOSPC's signature.
        let contents = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\"entries\":10", "\"entries\":14");
        std::fs::write(&path, contents).unwrap();
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "run_completed");
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.value.contains("14 written vs 10 on disk"), "{c:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fps_improvement_passes_and_line_initial_error_is_caught() {
        let base_dir = scratch_run_dir();
        let dir = scratch_run_dir();
        std::fs::write(
            base_dir.join("frametime.csv"),
            "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\nscene-high,100,100.0,90.0,120.0,99.0,110.0,115.0,120.0,10.0,8.7\n",
        )
        .unwrap();
        // 50% FASTER: must PASS with the improvement noted, never WARN.
        std::fs::write(
            dir.join("frametime.csv"),
            "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\nscene-high,100,50.0,45.0,60.0,49.0,55.0,58.0,60.0,20.0,17.2\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "fps_within_baseline");
        assert_eq!(c.status, CheckStatus::Pass, "{c:?}");
        assert!(c.value.contains("improved"), "{c:?}");

        // Log scan: a line-INITIAL ERROR is caught (the old substring
        // needed surrounding spaces), and a word merely containing it is
        // not.
        std::fs::write(
            dir.join("run.log"),
            "ERROR boot diagnostics failed\nnoting TERRORD is fine\n",
        )
        .unwrap();
        let artifacts = RunArtifacts::load(&dir, Some(&base_dir)).unwrap();
        let checks = evaluate_checks(&artifacts);
        let c = check(&checks, "log_clean");
        assert_eq!(c.status, CheckStatus::Fail, "{c:?}");
        assert_eq!(c.data["offending"], 1, "TERRORD must not count: {c:?}");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&base_dir);
    }
}
