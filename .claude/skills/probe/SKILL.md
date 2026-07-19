---
name: probe
description: Verify a change with the nova_probe run-harness - one command runs an autopilot example and produces a reviewable correctness+perf report (report.html + checks.json) with an OK/WARN/FAIL verdict. Use as the post-feature check in /work's verify step for gameplay-touching changes, for before/after evidence on bug and perf tasks, and whenever the user asks to "probe" a run or wants a run report.
---

# Probe - the Run-Harness Check

`nova_probe` runs an autopilot example headless and answers two questions in
one artifact: did the run behave correctly, and what did it cost? It is the
post-feature check of this repo's SDLC: after implementing a change that
touches gameplay, probe the affected example(s) and read the report before
calling the work verified. Built by the 20260719-112011 spike family; design
rationale lives in `tasks/20260719-112011/SPIKE.md`, user-facing docs in the
wiki's Performance section (`web/src/wiki/dev/development.md`).

## Commands

```sh
cargo run -p nova_probe -- run <example>              # clean pass -> report
cargo run -p nova_probe -- run <example> --profile    # + traced pass (top-N systems)
cargo run -p nova_probe -- run <example> --samply     # + named flamegraph
cargo run -p nova_probe -- run <example> --fps        # + DEDICATED capture-only pass
cargo run -p nova_probe -- run <example> --baseline <old-run-dir>   # FPS deltas
cargo run -p nova_probe -- run playable,scenario      # comma list -> aggregate index
cargo run -p nova_probe -- run ui                     # a whole category (sections|gameplay|ui|screenshots|perf)
cargo run -p nova_probe -- run --all                  # the whole catalog minus NOT_PROBED
cargo run -p nova_probe -- run perf_baseline --fps --release \
  --render gpu --scenario asteroid_field --preset high --preset low  # perf sweep (matrix)
cargo run -p nova_probe -- run <scenario> --platform web  # web/WebGPU frame capture
cargo run -p nova_probe -- report <run-dir> [--baseline <dir>]  # re-render (manifest-gated)
```

Two verbs is the whole surface: `run` and `report`. (The `sweep|web|profile`
aliases and the `trace` verb retired at the v0.8.0 cut - retired commands
error with a pointer to the `run` form; the top-N systems table renders
inside the report on `--profile` runs and re-renders via `probe report`.)

Multi specs (list, category, `--all`) resolve against the `[[example]]`
catalog in the root Cargo.toml, run SEQUENTIALLY with continue-on-failure
(one hung example FAILs its row, the sweep keeps going), and write an
aggregate above the per-example run dirs: `index.html`, `index.json` (the
agent surface: one file answers "does everything still work"), and
`probe-all.json` (the gate). The aggregate verdict is the WORST row and
the exit code mirrors it; each row shows verdict + measured n/total + the
six check statuses + a link to that example's own report. `--all` skips
the NOT_PROBED list (each entry has its reason, shown in the report);
bare `probe run` errors with the catalog instead of accidentally starting
a 30-minute fleet sweep. Expect a category to take single-digit minutes
warm and `--all` 25-40 min - categories are the everyday unit, `--all`
the pre-release/nightly sweep.

