# Release stress controls for the eight-worker candidate

## Claim

At `24cedcf99dd1bd11f2ed9a632b03c1378346400b`, eight compute workers improve
both tested stress scenes against a fresh default-16 reference on this host.
Stock reference mean falls 16.2% for bullets and 50.0% for point defense.
The gains remain with the fixed-step ceiling. No shipping default changed.

This follows the user's approval of release stress controls after `WORKERS.md`.
It is not a new arena/baseline comparison, a universal worker-count optimum,
or a measurement of identical fixed-tick worlds.

## Evidence and temporary changes

The fixtures use normal gameplay through `AppBuilder`:

- `examples/systems/stress_bullets.rs:61-69,167-168,343-348,430-445`:
  eight mounts, a 1,000-round floor, a two-second hold, and peak accounting.
- `examples/systems/stress_point_defense.rs:279-297,438-471,953-958`:
  capture already uses a 1,600-frame hold and an inbound-envelope gate.
- `crates/nova_probe/src/capabilities/frametime.rs:168-171,837-850,982`:
  180 warmup / 900 capture frames; frame samples use `Time<Real>`.
- `crates/nova_core/src/lib.rs:247-253`: `DefaultPlugins` assembly.

Temporary changes, shared by both arms:

1. Reuse the prior scratch compute-cap setting and actual-pool assertions.
   Default arm leaves the setting absent and reports 16 compute workers.
   Candidate sets `NOVA_EXPERIMENT_COMPUTE_CAP=8`. Every run reports four IO
   and four async-compute workers. No affinity or total-pool limit changed.
2. For armed bullet captures only, replace the two-second hold with
   `frames(1600)`. Add readiness at a cached live count of at least 1,000 and
   liveness while firing stays held and that count stays at least 1,000.
   This prevents a faster arm from measuring an empty fill/drain phase.
   Correctness-only runs retain the original two-second hold.
3. Keep the point-defense envelope gate and hold. Add capture liveness while
   its torpedo tubes remain open. No targeting, firing, damage, collision,
   lifetime, content, or wreck rules changed.
4. Extend existing peak-count systems with scalar population summaries.
   Bullets count from first reaching 1,000 through the post-hold assertion.
   Point defense counts from its full-envelope marker while tubes stay open.
   The latter records rounds, all live torpedoes, and inbound torpedoes.
   These summaries cover the saturation period, not exactly the 900 samples.
   They add no per-frame log output or new simulation system.

Both examples and `nova_core/src/lib.rs` were restored byte-for-byte before
measurement. Preserved experimental executables, not the restored source,
were measured. No public setting, new example, or runtime change remains.

The first build failed with E0502 in the temporary point-defense census:
an immutable `Peaks` borrow crossed `probe_marker(&mut World, ...)`.
Copying the census values before that call fixed the fixture compiler error.
No game or capture had run. The failed build log and patch remain in scratch.

## Controls and execution

Host: i9-12900F, 24 allowed logical CPUs, RTX 3060 Ti. For every timing run:

- Release, `--features debug`, size optimization and fat LTO; no frame-pointer
  override, Samply, custom trace, correctness recorder, or GPU timestamps.
  Normal capture/frame-cost collectors and the small matched census remain.
- Display `:0`, Vulkan, 1920x1080, default quality, immediate presentation.
- Gameplay seed `NOVA_SEED=20260816`. Neither stress fixture has a draft seed.
- Scripted autopilots; 180 warmup and 900 capture frames.
  Autopilot deadline 280 seconds; supervisor budget 310 seconds.
- Point defense: 12 mounts, 12 bays, `battery` camera, artificial frame floor
  explicitly zero. Other scales and camera views were not measured.
- Fresh, separate mod/data/config directories. Clear inherited `NOVA_*` and
  `TRACE_*` variables before setting the run environment.
- Stock leaves max delta alone. Ceiling sets
  `NOVA_PROBE_MAX_DELTA=0.015625`; it does not force one step per frame.

Five repeats per arm, fixture, and mode: 40 timing runs. Odd repeat blocks
run 16 then 8; even blocks run 8 then 16. Each block runs bullets then point
defense. Ceiling precedes stock. Builds finish before measurement and settle
for 25 seconds. Runs are serial, with three seconds between them. Analysis,
correctness, screenshots, and report rendering run after all timing sets.

Every run validates pool sizes, capture profile/backend/resolution, complete
rigs, capture completion before the post-hold assertions and drain, population
output, clean teardown, and absence of ERROR lines. All eight sets admit 5/5
under the existing 20% mean/median repeat band. No refresh-cap suspicion,
capture refusal, or failed gameplay assertion occurred. All 40 runs share
WARNs about XSETTINGS reload and the example mod's catalog-name fallback;
these are not severity-ERROR lines.

## Release results

References are medians of per-run means/medians. P99 is the median of admitted
per-run p99 values. These are capture statistics, not the script's incidental
whole-cycle FPS prints.

### Bullets

