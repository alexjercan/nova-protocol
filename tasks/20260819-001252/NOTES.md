# The sandbox 2 FPS, named

## It is not a leak. It is sixteen physics steps in one frame.

Total live entity count across the collapse is FLAT: 1934 at Play, 1924 after
135 s of sitting still. Nothing grows - not entities, not archetypes, not
colliders. The first thing to check was the only thing that could not have been
it.

What moves is `fixed steps per rendered frame`:

| sitting still | 5 s | 10 s | 15 s | 30 s | 45 s |
| --- | --- | --- | --- | --- | --- |
| steps/frame | 1.59 | 15.67 | **16.00** | **16.00** | **16.00** |
| frame | 119 ms | 364 ms | 372 ms | 370 ms | 377 ms |
| fps | 8.4 | 2.7 | 2.7 | 2.7 | 2.7 |

Sixteen is not a coincidence, it is the ceiling. `Time<Virtual>`'s `max_delta`
is bevy's default 0.25 s and the fixed timestep is 1/64 s, so one rendered
frame may pay for `0.25 / 0.015625` = 16 fixed steps and no more. The sandbox
sits pinned against that ceiling forever.

One step costs 22 ms (avian's own `avian/total_step_time`). 16 x 22 = 352 ms,
plus ~25 ms of everything else, which is the whole 370-380 ms frame.

**This is why "Physics Total Step 26.3 ms of a 575 ms frame" read as an alibi
and was the confession.** The F11 panel reports ONE step. The frame was paying
for sixteen of them. Every physics COUNT in that panel is identical before and
after the collapse because nothing about the scene changes - only how many
times a frame runs it.

It also explains the rest of the case with no extra theory:

- Main thread 96% R, compute pool idle: the fixed loop is main-schedule work,
  and avian chunks its narrow phase at 64 pairs, so 52 pairs is one chunk on
  one thread.
- 30-45 s to develop: the accumulator has to fill before it saturates.
- F1 restores 65 fps instantly: leaving the scenario despawns the field, the
  step goes cheap, and the accumulator drains in one frame.
- `asteroid_field` is fine: its step costs 0.19 ms, so it runs 1.48 steps a
  frame and never approaches the ceiling.

## What makes one step cost 22 ms

`avian/collision/update_contacts` is 21.9 of the 22.0 ms. The narrow phase is
the step.

At the same moment, `avian/solver/contact_constraint_count` is **0**. Fifty-two
contact pairs, nothing touching, 21.9 ms a step, sixty-four times a second,
forever.

The pairs are belt rocks whose AABBs overlap and whose surfaces never meet, and
they never go away, because avian sleeps only TOUCHING contact pairs
(`ContactGraph::sleep_entity_with`: `if !edge.is_touching() { continue; }`).
All 64 rocks carry `Sleeping` and it buys them nothing. Every one of those
pairs was a trimesh-against-trimesh manifold query, which is the most expensive
thing parry can be asked for.

Side by side, same box, same walk, sitting still:

| | `editor_sandbox` | `asteroid_field` |
| --- | --- | --- |
| colliders | 111 | 33 |
| contact pairs | 52 | 16 |
| `update_contacts` | 21.9 ms | 0.19 ms |
| steps/frame | 16.00 | 1.48 |
| fps | 2.7 | 44 |

Per PAIR that is 421 us against 12 us - 35x - so it is not the pair count, it
is what the pairs are made of.

## The fix

A pristine rock collides as its convex HULL. Carving is what buys the exact
trimesh back, on the rocks that have actually been shot - the same laziness
`seed_asteroid_fields` already applies to the carve grid, and `carve_surface`
already rebuilds the collider as a trimesh at exactly the moment the shape
stops being convex.

Measured on the same walk, sitting still for 135 s: `update_contacts`
0.08-0.29 ms, step 0.3-1.2 ms, 1.4-1.9 steps a frame, 30-51 fps, flat from
5 s to 135 s with no slide.

## What is NOT fixed, and should be decided by the owner

The 16x catch-up is still there. It is what turned a step 1.4x over budget into
a 2 FPS floor, and it will do it again to the next scene that goes over. The
scene-independent fix is to clamp `Time::<Virtual>::max_delta` in
`NovaGameplayPlugin` to a small multiple of the fixed timestep, so an
over-budget scene degrades into slow motion rather than a slideshow.

It is deliberately NOT in this change because the blast radius is the whole
harness: every autopilot beat that waits on `elapsed()` waits on VIRTUAL
seconds, and CI's software renderer already runs frames long enough to hit the
cap. Dropping the cap from 0.25 s to ~0.03 s makes sim time advance up to 8x
slower per wall-clock second on CI, which is a deadline change across every
scripted range. That is a call about the probe budget, not about this bug.

---

# Incidental findings, filed not fixed

Three things found while chasing the frame time. None of them is the 2 FPS.

## 1. The inspector inserts on a despawned camera during the menu -> sandbox swap

`nova_debug::inspector::keep_inspector_on_window_camera`
(`crates/nova_debug/src/inspector.rs`) ends with an unguarded

```rust
commands.entity(entity).insert(PrimaryEguiContext);
```

on `first_window_camera`. The entity comes from a query snapshot, so a camera
that is despawned later in the same frame is still a valid target when the
system runs and a dead one when the command flushes. The menu -> sandbox
hand-off swaps cameras in exactly that window, and the run dies with
"Encountered a panic when applying buffers".

Repro: `nix develop --command cargo run --features debug`, main menu ->
Sandbox. Intermittent - it killed 2 runs in 5 for the previous session. A
`try_insert`-style guard (or re-reading the entity inside the command) is the
shape of the fix.

## 2. `asteroid_field` spawns the player inside a scatter rock

Repro: `nix develop --command cargo run --features debug --scenario
asteroid_field`. The ship comes up wedged in a scatter rock at the origin -
camera buried in rock, 0.0 m/s.

The scenario scatters its rocks with `ScatterObjects`, which separates rocks
from each other and from earlier belts but knows nothing about hand-placed
bodies - the same hole `editor_sandbox` closes by hand with
`hand_placed_bodies_stay_out_of_the_belts`. `asteroid_field` has no equivalent
guard and scatters over the spawn.

Harmless to frame rate. A real scenario defect.

## 3. `trace_chrome` never emits for the BIN target

`--features debug,trace` with `TRACE_CHROME=<path>` produces NO file when the
game binary is the target. Verified by the previous session with a forced
rebuild and by running the binary directly, and again here indirectly: the same
feature set on an EXAMPLE target
(`cargo run --example sandbox_soak --features debug,trace`) writes a 2.4 GB
trace immediately, and that trace is what named this bug. Every profiled pass
the probe has ever run was an example, so this has never been noticed.

Two things the harness work will care about:

- The bin path is unprofiled, so "profile the game the player runs" is not
  currently possible through this route.
- The trace is enormous: 90 MB/s of wall clock at 2 FPS. Any harness pass that
  keeps one needs a duration cap and a disk budget.

Also worth recording: the per-system span filter is load-bearing. The game's
own log filter sets `bevy_ecs=warn`, which silences the
`info_span!("system", ...)` spans entirely, so `RUST_LOG` must carry
`bevy_ecs=info` or the trace comes out with no system spans in it.
`crates/nova_probe_cli/src/native/env.rs::trace_pass_env` already does this; a
hand-run trace has to remember it.
