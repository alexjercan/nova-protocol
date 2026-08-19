# The 118 ms build UI is a measurement of the BOX, not of the editor

## Verdict, up front

1. It is **NOT** the `Time<Virtual>::max_delta` catch-up. The build UI runs
   **one** fixed step per frame.
2. There is **no trimesh anywhere in the editor**, and the gallery tiles and
   the placement ghost carry **no collider at all**.
3. The 118 ms mean and the 2378 ms frame **do not reproduce**. Same box, same
   GPU, same 1280x720 / vsync-off conditions, same walk, and `crates/nova_editor`
   byte-identical to the tree that produced the row: **17.4 ms mean, 16.6 ms
   median, 66.7 ms worst frame** across the build view.
4. The 65 fps observation from `20260819-001252` and this measurement agree.
   The 118 ms row is the outlier, and the evidence says it was taken on a
   contended host.

The task as written - "the editor build UI runs at 118 ms and hitches for 2.4
seconds" - is not a defect that exists in this tree.

## How it was measured

`examples/systems/ship_editor.rs` now carries a per-frame diagnostic, armed by
`NOVA_EDITOR_FRAMELOG=1` and inert otherwise (`framelog`/`report_frame`, added
this task). It writes one line per rendered frame:

```text
framelog f=<n> ms=<wall> steps=<fixed steps> entities=<n> step_ms=<avian>
```

`Time<Real>`, not `Time<Virtual>`: the virtual clock is clamped by the very
`max_delta` under test, so a 2 s frame reports 250 ms there.

The capture reports one mean and one max over 900 frames and cannot say which
gesture paid for them. A framelog line plus the `autopilot: step ... begins`
line above it can.

Runs, all `--features dev`, Xvfb 1280x720, `NOVA_PERF=1` armed so the window is
forced to 1280x720 with vsync off and `WinitSettings::game()`
(`crates/nova_probe/src/capabilities/frametime.rs:475-489`) - the exact
conditions of the row being checked:

| run | host | mean | p50 | p95 | worst |
| --- | --- | --: | --: | --: | --: |
| run1 | quiet (load 1.5) | **17.4 ms** | 16.6 | 22.2 | 66.7 |
| run2 | another agent building (load 16) | 42.8 ms | 37.7 | 91.8 | 206.0 |
| run3 | run2's load + 20 spinners (load 30) | 36.6 ms | 35.4 | 56.6 | 109.0 |
| original `probe run ship_editor` | unknown | 117.9 ms | 96.7 | 108.7 | 2377.6 |

Rows 1-3 are the BUILD VIEW only - every frame between `editor: click New Ship`
and `editor: press Play`, so the menu and the asset-load screen are excluded and
the comparison is against the surface the task is about.

## 1. Not the catch-up

Fixed steps per rendered frame across the build UI, run1 (quiet):

| steps/frame | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 16 |
| --- | --: | --: | --: | --: | --: | --: | --: | --: |
| frames | 22 | **944** | 130 | 3 | 3 | 3 | 1 | 2 |

The two 16-step frames are both in `editor: reach the main menu`, at 256.6 ms
and 259.6 ms. They are 16 because the frame was already a quarter of a second
long (the load screen), not the other way round: `steps ~= ms / 15.6` holds
across the whole distribution, which is the accumulator doing exactly what it
is specified to do.

And the steps are free. `avian/total_step_time` in the build UI reads
**0.17 ms** (run1 mean), against the 22.0 ms that drove the sandbox collapse in
`20260819-001252`. Sixteen steps of 0.17 ms is 2.7 ms - one percent of the
260 ms frame that produced them. The step count is a follower here, never a
driver.

## 2. No trimesh, and the previews carry no collider at all

Static, and it is unambiguous:

- `crates/nova_editor/src/preview.rs:68-72` - every `PreviewRole::Display`
  entity ends with `entity.remove::<(SectionMarker, Collider)>()`. That is the
  gallery tiles (`gallery/scene.rs:156`) and the placement ghost
  (`placement.rs:652`). They have no collider to manifold.
