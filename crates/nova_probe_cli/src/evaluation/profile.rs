//! Chrome-trace post-processing: turn the JSON that a `--features trace`
//! run writes (bevy's `trace_chrome`, output path via the `TRACE_CHROME`
//! env var) into a top-N costliest-systems table - the profiling layer of
//! the run-harness.
//!
//! Bevy has NO per-system timing diagnostic; per-system costs exist only as
//! tracing SPANS compiled in under `bevy/trace`
//! (bevy_ecs-0.19.0/src/system/function_system.rs:52 and :54,
//! `info_span!(parent: None, "system", name = ...)` and the sibling
//! `"system_commands"`), which bevy_log's chrome layer renders as
//! `"system: name=<path>"` and `"system_commands: name=<path>"` entries. This
//! module aggregates both; everything else in the trace (schedules, render
//! internals) is left for Perfetto - attach the raw JSON for the deep dive.
//!
//! **Both, because half this codebase's expensive work is DEFERRED.** An
//! observer (`add_observer`) never gets a span of its own, and neither does a
//! `commands.queue(|world: &mut World| ...)` closure: they run when the
//! spawning system's commands are applied, inside `system_commands`. Counting
//! only `system` spans therefore reports zero for carve-shard spawning, mesh
//! slicing at death, and every other observer - not "cheap", but invisible.
//!
//! Honesty note: costs are reported per CALL and as a share of TOTAL
//! system-span time. Bevy 0.19 has no reliable universal frame span, so
//! per-frame figures would be fabricated - the FPS pass (the clean,
//! untraced run) owns frame-time truth. Tracing overhead inflates every
//! number here; use them to RANK systems, not to compare against the clean
//! pass (the two-pass rule, spike review M2).
//!
//! **The trace is read as a STREAM and never held.** A traced pass writes
//! about 28 MB per second for as long as it runs, so the file the host is
//! handed is routinely gigabytes; a whole-file read plus a whole-file DOM
//! puts the text and its parse tree in memory at once, which is how one
//! range took the host to 27.8 GB. Nothing here may grow with the file:
//! events are folded in one at a time and dropped, and the two maps that
//! do survive an event are capped.

/// Glob-import surface for the chrome-trace system-cost aggregation.
pub mod prelude {
    pub use super::{aggregate_system_costs, render_top_table, CostKind, SystemCost, TraceProfile};
}

use std::{
    cell::Cell,
    collections::HashMap,
    io::{BufReader, Read},
    rc::Rc,
};

use serde::{
    de::{DeserializeSeed, Error as _, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

/// Which span a cost row came from: the system body, or the flush of the
/// commands it deferred (where its observers and exclusive-world closures
/// actually run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CostKind {
    /// The `system` span: the body of the system itself.
    System,
    /// The `system_commands` span: applying that system's deferred commands -
    /// spawns, despawns, `queue(|world| ...)` closures and every observer they
    /// trigger.
    Commands,
}

impl CostKind {
    /// The chrome-trace span-name prefix this kind is read from.
    fn prefix(self) -> &'static str {
        match self {
            CostKind::System => "system: name=",
            CostKind::Commands => "system_commands: name=",
        }
    }

    /// The suffix the table shows, so a deferred row is never mistaken for
    /// the system's own body.
    fn suffix(self) -> &'static str {
        match self {
            CostKind::System => "",
            CostKind::Commands => " (commands)",
        }
    }
}

/// One system's aggregated cost over a trace.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemCost {
    /// The system's full path (the span's `name=` field).
    pub name: String,
    /// Whether this row is the system's body or its deferred command flush.
    pub kind: CostKind,
    /// Times the span was entered (roughly: runs).
    pub calls: u64,
    /// Total time inside the span, milliseconds.
    pub total_ms: f64,
    /// Mean time per call, milliseconds.
    pub mean_ms_per_call: f64,
    /// Share of the summed system-span time, percent.
    pub share_pct: f64,
}

/// What a trace yielded: the ranked costs, how big the file was, and whether
/// it ran out mid-array.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraceProfile {
    /// Per-system costs, sorted by total span time descending.
    pub costs: Vec<SystemCost>,
    /// The file opened as an array and ended without closing it, so these
    /// costs cover a PREFIX of the run. A traced child killed by the
    /// supervisor timeout ends its file exactly this way, which makes it a
    /// survivable state rather than a corrupt one - but the report has to say
    /// it, because the tail of the run is missing and a system that only ran
    /// late is missing with it.
    pub truncated: bool,
    /// Bytes read off the trace. Nothing caps the writer, so this is
    /// routinely gigabytes; it is reported because an operator whose disk is
    /// filling should learn that from the report rather than from the disk.
    pub bytes: u64,
}

