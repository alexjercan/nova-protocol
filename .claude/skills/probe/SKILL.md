---
name: probe
description: Verify a change with the nova_probe run-harness - one command runs an autopilot example and produces a reviewable correctness+perf report (report.html + checks.json) with an OK/WARN/FAIL verdict. Use as the post-feature check in /work's verify step for gameplay-touching changes, for before/after evidence on bug and perf tasks, and whenever the user asks to "probe" a run or wants a run report.
---

# Probe - the Run-Harness Check

`nova_probe` runs an autopilot example headless and answers two questions in
one artifact: did the run behave correctly, and what did it cost? It is the
post-feature check of this repo's SDLC: after implementing a change that
touches gameplay, probe the affected example(s) and read the report before
calling the work verified. Design rationale lives in
`tasks/20260719-112011/SPIKE.md`; user-facing docs in the wiki's Performance
section (`web/src/wiki/dev/development.md`).

## Commands

```sh
cargo run -p nova_probe -- run <example>              # clean pass -> report
cargo run -p nova_probe -- run <example> --profile    # + traced pass (top-N systems)
cargo run -p nova_probe -- run <example> --samply     # + named flamegraph
cargo run -p nova_probe -- run <example> --fps        # + DEDICATED capture-only pass
cargo run -p nova_probe -- run <example> --baseline <storage-base>  # FPS deltas
cargo run -p nova_probe -- run <example> --out <dir>  # storage base (default probe-runs)
cargo run -p nova_probe -- run player_path,scenario_grammar      # comma list
cargo run -p nova_probe -- run ui                     # a whole category
cargo run -p nova_probe -- run --all                  # the whole catalog
cargo run -p nova_probe -- run scene_baseline --fps --release \
  --render gpu --scenario asteroid_field --preset high --preset low  # perf sweep (matrix)
cargo run -p nova_probe -- run <scenario> --platform web  # web/WebGPU frame capture
cargo run -p nova_probe -- report <run-dir> [--baseline <dir>]  # re-render (manifest-gated)
```

Two verbs is the whole surface: `run` and `report`. (The `sweep|web|profile`
aliases and the `trace` verb retired at the v0.8.0 cut - retired commands
error with a pointer to the `run` form; the top-N systems table renders
inside the report on `--profile` runs and re-renders via `probe report`.)

Also: `--timeout <secs>` (default 180), `--display <:N>` to reuse an existing
X display, `--render gpu|sw` (`sw` = the lavapipe software floor, NOT a web
stand-in), `--platform native|web`.

## Categories - what probe does with each

The `[[example]]` catalog in the root Cargo.toml is the single source of
truth for WHAT the examples are. What an example can be JUDGED on is no
longer a table: the example DECLARES it at runtime by the probe plugins it
wires, and probe reads the declaration back from `probe-contract.json`.

| Wiring | Declares | The checks it feeds |
|---|---|---|
| `nova_probe::nova_timeline()` | `Timeline` | run_completed, reached_playing |
| `nova_probe::nova_invariants()` | `Invariants` | invariants_held |
| `nova_probe::nova_frametime()` | `FrameTime` | fps_within_baseline |

A check whose capability is undeclared reports that the example makes no such
claim, naming the call that would make it. A capability that IS declared,
WAS armed, and produced nothing is a FAILURE, not a shrug.

There is no launch-side opinion left at all: every cataloged example is a
probe target, `--all` is the catalog with nothing subtracted, and every
category expands - `screenshots/` included, since a capture producer is an
autopilot walk like any other. There is no opt-out, per-example or
per-category: an example that cannot survive a probe run FAILS, in the
report, rather than being quietly listed away, and one that declares no
capability grades UNPROBEABLE.

## Specs, run dirs, and baselines

