# Sustained-combat Samply profiles, 2026-09-13

## Claim

Transform-worker lock contention is the strongest new optimization candidate.
Three profiles put 23.8-24.1% of sampled CPU in one confirmed lock retry loop.
This is not a claim of equivalent frame-time savings or regression origin.
No runtime changes or new release comparisons were made in this initial pass.
The user then approved the worker-count experiment; completed results are in
`WORKERS.md`.

## Fixture and build

- Runtime revision: `24cedcf99dd1bd11f2ed9a632b03c1378346400b`.
- Samply 0.13.1, 1000 Hz, full symbols, frame pointers, inline frames.
- Build: `--profile profiling --features debug`. This inherits dev, with
  optimized dependencies. It is NOT the release profile.
- Same binary in all three runs. SHA-256:
  `b77f0d782783beece500ce99a3fefa07ceba4c8143061fbb0d6ed2d9af56d19d`.
- Windowed stock `wfc_arena`, seeded 4v4 roster, Vulkan, RTX 3060 Ti,
  display `:0`, 1920x1080, default quality, immediate presentation.
- Draft `--seed 20260816` AND gameplay `NOVA_SEED=20260816`.
- Scripted autopilot, 180-second game deadline, 240-second supervisor budget.
- `NOVA_PROBE=1`, 60 warmup and 360 capture frames. Both teams must fire and
  connect before readiness; the existing liveness gate must remain true.
- No fixed-delta ceiling, correctness recorder, trace spans, or screenshots.
- Build completed before sampling. Three runs were serial. Analysis and graph
  rendering ran afterward, not during the captures.

The ordinary Samply harness pass does not request capture, so it can exit at
its smoke predicate. These runs explicitly enabled the measured 4v4 contract.
See `crates/nova_probe_cli/src/native/env.rs:199` and
`examples/playable/wfc_arena.rs:437-448,635,667`.

## Verification and limits

All three collectors completed their live-combat windows. Logs contain no
ERROR, capture refusal, or sampler lost-event/sample report. Profiles and
local symbol sidecars exist and contain Nova frames. The generated all-thread
call-stack image was rendered and inspected. This was not a fresh gameplay
appearance check or a correctness-suite run.

The eight-ship roster matches across all three runs. Three status reports
inside each sampled window show rising shot and damage totals for both teams,
with four roots per team. Final status reports contain 12,219, 11,921, and
12,076 cumulative rounds respectively. These are not live-round counts or an
identical-workload assertion.

| Run | Combat window | Fixed steps | CPU samples | Sampled CPU | Unresolved leaf CPU |
|---|---:|---:|---:|---:|---:|
| stock-1 | 11.415 s | 730 | 64,331 | 61.813 s | 1.22% |
| stock-2 | 11.561 s | 741 | 65,004 | 62.492 s | 1.29% |
| stock-3 | 11.647 s | 744 | 66,086 | 63.223 s | 1.24% |

CPU is summed across threads and can exceed wall time. The denominator below
is sampled CPU, not frame wall time. Each positive sample is weighted by
`threadCPUDelta` in microseconds. Inclusive rows overlap and must not be added.
Inlining is expanded from the local symbol sidecar, innermost frame last.

Analysis includes only the interval from the collector's warmup-complete log
to its completed 360-frame summary. Startup and approach remain in the raw
profiles but are excluded from these results. These are log boundaries, not
instrumented frame markers. Samply's Linux timestamps were verified against
`CLOCK_MONOTONIC` with a separate busy-loop calibration. Wall-clock log times
were mapped through a clock pair recorded by the supervisor.

The Rust binary has no ELF build ID. Symbol lookup therefore matches its
Samply debug ID, not `codeId=None`. An initial analysis assertion caught that
lookup error; correcting it and expanding inline frames produced these
results. This was an analysis issue, not a failed gameplay capture.

A fixed frame window is not a fixed-workload comparison. The runs contain
730-744 fixed steps, damaged controllers, and evolving combat populations.
Root liveness does not prove all eight ships remain combat-effective. Sampling
and the profiling build also affect frame pacing and catch-up. Do not compare
these diagnostic frame times with the saved release FPS figures.

