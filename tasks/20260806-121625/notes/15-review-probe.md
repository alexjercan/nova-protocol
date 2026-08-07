# Code review - nova_probe, nova_autopilot, nova_events, examples

Source: dedicated reviewer, 2026-08-07. Spot-verified.

**This is the highest-stakes area in the review.** `nova_probe` is itself the
CI gate (`cargo run -p nova_probe -- run --all` blocks CI), so a defect here
does not just break a feature - it blinds the thing that catches every other
break. The reviewer was asked specifically to hunt checks that cannot fail.
It found several.

## The blind-gate cluster - VERIFIED

### 1. One unparseable artifact destroys the entire report

`crates/nova_probe/src/run_report/artifacts.rs:44`. `RunArtifacts::load` is
all-or-nothing - every parse error is `?`-propagated:

```rust
let timeline = read_opt("timeline.jsonl")?
    .map(|s| parse_timeline(&s).map_err(|e| format!("timeline.jsonl: {e}")))
    .transpose()?;
let runs = read_opt("frametime.csv")?    ...?;
let costs = read_opt("trace.json")?      ...?;
```

So `finish_report` returns `Err`, `report.html` and `checks.json` are never
written - and `clean_out_dir` already deleted the previous ones.

**This directly contradicts the code's own comment** at `run.rs:210-212`:

```
// Pass 2: PROFILED (optional; separate build so tracing overhead
// never touches pass 1's numbers). Failures degrade to "no trace" -
// a successful clean pass is never discarded.
```

VERIFIED - both the propagation and the contradicting comment.

Failure: `probe run playable --profile` where the traced pass is killed by the
supervisor timeout (`run.rs:232`). `trace.json` is a truncated JSON array,
`aggregate_system_costs` errors, and **the clean pass's timeline, invariants
and log evidence are all discarded**. The aggregate row becomes
`ERROR: not a chrome-trace JSON file`.

Two more inputs, same shape:

- `timeline.jsonl` with a torn final line after probe's own `child.kill()` ->
  no report, instead of `run_completed` FAIL. Contradicts
  `run_completed.rs:5`, where truncation is *supposed* to be the crash signal.
- `run.log` with non-UTF-8 bytes from a segfaulting child -> `read_to_string`
  errors at `artifacts.rs:50` -> no report at all.

Severity: bug. The check designed to detect a crash is disabled by the crash.

### 2. A panicking wasm app can verdict OK

`crates/nova_probe/src/run_report/artifacts.rs:65-67`:

```rust
// The game's logs: run.log (single run) plus run-<n>.log (sweep
// cells), concatenated in cell order. web-run.log stays OUT - it is
// chromium's own output, not the game's.
```

**The premise is wrong. It is both.** The wasm app's console output lands in
`web-run.log`, and the repo's own test proves it - `stats.rs:708` parses the
game's `nova perf:` line out of a chromium `INFO:CONSOLE` line.

No `run.log` exists on a web run, so `log_clean` returns SKIPPED.

Failure: `probe run asteroid_field --platform web --baseline <dir>` (allowed -
`cli.rs:221` bans only `--profile/--samply/--fps/--scenario/--preset`). The
wasm app logs `ERROR` lines or panics after the summary line. `process_exit`
PASS, `fps_within_baseline` PASS, `log_clean` **SKIPPED**, verdict **OK**,
exit 0. The panic evidence is sitting in the file the loader deliberately
refuses to read.

VERIFIED. Severity: bug.

### 3. Stale sweep-cell logs survive into the next report

`crates/nova_probe/src/bin/probe/native/run.rs:29` - `RUN_ARTIFACTS` is **12
literal filenames**. `RunArtifacts::load` (`artifacts.rs:74-92`) globs
`run-*.log` and concatenates them. Nothing deletes them.

VERIFIED: the literal list contains `run.log`, `fps-run.log`, `web-run.log`
and 9 others, but no `run-<n>.log` glob.

Failure: a `--scenario a --scenario b --scenario c --scenario d` run writes
`run-0..3.log`, one with a real ERROR -> FAIL. Fix the bug, re-run with
`--scenario a` only: this run writes `run.log`, the four stale cell logs are
still there, `log_clean` concatenates all five and **FAILs again on evidence
from a run that no longer exists.**

The comment at `run.rs:26` claims "nothing stale ... can present as this run's
evidence". Violated.

Severity: bug. Note this one fails *closed* (a false FAIL), which is much
better than 1, 2 and 4 - but it trains people to distrust the gate.

### 4. An errored run can inherit a stale OK verdict