Bare `probe run` ERRORS with the catalog listing instead of accidentally
starting a 30-minute fleet sweep. Multi specs (comma list, category, `--all`)
run SEQUENTIALLY with continue-on-failure - one hung example FAILs its row and
the sweep keeps going. Expect a category to take single-digit minutes warm and
`--all` 25-40 min: categories are the everyday unit, `--all` the
pre-release/nightly sweep.

Runs write to `<--out|probe-runs>/<short-commit>/<example>/`, SURGICALLY
CLEANED of probe's own artifacts at start (nothing stale survives into a
report). The commit-keyed layer is what makes baselines work; the aggregate
(`index.html`, `index.json`, `probe-all.json`) is written ABOVE the per-example
dirs **even for a single example**, so `index.json` is always the one file that
answers "does everything still work". The aggregate verdict is the WORST row
and the exit code mirrors it; each row shows verdict + measured n/total + the
six check statuses + a link to that example's own report.

`--baseline` names a STORAGE BASE, not a run dir: probe searches it for the
nearest previous commit dir and compares each example against
`<base>/<commit>/<example>/` when that has a `frametime.csv`. Without
`--baseline` it searches the `--out` base, or `probe-runs` - so a repeat run on
a new commit picks up its own predecessor automatically.

Per run dir: `probe-run.json` (the manifest: identity, passes, exit/timeout
outcomes), `timeline.jsonl`, `run.log` (or `run-<n>.log` per sweep cell),
`report.html`, `checks.json`, plus `trace.json`/`trace-run.log` (--profile),
`samply-profile.json.gz` (--samply), `frametime.csv` (--fps on a stress
example, or the sweep/web captures), `web-run.log` (--platform web). Exit code
mirrors the verdict: only OK and WARN exit 0 (FAIL, NO_DATA and UNPROBEABLE
are 1). Probe spawns its own Xvfb
(pid-derived display) and times out hung runs - a timed-out run still produces
a FAILing report.

