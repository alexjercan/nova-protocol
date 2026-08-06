//! The `report.html` renderer: every dimension gets a section, and a
//! missing artifact prints why instead of vanishing.

use std::path::Path;

use super::{
    artifacts::RunArtifacts,
    checks::{measured_count, overall_verdict, violations_by_name, Check},
};
use crate::{
    recorder::TimelineEvent,
    report::{escape, render_chart, render_table, STYLE},
};

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
         must confirm via the checklist at the bottom. SKIPPED = not measured \
         and N/A = nothing claimed; neither ever means \"held\".</span></div>\n",
        measured_count(checks),
        checks.len(),
    ));
    html.push_str("<table>\n<thead><tr><th>check</th><th>status</th><th>value</th><th>threshold</th><th>detail</th></tr></thead>\n<tbody>\n");
    for check in checks {
        html.push_str(&format!(
            "<tr><td>{}</td><td class=\"status-{}\">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            check.name,
            crate::run_report::status_class(check.status.as_str()),
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
    // A capture the example never claimed is not a missing one. The contract
    // says which, so a reader does not chase a number this example was never
    // written to produce.
    let unclaimed = artifacts
        .contract
        .as_ref()
        .is_some_and(|c| !c.declares(crate::contract::Capability::FrameTime));
    match (&artifacts.runs, unclaimed) {
        (None, true) => html.push_str(&format!(
            "<p class=\"note\">no frame-time claim: this example wires no \
             <code>{}</code>, so it makes no frame-cost assertion and there is \
             no window to measure. It runs for correctness only.</p>\n",
            crate::report::escape(crate::contract::Capability::FrameTime.wiring()),
        )),
        (None, false) => html.push_str(
            "<p>No frame-time capture in this run dir - probe run --fps (a \
             wired example) or the sweep matrix flags produce frametime.csv.</p>\n",
        ),
        (Some(runs), _) => {
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
         check means NOT MEASURED and an N/A one means the example claims no \
         such thing - neither means \"held\". An UNPROBEABLE verdict means the \
         run graded no claim at all.</li>\n\
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
    use super::{
        super::{
            checks::{evaluate_checks, CheckStatus},
            fixtures::*,
        },
        *,
    };
    use crate::contract::{Capability, ProbeContract};

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
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("src=\"http"));
    }

    #[test]
    fn an_unclaimed_frame_time_renders_an_honest_note_not_a_missing_capture() {
        // No frametime.csv (runs = None) AND a contract that declares no
        // FrameTime: the Performance section must say the example makes no
        // such claim, not the generic "no capture" line, so a reader is not
        // sent hunting for a window this example never promised.
        let artifacts = RunArtifacts {
            reloads: Vec::new(),
            manifest: Some(manifest_ok()),
            contract: Some(ProbeContract::of([Capability::Timeline])),
            ..Default::default()
        };
        let checks = evaluate_checks(&artifacts);
        let html = render_run_report(Path::new("probe-runs/editor"), &artifacts, &checks);
        assert!(
            html.contains("no frame-time claim"),
            "the contract note is missing: {html}"
        );
        assert!(
            html.contains("nova_frametime()"),
            "the note must name the wiring that would make the claim: {html}"
        );
        assert!(
            !html.contains("No frame-time capture in this run dir"),
            "the generic no-capture message should be replaced by the contract note"
        );
        // process_exit must not FAIL: no fps pass ran, so there is no
        // failed/timed-out pass to flip the verdict.
        assert_ne!(check(&checks, "process_exit").status, CheckStatus::Fail);
    }
}
