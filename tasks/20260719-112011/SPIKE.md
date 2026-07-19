# Spike: unified nova_probe run-harness (correctness + profile + FPS report over autopilot examples)

- DATE: 20260719-112011
- STATUS: RECOMMENDED
- TAGS: spike, v0.8.0, tooling, performance, testing

## Question

We want ONE tool that, run on an existing `BCS_AUTOPILOT` example scenario,
produces a single reviewable artifact answering two things at once:

1. **Correctness** - did the scenario run the way it was supposed to? What
   happened, step by step, and did it drift from the expected timeline?
2. **Performance** - FPS/frame-time stats, load, and a profile (flamegraph /
   per-system costs) showing where the time goes.

The intended use is the **post-feature regression check**: after implementing a
feature, run this on the affected autopilot example(s) and get an HTML report a
human OR an agent reads to give an OK / NOT-OK verdict - "did I break behavior
or perf?". The report should be as automated as possible (auto per-check
verdicts, drift numbers, FPS deltas), with a "what to look for" checklist, but
the final call stays with the reviewer.

A good answer defines: the architecture, the crate shape, how correctness is
captured and validated, how profiling is produced headlessly, and - the piece
the user explicitly wanted this spike to settle - **what goes in the HTML
report**. It then seeds the implementation tasks.

## Context

The repo already has the three ingredients, sitting apart:

- **Correctness harness (pass/fail only).** Every `examples/` example is
  `BCS_AUTOPILOT`-driven; `tests/examples_smoke.rs` runs 18 headless with
  panic-on-failure assertions + stall backstops. The scenario system emits
  structured signals through its own event handlers (kill tally, travel-lock
  echo, arrival flags) and tracks scenario variables. Today a green run tells
  you it did not panic - nothing about WHAT happened or how it drifted.
- **FPS / load.** `nova_perf` (`crates/nova_perf/src/lib.rs`) is an env-gated
  Bevy plugin: drives the real app to `Playing`, warms up, records `Time<Real>`
  deltas for a fixed window, writes percentile stats (JSON + `frametime.csv`).
  `scripts/perf-baseline.sh` (native gpu/sw sweep) and `scripts/perf-web.sh`
  (wasm/WebGPU via headless Chromium) drive it. The `20_perf_baseline` example
  and the `perf_web` bin wire scenario + graphics-preset selection.
- **Profiling.** Not wired. Only a `samply record` hint in the `nova_scenario`
  criterion bench comment.

Already built this cycle (branch `feature/perf-html-report`, commit a9af7789):
a `perf_report` HTML generator over `frametime.csv` (inline CSS + SVG chart +
`--baseline` deltas) plus a public CSV reader in the lib
(`parse_frametime_csv` / `FrameStats::from_csv_row` / `PerfRun`). That is the
**FPS-section renderer** of the tool this spike designs; it is reused, not
thrown away (task 20260718-152230, now folded in - see Next steps).

Separate and staying separate: the `nova_scenario` **criterion** bench is a
different layer (isolated CPU micro-hot-path, statistical, `target/criterion/`
HTML). It does not belong in a per-scenario run report.

## Options considered

Each fork below was decided with the user (2026-07-19 questionnaire); the
rejected options are kept so the choice can be trusted and revisited.

### Correctness validation -> INVARIANT ASSERTIONS + EXISTING ASSERTIONS
### (REVISED 2026-07-19 after review round 1; goldens deferred to backlog)

- **Invariant assertions + existing panic assertions + rendered timeline
  (CHOSEN, round-1 adjudication).** Continuous always-true checks evaluated
  during the run (health never negative, speed respects the cap, scenario
  acts/variables monotonic where designed, entity counts bounded), with
  violations recorded on the structured run timeline; the existing autopilot
  panic assertions stay the hard-failure layer; the timeline itself is
  RENDERED in the report for the reviewer (no golden diff). Invariants catch
  always-been-wrong bugs a golden cannot (goldens only detect change vs the
  last bless) and are immune to host timing noise. Task 20260719-114931.
- *Golden timeline compare (DEFERRED to backlog, 20260719-112245).* The
  round-1 review (REVIEW.md M3) showed the risk is bigger than first framed:
  llvmpipe CI runs differ structurally from dev-GPU runs (a total-order
  golden may never match cross-host; comparison needs per-track partial
  order), snapshot fatigue trains rubber-stamp blessing, and the queued
  campaign-polish tasks (20260718-152313, 20260716-174729) would churn every
  campaign golden immediately. User call (2026-07-19): neat idea, not liked
  enough to carry that cost now. Revisit after the recorder produces
  empirical stability data and the campaign content settles.