`report` REFUSES dirs without `probe-run.json` (or `probe-all.json` for an
aggregate dir - its rows are re-read fresh from each run's checks.json), so
stale hand-assembled folders cannot impersonate a run. Sweeps run with
`--release` (dev-profile frame numbers are not baselines - the report badges
each frame row's build profile).

Runs are PROFILE-SANDBOXED: the child gets an empty, probe-owned profile under
`<run-dir>/profile/` (`NOVA_MOD_CACHE_ROOT`, `XDG_DATA_HOME`,
`XDG_CONFIG_HOME`), wiped each run, so your local mod cache,
`installed.mods.ron`, `enabled_mods.ron` and `settings.ron` can never decide a
result - shipped `assets/` content still loads normally. Export any of those
three yourself to keep it (probe preserves what it finds set and says so),
which is how you deliberately probe your real installed mods.

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
  `log_clean` (ANSI-stripped, whole-word ERROR, plus command errors at ANY
  level - `remove`/`despawn` log theirs at WARN). Each check carries a
  structured `data` object; the top-level `run` object is the manifest.
- SKIPPED means NOT MEASURED and N/A means NOT CLAIMED; neither ever means
  "held". Zero measured checks = verdict NO_DATA. A run that measured
  something but graded no CLAIM = verdict UNPROBEABLE - `process_exit` and
  `log_clean` need no plugin, so an example that wires none of them still
  passes those two, and two rows about the process say nothing about the
  run. Both exit nonzero. An OK therefore always covers at least one
  declared capability; read `measured` for how much more.
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
  payloads, variable old/new around the failure). Probe again after; cite both
  commit dirs in TASK.md. Strict invariants
  (`NOVA_PERF_INVARIANTS=strict` on a manual run) panic at the moment of
  corruption when you need the exact frame.
- **Perf tasks (measure first)**: run the sweep matrix before and after the
  change, then `probe report <after> --baseline <base>` for the delta table.
  Use `--profile` (+ `--samply`) to RANK suspects before optimizing anything.
  A dev build is not a release proof, and the host must be quiet before any
  number is written down.
- **/review**: when the implementer cites a probe verdict, open checks.json
  and read `measured` first, then the SKIPPED rows - what was NOT measured
  is the first thing to challenge. For perf claims, confirm same-label
  baselines and a quiet host.
- **Release**: `probe run --all` is the pre-release sweep (see the `release`
  skill's pre-flight).
- **New examples**: wiring is three inert lines -
  `app.add_plugins(nova_probe::nova_timeline())`,
  `nova_probe::nova_invariants()` and `nova_probe::nova_frametime()` -
  every cataloged example carries them (fleet wiring, 20260719-210443).
  Monotonic variables (`.monotonic([...])`, only what the scenario DESIGN
  promises one-way) and `probe_marker` beats are the depth pass.

## Wired today

The WHOLE fleet carries timeline + invariants + frame capture (inert
without probe's env). The `screenshots/` producers wire timeline +
invariants but no frame capture: a posed walk has no steady-state window,
so a captured fps would measure the script. Depth beyond the generic
checks:

| Example | extra depth |
|---|---|
| systems/scenario_grammar | monotonics: beat, rocks_destroyed, round, area_entries, area_exits, escort_neutralized, ring_cleared |
| systems/player_path | monotonics: target_down, leg + per-beat markers |
| systems/outcomes | monotonic: hostile_down + distinct beat markers per cycle (kill -> defeat overlay -> activate -> kill -> activate -> done) |
| sections/* | one `outcome: <slug>` marker per asserted invariant, **27** across the five ranges, pinned both ways by `sections_assert_their_invariant_roster` (crates/nova_probe/tests/catalog_drift.rs): a roster slug with no marker is a removed invariant, a marker with no slug is one added without saying so |
| stress/* | stage markers (setup/steady/teardown) around the measured window; `scene_baseline` is the unmarked reference scene |

The ui/ flows carry no extra markers on purpose: they are state-transition
shaped, and the generic timeline already records every transition.

Probe addresses examples by NAME (`probe run scenario_grammar`); categories
come from `examples/<category>/`. `--fps` runs a DEDICATED capture-only pass -
the clean pass never arms the capture (the recorder's per-entry flush
contaminated fps-on-clean numbers), and the completion protocol keeps the app
alive until the window closes. Enrolled scenes RELOAD and replay while the
capture fills, so the window measures activity; reload intervals are EXCLUDED
from the stats (their count is host-speed-dependent) and reported as their own
line ("3 scene reloads - mean/max ms"). Frame rows carry their build profile
(schema v3); dev rows are labeled NOT a baseline.

The capture window is ONE window - `DEFAULT_WARMUP_FRAMES` 180 /
`DEFAULT_CAPTURE_FRAMES` 900 (`crates/nova_probe/src/capture.rs`) for every run
that captures at all, so probe numbers stay comparable with the sweep's. Your
own `NOVA_PERF_WARMUP`/`NOVA_PERF_FRAMES` always win.

The completion deadline is SIZED to the fps window, not a flat 120s: probe sets
`NOVA_AUTOPILOT_DEADLINE` for the fps pass to `(warmup + frames) / ~2fps +
margin`, so a legitimately-slow capture (a heavy scene in a dev build under
software rendering) COMPLETES instead of tripping the hang detector. A genuine
hang still fails, at a window-appropriate bound; your own
`NOVA_AUTOPILOT_DEADLINE` overrides it. Every example's `main` returns
`AppExit`, so a deadline expiry (or any harness error-exit) is a NON-ZERO
process exit that `process_exit` reports - not just a log-scan flag.

## Host knobs (flamegraphs)

samply needs `perf_event_paranoid <= 1` and, on many-core hosts, a raised
`perf_event_mlock_kb` (e.g. 16384). Load profiles with the URL `samply load`
prints (drag-dropping the file loses the local symbol server = hex frames);
driver-blob/libc frames stay hex regardless - judge by our modules' frames.
