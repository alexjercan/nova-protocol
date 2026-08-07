# L1 - Unblind the probe gate

**Baseline: NEUTRAL.** Behavior-only. No module renamed - that is L8.

Findings: **F01, F02, F03, F04, F05** (blind gate), **F76, F77, F78**
(example harness), **F58** (the proc macro), **F63, F70, F71** (report-writer
robustness).

**Depends on:** nothing. **This lane goes first among the code work.** Every
other lane is verified by the gate it repairs.

## F01 + F03 - one root cause, one change

`RunArtifacts::load` (`crates/nova_probe/src/run_report/artifacts.rs:44`) has
no per-artifact error isolation: any parse error `?`-propagates, `finish_report`
returns `Err`, and `report.html`/`checks.json` are never written - **after
`clean_out_dir` already deleted the previous ones**.

**Read the doc comment first.** It says the hard error is deliberate:

```rust
// crates/nova_probe/src/run_report/artifacts.rs:41-43  (today)
/// Load whatever exists in `dir`. Unreadable-but-present artifacts are
/// hard errors (a corrupt file must not read as "not captured");
/// absent files are simply `None`.
pub fn load(dir: &Path, baseline_dir: Option<&Path>) -> Result<Self, String>
```

That intent is correct and must survive. What is wrong is the **scope** of the
failure: one bad artifact currently discards the other eleven. Isolate per
artifact, and make the failure its own reported check.

```rust
// NEW  crates/nova_probe/src/run_report/artifacts.rs
/// A present-but-unloadable artifact. Never silently dropped: `load` keeps
/// the field `None` and records the reason here, and `artifacts_loadable`
/// turns it into a FAILED check so the run cannot verdict OK on it.
pub struct ArtifactFailure {
    pub name: String,
    pub reason: String,
}

// CHANGE  RunArtifacts - one added field
pub struct RunArtifacts {
    // ... the 8 existing fields, unchanged ...
+   pub failures: Vec<ArtifactFailure>,
}

// CHANGE  artifacts.rs:44 - signature keeps its Result (a missing out dir is
// still a hard error), but per-artifact parse errors no longer propagate.
pub fn load(dir: &Path, baseline_dir: Option<&Path>) -> Result<Self, String>

// NEW  the helper each artifact goes through
fn load_one<T>(
    failures: &mut Vec<ArtifactFailure>,
    name: &str,
    raw: Option<String>,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Option<T>
```

```rust
// NEW  crates/nova_probe/src/run_report/checks.rs (wherever evaluate_checks lives)
/// FAILED when any artifact was present and unloadable. This is what keeps
/// the doc comment's promise now that load() no longer aborts the report.
fn check_artifacts_loadable(artifacts: &RunArtifacts) -> Check
```

**F03 is the same edit's other half.** The loader excludes `web-run.log` as
"chromium's own output, not the game's" (`artifacts.rs:65`). It is **both** -
`stats.rs:708` parses the game's `nova perf:` line out of a chromium
`INFO:CONSOLE` line. No `run.log` exists on a web run, so `log_clean` returns
SKIPPED and a panicking wasm app verdicts **OK, exit 0**.

```rust
// CHANGE  artifacts.rs:65-70 - add web-run.log to log_parts
+ if let Some(web_log) = read_opt("web-run.log")? {
+     log_parts.push(web_log);
+ }
```

Plus: `log_clean` must **not** be allowed to SKIP on a run whose manifest says
a web pass happened. A SKIPPED log check on a platform that produced a log is
the failure mode, not the missing file.

## F05 - stale sweep-cell logs present as this run's evidence

```rust
// CHANGE  crates/nova_probe/src/bin/probe/native/run.rs:29
- const RUN_ARTIFACTS: [&str; 12] = [ ... 12 literal names ... ];
+ const RUN_ARTIFACTS: [&str; 12] = [ ... unchanged ... ];
+ /// Sweep-cell logs are numbered, so they cannot be listed literally.
+ /// `RunArtifacts::load` (artifacts.rs:74-92) globs and concatenates them;
+ /// clean_out_dir must remove the same set or a previous run's cell logs
+ /// present as this run's evidence.
+ fn stale_cell_logs(out: &Path) -> Vec<PathBuf>
```

