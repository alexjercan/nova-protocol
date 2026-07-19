# nova_probe: grow + rename nova_perf into the run-harness crate (frame-capture + perf_report as modules)

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.8.0, spike, tooling, refactor, performance

## Goal

Grow `nova_perf` into the unified run-harness crate and rename it to
`nova_probe` (name confirmed by user, 2026-07-19). The existing frame-time
capture harness and the `perf_report` HTML generator become MODULES of the new
crate; nothing about the measurement is lost. Also extend the capture schema
with run metadata - renderer/GPU, resolution, graphics preset, git SHA, host
class - today the renderer is inferred from the results dir NAME only, and
baseline deltas / per-renderer thresholds / the report's Run summary all need
it (spike review m2). This is the foundation the correctness, profiling, and
report tasks build on.

## Notes

- Spike: tasks/20260719-112011/SPIKE.md (foundation task, do FIRST).
- Move `crates/nova_perf/*` under the new name; update every dependent
  (`20_perf_baseline` example, `perf_web` bin, the two perf scripts, Cargo
  members, `perf.html`/Trunk wiring). Sweep the repo for the old crate name.
- Keep the `nova_scenario` criterion bench separate - not part of this crate.
- The built `perf_report` code (originally branch `feature/perf-html-report`,
  commit a9af7789) lands on master with the spike-branch squash; incorporate
  it under the new crate rather than re-writing.
