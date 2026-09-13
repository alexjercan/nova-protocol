# Performance boundary identified

## Claim

`6dadd95f21d392b41269df78b40ff668b03ab84d`, **Aim at how big the thing is**,
introduces a large, measured `wfc_arena` slowdown. It predates the combat
flight-computer series. Its immediate parent is
`f894d4c20d8af4a33ad8233fb07232bd94dd558b`.

The direct parent/commit comparison increases the reference mean by 31.4%
and the reference median by 33.6%. All five post-change repeats average
below 60 FPS. This identifies a major regression boundary, not the cost of
every post-release change. The smaller earlier gap is not isolated.

The shared sensor commit alone does not reproduce this large gap. Its set
also drifts downward, so it does not prove a small improvement or prove
that sensors cost nothing.

## Evidence: stock release captures

Display `:0`, NVIDIA GeForce RTX 3060 Ti, Vulkan, 1920x1080, default quality,
explicit `immediate` presentation. Scripted seed `20260816`; the same eight
seeded ships and section/weapon draft counts in all 35 diagnostic captures.
The fixture warms for 60 frames and measures 360 after both teams fire and
connect. Both teams must remain present throughout the window.

Builds use the unchanged release profile: `opt-level = "s"`, fat LTO,
`codegen-units = 1`, `--features debug`, pinned Nix nightly. Historical sources
are `git archive` snapshots. Each has its own `target/`. No concurrent builds
or GPU measurements. There is a 20-second settling interval after each build.

| Revision | Mean ms | Median ms | Admitted | Admitted mean range ms |
| --- | ---: | ---: | ---: | ---: |
| v0.13.2 `b3c6f579c` | 13.505 | 12.200 | 4/5 | 12.949-15.731 |
| Sensor pass `68c3ebe6f` | 13.245 | 12.464 | 5/5 | 11.991-15.072 |
| Cone parent `f894d4c20` | 16.097 | 13.974 | 4/5 | 14.459-18.888 |
| Cone change `6dadd95f2` | 21.153 | 18.671 | 5/5 | 19.383-23.081 |
| HEAD `7b07528d2` | 19.373 | 17.362 | 4/5 | 18.713-19.526 |

Mean and median are the medians of the five per-repeat statistics. Admission
uses the existing 20% mean/median band. Raw repeats remain in scratch, not
this task folder. Do not assume a discarded repeat is machine noise: the arena
can follow a different combat path. HEAD repeat 5 reaches the 16-step clamp
and has a 324.8 ms p99; the source of that outlier is not isolated.

| Revision | Admitted p99 ms | Admitted p99 range ms | Fixed steps, all repeats |
| --- | ---: | ---: | --- |
| v0.13.2 | 37.716 | 30.602-46.582 | 311, 304, 362, 299, 424 |
| Sensor pass | 28.408 | 20.736-39.212 | 347, 325, 305, 300, 276 |
| Cone parent | 44.529 | 33.285-56.396 | 334, 371, 435, 343, 483 |
| Cone change | 51.318 | 39.050-53.754 | 532, 522, 450, 487, 447 |
| HEAD | 43.923 | 41.827-49.421 | 434, 431, 446, 450, 680 |

The p99 ranges overlap. No p99 regression or improvement is established.
No set reports refresh-cap suspicion. All captures completed without ERROR
lines or scene-ended refusals. These were capture-only passes, not new
correctness runs; prior `checks.json` files were read but are not fresh proof.

## Evidence: the changed mechanism

At HEAD, `crates/nova_ship/src/sections/turret_section/aim.rs:69`:

```rust
pub fn on_target_cone(hit_radius: Option<f32>, distance: f32) -> f32 {
    let Some(hit_radius) = hit_radius else {
        return POINT_AIM_ON_TARGET_RAD;
    };
    (hit_radius.max(0.0) / distance.max(f32::EPSILON))
        .atan()
        .max(MUZZLE_SPREAD_RAD)
}
```

The parent used a fixed `1.6 / 100.0` radian cone, about 0.92 degrees.
The new cone grows with target radius and proximity. `muzzle_on_target` at
line 101 is read by the AI trigger and the fixed-clock muzzle fire path.
`crates/nova_ship/src/sections/hull_radius.rs:83` publishes the target radius.
This commit changes firing eligibility before the later AI flight changes.

The effect survives a comparison at the same nominal simulation-clock report,
not just at the ends of unequal wall-clock captures:

| Arm | Rounds at third status report, by repeat | Reference |
| --- | --- | ---: |
| Parent | 1302, 1260, 1040, 1369, 1264 | 1264 |
| Cone | 4284, 3950, 4011, 4243, 4133 | 4133 |

The new arm has 3.27 times the reference fired-round count. Assertions over
all 35 logs confirm the same eight-ship draft, no shots in the first two
status reports, and four ship roots per team in the third report. Root counts
do not prove that every ship still has an active controller.