- A build section's collider comes from `SectionCollider::to_collider()`
  (`crates/nova_ship/src/sections/base_section.rs:88-95`), which returns only
  `Collider::cuboid`, `sphere`, `capsule` or `cylinder`. There is no trimesh
  branch.
- Every `Collider::trimesh_from_mesh` / `convex_hull_from_mesh` call in the tree
  is in the asteroid and carve paths (`crates/nova_scenario/src/objects/`,
  `crates/nova_gameplay/src/integrity/chunk.rs`), none of which run in the
  editor - the Editor state never has a scenario loaded
  (`crates/nova_editor/src/lib.rs:265-269`).

Empirically the same: 0.17 ms a step is a narrow phase with nothing in it.

## 3. The 118 ms row is a floor, not a tail

The original row's own distribution says so:

```text
frames=900 mean=117.85 min=83.05 p50=96.72 p95=108.74 p99=762.79 max=2377.56
```

**The fastest frame in 900 was 83 ms.** A mean dragged up by a few stalls has a
normal minimum; this one does not. Whatever cost the time was charging every
frame.

The per-beat wall clock proves it was not the editor charging it. Both runs walk
the identical script, so a beat's frame count is fixed by its `frames(N)`
predicate; the wall time it takes is therefore a direct read of ms per frame.
Comparing the original run against run1, beat by beat:

| beat | original | run1 | ratio |
| --- | --: | --: | --: |
| `the count dropped back` (reads a usize, 2 frames) | 0.18 s | 0.04 s | 4.4 |
| `select mode placed nothing` (reads a usize, 2 frames) | 0.18 s | 0.04 s | 4.6 |
| `click the ship in delete mode: aim` | 1.01 s | 0.18 s | 5.7 |
| `release Select / Rebind` | 1.03 s | 0.18 s | 5.8 |
| `place the first section: press` | 1.14 s | 0.19 s | 6.1 |

Across all 112 beats the two runs share, the ratio has a **median of 5.8** and
**97 of 112 fall between 4x and 8x**. Every beat is slower by roughly the same
factor, including beats that do nothing but read a count off a static screen.
There is no editor code whose cost varies between `the count dropped back` and
`place the first section: press`, so a uniform 5-8x across both is a property of
the machine, not of the program.

The other 10 beats are all above 10x, up to 17.5x, and they are ALL gallery
opens or gallery rebuilds - see section 5.

The same night, on the same box, the harness task recorded the same signature
on a completely different scene: `scene_baseline` (`asteroid_field`) read
mean 90.89 / **min 80.75** / p50 89.91 at 00:24, and mean 22.97 / min 18.50
twelve minutes later at 00:32. The editor row was captured at 00:34-00:36 with
**min 83.05 / p50 96.72**. Two unrelated scenes sharing a ~85 ms floor is a
floor the box imposed on both.

Live confirmation, this session: run1 and run2 are the same binary on the same
box three minutes apart. Run1 at load 1.5 read 17.4 ms; run2, with another
agent's `rustc` on the box at load 16, read 42.8 ms. Twenty deliberate CPU
spinners on top (run3, load 30) made it no worse - 36.6 ms - so CPU contention
alone saturates at about 2.5x. The original 5.8x wanted the memory and page-cache
pressure of a linking build as well, which is what was on the box that night.

**The tree is not the variable.** `git log 64ddc76b..HEAD -- crates/nova_editor`
is empty: the editor is byte-identical between the tree that measured 118 ms
and the tree measured here.

## 4. The row is also mislabelled in the report

`tasks/20260818-221027/REPORT.md:182` glosses the row as "The editor at rest
with a 2-section ship on screen". It is not. Read back from the run's own log,
the 900-frame window opened at 21:34:41 and closed at 21:36:27 and contains
**96 autopilot beats**, from `editor: place the first section: press` through
`editor: raise a tower, second course: release` - most of the walk, including
every gallery open, every filter, and the tower. Nothing about it is "at rest".

