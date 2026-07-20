# probe: size the fps completion deadline to the capture window + examples propagate AppExit so a deadline expiry gates

- STATUS: IN_PROGRESS
- PRIORITY: 89
- TAGS: v0.8.0,bug,tooling,performance

## Story

As someone running the probe run-harness, I want a legitimately-slow-but-
progressing frame capture to COMPLETE rather than trip the harness hang
detector, and I want a genuine hang to fail as a first-class process error, so
that `probe run <example> --fps` neither false-fails on a heavy dev scene nor
silently half-passes when the deadline really does expire.

## Field finding (2026-07-20)

User ran `probe run perf_baseline --fps --profile` (a DEV build) and it FAILED.
Evidence (probe-runs/perf_baseline, git_sha a22f413c):

- verdict FAIL, on `log_clean` ONLY - process_exit/run_completed/reached_playing/
  invariants_held all PASS.
- The offending line is from the fps pass:
  `ERROR bevy_common_systems::completion: harness completion: deadline (120s)
  expired with collectors still pending: ["capture"]`.
- The fps pass armed the perf/ window (warmup=180, frames=900). Warmup ALONE
  took 79s (08:28:24 -> 08:29:43) = ~2.3 fps: perf_baseline is the heaviest
  scene (sustained combat burst) and this was a dev build under software
  rendering. At 2.3 fps, 180+900=1080 frames needs ~470s, but the bcs
  completion deadline is a flat 120s -> capture never completes, the deadline
  error-exits, and NO frametime.csv is written.

Two distinct defects:

1. DEADLINE IS FLAT, NOT WINDOW-SIZED. `BCS_HARNESS_DEADLINE` defaults to 120s
   (bcs `DEFAULT_DEADLINE_SECS`) and probe never overrides it. The deadline is
   a HANG detector, but a capture steadily accumulating frames is not hung - it
   just needs more wall-clock under slow rendering. A 900-frame window cannot
   fit 120s in a dev/software run. (perf/ keeps 180/900 by design - baselines
   must be full-window - so the category-window default from 20260719-233732
   neither caused nor fixes this.)
2. EXAMPLES SWALLOW AppExit. Every example `main()` does a bare `app.run();`
   and discards the returned `AppExit`, so the process always exits 0. When the
   deadline writes `AppExit::error()` (completion.rs), the process still exits
   0 -> probe's `process_exit` PASSES and only `log_clean` catches the ERROR by
   scraping the string. The completion protocol's "error-exit naming laggards"
   failure signal is defeated.

Not a regression from recent work: the flat deadline came from the completion
protocol (20260720-000609) and the swallowed AppExit is long-standing. Entirely
nova-side to fix - `BCS_HARNESS_DEADLINE` is already env-overridable, so no bcs
change is needed.

## Steps

### Fix A - size the completion deadline to the fps window (probe.rs)

- [x] Add a pure helper computing the deadline seconds for a capture window:
      `(warmup + frames) / FPS_FLOOR + LOAD_MARGIN_SECS`, with a conservative
      FPS_FLOOR (~2-3 fps, the measured software-render floor) and a load
      margin (~30s). Unit-test the arithmetic (e.g. 180/900 -> well above 120s;
      60/240 -> a small value).
- [x] In the fps pass, set `BCS_HARNESS_DEADLINE` in the child env to that
      value UNLESS the operator already set it (operator wins, same pattern as
      the window env). The warmup/frames used must be the SAME values the
      window env resolves (perf/ 180/900, non-perf 60/240, or operator
      overrides) so the deadline matches the actual window.
- [x] Raise the fps pass's supervisor `--timeout` to exceed the computed
      deadline (probe kills at --timeout; it must be > the in-process deadline
      or probe kills first). Keep the operator's `--timeout` if larger.
- [x] Confirm the deadline is only applied to the fps pass (clean/profiled/
      sweep unaffected) and that a generous deadline is harmless for fast runs
      (they exit on completion well before it).

### Fix B - examples propagate AppExit (examples/**)

- [x] Change every example `fn main()` from `{ ...; app.run(); }` to
      `fn main() -> AppExit { ...; app.run() }` (bevy's `App::run` returns
      `AppExit`). Sweep ALL examples under examples/ (21 targets); each `main`
      returns the runner's exit.
- [x] Verify: an armed example run whose deadline expires now exits NON-ZERO
      and probe's `process_exit` FAILS (fast repro: `BCS_HARNESS_DEADLINE=1`
      forces immediate expiry - currently the process exits 0, the bug).
- [x] Confirm normal completion still exits 0 (AppExit::Success), so no example
      regresses to a spurious failure.

### Verify + docs

- [x] E2e: a dev `probe run perf_baseline --fps` now COMPLETES and writes
      frametime.csv (or a bounded-window proxy if the full run is too slow to
      run locally); verdict OK.
- [x] Forced-hang test: `process_exit` FAILs when the completion deadline
      expires (via BCS_HARNESS_DEADLINE=1 on an armed example).
- [x] Docs: probe skill + development.md note that the fps deadline scales with
      the window and that `BCS_HARNESS_DEADLINE` overrides it; CHANGELOG.

## Definition of Done

- `probe run perf_baseline --fps` (dev) no longer false-FAILs on the completion
  deadline: the capture completes and the csv is written, or - if genuinely
  wedged - the deadline fails the run through a NON-ZERO process exit that
  `process_exit` reports (not a log-only flag).
- The deadline scales with the requested window (arithmetic, unit-tested);
  operator `BCS_HARNESS_DEADLINE` / `--timeout` still win.
- Every example propagates `AppExit`; a forced deadline expiry fails
  `process_exit`.

## Notes

- Ties to the 20260719-233732 retro note "fps e2e windows need arithmetic
  first" - the window and the deadline must be arithmetically consistent.
- Intended usage `probe run perf_baseline --fps --release` likely already fits
  120s (release ~10x faster); the fix makes dev runs and heavy scenes robust
  and makes true deadline hits report honestly.

## Verification (2026-07-20)

- Unit tests (3): `fps_deadline_secs` (180/900 -> 585s, 60/240 -> 195s, both
  clear the flat 120s; bigger window -> bigger deadline), `resolve_fps_window`
  per-category, `fps_window_and_deadline_env` sets window + sized deadline.
- Compile: `cargo check -p nova_probe --all-targets` + `cargo check --examples
  --features debug` both clean (all 21 examples return AppExit).
- E2e normal `probe run scenario --fps`: log shows "fps pass deadline 195s
  (window-sized)"; verdict OK, process_exit PASS, log_clean PASS - the capture
  filled its 240-frame window within the sized deadline (no flat-120s trip).
- E2e forced expiry `BCS_HARNESS_DEADLINE=1 probe run scenario --fps`: verdict
  FAIL, `run_completed` shows `exit: "Error(1)"` and `process_exit` FAIL - Fix
  B propagates the deadline's AppExit::error to a non-zero process exit (before,
  the process exited 0 and only log_clean caught it), and Fix A's operator
  BCS_HARNESS_DEADLINE wins over the sized value.
- perf_baseline itself was verified via the `scenario` proxy (loops, fills
  fast) + the arithmetic unit test proving perf/ gets a 585s deadline; a full
  dev `perf_baseline --fps` runs ~585s to fill 900 frames at the ~2fps floor,
  too slow to run in-loop, so it was not run end-to-end locally.
