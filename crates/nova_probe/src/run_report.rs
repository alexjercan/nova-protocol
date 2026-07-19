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

use std::{collections::BTreeMap, path::Path};

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
        let log = read_opt("run.log")?;
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

/// The provisional overall verdict: FAIL if any hard check failed, WARN if
/// anything warned, OK otherwise. The reviewer owns the final call.
pub fn overall_verdict(checks: &[Check]) -> &'static str {
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
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

    // run_completed: the timeline must CLOSE. Flush-per-entry means a
    // panicked/killed run leaves a bracket-less file - that absence is the
    // crash signal, by design.
    checks.push(match &artifacts.timeline {
        None => Check {
            name: "run_completed",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: "run_end present + AppExit Success".into(),
            detail: "timeline.jsonl not captured (arm NOVA_PERF_TIMELINE)".into(),
        },
        Some(timeline) => {
            let end = timeline.iter().rev().find(|e| e.kind == "run_end");
            match end {
                Some(end) if end.data["exit"].as_str().unwrap_or("").contains("Success") => Check {
                    name: "run_completed",
                    status: CheckStatus::Pass,
                    value: format!("run_end at frame {}", end.frame),
                    threshold: "run_end present + AppExit Success".into(),
                    detail: "the run closed its bracket cleanly".into(),
                },
                Some(end) => Check {
                    name: "run_completed",
                    status: CheckStatus::Fail,
                    value: format!("exit: {}", end.data["exit"]),
                    threshold: "run_end present + AppExit Success".into(),
                    detail: "the run ended with a non-success exit".into(),
                },
                None => Check {
                    name: "run_completed",
                    status: CheckStatus::Fail,
                    value: "timeline truncated (no run_end)".into(),
                    threshold: "run_end present + AppExit Success".into(),
                    detail: "flush-per-entry means truncation = the run died mid-flight".into(),
                },
            }
        }
    });

    // invariants_held: the summary entry carries the tally; per-name counts
    // ride in the detail.
    checks.push(match &artifacts.timeline {
        None => Check {
            name: "invariants_held",
            status: CheckStatus::Skipped,
            value: "no timeline".into(),
            threshold: "0 violations".into(),
            detail: "timeline.jsonl not captured".into(),
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
                    detail: "invariants not armed (arm NOVA_PERF_INVARIANTS)".into(),
                },
                summary => {
                    let violations = summary
                        .map(|s| s.data["violations"].as_u64().unwrap_or(0))
                        .unwrap_or_else(|| by_name.values().sum());
                    let checks_run = summary
                        .map(|s| s.data["checks"].as_u64().unwrap_or(0))
                        .unwrap_or(0);
                    if violations == 0 {
                        Check {
                            name: "invariants_held",
                            status: CheckStatus::Pass,
                            value: format!("0 violations over {checks_run} checked frames"),
                            threshold: "0 violations".into(),
                            detail: "every engine-guaranteed bound held".into(),
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
                        }
                    }
                }
            }
        }
    });

    // fps_within_baseline: soft gate, same labels only.
    checks.push(match (&artifacts.runs, &artifacts.baseline) {
        (Some(runs), Some(baseline)) => {
            let mut worst: Option<(String, f64)> = None;
            for run in runs {
                if let Some(base) = baseline.iter().find(|b| b.label == run.label) {
                    if base.stats.mean_ms > 0.0 {
                        let delta =
                            (run.stats.mean_ms - base.stats.mean_ms) / base.stats.mean_ms * 100.0;
                        if worst.as_ref().is_none_or(|(_, w)| delta.abs() > w.abs()) {
                            worst = Some((run.label.clone(), delta));
                        }
                    }
                }
            }
            match worst {
                None => Check {
                    name: "fps_within_baseline",
                    status: CheckStatus::Skipped,
                    value: "no matching labels".into(),
                    threshold: format!("|mean delta| <= {FPS_WARN_THRESHOLD_PCT}%"),
                    detail: "baseline shares no run labels with this capture".into(),
                },
                Some((label, delta)) if delta.abs() <= FPS_WARN_THRESHOLD_PCT => Check {
                    name: "fps_within_baseline",
                    status: CheckStatus::Pass,
                    value: format!("worst {label}: {delta:+.1}%"),
                    threshold: format!("|mean delta| <= {FPS_WARN_THRESHOLD_PCT}%"),
                    detail: "mean frame time within the soft gate".into(),
                },
                Some((label, delta)) => Check {
                    name: "fps_within_baseline",
                    status: CheckStatus::Warn,
                    value: format!("worst {label}: {delta:+.1}%"),
                    threshold: format!("|mean delta| <= {FPS_WARN_THRESHOLD_PCT}%"),
                    detail: "soft gate: frame numbers are host-noisy; reviewer judges \
                             (was the host quiet? is the delta consistent?)"
                        .into(),
                },
            }
        }
        _ => Check {
            name: "fps_within_baseline",
            status: CheckStatus::Skipped,
            value: "missing capture or baseline".into(),
            threshold: format!("|mean delta| <= {FPS_WARN_THRESHOLD_PCT}%"),
            detail: "needs both frametime.csv and --baseline <dir>".into(),
        },
    });

    // log_clean: panics and ERROR lines are hard failures.
    checks.push(match &artifacts.log {
        None => Check {
            name: "log_clean",
            status: CheckStatus::Skipped,
            value: "no run.log".into(),
            threshold: "no panics / ERROR lines".into(),
            detail: "log not captured alongside the run".into(),
        },
        Some(log) => {
            // Strip ANSI escapes first: a TTY-captured log wraps the level
            // in color codes and the exact-substring scan would miss it.
            let cleaned: Vec<String> = log.lines().map(strip_ansi).collect();
            let bad: Vec<&String> = cleaned
                .iter()
                .filter(|line| line.contains("panicked at") || line.contains(" ERROR "))
                .take(5)
                .collect();
            if bad.is_empty() {
                Check {
                    name: "log_clean",
                    status: CheckStatus::Pass,
                    value: "0 panic/ERROR lines".into(),
                    threshold: "no panics / ERROR lines".into(),
                    detail: "log scanned clean".into(),
                }
            } else {
                Check {
                    name: "log_clean",
                    status: CheckStatus::Fail,
                    value: format!("{} offending line(s)", bad.len()),
                    threshold: "no panics / ERROR lines".into(),
                    detail: format!("first: {}", bad[0].chars().take(160).collect::<String>()),
                }
            }
        }
    });

    checks
}

