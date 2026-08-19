# Heavy work runs on a worker behind a placeholder, never in a frame

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,wontdo

Epic: `20260818-220812`.

The shared answer to "this work cannot be predicted and cannot be done in a
frame". Today every heavy computation in the tree runs synchronously in the
system that discovers it needs doing.

## The pattern

Kick the work onto a worker, draw a placeholder, swap when it resolves, spread
across as many frames as it takes. Bevy's `AsyncComputeTaskPool` plus a polled
task component is the idiomatic shape; build it ONCE, as shared machinery, so
each site is a small adoption rather than its own invention.

Owner's position, verbatim in intent: better to run the game on the main thread
and compute on another, showing a placeholder until the real thing is ready.
Sacrificing the visual beats sacrificing the frame rate, because a visual that
costs gameplay is a visual nobody sees.

## Known callers, in the order they hurt

1. Asteroid field seed / remesh / collider rebuild (`asteroid_carve.rs`) -
   12.7 + 10.7 + 10.0 ms at 64^3.
2. ~~Section solidify on first hit~~ - WRONG, struck 2026-08-19. Came from a
   stale handoff; those files do not exist and sections never carve.
3. Explosions and death-cascade fragment spawning. `explode.rs` already defers
   through `FinaleQueue`, which is a hand-rolled instance of this pattern - it
   should become an adoption of the shared one, not a second implementation.
   NOTE: its tail was moved, not proven cheap; only 24-32 of ~200 queued bodies
   drained in the measured window.
4. The mesh slicer - 33.1 ms and the largest single item in the measured death
   frame. It did not scale with the cascade fix.

## Constraints

- A body must stay correct while its real geometry is pending: collisions,
  damage and the graph cannot observe a half-swapped state.
- Determinism matters. Scenario layout is seeded content and the probe asserts
  reproducibility, so a worker must not make the SIMULATION order vary. Visual
  swap timing may vary; simulation state may not.
- wasm has no threads in the same shape. Decide the single-threaded fallback
  deliberately - probably "do it in slices across frames" - and say so in the
  code, not in a task note.

## Done when

- One shared abstraction, adopted by at least the asteroid and section paths.
- The placeholder is visible and deliberate, not a popped-in mesh.
- Measured: no frame in the `PERF-HARNESS` cases owns a computation.

## CLOSED 2026-08-19 - every caller it was written for is gone

Owner: "if we ever find something that can be async we do it for that, we do not
keep a task open for it."

The four callers listed above, checked one at a time:

1. Asteroid seed / remesh / collider - already runs on `AsyncComputeTaskPool`
   (`crates/nova_scenario/src/objects/asteroid_carve.rs`). The 12.7 + 10.7 +
   10.0 ms figures were measured at 64^3; `FIELD_RESOLUTION_MAX` is 40 now, so
   they do not describe anything current either.
2. Already struck 2026-08-19 - came from a stale handoff, the files never
   existed and sections do not carve.
3. `FinaleQueue` - zero references left in the tree. It went with the slicer.
4. The mesh slicer - `crates/nova_gameplay/src/mesh/slice.rs` is deleted.

So the task asks for shared machinery with no site left to adopt it, which is
the speculative machinery `AGENTS.md` warns against. One real site reached the
pool without the abstraction existing, which is the evidence that the
abstraction was not the blocker.

## What was real here and is NOT preserved by closing

The **placeholder** half was never built and is still not built. `asteroid_carve`
offloads its work but draws nothing while the result is pending. If a future
site needs the visual, it needs designing then - the owner's position stands and
does not need a task to hold it: better to sacrifice the visual than the frame
rate, because a visual that costs gameplay is a visual nobody sees.

The wasm question is also unanswered: no threads in the same shape, so any
future offload owes a deliberate single-threaded fallback, stated in the code.

If `20260819-123928` measures a new heavy site, that site gets its own task with
its own number. It does not reopen this one.