`clean_out_dir` (`run.rs:43`) removes `RUN_ARTIFACTS` plus `probe-run.json`,
then also every path `stale_cell_logs` returns. The comment at `run.rs:26`
currently claims this already happens. Fails **closed** today (false FAIL),
so it is the least urgent of the five - but it is the one that trains people
to distrust the gate.

## F02 - CI passes on a commit that was never probed

```rust
// crates/nova_probe/src/bin/probe/native/sweep.rs:181  (today)
pub(crate) fn build_row(
    example: &str,
    category: &str,
    dir: &Path,
    run_error: Option<String>,
    duration_secs: u64,
) -> nova_probe::AllRow
```

`verdict` is taken verbatim from `checks.json` and `run_error` is stored as an
independent field that never influences it. If `run()` fails before
`clean_out_dir` (`run.rs:66` `create_dir_all`, `:69` `canonicalize`), the
**previous** run's `checks.json` is still on disk: the row reports `OK` with an
error attached and `aggregate_exit` (`sweep.rs:266`) returns SUCCESS.

Two changes, both needed - the first is the floor, the second is the fix:

```rust
// CHANGE  sweep.rs:187 - a run that errored cannot verdict better than ERROR
match (checks, run_error) {
    (_, Some(err)) => AllRow { verdict: "ERROR".into(), error: Some(err), .. },
    (Some(value), None) => { /* today's happy path */ }
    (None, None) => { /* today's "no checks.json" arm */ }
}

// NEW  identify the checks.json so a stale one cannot be read as this run's
//      crates/nova_probe/src/run_report/checks.rs
/// Written into checks.json alongside the verdict. build_row rejects a
/// checks.json whose stamp is not this run's.
pub struct RunStamp {
    pub git_sha: String,
    pub started_unix: u64,
}
```

`run_identity()` already produces the sha and `sweep` already holds
`started_unix`, so the stamp costs one struct and one comparison. Without it
the first change still leaves the "previous run's artifacts, no error this
time" case open.

## F04 - the AppExit race

`completion_watch` (`nova_autopilot/src/completion.rs:152`) writes `AppExit` in
`Last`; `record_run_end` and `record_invariant_summary`
(`nova_probe/src/recorder.rs:126`) read `MessageReader<AppExit>` in `Last`.
Nothing orders them, and bevy exits after the frame in which `AppExit` is
written - there is no next frame. On an unfavourable ordering a **healthy run**
reports "timeline truncated (no run_end)" and the whole sweep exits non-zero.

```rust
// NEW  crates/nova_probe/src/recorder.rs - name the set so the edge is real
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProbeRecorderSystems {
    /// Drains AppExit in `Last`. MUST run after every writer of AppExit.
    RunEnd,
}

// CHANGE  the recorder plugin's build()
app.configure_sets(Last, ProbeRecorderSystems::RunEnd.after(AutopilotCompletionSystems));
```

The behavior is "accidentally correct on today's executor", so the test matters
more than the fix: **a test that fails if the edge is removed.**

## F58 - a typo silently changes an event's name

```rust
// crates/nova_events_macros/src/lib.rs:37  (same shape at :42)
// `attr.parse_args()` is consumed via `if let Ok(...)`, so a malformed
// #[event_name(...)] silently falls back to the lowercased ident.
// #[event_name = "ondestroyed"] compiles cleanly with name() == "ondestroyedevent".
```

Dispatch self-matches so nothing breaks loudly, but every literal-name consumer
silently stops matching - `run_report/html.rs:18` stops filtering `onupdate`
noise, and the recorder's `ondestroyed` lookup never fires.