/// A reader that counts what passes through it. Counting here rather than
/// stat'ing a path keeps [`aggregate_system_costs`] a function of a READER -
/// which is what lets the tests hand it a byte slice and the harness hand it
/// an open file - while still reporting how much trace there was.
struct Counted<R> {
    inner: R,
    bytes: Rc<Cell<u64>>,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.bytes.set(self.bytes.get() + read as u64);
        Ok(read)
    }
}

/// One chrome-trace event, in the only four fields this module reads. Every
/// other key - `pid`, `cat`, `args`, tracing_chrome's `.file` and `.line` -
/// is skipped without allocating, which matters because they are the
/// majority of the bytes.
#[derive(Deserialize)]
struct TraceEvent {
    /// The chrome phase: `B` and `E` bracket a span, `X` is a complete one.
    #[serde(default)]
    ph: String,
    /// The thread the span belongs to. `B`/`E` pair per thread, never across.
    #[serde(default)]
    tid: i64,
    /// bevy_log renders a span as `<span name>: <fields>`, so a system reads
    /// `system: name="path::to::system"`.
    #[serde(default)]
    name: String,
    /// Microseconds since trace start (the chrome contract).
    #[serde(default)]
    ts: f64,
    /// Microseconds, on `X` events only.
    #[serde(default)]
    dur: f64,
}

/// Open spans one thread may stack before the trace is refused. Real depth is
/// schedule nesting - single digits - so this only ever trips on a file whose
/// `E` events are missing, which is the one shape that could still grow the
/// host without bound now that the file itself is streamed.
const MAX_OPEN_SPANS_PER_THREAD: usize = 4096;

/// Distinct span names kept before the trace is refused. A binary has a fixed
/// number of systems (low thousands), so passing this means the names are
/// generated rather than compiled - the other shape that could grow without
/// bound.
const MAX_TRACKED_SPANS: usize = 65_536;

/// The running fold: everything that survives one event. Both maps are
/// capped, so the whole aggregation is bounded by the binary's system count
/// and its schedule depth - never by the trace's length.
#[derive(Default)]
struct Aggregate {
    open: HashMap<i64, Vec<(String, f64)>>,
    totals: HashMap<(String, CostKind), (u64, f64)>,
    /// The opening `[` was consumed. Survives the error that ends a truncated
    /// file, which is what tells "ran out mid-array" from "never was one".
    entered: bool,
}

impl Aggregate {
    fn record(&mut self, name: &str, dur_us: f64) -> Result<(), String> {
        // The commands prefix has to be tried FIRST: "system: name=" is not a
        // prefix of "system_commands: name=", but keeping the order explicit
        // stops a future rename from silently folding the two together.
        for kind in [CostKind::Commands, CostKind::System] {
            let Some(system) = name.strip_prefix(kind.prefix()) else {
                continue;
            };
            // The field value arrives quoted (`name="path::to::system"`) -
            // bevy renders it via DebugName's Debug impl. Trim the quotes so
            // the table shows the bare path.
            let system = system.trim_matches('"');
            // One key allocation per event, not two: the cap is read BEFORE
            // the entry so the vacant arm can refuse without a second lookup.
            // This runs tens of millions of times on a real trace.
            let at_cap = self.totals.len() >= MAX_TRACKED_SPANS;
            match self.totals.entry((system.to_string(), kind)) {
                std::collections::hash_map::Entry::Occupied(mut slot) => {
                    let total = slot.get_mut();
                    total.0 += 1;
                    total.1 += dur_us;
                }
                std::collections::hash_map::Entry::Vacant(slot) => {
                    if at_cap {
                        return Err(format!(
                            "chrome trace names more than {MAX_TRACKED_SPANS} distinct \
                             systems; refusing to aggregate it"
                        ));
                    }
                    slot.insert((1, dur_us));
                }
            }
            return Ok(());
        }
        Ok(())
    }

