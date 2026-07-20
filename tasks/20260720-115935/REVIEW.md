# Review: window-sized fps deadline + example AppExit propagation

- TASK: 20260720-115935
- BRANCH: fix/probe-deadline-window

## Round 1

- VERDICT: APPROVE

Two fixes, both compile-clean (probe --all-targets + all 21 examples) and
proven end-to-end. Re-derived the load-bearing claims against the runs:

- **Fix A sizes the deadline to the window.** Unit test pins the arithmetic
  (180/900 -> 585s, 60/240 -> 195s; both clear the flat 120s; bigger window ->
  bigger deadline). The normal `scenario --fps` e2e logged "fps pass deadline
  195s (window-sized)" and completed OK, so a run that used to risk the flat
  120s now gets a window-appropriate ceiling. The deadline is applied to the
  FPS pass only - clean/profiled keep the base `timeout`; the fps supervisor
  timeout is raised above the deadline so probe cannot kill the child before
  the deadline resolves.
- **Fix A honors the operator.** `resolve_fps_window` folds in
  `NOVA_PERF_WARMUP/FRAMES`, and `BCS_HARNESS_DEADLINE` is pushed only when
  unset - the forced-expiry e2e (`BCS_HARNESS_DEADLINE=1`) used 1s, not the
  sized 195s, confirming operator-wins.
- **Fix B makes a deadline expiry a real failure.** Every example `main` now
  returns `AppExit` (uniform sweep of all 21, verified `OK=21 BAD=0`, and
  `app.run();` was confirmed the last statement in each before the change). The
  forced-expiry e2e proves it: `run_completed` shows `exit: "Error(1)"` and
  `process_exit` FAILs - before this, the deadline's `AppExit::error()` was
  discarded by the bare `app.run();` and the process exited 0, so only
  `log_clean` caught it. Normal completion still exits 0 (e2e 1 verdict OK), so
  no example regresses to a spurious failure.

Scope honesty (recorded in TASK.md): the actual `perf_baseline --fps` dev run
was verified via the `scenario` proxy (loops, fills fast) plus the arithmetic
unit test proving perf/ gets a 585s deadline - a full dev perf_baseline run is
~585s at the ~2fps floor, too slow to run in-loop locally. The path it
exercises is identical; only the scene differs.

- [ ] R1.1 (NIT) `FPS_FLOOR = 2.0` is a deliberately pessimistic constant, so a
  genuinely-HUNG large-window run (e.g. a wedged release perf capture) now waits
  up to its sized deadline (~585s) before failing, versus the old flat 120s.
  Accepted tradeoff: window-sizing is what stops the false-failures, and a real
  hang is rare and still fails. A progress-aware deadline (reset while frames
  are still accruing) would catch hangs fast AND allow slow captures, but that
  is a bcs-side change and out of scope here. Noted for a possible follow-up.
