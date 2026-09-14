# Compute-worker experiment, 2026-09-13

## Claim

Eight compute workers is the fastest tested setting for release `wfc_arena`
on this host. Against a fresh default-16 reference, stock mean frame time
fell 28.3% and p99 fell 33.5%. The fixed-step-ceiling comparison also improves.
The CPU profiles confirm much less transform-queue lock spinning.

This is a successful experiment, not a shipped default. The temporary source
change was removed after building and before captures began. No firing,
targeting, damage, lifetime, collision, or wreck rule changed. The task remains
open pending a shipping decision and wider performance validation.

## Change and controls

Base: `24cedcf99dd1bd11f2ed9a632b03c1378346400b`, on `master`.
The only temporary runtime change was in
`crates/nova_core/src/lib.rs:247`, before adding `DefaultPlugins`:

```rust
let mut task_pool_options = bevy::app::TaskPoolOptions::default();
if let Some(cap) = experiment_cap {
    task_pool_options.compute.max_threads = cap;
}
// Add .set(bevy::app::TaskPoolPlugin { task_pool_options }) to DefaultPlugins.
```

A scratch-only `NOVA_EXPERIMENT_COMPUTE_CAP` selected 4 or 8. The reference
left it unset and used Bevy's defaults, producing 16. After plugin setup, the
experiment asserted the requested actual compute count and logged compute,
IO, and async counts. Every run also checked that log: compute 4/8/16, IO 4,
async 4. Only compute changed. This was not a total-pool or affinity cap.
There is no new public Nova environment setting in the working tree.

- Host: Intel Core i9-12900F, 24 logical CPUs, hybrid P/E-core design.
- GPU/display: RTX 3060 Ti, Vulkan, real display `:0`, 1920x1080, default
  quality, immediate presentation. No affinity changes; CPUs 0-23 allowed.
- Fixture: measured 4v4 `wfc_arena`, scripted autopilot, draft `--seed 20260816`
  AND gameplay `NOVA_SEED=20260816`, isolated profile directories.
- Readiness and liveness retained; 60 warmup and 360 capture frames.
- Budgets: 180-second game deadline, 240-second recording supervisor.
- All binaries were built serially through Nix, respecting the Cargo job cap.
  Profiling: full DWARF and frame pointers, inherits dev. Clean FPS: release,
  `debug` feature, no trace feature or frame-pointer override.
- Three Samply runs per arm at 1000 Hz; then five release repeats per arm in
  each of two modes: ceiling `NOVA_PROBE_MAX_DELTA=0.015625`, then stock.
- Arm order rotated within repeat blocks. No builds or competing GPU runs
  overlapped captures. CPU analysis and report rendering ran afterward.
- The standard capture collectors were retained. No correctness recording,
  custom trace spans, GPU timestamp queries, or screenshots entered FPS sets.

The same binary served every arm within each build profile. Binary SHA-256s:

- Release:
  `a2aa404842ff832dbec34ea74ad5acceb39d090bc3535e749b2ea76ec373feb1`
- Profiling:
  `af730a45ee43206ac580e302d657d4b783395364026de7474a732bd336329f40`

## Clean release results

References are medians of five per-run means/medians; p99 is the median among
admitted runs. All six sets admit 5/5 under the existing 20% repeat band. No
refresh-cap suspicion, capture refusal, ERROR, or sampler lost-sample report
was found. The common XSETTINGS warning is a platform warning, not a game ERROR.

| Mode | Compute | Mean ms | Median ms | P99 ms | Mean range ms | P99 range ms |
|---|---:|---:|---:|---:|---:|---:|
| Stock | 16 | 18.6946 | 17.0669 | 41.1763 | 17.0176-19.6628 | 36.3914-49.2947 |
| Stock | 4 | 14.1070 | 14.2332 | 28.0036 | 13.9800-14.2200 | 26.5806-30.1050 |
| Stock | 8 | 13.4105 | 13.6151 | 27.3947 | 13.1366-13.8587 | 25.0355-29.5400 |
| Ceiling | 16 | 19.0118 | 17.7295 | 38.6545 | 18.4809-20.8465 | 36.4038-48.8589 |
| Ceiling | 4 | 14.0561 | 14.8387 | 21.2415 | 13.9569-14.2397 | 19.9873-21.8136 |
| Ceiling | 8 | 13.1782 | 13.6335 | 21.5774 | 12.6191-13.6057 | 19.1635-26.4130 |

Eight versus sixteen:

- Stock: mean -28.3%, median -20.2%, p99 -33.5%. About 75 versus 53 FPS
  from the reference mean. Mean and p99 repeat ranges do not overlap.