- *Deterministic replay (rejected).* Seeded RNG + fixed timestep + recorded
  inputs would make timelines exactly comparable and obsolete tolerance
  math - but avian physics + f32 accumulation + variable render rate make
  cross-host determinism impractical here (user concurred: too hard to get
  right with avian). Not pursued.
- *Assertions + captured log only (rejected).* No auto drift/violation
  detection at all; the reviewer eyeballs everything. Lighter, but loses the
  `invariants held` auto-check that makes the report gate-able.

### Profiling -> CHROME-TRACE SPANS + SAMPLY (two-pass)
### (REVISED 2026-07-19 after review round 1: M1 factual fix + M2 two-pass)

- **CHOSEN.** Capture Bevy's per-system tracing SPANS via `bevy/trace_chrome`
  and derive the "top-N costliest systems" table by POST-PROCESSING that
  chrome-trace JSON (aggregate span durations per system) - one capture, two
  products: the inline table and the Perfetto-openable attachment. Optionally
  wrap the native run in **samply** to attach a Firefox-profiler flamegraph.
  No live GUI required for the automated parts.
  CORRECTION (review M1): Bevy has NO per-system timing diagnostic -
  `SystemInformationDiagnosticsPlugin` reports OS process CPU%/memory only;
  per-system costs exist only as `trace`-feature spans. Nothing in the repo
  wires any trace feature today, so T4 adds it (feature-gated).
  TWO-PASS RULE (review M2): tracing serialization (and samply sampling)
  contaminates frame times, so the runner does pass 1 CLEAN (FPS + timeline,
  no tracing) and pass 2 PROFILED (chrome trace + samply, FPS discarded);
  the report labels which pass fed which section. 2x runtime, honest numbers.
- *Tracy (rejected).* `bevy/trace_tracy` is the richest per-system view but
  needs the live Tracy GUI - poor fit for headless/CI/agent runs. (Can still be
  a documented manual option for a human doing a deep dive.)

### Crate shape -> GROW + RENAME nova_perf

- **CHOSEN.** Evolve `nova_perf` into the unified run-harness crate; the
  frame-capture harness and the `perf_report` HTML become modules. Rename to fit
  what it now does. Working name **`nova_probe`** (a probe into a run); other
  candidates `nova_harness` (collides with the existing "BCS harness" term),
  `nova_report` (undersells the capture), `nova_run`. Naming is an open question
  (below), not load-bearing for the design.
- *New crate, keep nova_perf (rejected).* Cleaner single-responsibility split
  but more crates and a frame-capture/report boundary that has to be re-plumbed
  anyway.
- *Keep the name (rejected).* Least churn but "perf" understates correctness +
  profiling.

### Verdict -> AUTO CHECKS + HUMAN/AGENT FINAL

- **CHOSEN.** Compute provisional per-check verdicts (FPS delta vs a baseline
  over threshold; timeline drift within tolerance; assertions passed) and show
  them; the report explicitly leaves the final OK/NOT-OK to a human/agent with a
  "what to look for" checklist. The process exit code reflects the auto checks
  (so it can still gate CI), but a soft perf regression is a WARN the reviewer
  adjudicates, not an automatic hard fail.
- *Data-only (rejected).* No auto-verdict at all; no CI gate.
- *Strict auto-gate (rejected).* Hard thresholds fail the run with no human
  step - brittle to noisy perf numbers on a contended shared host
  (`quiet-host-before-measuring`).

## The HTML report content spec

The deliverable is a RUN DIRECTORY (review m3): a self-contained `report.html`
(inline CSS + SVG, opens offline) plus sidecar attachments (chrome-trace JSON,
samply profile when captured, raw timeline JSON) and a machine-readable
`checks.json` mirroring the verdict rows so an agent consumes results without
parsing HTML. Written so both a human and an agent can act on it; raw-ish data
+ a checklist beats prose. report.html sections, top to bottom:

1. **Verdict banner.** Overall provisional status (OK / WARN / FAIL) derived
   from the per-check results, plus an explicit "reviewer must confirm" line.
   Each check is a row: name, auto-result, the number behind it, threshold.
   Checks: `assertions passed`, `invariants held`, `FPS vs baseline within
   N%` (thresholds per renderer class - a flat 16.6 ms budget would
   permanently flag sw at ~86-126 ms and web at ~34-39 ms, so budget checks
   are informational outside native-GPU; review m4), `no unexpected
   error!/panic in log` (per-example allowlist for known-benign spam like the
   damage-0.00 impact lines; a growing allowlist is itself a smell; review
   m5), `run reached its terminal scenario state`.
2. **Run summary.** Scenario id, example, platform (native/web), renderer,
   graphics preset, window/resolution, warmup+capture frames, wall duration,
   process exit code, git SHA, timestamp.