## 5. What the editor DOES cost, measured

From run1, the quiet run, build UI only:

- Steady state, any beat: **16-19 ms mean** (~60 fps), 1 fixed step a frame.
- The worst frame in the build view is **66.7 ms**, on the FIRST gallery open
  (`editor: arm the hull: release`, the frame the overlay and its 12 preview
  tiles spawn). The next two frames of that beat cost 44.1 and 35.8 ms.
- Every later gallery open costs 10-25 ms over baseline, not 50.
- Everything above 70 ms in the whole run is the menu / asset-load screen.

The one-off first-open cost has an obvious candidate and it is NOT profiled
here: `crates/nova_core/src/lib.rs:331` sets
`synchronous_pipeline_compilation: true` for the whole game (deliberately, task
20260805-111329 - the async path SIGSEGVs at teardown), so the first draw of a
new mesh/material combination compiles its pipeline on the main thread. Twelve
tiles' worth of new parts in one frame is the shape of a 50 ms hitch. This is a
lead, not a finding: nothing here measured it.

Also worth recording, from run2 and run3, which both reached Play:

| run | build UI mean | `editor_sandbox` mean |
| --- | --: | --: |
| run2 | 42.8 ms | 40.2 ms |
| run3 | 36.6 ms | 48.7 ms |

The build UI and the flown sandbox cost about the same and move together with
host load. The build UI is not an outlier surface.

## What I did not measure

- **The 2378 ms frame was never reproduced.** Three runs, ~3800 build-UI frames,
  worst 66.7 ms in the build view (quiet) and 815 ms in the load screen (loaded).
  I cannot say what it was. Its position is known: the original run's beat
  timings put one ~2.2 s frame on each of about nine gallery beats, which
  matches the row's `p99=762 ms` over 900 frames, but the cause is unattributed.
- **I did not reproduce the original host conditions.** My load was CPU-only
  (spinners) and saturated at about 2.5x. A `rustc`/`lld` build's memory and
  page-cache
  pressure is a different load and I did not stage one.
- **No `trace_chrome` pass.** Once the headline number failed to reproduce by
  6x, a system-level attribution of frames that are already 16 ms would have
  been measuring the wrong thing.
- **Dev profile, one box, one GPU (RTX 3060 Ti), native only.** No release
  build, no software renderer, no WASM.
- **A small ship.** The walk builds about 8 sections; nothing here says what a
  200-section build costs, and `sync_editor_skin`
  (`crates/nova_editor/src/skin.rs:62`) respawns every plate whenever the
  structure signature changes, which is a cost that scales with the build.
- **The 65 fps figure was not re-taken directly**, only matched: run1's 16.6 ms
  median is 60 fps on the same surface.
- **The `ship_editor` walk is still flaky.** Run1 died at
  `editor: raise a tower, first course: it built`, the beat
  `20260818-221027` already documents as pre-existingly unstable. Runs 2 and 3
  passed it. That flakiness is untouched by this task.

## For the owner's `max_delta` decision

This task WEAKENS the case for clamping `Time::<Virtual>::max_delta`, in the
sense that it removes a supposed second instance: the editor is not a second
scene hitting the ceiling, and the 2378 ms frame is not 16 x 148 ms. The
catch-up has exactly one demonstrated victim, the pre-`e994cbb1` sandbox, and
that is fixed at the source.

It does not weaken the general argument in `20260819-001252` - a scene that goes
over budget still degrades into a slideshow rather than into slow motion. It
just means nothing new is pushing on it.

## The thing that should actually change

Not the editor. The harness cannot currently tell a slow program from a busy
box, and it has now produced one wrong headline number because of that. Both
inputs already exist in the run: `mean_ms` and `min_ms` are in
`frametime.csv`, and a run whose MINIMUM frame is 4x its own historical minimum
is a contended run, not a regression. The report already warned about this in
prose (`20260818-221027`, caveat 1, which measured `scene_baseline`'s minimum
moving 18.5 -> 80.8 ms) and the very next row in the same document was read as a
finding anyway.
