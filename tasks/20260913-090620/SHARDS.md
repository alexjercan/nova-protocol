# Carve chips as GPU particles, 2026-09-13

## Claim

The tripled carve-shard population was a real, removable cost. With chips
thrown as pooled `bevy_hanabi` particles instead of one kinematic body each,
the 4v4 `wfc_arena` release window at 8 compute workers falls from 13.06 ms
to 11.28 ms mean, and the single-fixed-step bucket from 13.55 ms to 11.45 ms
with per-repeat ranges DISJOINT. The 8-worker cap still wins by about a
third after the fix. About half of the single-step gap to v0.13.2 remains.

The change is committed on the sprout `compute-worker-cap` as `38297731e`,
on top of the cap commit `03a4a341b`. Not landed, not pushed.

## Change

`crates/nova_gameplay/src/integrity/spew.rs`, rewritten. A `CarveSpew`
observer no longer spawns `RigidBody + Collider + Mesh3d + TempEntity` per
chip. Each `CarveDebris` (metal, rock) owns one `EffectAsset` and a pool of
persistent `ParticleEffect` emitters:

```rust
struct ShardPool { effect: Handle<EffectAsset>, emitters: Vec<Entity>, next: usize, fired: usize }
const SHARD_EMITTERS: usize = 64;      // pool ceiling per material
const SHARD_EMITTER_FLOOR: usize = 4;  // warmed when a view exists
const SHARD_CAPACITY: u32 = 1024;      // particles per emitter
```

A carve aims the next idle emitter: translation to the hit, the cone basis
(`outward`, `right`, `up`) plus `lip` and `chip` as `EffectProperties`,
then `spawner.settings.set_count(n.into()); spawner.reset()`. Hanabi's
`SimulationSpace::Global` adds only the emitter translation, so the basis
must travel as properties; the emitter `Transform` rotation never reaches a
particle. One emitter serves one burst per frame; a frame that carves more
times than the pool holds drops the rest unchipped, visual only. Metal cools
white-hot to gunmetal through a colour gradient; rock stays brown; both
shrink to nothing over the last 20% of a 2.5 s life.

Ranges read `CarveShardTally.thrown` instead of counting live shard
entities (`examples/systems/system_destruction_finale.rs`,
`examples/systems/stress_hull_collapse.rs`). `docs/sections.md`,
`crates/nova_ship/src/sections/fixture.rs` and `CHANGELOG.md` follow.

Trade-offs, recorded in the module docs: chips are unlit, so the cold
colours are authored as lit values; the spray is GPU random, so a replay is
not chip-identical; the tally is a thrown count, not a live count.

## Evidence: release timing

Same host, same contract as `WORKERS.md` and the handoff: release,
`--features debug`, display `:0`, Vulkan RTX 3060 Ti, 1920x1080,
`NOVA_PROBE_PRESENT=immediate`, `--seed 20260816` and `NOVA_SEED=20260816`,
60 warm-up / 360 capture frames, isolated mod/data/config, inherited
`NOVA_*`/`TRACE_*` cleared, one process at a time, gate on no other
`wfc_arena` and 1-min loadavg below 3.5, 3 s between runs, arm order
rotated per repeat block. Five repeats per arm. The two arms are separate
binaries from the sprout tree with `MAX_COMPUTE_WORKERS` at 8 (committed)
and 16 (an uncommitted flip, restored after the build); each run verified
its binary by SHA-256 and counted the child's `Compute Task Pool` threads.
All ten runs valid, 5/5 admitted per arm, no ERROR line, eight roster
lines, no hanabi error. Reference sets are today's `cap-baseline` captures.