- Ceiling: mean -30.7%, median -23.1%, p99 -44.2%. Those ranges also do
  not overlap. Removing catch-up bursts does not remove the gain.
- Eight is about 4.9% faster by stock mean than four. Their p99 ranges
  overlap; there is no established tail advantage of eight over four.

The saved v0.13.2 means were about 13.5-15.2 ms, so this arena result is
numerically near that range. This is NOT a new matched v0.13.2 comparison.
Use the fresh 16-worker arm, not the earlier 20 ms sessions, for the causal
cap comparison. No cross-host or universal eight-worker optimum is established.

### Workload evidence and remaining gap

Every run retained the same eight-ship draft. The initial, pre-combat census
reported 29,061 entities, 14,099 mesh instances, 214 distinct meshes, and
1,742 skin plates. This census is taken at frame 90 after Playing, before
combat readiness; it is not a live-combat population control.

Fixed-step totals, repeat order:

| Mode | Compute | Steps in each 360-frame capture |
|---|---:|---|
| Stock | 16 | 402, 430, 392, 453, 452 |
| Stock | 4 | 322, 323, 325, 325, 328 |
| Stock | 8 | 302, 304, 318, 319, 309 |
| Ceiling | 16 | 346, 345, 348, 343, 339 |
| Ceiling | 4 | 305, 307, 305, 305, 306 |
| Ceiling | 8 | 283, 290, 289, 293, 290 |

Cumulative rounds at the third status report, nominally 15 simulation seconds:

| Mode | Compute | Rounds across five repeats |
|---|---:|---|
| Stock | 16 | 3575, 3256, 3486, 3253, 3507 |
| Stock | 4 | 3542, 3570, 3551, 3569, 3483 |
| Stock | 8 | 3534, 3504, 3324, 3582, 3535 |
| Ceiling | 16 | 5003, 4947, 4912, 4655, 4658 |
| Ceiling | 4 | 4892, 5127, 4728, 4838, 4906 |
| Ceiling | 8 | 4601, 4717, 4669, 4809, 4928 |

All thirty reports retain four roots per team. Shot totals remain comparable;
there is no evidence here of recovering FPS by suppressing the intended fire
rate. But these are cumulative shots, not live rounds, and roots do not prove
intact controllers. Faster frame windows simulate fewer steps, including under
the ceiling because zero-step frames remain possible. Exact fixed-tick and
live-population control is still missing; do not call these identical fights.

## Samply explanation

Nine usable profiles contain 388,184 CPU samples in their gated combat windows.
Local symbol sidecars supply names and inline frames. The analysis uses
positive `threadCPUDelta` weights and calibrated monotonic-clock boundaries,
as in `SAMPLY.md`. Unknown leaf CPU is about 1.2-2.4%.

The new binary's confirmed lock retry occupies
`0xdbfebd1 <= PC < 0xdbfebe6`. A unique byte-pattern match and fresh disassembly
identify the same `pause` / `lock cmpxchg` / backward branch as before.
Addresses from the earlier binary were not reused blindly.

Medians across three profiles per arm, summed across threads:

| Compute | CPU ms/frame | Lock-spin CPU ms/frame | Lock-spin CPU ms/fixed step | Spin share range |
|---|---:|---:|---:|---:|
| 16 | 178.17 | 41.48 | 20.20 | 23.3-24.3% |
| 4 | 65.79 | 0.57 | 0.38 | 0.7-0.9% |
| 8 | 92.07 | 7.56 | 4.99 | 8.2-8.7% |

Eight cuts the median sampled spin per frame by 81.8%; even divided by fixed
steps, it falls about 75.3%. Four removes still more spin and uses less total
CPU, but eight wins the clean release frame-time comparison. Minimizing CPU
work is not the same as minimizing frame latency.

Median inclusive path CPU ms/frame, again NOT additive frame budgets:

| Path | 16 | 4 | 8 |
|---|---:|---:|---:|
| Transform propagation | 59.59 | 7.85 | 18.57 |
| Physics broad phase | 19.96 | 8.20 | 9.39 |
| Mesh extract/collect | 18.68 | 8.21 | 11.89 |
| Round step | 2.61 | 2.06 | 2.29 |
| Sensor contacts | 2.19 | 1.77 | 1.97 |

Caller scopes overlap, and the profile fights run different fixed-step totals:
16 workers 779/709/851, four 567/507/533, eight 502/589/569. Normalizing by
steps is only a coarse check, not equal-work instrumentation. Changing the
whole compute pool also changes scheduling and useful parallelism. The result
supports contention as a major cost, not attribution of every saved release
millisecond to one lock. No GPU execution-time claim is made.

Source evidence remains:

- `bevy_transform-0.19.0/src/systems.rs:574-610`: pooled/local propagation
  workers, tight queue-lock retries, and waiting for other busy workers.
- `crates/nova_core/src/lib.rs:247-253`: default plugin assembly point.
- `bevy_app-0.19.0/src/task_pool_plugin.rs:113-146,229-258`: default pool
  allocation and compute-pool creation.

Dependency paths are relative to the local Cargo registry directory named in
`SAMPLY.md`. No dependency files were changed.

## Correctness and rendered output

Separate dev-profile correctness-only runs used the same temporary cap binaries
at sixteen and eight workers. The preserved executables were launched with the
probe's normal correctness environment, timeline, invariants, and contract
output; the existing `probe report` rendered their honest run manifests. The
CLI build step was not rerun, because it would replace the preserved variant
with restored source. All six reports are OK; every claimed check is PASS.
Frame-time checks are N/A, not performance passes.

| Fixture | 16-worker evidence | 8-worker evidence |
|---|---|---|
| `wfc_arena` | smoke duel passed; 650 invariant frames, 0 violations | smoke duel passed; 685 frames, 0 violations |
| `stress_bullets` | 8 mounts/16 sections; peak 1552 rounds; drain and clean teardown; 822 invariant frames | same rig; peak 1632; drain and clean teardown; 524 invariant frames |
| `stress_point_defense` | 12 mounts/12 bays; all mounts working; peak 74 inbound, 51 intercepts, peak 2153 rounds; drain/teardown; 1700 invariant frames | same rig; peak 75 inbound, 54 intercepts, peak 2153 rounds; drain/teardown; 1730 invariant frames |

Both stress fixtures' outcome markers and final zero counts were inspected,
not only process exits. WFC's ordinary correctness path is a smoke duel, not
the measured 4v4. Six correctness passes do not establish stress-scene FPS.
Their incidental debug FPS prints are not clean release comparisons.

A separate eight-worker 4v4 run produced an inspected 1920x1080 combat image:
ships, effects, debris, and rounds are visible with no apparent rendering
failure in that frame. Its focused/screenshot capture is explicitly excluded
from timing (`stock-8-99`, not repeats 1-5). This is not pixel-equivalence proof.
The generated release report was rendered and inspected separately as well.

No full workspace tests or Clippy were run. Source restoration was checked
byte-for-byte, and `git diff --check` passed.

## Next decision and blast radius

At the arena-only stage, the recommendation was to take the eight-worker
candidate through clean release stress controls before a default decision.
The user approved those controls. Completed results in `STRESS.md` show gains
in bullets and point defense too; no global default has changed. The smallest
potential shipping change remains:

```rust
let mut task_pool_options = bevy::app::TaskPoolOptions::default();
task_pool_options.compute.max_threads = 8;
// Set TaskPoolPlugin in DefaultPlugins. No experiment env/assertions/logging.
```

The pool serves physics, transforms, and rendering. This could reduce useful
parallel throughput in other scenes or on other CPUs; the hybrid i9 result
cannot settle that. IO/async pools, fixed scheduling, and gameplay/content
rules would remain unchanged. A global default is a separate user decision.

Alternative: prototype a transform-only queue/worker change, preserving full
parallelism elsewhere. That is narrower at runtime but requires a larger
engine-side change and its hierarchy/thread-safety proof. This experiment
neither implements nor compares that alternative.

## Artifacts and cleanup

Scratch root: `/tmp/nova-perf-20260913/workers-24cedcf99/`.

- `runtime.patch`, original/experimental core snapshots, build logs, binary
  hashes, preserved profiling/release/correctness executables.
- `samply-{4,8,16}-{1,2,3}`: raw profiles, symbols, manifests, analyses, SVGs.
- `{stock,ceiling}-{4,8,16}-{1..5}`: captures, CSVs, logs, manifests, census.
- `summary.json`, `summary.log`, `census-summary.json`, `report.html/png`.
- `correctness/{16,8}/{wfc_arena,stress_bullets,stress_point_defense}`.
- `stock-8-99/combat.png`, marked visual-only and excluded from FPS.
- Build, run, analysis, correctness, screenshot, and spin-location helpers.

The original Samply binary was preserved at
`/tmp/nova-perf-20260913/samply-24cedcf99/wfc_arena-profiled` before rebuilding.
Only Markdown findings stay in Git. All raw artifacts are disposable.

All owned builds, game runs, sampling, and report-rendering processes finished.
No experiment source remains. Profiling access restoration via `sudo -n`
again required a password, so settings remain paranoid=1 and mlock_kb=16384.
The user must restore the original values:

```sh
sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516
```