`crates/nova_probe/src/bin/probe/native/sweep.rs:187`. `build_row` reads
`checks.json` and takes `verdict` verbatim; `run_error` is stored in the row's
`error` field and **never influences the verdict**. `aggregate_verdict` /
`aggregate_exit` (`sweep.rs:266`) look only at `verdict`.

VERIFIED by read - the `Some(value)` arm sets `verdict` from the JSON and
`error: run_error` as an independent field.

Failure: re-run `probe run --all` at the same commit after a successful sweep.
If `run()` fails *before* `clean_out_dir` - at `run.rs:66` `create_dir_all` or
`run.rs:69` `canonicalize` (stale mount, permission change, out dir replaced by
a file) - the previous run's `checks.json` is still on disk. The row reports
`verdict: "OK"` with an error attached, `aggregate_exit` returns SUCCESS, and
**CI passes on a commit that was never probed.**

Severity: bug. An error row must never be able to inherit a verdict.

## Correction to `04-nova-probe.md`

That note said of `run_report/`:

> **Already exists and is the best code in the crate.** Rename, do not rebuild.

**Amended 2026-08-07.** The *shape* of `run_report/` is still right -
`RunArtifacts` (load) -> `checks/*` (evaluate) -> `html.rs` (render), each
check a module with a single roster table, is a good design and the rename
recommendation stands.

But the loader at the front of that pipeline carries four gate defects, three
of which fail **open**. "Do not rebuild" was correct about the structure and
wrong about the confidence. The loader needs per-artifact error isolation -
a failed parse should degrade that one artifact to `None` and let its check
report the failure, not abort the report.

That is the single change that fixes findings 1 and, in spirit, 2.

## Ordering and correctness

### 5. `AppExit` is written and read in the same schedule with no ordering

`crates/nova_probe/src/recorder.rs:126` + `crates/nova_autopilot/src/completion.rs:152`.

`completion_watch` writes `AppExit` in `Last`. `record_run_end` and
`record_invariant_summary` read `MessageReader<AppExit>` in `Last`. Nothing
orders them - the recorder chains only against `record_variable_changes`, the
invariants chain only `.before(record_variable_changes)`.

Bevy exits after the frame in which `AppExit` is written, so **there is no next
frame in which to read it.**

Failure: on any frame where the executor schedules `record_run_end` before
`completion_watch`, the app exits with no `run_end` line. `run_completed` then
returns FAIL "timeline truncated (no run_end)" on a **completely healthy run**,
and the whole `--all` sweep exits non-zero. Same for `invariant_summary`,
which flips `invariants_held` from PASS to FAIL via the `ArmedButAbsent` path
when the run had zero violations.

Severity: bug. The ambiguity is certain; whether today's executor happens to
order them favourably is pinned by nothing. This is a latent CI flake.

### 6. A malformed `#[event_name(...)]` is silently discarded

`crates/nova_events_macros/src/lib.rs:37` (and `:42`) - `attr.parse_args()` is
consumed via `if let Ok(...)`, so a parse failure falls back to the lowercased
ident.

Failure: writing the name-value form `#[event_name = "ondestroyed"]` instead of
the list form compiles cleanly with `name() == "ondestroyedevent"`. Dispatch
still self-matches (both sides go through `E::name()`), so nothing breaks
loudly - but **every literal-name consumer silently stops matching**:
`run_report/html.rs:18` stops filtering `onupdate` noise, and the recorder's
`ondestroyed` lookup never fires, so the probe report shows a destruction that
happened as not recorded. `#[event_info([u8; 4])]` has the same shape.

Severity: bug. A proc macro that accepts a typo and changes behavior is a
particularly bad failure mode - `compile_error!` is the correct response.

### 7. Serialization failure is indistinguishable from "no payload"

`crates/nova_events/src/engine.rs:170` - `GameEventInfo::from_data` maps a
`serde_json::to_value` error to `None`, with no log at any level.

Failure: a payload field holding `NaN`/`inf` (or a non-string-keyed map) gives
`data: None`. `EntityFilterConfig::filter`
(`nova_scenario/src/filters.rs:71`) then returns `false`, so **every
entity-filtered handler for that kind stops firing permanently and the scenario
simply never advances**, silently.

Today's event vocabulary is all-`String`, so this is one added float field away
from live. Severity: bug.

This is the crate `08-tests-ci-risk.md` flagged as risk #3 - 570 vendored
lines, 4 tests, the mandated scenario dispatch path. The risk register was
aimed correctly.

## Smells