    fn event(&mut self, event: TraceEvent) -> Result<(), String> {
        match event.ph.as_str() {
            "B" => {
                let stack = self.open.entry(event.tid).or_default();
                if stack.len() >= MAX_OPEN_SPANS_PER_THREAD {
                    return Err(format!(
                        "chrome trace stacks more than {MAX_OPEN_SPANS_PER_THREAD} unclosed \
                         spans on thread {}; refusing to aggregate it",
                        event.tid
                    ));
                }
                stack.push((event.name, event.ts));
            }
            "E" => {
                if let Some((name, begin)) = self.open.entry(event.tid).or_default().pop() {
                    self.record(&name, (event.ts - begin).max(0.0))?;
                }
            }
            "X" => self.record(&event.name, event.dur)?,
            _ => {}
        }
        Ok(())
    }

    fn finish(self, truncated: bool, bytes: u64) -> TraceProfile {
        let grand_total_us: f64 = self.totals.values().map(|(_, us)| us).sum();
        let mut costs: Vec<SystemCost> = self
            .totals
            .into_iter()
            .map(|((name, kind), (calls, us))| SystemCost {
                name,
                kind,
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
        TraceProfile {
            costs,
            truncated,
            bytes,
        }
    }
}

/// Folds the top-level array element by element instead of collecting it.
/// This is the whole point of the module's streaming rule: serde hands each
/// event over, [`Aggregate`] adds it in, and the event is dropped before the
/// next one is read.
struct FoldEvents<'a>(&'a mut Aggregate);

impl<'de> DeserializeSeed<'de> for FoldEvents<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for FoldEvents<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a chrome-trace JSON array of events")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        self.0.entered = true;
        while let Some(event) = seq.next_element::<TraceEvent>()? {
            self.0.event(event).map_err(A::Error::custom)?;
        }
        Ok(())
    }
}

/// Stream a chrome-trace JSON file (the `trace_chrome` output) and aggregate
/// the per-system and per-command-flush spans into costs, sorted by total time
/// descending.
///
/// Takes a reader, not a string, and that is load-bearing: these files reach
/// several gigabytes, so the contents are never materialised and neither is a
/// parse tree over them. Peak memory is the read buffer plus one event plus
/// the two capped maps.
///
/// Handles both duration styles the format allows: `B`/`E` begin-end pairs
/// (what tracing_chrome emits; paired per `tid` as a stack) and complete
/// `X` events carrying `dur`. Timestamps and durations are microseconds
/// (the chrome contract). Any other span is counted into nothing - they
/// stay in the raw file for Perfetto.
///
/// A file that is not a JSON array is rejected loudly. A file that IS one and
/// ends mid-array keeps the events it did contain and sets
/// [`TraceProfile::truncated`]: the supervisor kills a traced child that
/// overruns its timeout, and rejecting what that child wrote throws away a
/// ranking that had already converged.
pub fn aggregate_system_costs(trace: impl Read) -> Result<TraceProfile, String> {
    let mut aggregate = Aggregate::default();
    let bytes = Rc::new(Cell::new(0));
    let counted = Counted {
        inner: trace,
        bytes: Rc::clone(&bytes),
    };
    let mut de = serde_json::Deserializer::from_reader(BufReader::with_capacity(1 << 20, counted));
    match FoldEvents(&mut aggregate).deserialize(&mut de) {
        Ok(()) => {
            // Trailing junk after the array is a broken file, not a partial
            // one: a truncated trace has nothing after it to trail.
            de.end()
                .map_err(|e| format!("not a chrome-trace JSON file: {e}"))?;
            Ok(aggregate.finish(false, bytes.get()))
        }
        // Ran out of input INSIDE the array: everything read is still true.
        // An empty file, or one that is not an array at all, never gets that
        // far and is still rejected.
        Err(error) if aggregate.entered && error.classify() == serde_json::error::Category::Eof => {
            Ok(aggregate.finish(true, bytes.get()))
        }
        Err(error) => Err(format!("not a chrome-trace JSON file: {error}")),
    }
}

impl SystemCost {
    /// The row's display name: the system path, with deferred rows marked so
    /// the flush is never read as the system's own body.
    pub fn display_name(&self) -> String {
        format!("{}{}", self.name, self.kind.suffix())
    }
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
            cost.display_name(),
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