The status period is five simulation seconds, not five capture frames:
`examples/playable/wfc_arena.rs:1696-1711` uses `Res<Time>` and
`time.elapsed_secs()`. Thus the third status report is nominally 15 seconds.
It can overshoot by a frame on each interval. This is a bounded clock-based
workload observation, not an exact fixed-tick population fixture. Shot counts
come from `count_shots` at line 1603. Full status lines remain in the raw logs
under `/tmp/nova-perf-20260913/runs/`.

## Evidence: remove catch-up bursts

A second five-repeat parent/commit pair uses the same binaries and settings,
with `NOVA_PROBE_MAX_DELTA=0.015625`. Every recorded frame runs at most one
fixed step. This is a diagnostic ceiling, not a shipping change.

| Arm | Mean ms | Median ms | Mean range ms | Fixed steps |
| --- | ---: | ---: | ---: | --- |
| Parent | 15.174 | 13.446 | 14.008-16.540 | 299, 282, 284, 304, 314 |
| Cone | 17.926 | 17.062 | 17.467-19.701 | 339, 341, 350, 341, 339 |

All ten repeats are admitted, with no refresh-cap suspicion. The gap remains
18.1% by mean and 26.9% by median without catch-up bursts. It is therefore not
caused only by the fixed-loop amplifier. This does NOT hold one fixed step
per frame: some frames run zero steps, and populations still evolve.

At the third simulation-clock report, the capped parent fired 1685-1905 rounds
and the capped cone arm fired 4346-4633. The heavier firing also survives this
control. Do not subtract these diagnostic frame times from the stock captures
to assign an exact percentage to feedback.

## Change and blast radius

Identification changed no runtime code, content, interfaces, defaults, or
scheduling. It ran on `master` at `7b07528d2`; no commit was made.
The later retained optimization is documented in `DIRECT-COLLIDER.md`.

The frame-cost tables put much of the added work in `RunFixedMainLoop`, not
just `Update`. This supports profiling projectile processing and combat load
next. It does not isolate `advance_rounds`, physics, rendering, or destruction
as the sole expensive path. The old dev traces mostly precede the long
combat capture and cannot settle that question.

## Verification and limits

- 35 capture-only runs: five stock sets and two one-step-ceiling sets.
- Raw frame data, manifests, reports, and logs remain in scratch. This task
  folder retains Markdown findings only, as requested.
- `/tmp/nova-perf-20260913/visual/combat.png` was read. It shows ships, debris,
  and projectile streams. This separate run focused and photographed the
  window; its FPS readings are excluded. It is not a visual before/after.
- Historical `wfc_arena` source, `Cargo.lock`, and the frame collector match
  HEAD. Draft assertions confirm the same eight ships in every measured run.
- No full workspace tests or Clippy. This identification phase made no fix
  and claims no post-fix proof or new correctness verdict.
- Seed clarification from the later experiment: `--seed 20260816` pins the
  draft only. These 35 captures did not set `NOVA_SEED`, so their gameplay RNG
  was not pinned. The capture helper now also sets `NOVA_SEED`; the historical
  manifests in scratch record the original, draft-only seed environment.
  See `DIRECT-COLLIDER.md` for the fresh before/after comparison.
- Exact live projectile, section, and wreck populations are not yet pinned.
  A non-ending, fixed-tick workload fixture and a projectile stress control
  remain required before claiming the complete cost mechanism or a fix.
- A smaller pre-cone regression remains possible. These sets identify the
  large direct cone boundary, not the earliest measurable micro-regression.

## Next

Preserve the intended size-aware firing behavior. Build the matched-workload
control, profile the combat window, and reduce processing cost rather than
restore FPS by making the AI hold fire. Start with `advance_rounds` and its
candidate/exact-hit work, alongside `stress_bullets`, then distinguish render,
physics, and destruction cost. Keep the sensor cost as a separate candidate.

## Scratch data and reproduction

The task folder keeps Markdown only. Original data, source snapshots, and
build logs remain under `/tmp/nova-perf-20260913/`, with repeat sets in `runs/`.
The earlier task evidence remains under `/tmp/nova-release-perf/`. These paths
are not durable; the findings above are the retained record.

The capture helper is now `/tmp/nova-perf-20260913/tools/capture.py`.
It uses the shipped collector without correctness passes. Its current version
pins both draft and gameplay seeds; historical identification pinned only the
draft. A new run with this helper is not the same RNG control as the old sets.

From the main repository, build an archived revision, then run the collector:

```sh
rev=6dadd95f21d392b41269df78b40ff668b03ab84d
src=/tmp/nova-perf-20260913/sources/$rev
# For a fresh source directory: mkdir -p "$src"; git archive "$rev" | tar -x -C "$src"
nix develop --command bash -c \
  'cd "$1"; cargo build --release --features debug --example wfc_arena' _ "$src"
# Let the machine settle. Choose a NEW output directory for every set.
nix develop --command python3 /tmp/nova-perf-20260913/tools/capture.py \
  "$src" "$rev" /tmp/nova-cone-new-set --repeat 5
# Diagnostic control: append --max-delta 0.015625, with another output path.
```
