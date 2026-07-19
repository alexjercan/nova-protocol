# Wire the whole example fleet for probe (timeline+invariants+frametime everywhere) + RunMeta build-profile label; exit gate: probe run --all with every report read

- STATUS: CLOSED
- PRIORITY: 58
- TAGS: v0.8.0,tooling,testing,examples

## Goal

Every autopilot example becomes probe-evaluable: wire
`nova_probe::nova_timeline()` + `nova_probe::nova_invariants()` (2 inert
lines) into all 16 unwired autopilot examples, and `nova_frametime()`
everywhere alongside (user adjudication 2026-07-19: fps wiring
EVERYWHERE, inert, with a small dev/release label in the report table).
This turns run_completed / reached_playing / invariants_held MEASURED on
the whole fleet (rows jump from 2/6 to 5/6 in the T1 aggregate), and
makes `--fps` askable ad hoc on any example.

- RunMeta gains `profile: "dev" | "release"` (cfg!(debug_assertions) at
  capture time); the frame-stats tables (per-run report AND the T1
  aggregate row, where fps was captured) show a small profile label, and
  dev rows are marked not-a-baseline. Pre-profile CSV rows (v1/v2
  without the field) still load as "unknown".
- Wiring bar: the generic plugins only - NO per-example monotonics or
  markers in this task (that judgment work is T3; the standing rule is
  only-what-the-design-promises).

## Steps

- [x] Wire the 16: sections (7), gameplay/broadside, ui (3),
      screenshots (6 - capture producers run a full harnessed cycle, so
      timeline+invariants are meaningful there too). nova_frametime()
      added everywhere it is missing (inert without env).
- [x] RunMeta.profile: field + capture-time detection + CSV
      schema bump with backward-compat load (pre-field rows ->
      "unknown"); report + aggregate row label ("dev - not a baseline"
      styling kept small per the user).
- [x] Probe skill wired-table: replace the 3-row table + "others
      SKIPPED" with the new fleet-wide coverage statement.
- [x] EXIT GATE (the substance): one full `probe run --all`, EVERY row's
      report read. Each firing invariant is adjudicated: a real bug ->
      filed as a task and noted here; a wrong bound -> tuned with the
      reasoning recorded. The close-out records the full aggregate
      (verdicts + measured per row).
- [x] Docs: wiki Performance section (fleet coverage + profile label),
      CHANGELOG Unreleased.
- [x] Verify: fmt; cargo test -p nova_probe (schema/meta tests); the
      touched examples compile via cargo check --examples --features
      debug.

## Notes

- Spike: tasks/20260719-205543/SPIKE.md. Depends on T1 (multi-run) for
  the --all exit gate.
- perf_baseline / scenario / playable already wired; render_scale_shot
  stays NOT_PROBED (T1's list) and gets no wiring.
- Invariant thresholds were tuned on scenario/playable only - false
  positives at fleet scale are EXPECTED to surface at the exit gate;
  that is the point of the gate, not a reason to skip it.

## Close-out (2026-07-19, branch feature/probe-fleet-wiring)

The fleet measures. Exit gate: the first real `probe run --all` - 20
examples sequentially, every row's report read.

- 19/20 rows OK measured 5/6 on the FIRST pass: run_completed (clean
  bracket), reached_playing, invariants_held (0 violations on every
  wired example - ZERO false positives at fleet scale; the soft-cap
  bounds tuned on scenario/playable held everywhere), process_exit,
  log_clean. Spot-read details across four categories confirmed real
  timelines, not vacuous ones.
- The 20th row was the gate doing its job: perf_baseline FAILed at the
  184s timeout - the spike ASSUMED it "runs fine headless", but it is
  the one example with no autopilot, so a plain run had NO exit path at
  all. Adjudication: an exit-ownership gap, not an invariant bug - fixed
  in the example: `if !nova_probe::perf_armed()` add autopilot +
  timeline + invariants (the capture owns the exit when armed, the
  autopilot exactly otherwise - never both, so the harness can never
  preempt a measurement window). Re-probed: OK measured 5/6;
  `probe report probe-runs` refreshed the aggregate to ALL GREEN
  (19 OK + perf_baseline OK + render_scale_shot NOT_PROBED with reason).
- --fps non-regression after the fix: `probe run perf_baseline --fps`
  (short window) - capture owned the exit, wrote a schema-v3 row:
  18 columns ending `...,git_sha,host,profile` with `dev` recorded and
  the report badging it not-a-baseline. The v2->v3 compat surface is
  unit-pinned (v1/v2 load with profile unknown; append REFUSES to mix
  schemas in one file).
- nova_probe tests 80/80 after the schema bump (one legitimately
  outdated column-count pin updated 17 -> 18).

Deviation from the plan, recorded: the task counted "16" unwired
examples; the enumeration was 17 (screenshots holds 6 + render_scale_shot,
which stays unwired). All 17 got the three plugins; scenario/playable
gained frametime; perf_baseline gained the conditional harness above.
