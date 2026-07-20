# Frame capture is all-or-nothing: emit partial windows on early exit (marked honest) + category-aware --fps window defaults

- STATUS: CLOSED
- PRIORITY: 88
- TAGS: v0.8.0,bug,tooling,performance

## Goal

Field finding (user's `probe run gameplay --fps --profile`, then again
with WARMUP=60/FRAMES=240): only playable produced a Performance section.
The run.log data shows why - all three captures ARMED and finished
warmup, but the capture is ALL-OR-NOTHING: scenario's app exited at frame
308 with the window needing ~319 (ELEVEN frames short) and every one of
its 229 captured samples was discarded; broadside self-ended at 181,
same story. A fixed frame-count window racing each example's wall-clock
lifetime at unknown fps loses somewhere on every host.

Fix in two parts:

### 1. Partial-window emit-on-exit (the correctness fix, capture-side)

When the app exits mid-CAPTURE (post-warmup) with at least MIN_FRAMES
(~60) recorded, the capture emits what it has - summary line, JSON
sidecar, CSV row - marked PARTIAL:

- The CSV row stays schema v3 (the `frames` column already records the
  ACTUAL count; percentiles over 229 frames are exactly as valid as over
  240 - nearest-rank just gets a coarser tail). Partial-ness lives in
  the summary line (`frames=229/240 partial`) and the JSON sidecar
  (requested vs captured); NO schema v4.
- The report's Performance section notes partial rows next to the
  profile badge ("partial window: 229/240 - the app exited first").
- Exits DURING warmup or below MIN_FRAMES keep today's no-emit, but the
  report's skip message gains the diagnosis ("app exited at 43/240
  captured frames / during warmup" instead of the generic "no
  frame-time capture in this run dir") - the current message sent the
  user hunting through knobs when the answer was an 11-frame miss.
- fps_within_baseline: partial rows are EXCLUDED from baseline gating
  (compared windows must be like-for-like); the check's detail says so.

### 2. Category-aware window defaults (the ergonomics fix, probe-side)

When `--fps` runs an example OUTSIDE perf/, probe defaults the child env
to WARMUP=60/FRAMES=240 (operator env always wins; perf/ and the sweep
matrix keep 180/900 - baselines stay full-window). With part 1 in place
this is comfort, not correctness: whatever fits the example's lifetime
gets reported honestly.

## Rejected alternatives (recorded)

- Capture-owns-the-exit fleet-wide (extend autopilot lifetime when
  armed): re-opens exit ownership across 19 examples, breaks self-ending
  scripts (broadside's completion guard panics if the capture exits
  first), and shifts every in-example assertion's timing. The
  perf_baseline lesson (exclusive ownership) says no.
- Time-based windows: redefines the measurement unit all existing
  baselines are built on; partial-emit delivers the benefit without
  moving the unit.
- Smaller fixed defaults: scenario missed by 11 frames AT 60+240 on a
  fast host - there is no constant that fits every example on every
  machine.

## Steps

RE-SCOPED 2026-07-20 by a reproduce-first check (see the resolution note
below): the loop work (loop_while_pending, task 20260720-000616) already
landed and ALREADY fills windows for the cycling examples, which falsified
Part 1's motivating case. Verified in the user's own post-loop field data
(git_sha f2663b00): playable AND scenario both emit frametime.csv with
fps=ok; only broadside times out. So Part 1 (partial-emit) and the
yield-on-primary-done flush are CROSSED OUT - the user prefers the cycling
method, and the only non-cycling case (broadside) is handled by exemption,
not by emitting stubby partial data.

Done:

- [x] catalog.rs: configurable fps-exempt list, parsed from
      `[package.metadata.nova_probe] fps_exempt` (fail-open; single- and
      multi-line array forms), with unit tests. Root Cargo.toml lists
      `broadside`.
- [x] probe.rs: `--fps` SKIPS the capture pass for fps-exempt examples (they
      run clean + profiled only), so a narrative one-shot no longer idles to
      the deadline and hard-timeouts; category-aware window defaults
      (60/240 outside `perf/`; `perf/` + sweep keep 180/900; operator
      `NOVA_PERF_WARMUP`/`FRAMES` always win). Manifest carries `fps_exempt`.
- [x] run_report.rs: the Performance section renders an honest "fps-exempt:
      <reason>" note instead of the generic "no frame-time capture" line;
      process_exit stays green (no fps pass = no failed pass). Unit test pins
      the honest note + green verdict.
- [x] Tests: catalog exempt-parse (absent/single/multi-line/wrong-table),
      window-default precedence (perf short-circuit + non-perf default),
      manifest fps_exempt json round-trip, exempt-render note. E2e (manual,
      user field-test): `probe run gameplay --fps` -> playable full +
      scenario full (via cycling) + broadside honest-exempt, NO bare FAIL.
- [x] Docs: probe skill, wiki capture paragraph (development.md), CHANGELOG
      Unreleased.

Crossed out (superseded by cycling + exemption):

- [-] ~~capture.rs exit-observer partial-emit (MIN_FRAMES, partial marker in
      summary/sidecar), warmup/too-few skip diagnostics~~ - cycling fills real
      windows; narrative examples are exempted, not emitted as partial stubs.
- [-] ~~run_report partial badges + baseline exclusion of partial rows~~ - no
      partial rows are produced.
- [-] ~~yield-on-primary-done flush~~ - exempt examples never arm capture;
      cycling examples keep the autopilot pending until capture is done, so
      nothing idles to the deadline. No caller remained.

## Notes

