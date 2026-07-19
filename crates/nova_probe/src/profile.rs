//! Chrome-trace post-processing: turn the JSON that a `--features trace`
//! run writes (bevy's `trace_chrome`, output path via the `TRACE_CHROME`
//! env var) into a top-N costliest-systems table - the profiling layer of
//! the run-harness (spike tasks/20260719-112011/SPIKE.md, task
//! 20260719-112253).
//!
//! Bevy has NO per-system timing diagnostic; per-system costs exist only as
//! tracing SPANS compiled in under `bevy/trace`
//! (bevy_ecs-0.19.0/src/system/function_system.rs:52,
//! `info_span!(parent: None, "system", name = ...)`), which bevy_log's
//! chrome layer renders as `"system: name=<path>"` entries. This module
//! aggregates those spans; everything else in the trace (schedules, render
//! internals, commands) is left for Perfetto - attach the raw JSON for the
//! deep dive.
//!
//! Honesty note: costs are reported per CALL and as a share of TOTAL
//! system-span time. Bevy 0.19 has no reliable universal frame span, so
//! per-frame figures would be fabricated - the FPS pass (the clean,
//! untraced run) owns frame-time truth. Tracing overhead inflates every
//! number here; use them to RANK systems, not to compare against the clean
//! pass (the two-pass rule, spike review M2).

use std::collections::HashMap;

/// One system's aggregated cost over a trace.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemCost {
    /// The system's full path (the span's `name=` field).
    pub name: String,
    /// Times the span was entered (roughly: runs).
    pub calls: u64,
    /// Total time inside the span, milliseconds.
    pub total_ms: f64,
    /// Mean time per call, milliseconds.
    pub mean_ms_per_call: f64,
    /// Share of the summed system-span time, percent.
    pub share_pct: f64,
}

