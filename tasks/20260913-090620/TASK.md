# Restore wfc_arena performance to the v0.13.2 baseline

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.14.0, bug, performance, ai, targeting, projectile, probe

## Goal

Find and fix the post-v0.13.2 `wfc_arena` performance regression without
undoing the intended AI, targeting, combat, or wreck behavior. Restore the
release-profile 1920x1080 4v4 benchmark to a justified budget near the
v0.13.2 baseline.

## Confirmed regression

Matched five-repeat `wfc_arena` probes were run on display `:0` with the
NVIDIA GeForce RTX 3060 Ti, release profile, default quality, 1920x1080, and
the fixture's fixed scripted seed and measured 4v4 roster.

| Revision | Reference mean | Reference median | Reference p99 |
|---|---:|---:|---:|
| v0.13.0 `d101f1d43` | 15.09 ms | 13.77 ms | 35.78 ms |
| v0.13.2 `b3c6f579c` | 15.17 ms | 13.64 ms | 37.53 ms |
| pre-session `12200a16b` | 20.75 ms | 19.11 ms | 47.10 ms |
| HEAD `7b07528d2` | 19.57 ms | 17.52 ms | 43.28 ms |

HEAD is 29.1% slower by mean and 28.4% slower by median than v0.13.2.
Every HEAD repeat averaged below 60 FPS. The p99 increase is not established:
it remains within the repeat spreads.

The regression predates session `20260909-214629`. HEAD improved by 5.7% mean,
8.3% median, and 8.1% p99 against its immediate pre-session revision
`12200a16b`. The commits in `34d5e9349..7b07528d2` are not the origin of this
`wfc_arena` regression.

All measured runs completed without capture refusal, invariant failure,
refresh-cap evidence, or offending log lines. One repeat was discarded by the
set gate for each release; all five were admitted for `12200a16b` and HEAD.

## Workload evidence

The `wfc_arena` example source is unchanged from v0.13.0 through HEAD. v0.13.2
and HEAD field the same eight seeded hulls with the same section and weapon
counts. The current fight is dynamically heavier:

- Representative v0.13.2 capture status lines report about 1,200 combined
  kinetic and pierce rounds fired.
- HEAD capture status lines commonly report about 3,000-8,000 rounds.
- `12200a16b` is also much heavier than v0.13.2 and is slower than HEAD.
- A 360-frame window contains more fixed steps when rendering slows, which can
  amplify the projectile load. Compare fixed-step totals as well as frames.

Do not treat fired counts at the final status line as a perfectly matched
quantity. The capture is frame-bounded, so a slower run simulates more fixed
steps. Add a matched simulated-time or workload assertion before assigning the
whole delta to projectile count.

The benchmark contract is in `examples/playable/wfc_arena.rs`:

- readiness and liveness gates: lines 437-448 at `7b07528d2`;
- four ships per team: line 635;
- warmup/capture window `(60, 360)`: line 667.

## Targeting candidate

Commit `68c3ebe6f` (`See what a ship can see, once`) introduced the shared
sensor pass before `12200a16b`.

At `7b07528d2`:

- it is scheduled every Update in
  `crates/nova_ship/src/input/targeting/mod.rs:143-148`;
- `update_sensor_contacts` loops over each observer and then every candidate,
  with possible occlusion scans, in
  `crates/nova_ship/src/input/targeting/sensing.rs:215-291`.

Diagnostic dev-profile traces measured:

- HEAD `update_sensor_contacts`: about 1.55 ms per call;
- `12200a16b`: about 1.53 ms per call;
- v0.13.2 `update_ai_target`, which it replaced: about 0.93 ms per call.

This supports a targeting cost, but does not explain the complete release FPS
delta by itself. Trace timings are diagnostic dev-profile timings, not the
release FPS measurements.

The post-release AI flight series was another starting candidate. Fresh
measurements now locate a large direct regression at the earlier `6dadd95f2`
(`Aim at how big the thing is`). See `IDENTIFICATION.md`. Smaller earlier
costs and the complete per-path cost split remain unisolated.

## Existing local evidence

The original machine may still contain:

- `/tmp/nova-release-perf/runs/d101f1d43/wfc_arena/report.html`
- `/tmp/nova-release-perf/runs/b3c6f579c/wfc_arena/report.html`
- `/tmp/nova-release-perf/runs/12200a16b/wfc_arena/report.html`
- `/tmp/nova-release-perf/runs/7b07528d2/wfc_arena/report.html`
- corresponding `frametime.csv`, `checks.json`, FPS logs, census, and traces;
- `/tmp/nova-release-perf/{revision}-systems.json`, locally aggregated from
  trace spans.

These `/tmp` files are not durable. The measurements and conclusions required
for handoff are recorded above. Rerun proof before landing a fix.

## Identification result, 2026-09-13