- Trigger: user field testing 2026-07-19 (probe-runs data preserved in
  the main checkout: scenario 308/319 frames, broadside 181, playable
  complete).
- trace.json confirmed present for all three - the profile section was
  never affected; only the frame capture is all-or-nothing.

## Note (2026-07-19, post-filing)

Spike 20260719-235305 now owns the ARCHITECTURE here (harness completion
protocol + scene looping); this task re-scopes after the spike's
adjudication - part 1 (partial emit) shrinks to the deadline safety net
plus the diagnostic skip messages, part 2 (category window defaults)
stays as ergonomics. Do not start this task before the spike lands.

## Field finding (2026-07-20, broadside HARD-TIMEOUT - facet part 1 misses)

User re-ran `NOVA_PERF_WARMUP=60 NOVA_PERF_FRAMES=240 probe run gameplay
--fps --profile`. broadside's report is now a bare `process_exit FAIL:
1/2 pass(es) failed - fps (timed out)`, NOT the "partial 181" this task
predicted. clean + profiled PASS; only the fps pass dies. Evidence
(probe-runs/broadside/fps-run.log, git_sha f2663b00):

- 06:24:10.548 fps process start.
- ~72s of cold load: broadside spawns many ships (gunship + dozens of
  explodable sections, racer, cargoa, cargob) AND reloads the whole scene
  once (its die -> Defeat -> Retry beat loads twice). Under lavapipe that
  is the bulk of the wall-clock.
- 06:25:22.926 warmup(60) done, "capturing 240 frames" begins.
- 06:25:39 gunship broken / Victory; 06:25:46 "victory capture settling".
- 06:26:13.578 "script complete, exiting"; autopilot marks done; log ends
  with `completion: autopilot done (1 still pending)` = the fps CAPTURE
  collector, still short of 240. NO frametime.csv was ever flushed.
- Elapsed to script-complete: 123.0s. Default BCS_HARNESS_DEADLINE is
  120s -> the process is HARD-KILLED at the deadline, ~3s after the
  script finished, mid-capture.

Why part 1 (exit-observer emit-on-exit) does NOT rescue this: broadside
never reaches a graceful AppExit inside the deadline - it is killed. An
exit observer cannot fire on a SIGKILL, so no partial window flushes. Two
additions this task needs:

1. Yield-on-primary-done: when the self-ending script (the primary
   collector) completes with capture still pending, the capture flushes
   its partial window and the app exits Success IMMEDIATELY, instead of
   idling on the victory overlay until the deadline kills it. (On this
   run script-complete was 123s vs a 120s deadline, so even this loses by
   3s here - hence #2.)
2. broadside is structurally a poor fps target: its ENTIRE lifetime is
   ~181 frames (clean pass run_end at frame 181), it carries a mid-run
   scenario reload, and it double-loads a heavy multi-ship scene. It can
   never supply a 240-frame post-warmup window. Decide explicitly:
   mark broadside fps-EXEMPT (report "no stable frame-time window:
   narrative scenario with a mid-run reload" honestly, run it clean+
   profiled only), OR give narrative examples a much smaller window +
   a longer deadline. Recommendation: fps-exempt - playable/scenario
   (loopable) are the gameplay fps targets; broadside is a correctness
   smoke test. This is a per-example "fps-capturable" capability flag.

Add to Steps: an fps-exempt/capability flag in the example catalog +
probe honoring it; the yield-on-primary-done flush; an e2e that
`run gameplay --fps` yields playable(full) + scenario(partial) +
broadside(exempt, honest skip) and NO bare FAIL.

## Grooming (2026-07-20): REPRIORITIZED 59 -> 88 (top of the OPEN queue)

Field-validated playtest bug: the user hit the broadside fps timeout live
twice, and it blocks the perf-capture workflow the whole probe strand
exists to serve. The v0.8.0 plan's policy files playtest bugs at p90+; this
is the top open task under the already-closed p90 CI-red bug. Scope is now
three parts (see field finding above): yield-on-primary-done flush,
partial-emit + skip diagnostics, and the per-example fps-capable flag +
category window defaults - reasonable to split into sub-tasks when picked up.

## Resolution (2026-07-20): reproduce-first shrank this to exempt + defaults

Bug-playbook reproduce-first, run BEFORE writing any code, against the
current tree (the loop work had since landed): inspected the user's own
probe-runs field data at git_sha f2663b00 - `playable` and `scenario` BOTH
carry a `frametime.csv` with `fps=ok`; only `broadside` is `no-csv` +
`fps=TIMEOUT`. So `loop_while_pending` (task 20260720-000616) already fills
the window for every cycling example, which FALSIFIES Part 1's motivating
case (the "scenario 308/319 discarded" the task was filed on no longer
happens - scenario now loops to a full window).

With the user's decision to prefer the cycling method and to make narrative
examples configurably exempt, the fix reduced to two pieces, both in
nova_probe with no capture.rs/stats.rs surgery:

1. Configurable fps-exempt via `[package.metadata.nova_probe] fps_exempt`
   (broadside listed). `--fps` skips the capture pass for exempt examples;
   they keep the clean + profiled correctness passes; the report shows an
   honest "fps-exempt" note; process_exit stays green.
2. Category-aware default window (60/240 outside `perf/`) so a bare
   `probe run gameplay --fps` fits the completion deadline without the
   operator hand-passing FRAMES.

The yield-on-primary-done flush and the whole partial-emit pipeline were
crossed out: with cycling filling windows and narrative examples exempt, no
example idles to the deadline, so there is no partial window left to emit.
See the Steps section for the done/crossed-out breakdown.