3. **Correctness.** Three sub-parts: (a) the INVARIANT results table - each
   invariant, held/violated, and for violations the frame + values (the heart
   of "did it run correctly"); (b) the rendered run TIMELINE - the ordered
   structured events with key variable values, for the reviewer to sanity-read
   against the scenario's intent; (c) the ASSERTIONS section - which
   panic-assertions the example carries and that they passed. (A drift-vs-
   golden diff slots in here IF the deferred golden task 20260719-112245 is
   ever picked up; the report layout reserves the spot but ships without it.)
4. **Performance.** FPS/frame-time percentiles (p50/p95/p99/p999/max, mean,
   1%-low) with the existing SVG bar chart + 60fps budget line, and deltas vs a
   `--baseline` run. (Reuses the built `perf_report` renderer.)
5. **Profile.** Top-N costliest systems table (name, mean ms/frame, % of
   frame) derived by post-processing the `trace_chrome` span JSON; links to
   that JSON (Perfetto) and, when captured, the samply flamegraph. Labeled as
   coming from the PROFILED pass (pass 2), whose frame times are NOT the
   report's FPS numbers. Enough inline to triage; the attachments for the
   deep dive.
6. **Log timeline (collapsible).** The captured structured run-event stream
   (timestamp, frame, event, key vars) and any WARN/ERROR lines - raw, for the
   reviewer/agent to scan.
7. **What to check (reviewer checklist).** Explicit step-by-step: "confirm the
   verdict banner; if FPS WARN, compare the chart to baseline and check host was
   quiet; scan drift rows - are missing/extra events expected for this feature?;
   skim the log for unexpected WARN; open the flamegraph if a system jumped."
   Ends with an OK / NOT-OK line for the reviewer to fill.

## Architecture

- **`RunProbePlugin`** (opt-in, in the renamed crate): composes the existing
  frame-capture, a **run-event recorder**, and (native, when armed) the
  profiling arming. An autopilot example adds it the way it adds the autopilot
  today; inert unless armed by env/flag so normal runs pay nothing.
- **Structured run-events + logging.** Define a small `ProbeEvent`
  (timestamp, frame, kind, scenario-variable snapshot). Instrument the game to
  emit these at the moments that matter - `GameStates` transitions, scenario
  variable changes, the scenario event-handler signals (kill/travel-lock/
  arrival), autopilot script beats. This is the "improve in-game logging" work
  and the crux of the effort; prefer reusing the scenario's existing event
  stream over inventing a parallel one.