| Arm | Mean | Median | P99 | Mean range | 1-step | 1-step range |
|---|---:|---:|---:|---:|---:|---:|
| chips `38297731e` @8 | 11.2805 | 11.4542 | 24.0480 | 10.1430-13.2431 | 11.4512 | 10.6779-12.7472 |
| chips `38297731e` @16 | 17.7054 | 15.6575 | 42.5184 | 15.9142-19.3109 | 15.6090 | 14.6407-16.1401 |
| HEAD `24cedcf99` @8 | 13.0632 | 13.1757 | 25.0167 | 12.7790-13.5079 | 13.5510 | 13.3227-13.8886 |
| HEAD `24cedcf99` @16 | 19.6870 | 17.2300 | 48.1691 | 17.7149-21.9845 | 16.4256 | 15.9089-18.0793 |
| cone `6dadd95f2` @8 | 13.8916 | 14.2320 | 24.1762 | 12.9386-14.5036 | 14.2900 | 13.4957-14.4645 |
| parent `f894d4c20` @8 | 8.9237 | 9.4412 | 15.1992 | 8.6123-11.2007 | 10.0553 | 9.7185-11.7622 |
| v0.13.2 `b3c6f579c` @8 | 8.0997 | 7.9510 | 14.3500 | 7.7738-8.9313 | 9.2001 | 8.7491-9.8660 |
| v0.13.2 `b3c6f579c` @16 | 13.2363 | 12.4382 | 32.0523 | 12.6525-15.9677 | 13.0842 | 12.5460-14.4290 |

Milliseconds. Mean and median are medians of the per-repeat statistics;
p99 is the median among admitted repeats; "1-step" is the mean over frames
that ran exactly one fixed step, the amplifier-free comparison.

| Comparison | Mean | 1-step | Ranges |
|---|---:|---:|---|
| chips@8 against HEAD@8 | -13.6% | -15.5% | 1-step disjoint; whole-window overlap (two chip repeats sit inside HEAD's range) |
| chips@16 against HEAD@16 | -10.1% | -5.0% | overlapping on both |
| chips@8 against chips@16 | -36.3% | -26.6% | disjoint on both |
| chips@8 against v0.13.2@8 | +39.3% | +24.5% | disjoint on both |
| chips@8 against parent@8 | +26.4% | +13.9% | overlapping |

Of the HEAD@8 to v0.13.2@8 single-step gap (4.35 ms), the chips recover
2.10 ms, 48%. Of the whole-window mean gap (4.96 ms), 1.78 ms, 36%.

Workload check: rounds fired by the third status report were 3277-3643 for
chips@8 against 3498-3648 for HEAD@8; the AI shoots the same. Fixed-step
totals fell 233-305 against 294-311 and zero-step frames rose 78-130
against 58-81, as expected for a faster frame-bounded arm. P99 did not
move: 24.05 against 25.02 with overlapping ranges. The spikes are
elsewhere.

Per-repeat means, chips@8 in run order: 11.14, 10.14, 12.64, 11.28, 13.24.
Four of the five started at a 1-min loadavg of 3.08-3.38, inside the gate
but higher than the first run's 1.25; the spread is wider than HEAD@8's.

Raw data: `/tmp/nova-perf-20260913/shards-hanabi/` with `record.py`,
`run-set.sh`, `gate.sh`, `analyze.py`, `arms.json`, both binaries, and run
directories `shards-{8,16}-{1..5}`.

## Evidence: population

A census at frame 1850 after `Playing` on the chips build (release,
rendered, `census-8-1`) against the cone build's census at the same frame
(`trace-8w/6dadd95f2...-cap8-census1850`, headless dev). Only the shard
rows are comparable; the headless run never loaded section art.

| Row | cone @8 | chips @8 |
|---|---:|---:|
| `Carve Shard` mesh instances (by origin) | 8,610 | 0 |
| `Position` entities | 13,406 | 4,896 |
| `ColliderDensity` entities | 4,713 | 4,713 |
| `RigidBody` entities | 8,958 | below 1,696 |
| `TempEntity` entities | 9,559 | below 1,696 |
| `ShardCooling`, `CarveShardMarker` | 8,610 | below 1,696 |

The census lists the 40 most populous components; "below 1,696" means the
component fell out of that table, whose last entry holds 1,696 entities.
Colliders are steady while `Position` drops by 8,510, so the live body
population is back near the ship, asteroid and wreck count. Sim time at
frame 1850 differs between the two runs (headless dev was clamped to
15.6 ms per frame); read the rows as before/after, not as a matched pair.

## Evidence: appearance

One rendered 1920x1080 combat frame (`shards-8-98/combat-1.png`, timings
discarded) was inspected. Chips read as a spray of small grey grit leaving
the hull with a few brighter hot specks, spreading as the old meshes did.
World-space HUD target markers sit on their hulls. The log shows hanabi
initialised for the RTX 3060 Ti and no hanabi warning or error in any run.
No colour tuning was needed.

## Verification

- `cargo check -p nova_gameplay --features debug --tests` clean; 22 spew
  unit tests pass, including "a fight of carves leaves the solver and the
  renderer nothing to carry" (zero `RigidBody`, `Collider`, `Mesh3d` after
  40 frames of carves) and "one frame cannot be made to throw more than
  its pool".