/// Parse a chrome-trace JSON file (the `trace_chrome` output) and aggregate
/// the per-system spans into costs, sorted by total time descending.
///
/// Handles both duration styles the format allows: `B`/`E` begin-end pairs
/// (what tracing_chrome emits; paired per `tid` as a stack) and complete
/// `X` events carrying `dur`. Timestamps and durations are microseconds
/// (the chrome contract). Non-system spans are counted into nothing - they
/// stay in the raw file for Perfetto. A file that is not a JSON array is
/// rejected loudly (a killed run can truncate the file; profile clean
/// exits).
pub fn aggregate_system_costs(contents: &str) -> Result<Vec<SystemCost>, String> {
    let events: serde_json::Value =
        serde_json::from_str(contents).map_err(|e| format!("not a chrome-trace JSON file: {e}"))?;
    let events = events
        .as_array()
        .ok_or("chrome trace must be a JSON array of events")?;

    // Pair B/E per tid with a stack (chrome semantics: E closes the most
    // recent open B on the same thread); X events carry their duration.
    let mut open: HashMap<i64, Vec<(String, f64)>> = HashMap::new();
    let mut totals: HashMap<String, (u64, f64)> = HashMap::new();
    let mut record = |name: &str, dur_us: f64| {
        if let Some(system) = name.strip_prefix("system: name=") {
            // The field value arrives quoted (`name="path::to::system"`) -
            // bevy renders it via DebugName's Debug impl. Trim the quotes so
            // the table shows the bare path.
            let system = system.trim_matches('"');
            let entry = totals.entry(system.to_string()).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += dur_us;
        }
    };
    for event in events {
        let phase = event.get("ph").and_then(|p| p.as_str()).unwrap_or("");
        let tid = event.get("tid").and_then(|t| t.as_i64()).unwrap_or(0);
        match phase {
            "B" => {
                let name = event
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();
                let ts = event.get("ts").and_then(|t| t.as_f64()).unwrap_or(0.0);
                open.entry(tid).or_default().push((name, ts));
            }
            "E" => {
                let ts = event.get("ts").and_then(|t| t.as_f64()).unwrap_or(0.0);
                if let Some((name, begin)) = open.entry(tid).or_default().pop() {
                    record(&name, (ts - begin).max(0.0));
                }
            }
            "X" => {
                let name = event
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();
                let dur = event.get("dur").and_then(|d| d.as_f64()).unwrap_or(0.0);
                record(name, dur);
            }
            _ => {}
        }
    }

    let grand_total_us: f64 = totals.values().map(|(_, us)| us).sum();
    let mut costs: Vec<SystemCost> = totals
        .into_iter()
        .map(|(name, (calls, us))| SystemCost {
            name,
            calls,
            total_ms: us / 1000.0,
            mean_ms_per_call: us / 1000.0 / calls.max(1) as f64,
            share_pct: if grand_total_us > 0.0 {
                us / grand_total_us * 100.0
            } else {
                0.0
            },
        })
        .collect();
    costs.sort_by(|a, b| {
        b.total_ms
            .partial_cmp(&a.total_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(costs)
}

/// Render the top `n` costs as a markdown table (also readable as plain
/// text). The header names the honesty constraints so a pasted table cannot
/// silently overclaim.
pub fn render_top_table(costs: &[SystemCost], n: usize) -> String {
    let mut out = String::from(
        "Top systems by total span time (traced run - use to RANK, not to \
         compare with the clean pass):\n\n\
         | # | system | calls | total ms | mean ms/call | share |\n\
         |--:|--------|------:|---------:|-------------:|------:|\n",
    );
    for (i, cost) in costs.iter().take(n).enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} | {:.2} | {:.4} | {:.1}% |\n",
            i + 1,
            cost.name,
            cost.calls,
            cost.total_ms,
            cost.mean_ms_per_call,
            cost.share_pct,
        ));
    }
    if costs.len() > n {
        out.push_str(&format!(
            "\n({} more systems below the top {n}; open the raw trace in \
             Perfetto for the full picture)\n",
            costs.len() - n
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-written trace: two systems (one via B/E pairs on two tids,
    /// one via an X event), one nested non-system span, one non-system X.
    /// Values chosen so every aggregate is a round literal.
    fn fixture() -> String {
        r#"[
            {"ph":"B","ts":1000.0,"tid":1,"name":"system: name=game::alpha"},
            {"ph":"B","ts":1200.0,"tid":1,"name":"check_conditions: name=game::alpha"},
            {"ph":"E","ts":1300.0,"tid":1},
            {"ph":"E","ts":2000.0,"tid":1},
            {"ph":"B","ts":1000.0,"tid":2,"name":"system: name=game::alpha"},
            {"ph":"E","ts":2000.0,"tid":2},
            {"ph":"X","ts":3000.0,"tid":1,"dur":500.0,"name":"system: name=\"game::beta\""},
            {"ph":"X","ts":4000.0,"tid":1,"dur":9000.0,"name":"multithreaded executor"}
        ]"#
        .to_string()
    }

    #[test]
    fn aggregates_be_pairs_and_x_events_with_literal_values() {
        let costs = aggregate_system_costs(&fixture()).expect("fixture parses");
        assert_eq!(
            costs.len(),
            2,
            "non-system spans are not counted: {costs:?}"
        );

        // alpha: two 1000 us calls (nested non-system span pops first on
        // tid 1 - stack pairing) = 2.0 ms total, 1.0 ms mean.
        assert_eq!(costs[0].name, "game::alpha");
        assert_eq!(costs[0].calls, 2);
        assert!((costs[0].total_ms - 2.0).abs() < 1e-9);
        assert!((costs[0].mean_ms_per_call - 1.0).abs() < 1e-9);
        // share: 2000 of 2500 us = 80%.
        assert!((costs[0].share_pct - 80.0).abs() < 1e-9);

        // beta: one X event, 500 us = 0.5 ms, 20%.
        assert_eq!(costs[1].name, "game::beta");
        assert_eq!(costs[1].calls, 1);
        assert!((costs[1].total_ms - 0.5).abs() < 1e-9);
        assert!((costs[1].share_pct - 20.0).abs() < 1e-9);
    }

    #[test]
    fn renderer_cuts_to_top_n_and_notes_the_rest() {
        let costs = aggregate_system_costs(&fixture()).expect("fixture parses");
        let table = render_top_table(&costs, 1);
        assert!(table.contains("game::alpha"), "{table}");
        assert!(!table.contains("game::beta"), "cut to top 1: {table}");
        assert!(table.contains("1 more systems below the top 1"), "{table}");
        assert!(
            table.contains("RANK, not to"),
            "honesty note present: {table}"
        );
        let full = render_top_table(&costs, 10);
        assert!(full.contains("game::beta"));
        assert!(!full.contains("more systems below"));
    }

    #[test]
    fn rejects_a_non_array_file() {
        assert!(aggregate_system_costs("{}").is_err());
        assert!(aggregate_system_costs("not json").is_err());
    }

    #[test]
    fn empty_trace_yields_empty_costs() {
        let costs = aggregate_system_costs("[]").expect("empty array parses");
        assert!(costs.is_empty());
    }
}
