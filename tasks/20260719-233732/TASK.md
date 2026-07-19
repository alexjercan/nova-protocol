# Frame capture is all-or-nothing: emit partial windows on early exit (marked honest) + category-aware --fps window defaults

- STATUS: OPEN
- PRIORITY: 63
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

- [ ] capture.rs: exit-observer emit path (MIN_FRAMES floor, partial
      marker in summary + sidecar), warmup/too-few skip diagnostics.
- [ ] run_report.rs: partial note in the Performance section; skip
      message upgraded with captured/requested counts; baseline gate
      excludes partial rows with a saying-so detail.
- [ ] probe.rs: category-aware WARMUP/FRAMES defaults for --fps outside
      perf/ (env passthrough wins; sweep untouched); parse/env pins.
- [ ] Tests: partial-emit stats math (pure), skip-diagnosis strings,
      env-default precedence; e2e: `probe run gameplay --fps` produces
      three Performance sections (playable full, scenario/broadside
      partial with counts) with NO env knobs.
- [ ] Docs: skill (--fps works fleet-wide, partial semantics), wiki
      capture paragraph, CHANGELOG Unreleased.

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