```rust
// CHANGE  lib.rs:37 and :42
- if let Ok(parsed) = attr.parse_args() { ... }
+ match attr.parse_args() {
+     Ok(parsed) => ...,
+     Err(e) => return compile_error_at(attr, "expected #[event_name(\"name\")]", e),
+ }
```

## F63, F70, F71 - report-writer robustness

```rust
// CHANGE  crates/nova_probe/src/run_report/html.rs:217
- intervals.iter().sum::<f64>() / intervals.len() as f64
+ // capture.rs:499 is the identical line WITH the guard; copy it.
+ if intervals.is_empty() { None } else { Some(sum / len) }
//   prints NaN into the report HTML today

// CHANGE  crates/nova_probe/src/capture.rs:522
//   The in-app CSV append has no schema-version guard. The public writer
//   `append_frametime_row` (stats.rs:415-426) refuses a v3 row under a
//   pre-v3 header and comments on exactly this case. Reuse it rather than
//   re-implementing the append.

// CHANGE  crates/nova_probe/src/bin/probe/native/run.rs:180
- env.retain(|(k, _)| k != "NOVA_PERF_TIMELINE" && k != "NOVA_PERF_INVARIANTS");
+ env.retain(|(k, _)| !matches!(*k, "NOVA_PERF_TIMELINE" | "NOVA_PERF_INVARIANTS"
+                                 | "NOVA_PERF_CONTRACT"));
//   NOVA_PERF_CONTRACT (set at env.rs:98) survives today, so the fps pass
//   rewrites probe-contract.json despite the module doc declaring the clean
//   pass owns it. Benign now; real the moment the fps pass diverges.
```

**Re-assess F70 after F01 lands.** A schema mismatch is catastrophic today only
*because* F01 destroys the whole report instead of rejecting one row. This is
the one place in the epic where fixing the bigger bug may retire the smaller.

## F76, F77, F78 - the examples ARE the harness

Not a separate examples lane. A lane that fixes the loader but leaves an
assertion that cannot fail has not finished its job.

```rust
// CHANGE  examples/screenshots/screenshot_ui.rs:171
//   The "settings panel is up" assertion cannot fail: the panel is toggled by
//   Visibility only, and ui_node_rect (nova_autopilot/src/input.rs:135-151)
//   queries (Name, UiGlobalTransform, ComputedNode) without checking it.
//   wiki-settings.png ships as a shot of the bare main menu, exit 0.

// NEW  crates/nova_autopilot/src/input.rs - the fix belongs in the harness,
//      not in the one example that noticed
pub fn ui_node_rect(...) -> Option<Rect>   // CHANGE: add InheritedVisibility
                                           // to the query and reject hidden
// and a distinct assertion the example can use:
pub fn assert_named_visible(name: &str) -> ...

// CHANGE  examples/systems/player_path.rs:182
//   reload_the_run (:379) does not release held keys; the in-run restart
//   replay_the_run (:537-543) does and documents why. Extract the release
//   into one helper both call.
fn release_all_held_keys(input: &mut ButtonInput<KeyCode>)

// CHANGE  examples/sections/turret_section.rs:404-406
//   tag_gate fires on every Add<AsteroidMarker> and inserts RangeGateMarker
//   unconditionally, so the gravity planetoid is tagged as a range gate.
//   Gate on the spawner's own marker; report_status prints 6 gates for 5.
```

## Verified by

This is the hard part: the thing being fixed is the thing that normally does
the verifying. It needs its own harness.

| Fixture | Asserts |
| --- | --- |
| truncated `trace.json` | report still written, `artifacts_loadable` FAILED |
| torn `timeline.jsonl` | same, plus timeline check FAILED not SKIPPED |
| non-UTF-8 `run.log` | same |
| `web-run.log`-only run | `log_clean` runs, does not SKIP |
| stale `run-<n>.log` | cleaned, absent from this run's log |
| pre-existing `checks.json` + errored run | row verdicts ERROR, sweep exits non-zero |

Plus `probe run --all` before and after, byte-comparing verdicts on a tree
known to be healthy: **the fixes must not change a healthy run's answer.**