`report` REFUSES dirs without `probe-run.json` (or `probe-all.json` for an
aggregate dir - its rows are re-read fresh from each run's checks.json) -
a report can only be built from dirs probe itself produced, so stale
hand-assembled folders cannot impersonate a run. Sweeps run with
`--release` (dev-profile frame numbers are not baselines - the report
badges each frame row's build profile) and `--render sw` gives the
lavapipe software floor.

`run` writes to `probe-runs/<example>/` (gitignored), SURGICALLY CLEANED of
probe's own artifacts at start (nothing stale survives into a report):
`probe-run.json` (the manifest: identity, passes, exit/timeout outcomes),
`timeline.jsonl`, `run.log` (or `run-<n>.log` per sweep cell),
`report.html`, `checks.json`, plus `trace.json`/`trace-run.log` (--profile),
`samply-profile.json.gz` (--samply), `frametime.csv` (--fps on a wired
example, or the sweep/web captures), `web-run.log` (--platform web). Exit
code mirrors the verdict (FAIL and NO_DATA = 1). It spawns its own Xvfb
(pid-derived display; `--display :0` to reuse one) and times out hung runs
(`--timeout <s>`, default 180) - a timed-out run still produces a FAILing
report.

## Reading the verdict (the honesty rules)

- `checks.json` is the agent-readable mirror - read it instead of parsing
  HTML, and read `verdict` TOGETHER WITH `measured` ("n/total"), never the
  verdict alone. Checks: `process_exit` (the child's real outcome from the
  probe-run.json manifest; a timeout is a FAIL), `run_completed` (a
  TRUNCATED timeline means the run died - entries are flushed as written -
  and the bracket's entry count must match the file), `reached_playing`
  (the smoke contract), `invariants_held` (violations counted per name;
  one stuck entity violates every frame), `fps_within_baseline` (soft
  gate; only REGRESSIONS beyond the threshold WARN - improvements PASS),
  `log_clean` (ANSI-stripped, whole-word ERROR). Each check carries a
  structured `data` object; the top-level `run` object is the manifest.
- SKIPPED means NOT MEASURED, never "held". Zero measured checks =
  verdict NO_DATA and a nonzero exit. An OK on an UNWIRED example is
  OK-with-coverage: it proves the example's own assertions (exit status)
  only - gameplay-verification claims require `run_completed` and
  `invariants_held` MEASURED, and the skip details say when an example
  simply is not wired.
- The profile table RANKS systems; shares overlap (parent and child spans
  both count) so they are never summed, and traced-run numbers never compare
  against the clean pass.
- The tool's verdict is PROVISIONAL. The reviewer (you, in /review) owns the
  final OK/NOT-OK, via the checklist at the bottom of report.html.

## Where it plugs into the SDLC

- **/work, verify step**: for a change touching gameplay/scenario/flight/
  sections, `probe run` the affected example(s) after tests pass. Record the
  invocation and verdict in TASK.md's close-out. A FAIL is a finding, not an
  inconvenience - read the timeline around the failing frame.
- **Bug tasks (reproduce first)**: probe the reporting scenario BEFORE the
  fix - the timeline is the diagnosis evidence (states, events with
  payloads, variable old/new around the failure). Keep the pre-fix run dir;
  probe again after; cite both in TASK.md. Strict invariants
  (`NOVA_PERF_INVARIANTS=strict` on a manual run) panic at the moment of
  corruption when you need the exact frame.
- **Perf tasks (measure first)**: run the sweep matrix before and after the
  change into separate run dirs, then `probe report <after> --baseline
  <before>` for the delta table. Use `--profile` (+ `--samply`) to RANK
  suspects before optimizing anything; the ledger's perf lessons
  (quiet-host-before-measuring, isolate-the-lever) still apply.
- **/review**: when the implementer cites a probe verdict, open checks.json
  and read `measured` first, then the SKIPPED rows - what was NOT measured
  is the first thing to challenge. For perf claims, confirm same-label baselines and a quiet host.
- **New examples**: wiring is three inert lines -
  `app.add_plugins(nova_probe::nova_timeline())`,
  `nova_probe::nova_invariants()` and `nova_probe::nova_frametime()` -
  every cataloged example carries them (fleet wiring, 20260719-210443).
  Monotonic variables (`.monotonic([...])`, only what the scenario DESIGN
  promises one-way) and `probe_marker` beats are the depth pass (T3).

## Wired today

The WHOLE fleet carries timeline + invariants + frame capture (inert
without probe's env); `render_scale_shot` alone is unwired (NOT_PROBED -
real-GPU pixel capture). Depth beyond the generic checks:

| Example | extra depth |
|---|---|
| gameplay/scenario | monotonics: beat, rocks_destroyed |
| gameplay/playable | monotonics: target_down, leg + 7 beat markers |
| gameplay/broadside | a marker per script stage (11: picker -> defeat -> Retry -> acts -> victory) |
| sections/* | outcome markers at the assertion sites (turret fired/gate damaged; the torpedo fire->arm->detonate->hit chain; hull partial-exact + destroyed-ship-survives; attitude error_rad; burn speeds; com/camera drifts) |
| perf/perf_baseline | combat-burst fps driver (the sweep scene) |

torpedo_guidance and the ui/ flows carry no extra markers on purpose:
guidance asserts at scenario-load level (no outcome flags exist), and the
ui flows are state-transition shaped - the generic timeline already
records every transition.

Probe addresses examples by NAME (`probe run scenario`); categories come
from `examples/<category>/` (catalog in the root Cargo.toml). `--fps` runs
a DEDICATED capture-only pass (task 20260720-000616) - the clean pass
never arms the capture (the recorder's per-entry flush contaminated
fps-on-clean numbers), and the completion protocol keeps the app alive
until the window closes. Enrolled scenes (gameplay/scenario, playable -
`loop_while_pending`) RELOAD and replay while the capture fills, so the
window measures activity; reload intervals are EXCLUDED from the stats
(their count is host-speed-dependent) and reported as their own line
("3 scene reloads - mean/max ms"). Frame rows carry their build profile
(schema v3); dev rows are labeled NOT a baseline - baselines come from
`--release` runs.

## Host knobs (flamegraphs)

samply needs `perf_event_paranoid <= 1` and, on many-core hosts, a raised
`perf_event_mlock_kb` (e.g. 16384). Load profiles with the URL `samply load`
prints (drag-dropping the file loses the local symbol server = hex frames);
driver-blob/libc frames stay hex regardless - judge by our modules' frames.