The large measured boundary is `6dadd95f2`, **Aim at how big the thing is**.
It replaced the fixed 0.92-degree firing cone with a target-sized cone.

- Immediate parent `f894d4c20`: 16.10 ms mean, 13.97 ms median.
- `6dadd95f2`: 21.15 ms mean, 18.67 ms median.
- Direct increase: 31.4% mean and 33.6% median. Five repeats per arm.
- With at most one fixed step per frame: 15.17 -> 17.93 ms mean, +18.1%.
  Catch-up bursts are not the only cause. Zero-step frames still exist.
- At the same nominal 15-second simulation-clock report: 1040-1369 rounds
  before versus 3950-4284 after. Both retain four ship roots per team.
  This is not an exact live-population fixture or a complete cost attribution.
- The sensor commit alone measures 13.25 ms mean, versus 13.50 ms for the
  fresh v0.13.2 set. It does not reproduce the large gap. A smaller earlier
  regression remains possible; the signature boundary is not isolated.
- All 35 capture-only runs completed without capture refusal or ERROR lines.
  Draft assertions confirm identical eight-ship section and weapon rosters.
  P99 ranges overlap; no tail change is established.

Findings, caveats, repeat spreads, fixed-step counts, and reproduction are in
`IDENTIFICATION.md`. A separate rendered combat frame was inspected and
excluded from timing. These were capture-only runs, not correctness passes.

Identification ran on `master` at `7b07528d2`, without gameplay edits or a
commit. Historical builds use `git archive` snapshots, each with its own
`target/`.
The user then approved sustained-combat profiling and a direct-collider test
replacement. The experiment is complete; see `DIRECT-COLLIDER.md`.

- The direct helper is 44.3% faster in a fixed-workload dev comparison, with
  identical hit counts and impact times. This is not release arena FPS.
- 33 projectile tests pass, including 324 spatial-reference comparisons.
  The work-removal assertion fails on the old path and passes on the new one.
- `wfc_arena`, `stress_bullets`, and `stress_point_defense` correctness probes
  all report OK. A separate rendered combat image was inspected.
- Twenty release captures show no clear arena gain: stock mean -2.8% but
  median +1.5%; capped mean +2.5%, median +1.3%. Repeat ranges overlap.
- Idle-queue profiling puts exact tests at only 7.1% of the round-step span.
  Candidate searches, sensor updates, physics, and round command flushes cost
  more. The headless trace does not attribute rendering cost.
- Fresh comparisons pin `NOVA_SEED=20260816` as well as the draft's `--seed`.
  Earlier identification captures pinned the draft, not the gameplay RNG.
- The user approved keeping the direct-collider optimization independently.
  Code and tests stay; no arena FPS gain is claimed.
  All owned processes have finished.
- Keep this task folder Markdown-only, as requested. Findings stay in these
  notes; raw runs and helper scripts stay outside the repository under
  `/tmp/nova-perf-20260913/`. Those scratch files are not durable.

This does not meet acceptance. Fixed-tick/live-population control, full cost
attribution, and restoration near the release baseline remain open.

## Required investigation

1. Reproduce the v0.13.2 versus HEAD result with matched five-repeat release
   sets. Inspect reports, checks, frame CSV, logs, census, fixed-step counts,
   workload counts, and rendered output.
2. Add or adapt a non-ending, simulated-time-bounded combat probe that asserts
   comparable live projectile, ship, section, and wreck populations. Keep an
   easy scene's trigger intact.
3. Bisect `b3c6f579c..12200a16b`. Start around `68c3ebe6f` and `f3f745791`.
   Separate sensor-pass cost from intentional changes that make the AI fire
   more often.
4. Profile the established boundary. Check sensor candidate collection,
   occlusion queries, projectile spawn/render/expiry, physics broad and narrow
   phases, fixed-step feedback, AI acquisition, and wreck cleanup.
5. Optimize the responsible paths. Preserve sensor, lock, AI, damage, wreck,
   and combat semantics unless a behavior change is explicitly approved.
6. Add a failing assertion or performance comparison that distinguishes the
   regression. Use a named, matched release reference; never assert timing from
   one run.

## Acceptance

- Correctness assertions and scenario behavior still pass.
- The workload is demonstrably comparable before and after the fix.
- A matched five-repeat release-profile 1920x1080 `wfc_arena` set shows a
  material improvement against `7b07528d2` and no unexplained regression
  against v0.13.2.
- State the achieved mean, median, p99, repeat spreads, fixed-step counts, and
  remaining gap. Do not claim p99 improvement outside the evidence.
- Run a projectile stress control such as `stress_bullets` to separate arena AI
  behavior from projectile engine throughput.
- Inspect generated reports and rendered output. A headless run does not prove
  appearance.
- Update affected performance documentation and the changelog if the shipped
  behavior or user-visible performance changes.
