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

## Samply follow-up

The user requested sampled function stacks at `24cedcf99`, not another
release comparison. Prior profiling used Bevy spans, not Samply.

The initial preflight was blocked by profiling permissions. The user enabled
paranoid=1 and mlock_kb=16384; the next preflight passed. Three serial,
symbolicated 4v4 profiles then completed the capture readiness/liveness gates,
with draft/gameplay seed 20260816 and display :0. Analysis excludes startup
and approach. See `SAMPLY.md` for methods, callers, source, and caveats.

- 195,421 CPU samples in three 360-frame combat windows; 730-744 fixed steps.
- Transform propagation: 34.1-34.6% of all sampled CPU. Disassembly places
  23.8-24.1% in a worker queue's atomic-lock retry loop, not transform math.
- Physics broad phase: 11.0-11.5%; mesh extraction/collection: 10.5-11.0%.
- These inclusive all-thread CPU shares overlap. They are not wall-time
  budgets or predicted FPS gains. The profiling build inherits dev, not release.
- The user then approved a temporary 4/8-worker experiment against the
  16-worker reference. Its completed results are below and in `WORKERS.md`.
- All owned builds and sampling runs finished. Raw profiles, symbols, scripts,
  and graphs remain outside Git under
  `/tmp/nova-perf-20260913/samply-24cedcf99/` and are not durable.
- Restoring the original sysctls with `sudo -n` was blocked by its password
  requirement. Values remain 1 and 16384. User action is still needed:
  `sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516`.

## Worker-cap result

The approved matrix is complete: nine Samply profiles and thirty clean release
captures, plus six separate correctness runs. See `WORKERS.md`.

- Eight is the fastest tested compute count on this i9-12900F / RTX 3060 Ti.
- Stock 16 -> 8: mean 18.6946 -> 13.4105 ms (-28.3%), median
  17.0669 -> 13.6151 ms (-20.2%), p99 41.1763 -> 27.3947 ms (-33.5%).
- Ceiling 16 -> 8: mean 19.0118 -> 13.1782 ms (-30.7%). The gain remains
  without catch-up bursts. Zero-step frames and workload differences remain.
- All six release sets admit 5/5. Mean and p99 ranges for either smaller cap
  do not overlap the 16-worker reference. No refresh-cap suspicion or refusal.
- Eight reduces median sampled lock-spin CPU per frame by 81.8%. Four uses
  still less CPU, but eight has better clean release mean frame time.
- Eight-ship drafts match; nominal 15-second shot totals remain comparable.
  This is not exact fixed-tick/live-population control or a new v0.13.2 set.
- At 16 and 8 workers, `wfc_arena`, `stress_bullets`, and
  `stress_point_defense` correctness checks pass with zero invariant violations.
  An eight-worker combat image and the generated report were inspected.
- Temporary source changes were removed byte-for-byte before captures.
  The improved binaries remain scratch experiments, not the shipping default.
- The user approved release stress controls for the eight-worker candidate.
  Their completed results are below and in `STRESS.md`. No default changed.
  Task stays OPEN; profiling sysctl restoration still needs the user's sudo.

## Release stress controls

Forty release captures compare default 16 versus eight compute workers in
saturated `stress_bullets` and `stress_point_defense`, with five repeats per
arm in stock and fixed-step-ceiling modes. See `STRESS.md` for fixture
controls, spreads, counts, limitations, and reproduction.

| Stock fixture | 16-worker mean | 8-worker mean | Change |
|---|---:|---:|---:|
| Bullets | 3.0000 ms | 2.5130 ms | -16.2% |
| Point defense | 14.8148 ms | 7.4145 ms | -50.0% |

- Ceiling mean gains remain: -19.5% for bullets, -55.1% for point defense.
- All eight sets admit 5/5. Mean and p99 ranges do not overlap between arms.
  No refresh-cap suspicion, capture refusal, or ERROR line occurred.
- Bullet captures hold firing at saturation, with cached live-count liveness
  at 1,000 rounds. All reach a peak of 1,616. PD saturation-period means
  remain about 2,034-2,064 rounds and 103-108 live torpedoes across both arms.
- Fixed-step totals and whole-period collider peaks differ. This is not
  exact fixed-tick or wreck-population control. Wreck-specific counts were
  not collected; an interim attribution to wrecks was corrected.
- Four separate release correctness runs report OK with zero invariant
  violations. Both fixtures drain ordnance and complete clean teardown.
  Two visual-only images and the generated timing report were inspected.
- Temporary core and fixture changes were removed byte-for-byte before
  capture. Only preserved experimental binaries use the cap and census.
- Potential improvement only: set `compute.max_threads = 8` in
  `AppBuilder`'s task-pool options. The user requested recording this candidate
  and will run further tests before deciding. Keep the current default;
  implementation and further agent-run experiments need new approval.
  Smaller allocations would stay smaller, but other CPUs, larger guided-
  torpedo stress, and headless throughput remain unverified. A transform-only
  engine change remains the alternative.
- No shipping change, new commit, new Samply samples, full workspace tests,
  or Clippy. Owned processes finished; privileged sysctl cleanup is blocked.

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

## In-loop propagation experiment, rejected

The user approved building the review's Finding 3. It was built, proven
correct, measured, and REVERTED. See `PROPAGATION-PASSES.md`.

- Avian runs a second full-hierarchy propagation per fixed step, behind a
  dependency default Nova never set. Removing it is mechanically sound:
  `propagate_parent_transforms` drops 2.039 -> 1.004 calls per frame, and
  the 1.303 ms pass is replaced by a 0.004 ms root-body sync.
- The frame does not move. B against A at 16 workers: mean -1.89% with
  overlapping ranges; one-fixed-step bucket -6.77%, p approx 0.095.
- `RunFixedMainLoop` falls 7.2% per step while `PostUpdate` RISES 12.8% per
  frame in 24 of 25 pairs. The fixed-loop pass was pre-cleaning trees for
  `PostUpdate`. The work is shifted, not removed.
- The worker cap is NOT made redundant: changed at 8 workers beats changed
  at 16 by 33.62% mean and 17.18% on the bucket, all ranges disjoint.
- Correctness was clean throughout and is not the reason for rejection.
- Working tree restored; `cargo check` passes. Code kept outside Git at
  `/tmp/nova-bodytx-20260913/rejected-change/`.

This removes "too many propagation passes" as an explanation and leaves the
cost PER pass - the queue spin - as the contention mechanism. Nova cannot
quiet that from its own side while ships move.
