# The first gallery open costs a 67 ms frame

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.11.0, performance, editor

Epic: `20260818-220812`. Evidence: `NOTES.md` beside this file.

## This task used to claim something false

It was filed at p92 as "the editor build UI runs at 118 ms mean and hitches for
2.4 seconds", off a `frametime.csv` row reading `mean=117.851ms
max=2377.557ms`.

**That row measured the BOX, not the editor.** Same machine, same GPU, same
forced 1280x720 window, same walk, editor byte-identical: **17.4 ms mean, 16.6
ms p50** on a quiet host. Three independent proofs it was contention:

- The row's own MINIMUM frame is 83.05 ms (p50 96.7). A mean dragged up by
  stalls has a normal minimum; this cost was charged to every frame.
- Across the 112 beats the two runs share, the slowdown ratio has a median of
  **5.8x**, with 97 of 112 between 4x and 8x - including a beat that only reads
  a `usize` off a static screen (0.18 s against 0.04 s). No editor code differs
  between that beat and the ones doing real work.
- The same night on the same box, `scene_baseline` read min 80.75 ms at 00:24
  and min 18.50 ms twelve minutes later. The editor row was captured 00:34-00:36
  at min 83.05. Two unrelated scenes sharing an 85 ms floor is the host.

Reproduced live: same binary three minutes apart, 17.4 ms at load 1.5 against
42.8 ms at load 16 with another agent's `rustc` running.

**The contention was almost certainly self-inflicted** - parallel agents
building on the same box while a frame-time capture ran.

Two other claims died with it:

- It was NOT the `Time<Virtual>::max_delta` catch-up. 944 of 1108 frames run
  exactly ONE fixed step, and `avian/total_step_time` is 0.17 ms in the build
  UI against the 22.0 ms that drove the sandbox collapse. Sixteen of those is
  2.7 ms - 1% of the frame. The step count follows frame length, it does not
  drive it.
- The editor carries NO trimesh colliders. `preview.rs:71` removes `Collider`
  from every `PreviewRole::Display` entity - gallery tiles and the placement
  ghost both - and build sections only ever get cuboid/sphere/capsule/cylinder
  from `SectionCollider::to_collider()`. Every `trimesh_from_mesh` in the tree
  is asteroid code that never runs in the Editor state.

`65 fps` on F1 and `118 ms` were never in conflict. Quiet p50 is 16.6 ms, which
is 60 fps on that exact surface, and in runs that reached Play the build UI and
the flown sandbox cost the SAME and moved together with host load.

## What is actually left

On a quiet box the worst build-view frame is **66.7 ms, on the FIRST gallery
open** - the frame the overlay and its 12 preview tiles spawn. Later opens cost
10-25 ms over baseline.

Unprofiled candidate: `crates/nova_core/src/lib.rs:331` sets
`synchronous_pipeline_compilation: true` game-wide, deliberately, under task
`20260805-111329`. If that is it, the first draw of a new mesh/material combo
compiles on the main thread. Those same gallery beats are also the ten whose
contended-vs-quiet ratio exceeded 10x, so whatever the gallery does once is what
contention amplified into the 2.4 s frame.

67 ms once, on a deliberate user action, is a small thing. Priority reflects
that.

## Also unmeasured

- The 2378 ms frame was never reproduced. ~3800 build-UI frames over three runs;
  worst quiet is 66.7 ms, worst loaded is 206 ms.
- `sync_editor_skin` (`skin.rs:62`) respawns every plate on a structure-signature
  change. That scales with ship size and was measured only on an 8-section ship.
- Dev profile, one box, one GPU, native only. No release, no software renderer,
  no wasm.

## Tooling note landed with this

`examples/systems/ship_editor.rs` gained an env-gated per-frame diagnostic
(`NOVA_EDITOR_FRAMELOG`): wall time, fixed-step count, live entity count and
avian step time per frame, using `Time<Real>` because `Time<Virtual>` is clamped
by `max_delta` and would report 250 ms for a one-second frame. It is what turned
"the editor is slow" into "the box was busy", and it is off by default.