    /// The aggregation this module did when it read the whole file into a
    /// string and then built a whole-file `serde_json::Value` over it - the
    /// shape that took the host to 27.8 GB. Kept ONLY as the reference the
    /// streaming path is differenced against: if the two ever disagree, the
    /// streaming rewrite changed a number in the report.
    fn dom_reference(contents: &str) -> Vec<SystemCost> {
        let events: serde_json::Value = serde_json::from_str(contents).expect("valid json");
        let events = events.as_array().expect("an array");
        let mut open: HashMap<i64, Vec<(String, f64)>> = HashMap::new();
        let mut totals: HashMap<(String, CostKind), (u64, f64)> = HashMap::new();
        let mut record = |name: &str, dur_us: f64| {
            for kind in [CostKind::Commands, CostKind::System] {
                let Some(system) = name.strip_prefix(kind.prefix()) else {
                    continue;
                };
                let system = system.trim_matches('"');
                let entry = totals.entry((system.to_string(), kind)).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += dur_us;
                return;
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
            .map(|((name, kind), (calls, us))| SystemCost {
                name,
                kind,
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
        costs
    }

    /// A trace with the shape a real one has: tracing_chrome's `.file` /
    /// `.line` / `cat` / `pid` noise, quoted `name=` field values, nested
    /// non-system spans, several threads, and both duration styles.
    fn realistic_trace(frames: usize) -> String {
        let mut out = String::from("[\n");
        let mut ts = 0.0_f64;
        for frame in 0..frames {
            for tid in 1..=4_i64 {
                for system in 0..6 {
                    let name = format!("game::sub{system}::system_{}", system * 7 % 5);
                    ts += 1.0 + f64::from(frame as u32 % 3);
                    out.push_str(&format!(
                        "{{\".file\":\"/src/lib.rs\",\".line\":{system},\
                         \"cat\":\"bevy_ecs::system::function_system\",\
                         \"name\":\"system: name=\\\"{name}\\\"\",\
                         \"ph\":\"B\",\"pid\":1,\"tid\":{tid},\"ts\":{ts}}},\n"
                    ));
                    ts += 3.0;
                    out.push_str(&format!(
                        "{{\"cat\":\"bevy_ecs::query::state\",\"name\":\"par_for_each\",\
                         \"ph\":\"B\",\"pid\":1,\"tid\":{tid},\"ts\":{ts}}},\n"
                    ));
                    ts += 5.0;
                    out.push_str(&format!(
                        "{{\"ph\":\"E\",\"pid\":1,\"tid\":{tid},\"ts\":{ts}}},\n"
                    ));
                    ts += 2.0;
                    out.push_str(&format!(
                        "{{\"ph\":\"E\",\"pid\":1,\"tid\":{tid},\"ts\":{ts}}},\n"
                    ));
                    ts += 1.0;
                    out.push_str(&format!(
                        "{{\"cat\":\"bevy_ecs::system::function_system\",\
                         \"dur\":{},\"name\":\"system_commands: name=\\\"{name}\\\"\",\
                         \"ph\":\"X\",\"pid\":1,\"tid\":{tid},\"ts\":{ts}}},\n",
                        4.0 + f64::from(system)
                    ));
                }
            }
        }
        out.push_str("{\"args\":{\"name\":\"main\"},\"name\":\"thread_name\",\"ph\":\"M\",\"pid\":1,\"tid\":0}\n]");
        out
    }

    /// The claim the streaming rewrite has to earn: it is not an
    /// approximation. Every row, every call count, every millisecond and
    /// every share matches what the whole-file DOM produced on the same
    /// bytes - so the report is unchanged by the memory fix.
    #[test]
    fn the_streamed_aggregate_equals_the_whole_file_dom_aggregate() {
        for trace in [fixture(), realistic_trace(40)] {
            let streamed = aggregate_system_costs(trace.as_bytes()).expect("trace parses");
            assert!(!streamed.truncated, "a complete file is not truncated");
            assert_eq!(
                streamed.costs,
                dom_reference(&trace),
                "streaming changed a number the report prints"
            );
            assert!(!streamed.costs.is_empty(), "the fixture has systems in it");
        }
    }

    #[test]
    fn aggregates_be_pairs_and_x_events_with_literal_values() {
        let costs = aggregate_system_costs(fixture().as_bytes())
            .expect("fixture parses")
            .costs;
        assert_eq!(
            costs.len(),
            2,
            "non-system spans are not counted: {costs:?}"
        );

        // alpha: two 1000 us calls (nested non-system span pops first on
        // tid 1 - stack pairing) = 2.0 ms total, 1.0 ms mean.
        assert_eq!(costs[0].name, "game::alpha");
        assert_eq!(costs[0].kind, CostKind::System);
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

    /// The deferred half. An observer and a `queue(|world| ...)` closure run
    /// inside the SPAWNING system's `system_commands` span, so a reader that
    /// counts only `system` spans reports zero for them - which reads as
    /// "cheap" and is really "invisible".
    #[test]
    fn a_command_flush_is_counted_and_never_folded_into_the_system_body() {
        let trace = r#"[
            {"ph":"X","ts":1000.0,"tid":1,"dur":1000.0,"name":"system: name=\"game::alpha\""},
            {"ph":"B","ts":3000.0,"tid":1,"name":"system_commands: name=\"game::alpha\""},
            {"ph":"E","ts":6000.0,"tid":1}
        ]"#;
        let costs = aggregate_system_costs(trace.as_bytes())
            .expect("fixture parses")
            .costs;
        assert_eq!(costs.len(), 2, "one row each, never merged: {costs:?}");

        // The flush is the bigger row and sorts first.
        assert_eq!(costs[0].name, "game::alpha");
        assert_eq!(costs[0].kind, CostKind::Commands);
        assert!((costs[0].total_ms - 3.0).abs() < 1e-9);
        assert_eq!(costs[0].display_name(), "game::alpha (commands)");

        assert_eq!(costs[1].kind, CostKind::System);
        assert!((costs[1].total_ms - 1.0).abs() < 1e-9);
        assert_eq!(costs[1].display_name(), "game::alpha");
    }

    #[test]
    fn renderer_cuts_to_top_n_and_notes_the_rest() {
        let costs = aggregate_system_costs(fixture().as_bytes())
            .expect("fixture parses")
            .costs;
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
        assert!(aggregate_system_costs("{}".as_bytes()).is_err());
        assert!(aggregate_system_costs("not json".as_bytes()).is_err());
        // An EMPTY file is not a truncated array - it never opened one, and
        // reading it as "a run with no systems" would be an invented zero.
        assert!(aggregate_system_costs("".as_bytes()).is_err());
    }

    #[test]
    fn empty_trace_yields_empty_costs() {
        let profile = aggregate_system_costs("[]".as_bytes()).expect("empty array parses");
        assert!(profile.costs.is_empty());
        assert!(!profile.truncated);
    }

    /// A traced child stopped mid-write - by the supervisor's trace cap or by
    /// its timeout - leaves the array unclosed. Rejecting that threw away a
    /// ranking that was already converged; keeping it is only honest if the
    /// profile SAYS it is a prefix, which is what the flag is for.
    #[test]
    fn a_trace_cut_off_mid_array_keeps_its_events_and_says_it_is_a_prefix() {
        let whole = realistic_trace(20);
        let cut = &whole[..whole.len() * 2 / 3];
        let partial = aggregate_system_costs(cut.as_bytes()).expect("a prefix still aggregates");
        assert!(partial.truncated, "the report has to be told");
        assert_eq!(
            partial.bytes,
            cut.len() as u64,
            "the size is what was READ, which is the point of counting it here"
        );
        assert!(
            !partial.costs.is_empty(),
            "the events before the cut are still true"
        );

        let complete = aggregate_system_costs(whole.as_bytes()).expect("the whole file parses");
        assert!(!complete.truncated);
        // Same systems, fewer calls: a prefix loses the tail, never invents.
        let names = |profile: &TraceProfile| {
            let mut names: Vec<String> =
                profile.costs.iter().map(SystemCost::display_name).collect();
            names.sort();
            names
        };
        assert_eq!(names(&partial), names(&complete));
        let partial_calls: u64 = partial.costs.iter().map(|c| c.calls).sum();
        let complete_calls: u64 = complete.costs.iter().map(|c| c.calls).sum();
        assert!(
            partial_calls < complete_calls,
            "{partial_calls} vs {complete_calls}"
        );
    }

    /// The guard. Streaming makes the FILE's size irrelevant, so the only
    /// remaining way a trace can grow the host is a shape that grows one of
    /// the two surviving maps - unclosed spans, or generated span names.
    /// Both refuse rather than allocate.
    #[test]
    fn a_trace_that_would_grow_the_host_is_refused_rather_than_aggregated() {
        let mut unclosed = String::from("[\n");
        for i in 0..=MAX_OPEN_SPANS_PER_THREAD {
            unclosed.push_str(&format!(
                "{{\"ph\":\"B\",\"ts\":{i}.0,\"tid\":1,\"name\":\"system: name=game::a\"}},\n"
            ));
        }
        unclosed.push_str("{\"ph\":\"E\",\"ts\":1.0,\"tid\":1}\n]");
        let error = aggregate_system_costs(unclosed.as_bytes()).expect_err("refused");
        assert!(error.contains("unclosed spans"), "{error}");
    }
}