/// The machine-readable mirror of the verdict rows.
pub fn checks_json(checks: &[Check]) -> serde_json::Value {
    serde_json::json!({
        "verdict": overall_verdict(checks),
        "reviewer_confirmation_required": true,
        "checks": checks.iter().map(|c| serde_json::json!({
            "name": c.name,
            "status": c.status.as_str(),
            "value": c.value,
            "threshold": c.threshold,
            "detail": c.detail,
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

    // 1. Verdict banner.
    html.push_str(&format!(
        "<div class=\"banner {}\">Provisional verdict: {verdict}\
         <span class=\"confirm\">Auto checks only - a reviewer (human or agent) \
         must confirm via the checklist at the bottom.</span></div>\n",
        verdict.to_lowercase()
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

    // 2. Run summary (from the timeline's run_start, when present).
    html.push_str("<h2>Run summary</h2>\n");
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
            "<p>No frame-time capture in this run dir - the CLEAN pass \
             (scripts/perf-baseline.sh, no tracing) produces frametime.csv.</p>\n",
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
        }
    }

    // 5. Profile (top-N; ranking only - see the module docs).
    html.push_str("<h2>Profile</h2>\n");
    match &artifacts.costs {
        None => html.push_str(
            "<p>No trace in this run dir - the PROFILED pass \
             (scripts/perf-profile.sh) produces trace.json; open it in \
             Perfetto for the deep dive.</p>\n",
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
        assert_eq!(check(&checks, "invariants_held").status, CheckStatus::Pass);
        // No baseline passed -> FPS check skipped even though runs exist.
        assert_eq!(
            check(&checks, "fps_within_baseline").status,
            CheckStatus::Skipped
        );
        assert_eq!(check(&checks, "log_clean").status, CheckStatus::Pass);
        assert_eq!(overall_verdict(&checks), "OK");
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
    fn missing_artifacts_are_skipped_never_passed() {
        let dir = scratch_run_dir();
        for name in ["timeline.jsonl", "frametime.csv", "trace.json", "run.log"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
        let artifacts = RunArtifacts::load(&dir, None).unwrap();
        let checks = evaluate_checks(&artifacts);
        assert!(checks.iter().all(|c| c.status == CheckStatus::Skipped));
        // All-skipped is OK-with-nothing-measured; the report says so and
        // the reviewer checklist makes SKIPPED explicit.
        assert_eq!(overall_verdict(&checks), "OK");
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
    fn checks_json_mirrors_the_rows() {
        let artifacts = RunArtifacts::load(&fixture(), None).expect("fixture loads");
        let checks = evaluate_checks(&artifacts);
        let json = checks_json(&checks);
        assert_eq!(json["verdict"], "OK");
        assert_eq!(json["reviewer_confirmation_required"], true);
        assert_eq!(json["checks"].as_array().unwrap().len(), checks.len());
        assert_eq!(json["checks"][0]["name"], "run_completed");
        assert_eq!(json["checks"][0]["status"], "PASS");
    }
}