| Mode | Workers | Mean ms | Median ms | P99 ms |
|---|---:|---:|---:|---:|
| Stock | 16 | 3.0000 | 2.8455 | 4.7218 |
| Stock | 8 | 2.5130 | 2.4073 | 3.5263 |
| Ceiling | 16 | 3.1224 | 2.8163 | 5.6945 |
| Ceiling | 8 | 2.5149 | 2.4232 | 3.6203 |

Eight versus sixteen: stock mean -16.2%, median -15.4%, p99 -25.3%;
ceiling mean -19.5%, median -14.0%, p99 -36.4%.

### Point defense

| Mode | Workers | Mean ms | Median ms | P99 ms |
|---|---:|---:|---:|---:|
| Stock | 16 | 14.8148 | 13.5083 | 36.7030 |
| Stock | 8 | 7.4145 | 5.9578 | 24.9978 |
| Ceiling | 16 | 14.1324 | 12.8977 | 33.8828 |
| Ceiling | 8 | 6.3456 | 5.1763 | 18.3159 |

Eight versus sixteen: stock mean -50.0%, median -55.9%, p99 -31.9%;
ceiling mean -55.1%, median -59.9%, p99 -45.9%.

### Repeat spreads

B = bullets; PD = point defense. Both mean and p99 ranges are disjoint between
16 and 8 workers in every matched comparison. Point-defense eight-worker
runs still vary substantially; passing the repeat gate does not remove that.

| Case | Mode | Workers | Mean range ms | P99 range ms |
|---|---|---:|---:|---:|
| B | Stock | 16 | 2.8910-3.5051 | 4.0248-11.8860 |
| B | Stock | 8 | 2.4965-2.5476 | 3.4906-3.6971 |
| B | Ceiling | 16 | 2.9014-3.2006 | 4.2364-8.0342 |
| B | Ceiling | 8 | 2.4799-2.5661 | 3.5551-3.6781 |
| PD | Stock | 16 | 12.9517-15.8891 | 34.5600-37.7218 |
| PD | Stock | 8 | 6.0293-8.4521 | 17.6387-27.3584 |
| PD | Ceiling | 16 | 13.9005-14.7243 | 33.3004-34.8091 |
| PD | Ceiling | 8 | 5.2518-7.3678 | 12.1763-26.8550 |

## Workload and attribution limits

The rigs, controls, content, and seeds match. All 20 bullet runs reach a peak
of 1,616 rounds. Their saturation-period minima are 1,008. Mean populations
are similar, not identical:

| Mode | Workers | Range of mean live rounds |
|---|---:|---:|
| Stock | 16 | 1550.9-1563.4 |
| Stock | 8 | 1540.7-1543.8 |
| Ceiling | 16 | 1550.5-1560.2 |
| Ceiling | 8 | 1541.8-1546.4 |

Across the 20 point-defense runs, saturation-period means span 2033.7-2064.3
rounds, 103.4-107.7 live torpedoes, and 61.2-65.4 inbound torpedoes. All 12
mounts work the stream; every run records interceptions and then drains cleanly.
These data do not indicate that the cap wins by suppressing intended firing.

Whole-engagement counters are not matched-time quantities. For example,
stock 16-worker runs spend 29,583-31,579 rounds over 33.9-36.0 simulated
seconds; eight-worker runs spend 17,217-20,865 over 22.0-25.4 seconds. They
end the same frame-based hold at different simulation times. The rates also
include different proportions of initial acquisition/fill. The code measures
from tube opening, not capture start:
`examples/systems/stress_point_defense.rs:1073-1118`.

Total collider peaks differ: stock 535-569 at 16 versus 414-453 at eight;
ceiling 484-522 versus 397-436. These are whole-period peaks, often reached
after capture. They are not live counts restricted to the measured window.
An interim update attributed the increase to wrecks. That was too strong:
no wreck-specific census was collected. Wreck accumulation is a hypothesis,
not established population attribution.

Fixed steps per 900 captured frames:

| Case | Mode | Workers | Five counts |
|---|---|---:|---|
| B | Stock | 16 | 168, 201, 166, 173, 186 |
| B | Stock | 8 | 147, 144, 145, 145, 144 |
| B | Ceiling | 16 | 183, 180, 167, 184, 170 |
| B | Ceiling | 8 | 147, 144, 145, 143, 146 |
| PD | Stock | 16 | 853, 746, 810, 915, 873 |
| PD | Stock | 8 | 348, 349, 479, 427, 487 |
| PD | Ceiling | 16 | 691, 706, 694, 682, 676 |
| PD | Ceiling | 8 | 393, 301, 362, 342, 400 |

Every ceiling capture has at most one fixed step per frame, but zero-step
frames remain in every set. Bullet stock captures also stay at zero/one;
point-defense stock maxima are two to four. Thus feedback through fixed-step
frequency remains, and the complete FPS gain cannot be assigned to lock-spin
removal or useful-work throughput at equal ticks. No new Samply samples were
collected here. Optional profiling binaries were built but not used as proof.

