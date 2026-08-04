//! The automatic checks over a run: the OK/WARN/FAIL rows, the provisional
//! verdict, and the `checks.json` mirror an agent reads instead of the HTML.

use std::collections::BTreeMap;

use super::{
    artifacts::RunArtifacts,
    manifest::{PassRecord, RunManifest},
};
use crate::recorder::TimelineEvent;

/// Soft FPS gate: the worst same-label mean-frame-time delta against the
/// baseline may move this many percent before the check turns WARN. One
/// tunable on purpose; frame numbers on a shared host are noisy, so this is
/// a flag for the reviewer, never a hard failure.
pub const FPS_WARN_THRESHOLD_PCT: f64 = 10.0;

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
    /// The property held.
    Pass,
    /// A soft-gate concern; never fails the run.
    Warn,
    /// The property was violated; fails the run.
    Fail,
    /// The input artifact was not captured, so the check could not run.
    Skipped,
}

impl CheckStatus {
    pub(super) fn as_str(self) -> &'static str {
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
pub(super) fn violations_by_name(timeline: &[TimelineEvent]) -> BTreeMap<String, u64> {
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
    // on whether probe ARMED the surface.
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

#[cfg(test)]
mod tests {
    use super::{super::fixtures::*, *};

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
        // A dir with zero evidence must never read as a passing run.
        assert_eq!(overall_verdict(&checks), "NO_DATA");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn checks_json_mirrors_rows_with_coverage_and_run_identity() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000000,
            git_sha: "abc123".into(),
            full_git_sha: "abc123def".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: false,
            fps_skipped: None,
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
            full_git_sha: "abc123def".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: true,
            fps_skipped: Some("narrative scenario".into()),
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
        // (a hung run must produce a failing report).
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