- **Invariant layer.** Always-true checks evaluated during the run (bounds
  derived from the engine's decision constants, not hand-written numbers);
  violations ride the same structured event stream as run-events. Replaces
  the golden compare as the automated correctness mechanism (goldens deferred
  to backlog 20260719-112245 - see Options).
- **Run metadata in the capture schema (review m2).** Extend the per-run
  JSON/CSV with renderer/GPU, resolution, graphics preset, git SHA, and host
  class - today the renderer is inferred from the results dir NAME only.
  Baseline deltas, per-renderer thresholds and the report's Run summary all
  need it. Lands with T1 (schema) and is consumed by T5.
- **Report renderer.** Grows the built `perf_report` into the full run report
  (adds the correctness, profile, log, checklist sections around the existing
  FPS section).
- **Runner CLI (two-pass).** One entrypoint (`cargo run -p <crate> -- ...`)
  that runs a named example headless (native or `--platform web`) TWICE -
  pass 1 clean (FPS + timeline + invariants), pass 2 profiled (chrome trace +
  samply; FPS discarded) - collects the artifacts into the run directory
  (report.html + sidecars + checks.json), and writes the report. Folds
  `perf-baseline.sh` / `perf-web.sh` into subcommands over time. `--profile`
  enables pass 2; `--export csv,html,json`.
- **Native vs web.** Correctness + FPS work on both (web scrapes the console,
  no fs). Profiling (samply, per-system diagnostics) is native-only; the web
  report simply omits the profile section. Native is the primary target for the
  post-feature check.
- **Relationship to `examples_smoke.rs`.** The smoke suite stays the fast
  pass/fail gate on every push. The probe is the heavier, opt-in, report-
  producing deep run for a specific scenario - a superset, not a replacement.

## Recommendation

Build the unified run-harness by growing+renaming `nova_perf` into
`nova_probe` (name confirmed by user, 2026-07-19), in dependency order: crate
skeleton + rename + schema metadata first, then the correctness capture (the
crux), then the invariant layer, then profiling (two-pass), then the unified
report (absorbing the built FPS renderer), then the runner CLI + example
opt-in + docs. Correctness is the riskiest/highest-value part and should be
de-risked early; the report is the easy downstream assembly. Keep every auto
threshold a single tunable (and per renderer class) so a noisy perf host does
not turn the tool into a false-alarm generator. The whole family leads the
v0.8.0 queue (user, 2026-07-19: this tooling feeds the release's docs +
consolidation theme); only the golden compare sits in the backlog.

## Open questions

- **Crate name.** RESOLVED 2026-07-19: `nova_probe`, confirmed by user.
- **Timeline stability.** How noisy are the scenario timelines run-to-run
  under llvmpipe throttling? Still resolved empirically in the recorder task
  (T2) - it informs how the timeline is rendered/compared and is the entry
  gate for ever picking the deferred golden task (20260719-112245) back up.
- **samply availability / perms.** Sampling may need `perf_event_paranoid`
  tuning or capabilities; the flamegraph attachment must degrade gracefully
  (skip with a note) when samply is unavailable, so the report never fails on a
  missing profiler.
- **Web profiling depth.** Web gets correctness + FPS but no samply; is Bevy's
  chrome-trace enough on wasm, or is web profile explicitly out of scope?
  Defer - native first.

## Next steps

Direction-level tasks seeded for `/plan` to break into steps (dependency order;
all reference this spike). Task 20260718-152230 is FOLDED into the report task
below - its committed `perf_report` code becomes the FPS section.

Priorities re-slotted after review round 1 (M5b): strictly descending along
the dependency chain, and the family leads the v0.8.0 queue (user direction:
land this before anything else in the sprint).

- tatr 20260719-112231 (T1, p76): grow + rename nova_perf -> nova_probe (move
  frame-capture + perf_report HTML in as modules; extend the capture schema
  with run metadata: renderer/GPU, resolution, preset, git SHA, host class).
- tatr 20260719-112238 (T2, p74): structured run-event logging + timeline
  recorder. THE CRUX - de-risk first. Depends on T1.
- tatr 20260719-114931 (T3, p72): continuous invariant assertions during
  autopilot runs (health/speed/state-machine bounds). Depends on T2.
- tatr 20260719-112253 (T4, p70): profiling layer (chrome-trace spans ->
  top-N systems table + Perfetto attachment + optional samply flamegraph,
  graceful when absent; profiled pass separate from the FPS pass). Depends
  on T1.
- tatr 20260719-112304 (T5, p68): unified run report + verdict (absorbs
  20260718-152230; correctness [invariants + timeline + assertions] + FPS +
  profile + log + checklist; run directory + checks.json; auto per-check
  verdicts + exit code). Depends on T2/T3/T4 + the FPS renderer.
- tatr 20260719-112317 (T6, p66): runner CLI + example opt-in + docs (one
  command per autopilot example, two-pass; fold the perf scripts into
  subcommands; wire into the post-feature workflow and the dev wiki / README
  tools section). Depends on T5.
- tatr 20260719-112245 (backlog, p0): golden run-timeline compare + bless
  workflow. DEFERRED per review M3 + user decision 2026-07-19; revisit after
  T2's stability data exists and the campaign-polish tasks land.

Dependency order: T1 -> {T2 -> T3, T4} -> T5 -> T6.

## Fix record

(Appended by each implementing task as it lands - keeps this doc the family's
single source of current state.)

- 2026-07-19 T1 (20260719-112231, CLOSED): crate renamed nova_perf ->
  nova_probe and split into capture/stats/report modules (report rendering
  now a lib module for T5); capture schema v2 adds RunMeta
  (backend/adapter from main-world RenderAdapterInfo, resolution, quality,
  git SHA, host) to CSV+JSON, parser reads v1 AND v2 so the v0.7.0
  baseline keeps loading. NOVA_PERF_* env surface + bin names deliberately
  unchanged (T6's). 24 tests; details in the task's Close-out.
- 2026-07-19 T2 (20260719-112238, CLOSED): run-timeline recorder shipped -
  `nova_timeline()` JSONL recorder (states, every scenario event with
  payload, variable old/new diffs, script beat markers via `probe_marker`),
  flush-per-entry, native-only, wired into 10_playable (7 beats) +
  08_scenario. Unblocked by bcs v0.19.2 (GameEvent read accessors,
  user-approved upstream release). STABILITY ANSWERED: meaningful sequences
  identical across same-host runs (order+names+values); per-frame onupdate
  pulse varies by design; cross-host still unmeasured - the golden task's
  entry gate is half-met. 27 tests; details in the task's Close-out.
- 2026-07-19 T3 (20260719-114931, CLOSED): continuous invariants shipped -
  nova_invariants() (NOVA_PERF_INVARIANTS; strict panics): health bounds,
  velocity finiteness + 10x soft-cap absurdity bound, variable finiteness,
  OPT-IN monotonic variables (both examples register theirs), entity leak
  bound. Violations warn + ride the timeline (kind "invariant");
  InvariantState tallies for T5's `invariants held` check. E2E armed
  10_playable: zero violations over the full window. 35 tests.