- `cargo check --features debug` on `system_destruction_finale`,
  `stress_hull_collapse` and `wfc_arena` clean. Those two ranges were
  CHECKED, not run; their thrown-count assertions are unproven live.
- `cargo fmt --check` clean on `nova-protocol`, `nova_gameplay`,
  `nova_ship`.
- Not run: workspace tests, Clippy, the probe correctness pass, `stress_*`
  release timing, any second host, headless, wasm.

## Limits

- One host, one GPU, one fixture, five repeats. The whole-window mean
  ranges overlap HEAD@8; only the single-step bucket separates.
- Frame-bounded window: the faster arm simulates less world.
- The census tables are truncated (top 20 archetypes, 8-name signatures,
  top 40 components); exact body and emitter counts are not recoverable
  from them.
- A frame that carves more than 64 times per material drops the rest
  unchipped. Not observed; not instrumented.

## Landed and re-measured on master, 2026-09-14

The sprout was squash-landed onto master as `83f4da4de` with the owner's
approval. That one commit carries both the chips and the eight-worker cap,
so the cap now ships as a default. The sprout worktree and branch were
removed afterwards. Nothing was pushed.

A fresh release build of master (code-identical to `026ecdf4e`, the
owner's docs commit on top) was measured with the same contract, five
gated repeats at the shipped 8 workers, all valid, no ERROR line, no
hanabi error, binary SHA `87af72702d7b`. Raw data:
`/tmp/nova-perf-20260913/master-landed/`.

| Arm | Mean | Median | P99 | Mean range | 1-step | 1-step range | Adm |
|---|---:|---:|---:|---:|---:|---:|---:|
| master `83f4da4de` @8 | 9.7706 | 9.4776 | 15.3296 | 9.2684-12.5412 | 10.3525 | 9.9609-12.3243 | 4/5 |
| chips on sprout @8 | 11.2805 | 11.4542 | 24.0480 | 10.1430-13.2431 | 11.4512 | 10.6779-12.7472 | 5/5 |
| HEAD before `24cedcf99` @8 | 13.0632 | 13.1757 | 25.0167 | 12.7790-13.5079 | 13.5510 | 13.3227-13.8886 | 5/5 |
| HEAD before `24cedcf99` @16 (old default) | 19.6870 | 17.2300 | 48.1691 | 17.7149-21.9845 | 16.4256 | 15.9089-18.0793 | 5/5 |
| v0.13.2 @8 | 8.0997 | 7.9510 | 14.3500 | 7.7738-8.9313 | 9.2001 | 8.7491-9.8660 | 5/5 |
| v0.13.2 @16 (its shipped default) | 13.2363 | 12.4382 | 32.0523 | 12.6525-15.9677 | 13.0842 | 12.5460-14.4290 | 4/5 |

- master@8 against HEAD-before@8: mean -25.2%, 1-step -23.6%, p99 -38.7%;
  mean, 1-step and p99 ranges all disjoint.
- master@8 against v0.13.2@8: mean +20.6%, 1-step +12.5%, ranges disjoint.
  Against v0.13.2 at its shipped 16-worker default, master is -26.2% mean.
- master@8 against the same code measured on the sprout: mean -13.4% with
  overlapping ranges. Same binary contents apart from the build path; the
  sprout set ran minutes after two release builds at a 1-min loadavg of
  3.1-3.4 for four of its five repeats, the master set at 0.9-2.9. Treat
  the two as one spread of 9.27-13.24 for this code, not as two results.
- The first master repeat (12.54 mean, p99 33.0) was the first run of a
  fresh binary and fell outside the 20% admission band; the other four sit
  at 9.27-9.93.
- Rounds by the third status report 3416-3622: the AI shoots the same.

Profiling sysctls are still unrestored (needs a password):
`sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516`.