Context-switch markers were requested, but the profiles contain only mmap
markers. Sleeping/waiting intervals and GPU execution are not attributed.
The spin finding is active CPU, established from sampled instruction addresses.
No baseline was profiled, so these stacks do not date the regression.

## Expensive paths

| Inclusive path | Share of all sampled CPU, three runs |
|---|---:|
| Transform propagation | 34.1-34.6% |
| Confirmed worker lock retry, subset of propagation | 23.8-24.1% |
| Physics broad phase | 11.0-11.5% |
| Mesh extraction and collection | 10.5-11.0% |
| Physics solver/integrator | 8.1-8.6% |
| Transform dirty marking | 2.0-2.2% |
| Round step | 1.2-1.5% |
| Sensor contacts | 1.2-1.3% |

These path unions use caller names, including generic task wrappers. Worker
execution and nested executor scopes can overlap other rows; they are not an
exclusive system-time partition. Deferred-operation ancestor scopes account
for another 11.7-12.3% inclusively, overlapping physics/rendering. Do not call
that the projectile spawn/destruction cost.

The main thread uses 20.1-20.5 sampled CPU ms per captured frame. The pipelined
render thread uses 10.1-10.3, with further rendering work on compute workers.
Its OS name is also `wfc_arena`; its callers identify it as the render thread.
Sixteen compute workers account for most CPU. Work on these threads overlaps;
these values are not additive frame budgets.

### 1. Transform queue contention

The largest self-CPU symbol is the future-poll wrapper for
`bevy_transform::systems::parallel::propagate_parent_transforms` workers:
26.2-26.6% of all sampled CPU. It is not all transform arithmetic.

The hot instruction addresses are inside this loop in the tested binary:

```text
0xdbfc651: pause
0xdbfc653: movq ...
0xdbfc65e: lock cmpxchgl ...
0xdbfc664: jne 0xdbfc651
```

Samples at `0xdbfc651 <= PC < 0xdbfc666` account for 23.8-24.1% of CPU. This
identifies the failed lock acquisition path, rather than guessing from the
wrapper's name. It does not include other possible spin sites. Sampling skid
and inline attribution still apply; this is not an exact instruction timer.

Local dependency evidence, under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

- `bevy_transform-0.19.0/src/systems.rs:574-580` starts
  `thread_num() - 1` pooled propagation workers plus one local worker.
- `systems.rs:595-610` retries `queue.receiver.try_lock()` and busy-waits
  while another worker can produce work.
- `systems.rs:786` uses 512-entity work chunks. Few initial chunks can leave
  many workers contending. That queue occupancy is a hypothesis, not a census.
- `avian3d-0.7.0/src/physics_transform/mod.rs:95-104` also registers Bevy
  propagation before physics; `bevy_transform-0.19.0/src/plugins.rs:38-46`
  registers the regular PostUpdate pass. Removing either is not a safe fix
  without proving transform/physics ordering and sampled collider poses.

Nova uses the default task-pool plugin at
`crates/nova_core/src/lib.rs:247-253`. Bevy's default reserves up to four IO
and four async workers and assigns the remainder to compute:
`bevy_app-0.19.0/src/task_pool_plugin.rs:113-146`.

### 2. Broad-phase tree traversal

Self CPU in `obvhs::bvh2::Bvh2::aabb_traverse` specialized for Avian's
`query_tree` is 6.5-6.7%, with another 2.4-2.7% in its query closure. The broad
phase is a larger target than the already optimized exact round helper.

`avian3d-0.7.0/src/collision/broad_phase/bvh_broad_phase.rs:67-163` queries
collider trees for moving proxies. At `:230-267`, layer and same-body rejects
occur after reaching leaves. A collider/moved-proxy census could identify
wasted traversal of dense same-body sections, sensors, or wrecks. This profile
does not establish which of those populations dominates. Rounds themselves
are not rigid bodies; do not equate the broad-phase count with round count.

