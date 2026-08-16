# Never block the main thread on a scenario transition

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.11.0,performance,scenario,ui

## Goal

Owner: "we should never block the main thread, in between transition of scenarios
should have a loading screen + non blocking".

The loading panel landed with `bd5c4a0f`, and the 11-second stall that made it
necessary is gone - a rock's noise graph is built once instead of per vertex, so
a chapter load went from one frozen 11 167 ms frame to about 300 ms. This task
finishes the job: remove the remaining block.

## What still blocks

**One frame of about 300 ms**: the `state_to_world` queued-command flush at
`crates/nova_scenario/src/world.rs:237-249`, which drains every OnStart spawn in
a single `queue.apply(world)`. The panel is up and rendered across it, so a
player sees a moving screen rather than a freeze - but that frame itself cannot
animate, and by the owner's rule it should not exist.

Two smaller costs flank the spawn, both understood:

- avian collider ingest
- synchronous pipeline compilation on first draw. `synchronous_pipeline_compilation:
  true` is deliberate (task 20260805-111329, a device-teardown SIGSEGV), so this
  one is a documented trade rather than a defect.

## Off-thread is not available

Spawning needs `&mut World`, so the work cannot move to another thread. The only
route is CHUNKING across frames: drain the `VecDeque` under a per-frame time
budget, applying per command. Each closure is one object, so objects stay atomic
and a ship's sections still land in one `Added<SectionLinkPoints>` batch.

The load-screen lane scoped that at about 20 lines and deliberately did not ship
it. The code is small; the RISK is scenario semantics.

## The risk, and the proposed answer

An `OnUpdate` handler whose predicate reads object counts would see a
half-populated world for a frame or two.

**Gate the scenario script until the spawn queue drains** rather than testing
around the inconsistency. Do not run `OnStart`/`OnUpdate` handlers while objects
are still arriving. That turns "the world is briefly inconsistent" into "the world
is not yet live", which is a far easier property to defend and to test. The owner
agreed with this shape.

Consider whether the loading panel should stay up until the gate lifts, so the
two facts are the same fact.

## Definition of done

- no frame over the budget during a scenario transition, measured the way the
  load-screen lane measured: `Time<Real>` per-frame deltas plus `Instant` spans
- the loading panel animates across the WHOLE transition, proven by consecutive
  frames that differ
- scenario scripts cannot observe a partly built world
- the scenario harness examples pass unchanged - they are the gate on semantics