- `capture.rs:522` - the in-app CSV append has **no schema-version guard**,
  unlike the public writer `append_frametime_row` (`stats.rs:415-426`), which
  refuses a v3 row under a pre-v3 header and has a comment naming exactly this
  case. `NOVA_PERF_OUT` into an old results dir appends 18-column rows under an
  11-column header; `parse_frametime_csv` then errors, which via finding 1
  destroys the whole report rather than rejecting one row.
- `native/env.rs:76` - the fps pass rewrites `probe-contract.json` despite the
  module doc declaring the clean pass owns it. `run.rs:180` strips only
  `NOVA_PERF_TIMELINE` and `NOVA_PERF_INVARIANTS`; `NOVA_PERF_CONTRACT`
  survives. Benign today (same binary, same plugins); becomes real the moment
  the fps pass diverges in features.

## Example-harness defects

These matter because the examples *are* the harness.

| Site | Defect |
| --- | --- |
| `examples/screenshots/screenshot_ui.rs:171` | **The "settings panel is up" assertion cannot fail.** The panel is toggled by `Visibility` only, and `ui_node_rect` (`nova_autopilot/src/input.rs:135-151`) queries `(Name, UiGlobalTransform, ComputedNode)` without checking visibility. Rename `Settings Button` -> `click_named` warns and continues, the panel never opens, the node still exists hidden, the assert passes, and `wiki-settings.png` ships as a shot of the bare main menu with exit 0 |
| `examples/sections/turret_section.rs:406` | `tag_gate` fires on every `Add<AsteroidMarker>`, so the gravity planetoid is tagged as a range gate. A round hitting the planetoid flips `outcome.gate_damaged`, and the example reports "a turret round connected with a gate" on a run where no target gate was hit. `report_status` prints 6 gates for 5 |
| `examples/systems/player_path.rs:182` | The capture-loop restart hook does not release held keys, while the in-run restart `replay_the_run` (`:537-543`) does and documents why it must. Under `NOVA_PERF=1` the loop restarts with G latched; `ButtonInput::press` on an already-pressed key raises no `just_pressed` edge, so the looped cycle produces no GOTO edge and error-exits on its 20 s deadline |

## Checked and cleared

Worth recording - these were the specific hypotheses put to the reviewer:

- **`recorder.rs:207-246` - the `File::create` truncation comment matches the
  code.** `truncate(false)` -> `try_lock` -> `set_len(0)`. Correct order, and
  the lock is cross-process. The hypothesis in `08-tests-ci-risk.md` was wrong
  here, and this is the atomic-write reference the `nova_assets` fix should
  copy (see `11-review-assets-scenario.md`).
- `contract.rs:164` - temp-file + rename, genuinely atomic.
- **`supervise.rs:117-159` - no pipe deadlock.** stdout/stderr go straight to a
  file handle, never a pipe. The timeout is enforced. A signal death is
  correctly false via `status.success()`.
- `completion.rs:176-184` - a deadline expiry writes `AppExit::error()`, so a
  hung run cannot pass `process_exit` or `run_completed`.
- `fps_within_baseline.rs:129` `best_note.expect(...)` is unreachable.
- **`stats.rs:157-185` nearest-rank percentiles are correct**, pinned by a
  literal ramp test. (This closes the one `cast_*` site
  `09-clippy-and-lints.md` left open for a read.)
- **`aggregate_exit` does propagate.** `run.rs:347` and `report.rs:67` both
  fail-closed on FAIL/NO_DATA/UNPROBEABLE, and `verdict_severity` ranks unknown
  verdicts as FAIL. The propagation itself is sound - findings 1-4 corrupt the
  *input* to it, not the logic.
- `engine.rs` dispatch: no dropped events, no HashMap-ordered dispatch, no
  unbounded recursion (actions get only `&mut W` and cannot enqueue), no list
  mutated during iteration, no unwrap/panic on the dispatch path. **The
  hypotheses in the brief were all wrong**; the one real `engine.rs` defect is
  finding 7, which is a different thing entirely.

## Bearing on the epic

`04-nova-probe.md` already put the probe restructure first among the code moves
because it is the CI gate. **The review makes that ordering non-negotiable, and
adds a prerequisite:** findings 1-4 should land *before* the restructure, not
during it.

The reason is verification. Every other lane in the epic is verified by the
probe gate. If the gate can pass a run that was never probed (finding 4), skip
the log check on a whole platform (finding 2), or discard all evidence because
one artifact was truncated (finding 1), then a green sweep after a large refactor
means less than it appears to. Fixing the gate is what makes the rest of the
work checkable.

Finding 5 is the sharpest scheduling risk - it is a latent flake that will
surface as an unreproducible CI failure at the worst possible moment, i.e.
during a large refactor when everyone assumes the refactor caused it.