### 3. Render preparation

Mesh extraction/collection uses 10.5-11.0% of total CPU. Relevant paths are
`collect_meshes_for_gpu_building`, `extract_meshes_for_gpu_building`,
`extract_mesh_for_gpu_building`, the render-mesh hash lookup, and
`AtomicSparseBufferVec::set`. Visibility and shadow specialization also appear.

See `bevy_pbr-0.19.0/src/render/mesh.rs:1901,2407`. A useful follow-up is a
render-entity/change census: which round, ship, and wreck meshes are rebuilt
or re-extracted each frame? Batching is a candidate, not an approved change.
No GPU-time or shadow-removal gain is established here.

### 4. Sensor and projectile work

Sensor contacts use 2.10-2.21 aggregate sampled CPU ms per captured frame;
round advancement uses 2.13-2.63. Their small all-core percentages do not make
them free on the frame's critical path.

- `crates/nova_ship/src/input/targeting/sensing.rs:221-300` scans candidates
  per observer and asks `OcclusionScan::is_occluded` after cheap rejects.
- `crates/nova_ship/src/input/targeting/occlusion.rs:47-70` performs the ray
  query. Candidate/occluder indexing is a narrower possible optimization.
- `crates/nova_gameplay/src/rounds.rs:643-680` still runs the candidate cast.
  `TipWalk::next` includes exact validation too; its inclusive cost is not a
  candidate-only measurement. `rest_frame_impact` was not independently
  resolved here. The earlier span trace, not these profiles, isolated it.
- Named damage/destruction functions are individually small. Their downstream
  ECS, physics, transform, and render work is not isolated by that observation.
  It does not justify dropping wrecks, damage, or cleanup behavior.

## Proposed next decision

Recommendation: first isolate compute-pool contention with temporary 4/8-worker
variants against the current 16-worker reference. Do not ship a new default
based on aggregate CPU savings alone.

Proposed experiment uses existing Bevy fields, not a new public Nova setting:

```rust
let mut options = TaskPoolOptions::default();
options.compute.max_threads = cap; // Experiment arms: 4, 8, current default.
// Set TaskPoolPlugin { task_pool_options: options } in DefaultPlugins.
```

- **Change:** temporary task-pool configuration only. No approved runtime
  edit yet; no change to firing, lifetime, damage, targeting, or collision.
- **Blast radius:** the compute pool serves transforms, physics, and rendering.
  A cap can reduce contention while slowing useful parallel work elsewhere.
  A native diagnostic is not a validated cross-platform shipping policy.
- **Verification:** match revision, workload, seeds, quality, and presentation.
  Compare serial repeat sets against the named 16-worker reference. Inspect
  sampled spin, useful work, fixed steps, combat counts, and clean release
  frame times separately. Retain liveness and the expensive combat trigger.
  If promising, run affected arena and projectile correctness flows before
  proposing a shipping change. Assert behavior/work, never elapsed time.
- **Alternative:** start with sensor candidate/occluder indexing. Its scope is
  narrower, but the sampled CPU opportunity is smaller and visibility/lock
  behavior needs dedicated assertions.

## Artifacts and profiling access

All raw data and scripts are disposable, outside Git:
`/tmp/nova-perf-20260913/samply-24cedcf99/`.

- `build.log`, `build.exit`, build/run PID files and manifests.
- `stock-{1,2,3}/profile.json.gz`, `profile.json.syms.json`.
- Each run's log, capture JSON, manifest, `analysis.json`, and SVG stack views.
- `stock-1/all-threads.png` is the inspected graph, not a gameplay screenshot.
- `record.py`, `analyze.py`, `run.sh`, clock calibration, and disassembly.

Only Markdown findings are retained in the task. Scratch artifacts may vanish.

The user enabled paranoid=1 and mlock_kb=16384; the enabled preflight passed.
All owned builds and sampling runs have finished. Restoration via `sudo -n`
failed because a password is required. Settings remain 1 and 16384. Restore
the original values with:

```sh
sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516
```