## Separate correctness and rendered output

Four additional release runs use the probe's correctness-only environment,
timeline, contract, and continuous invariants. They use the original short
holds, not the measurement hold. Existing `nova-protocol probe report` reads
honest manifests for the preserved binaries; it does not rebuild them.

| Fixture | Workers | Outcome evidence | Invariant frames |
|---|---:|---|---:|
| Bullets | 16 | 8 mounts/16 sections; peak 1616; drain/teardown | 1359 |
| Bullets | 8 | Same rig and peak; drain/teardown | 1469 |
| PD | 16 | 12 mounts/12 bays; peak 79 inbound; 33 intercepts | 2709 |
| PD | 8 | Same rig; peak 69 inbound; 42 intercepts | 4267 |

Both PD runs give all 12 mounts to the computer, work all 12, spend 7,640 and
7,610 rounds respectively, and reach peaks of 2,420 and 2,179 rounds. Both
ordnance counts drain to zero. Teardown leaves zero roots, sections, rounds,
and torpedoes. All four `checks.json` verdicts are OK: every claimed check
passes and invariant violations are zero. FPS checks are N/A. Outcome markers,
final counts, summaries, logs, and reports were inspected, not just exits.

Two separate eight-worker visual runs show assembled batteries, sustained
round streams, muzzle flashes, and effects without an apparent rendering
failure in the inspected images. They were focused and photographed after
capture readiness/warmup, so all their timings are excluded. The desktop
images are 1920x1069 crops; they are not full-surface pixel-equivalence proof.
The collector validates 1920x1080 render surfaces. The previous i3 workspace
was restored. The generated release report was also rendered and inspected.
Chromium's UPower D-Bus error belongs to that later report-rendering process,
not to a measured game run.

Scripted rigs are sufficient for this scheduler experiment. No new player
interaction was introduced. No agent-bench flow, full workspace test suite,
Clippy, larger torpedo stress scene, other PD view, or cross-host test ran.

## Next decision and blast radius

Potential improvement only. The user chose to record the eight-worker
candidate and run further tests before deciding. Keep the current default;
implementation and further agent-run experiments need new approval.
Arena and both stress controls support the candidate on this host, not a
universal optimum.

Proposed change in `AppBuilder`, without the experimental setting or logging:

```rust
let mut task_pool_options = bevy::app::TaskPoolOptions::default();
task_pool_options.compute.max_threads = 8;
// Set TaskPoolPlugin in DefaultPlugins with these options.
```

Bevy 0.19's `task_pool_plugin.rs:72-89,113-146,229-258` clamps the desired
compute count to this maximum and initializes the global pool once. Smaller
default allocations stay smaller; IO/async policies remain unchanged. The
shared pool affects physics, transforms, rendering, and headless workloads.
Other CPUs and larger guided-torpedo workloads remain unverified.

Alternative: a transform-only queue/worker fix leaves more parallelism
available elsewhere, but requires a dependency-side change and hierarchy /
thread-safety proof. This stage does not implement or compare that option.

If the cap is approved, add a failing policy assertion, apply the small
configuration change, rerun affected correctness, and confirm the final
non-experimental arena binary with matched release repeats. No timing
assertion, public environment option, or workspace-wide check is proposed.
The task stays OPEN until a shipping change and its evidence are accepted.

## Reproduction and artifacts

All raw data are disposable, outside Git:
`/tmp/nova-perf-20260913/stress-workers-24cedcf99/`.

- `experiment.patch`, original/experimental snapshots, build logs and hashes.
- `build-attempt-1/`: failed census compile, before the borrow fix.
- `stress_{bullets,point_defense}/{stock,ceiling}-{16,8}-{1..5}`:
  manifests, aggregate CSV/JSON, normal census, contracts, and logs.
- `summary.json`, `summary.log`, rendered `report.html/png`.
- `correctness/{16,8}/{stress_bullets,stress_point_defense}`:
  timelines, manifests, checks, logs, and HTML reports.
- `visual/{stress_bullets,stress_point_defense}/saturation.png`, each marked
  visual-only; build/run/analysis/correctness/visual helpers and exit records.

Build the temporary sources serially, preserve the executables, then restore
sources before measuring:

```sh
nix develop --command cargo build --release --features debug --jobs 1 \
  --example stress_bullets --example stress_point_defense
```

Release executable SHA-256 values, in the same order:

```text
9a085dfece8c09bd84dfde5a6e763a0ccbe6471b0e0745ef75231a3d58693065
ff82ba7dd50c83da3b407709ecd3ef30f4b3a57f250aa2afde2dfc9771040faa
```

Both arms use the same binary for each fixture. Only Markdown findings stay
in the task folder. No runtime diff or new commit remains. Build, measurement,
verification, visual, and report processes finished. Profiling sysctl cleanup
still requires the user's password; `sudo -n` failed and values remain 1 and
16384. Restore the original settings with:

```sh
sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516
```
