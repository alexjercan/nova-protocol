# Retro: NOVA OS ship computer 3D app + `ship` CLI verbs

- TASK: 20260726-115339
- BRANCH: feat/nova-os-ship-app
- REVIEW ROUNDS: 2 (Round 1 out-of-context REQUEST_CHANGES; Round 2 in-session APPROVE)

See TASK.md / NOTES.md for what changed and the architecture; this is process only.

## What went well

- Front-loading exploration paid off. Three parallel explore agents mapped the
  command model, the map app, and the section data model BEFORE planning, which
  surfaced the two facts that shaped the whole design: the map app already solves
  RTT + click-picking (via projected UI blips), and real ships use unreadable
  grid-coord `EntityId`s (forcing the `SectionCode` decision). Planning against
  those facts instead of guesses avoided a re-cut mid-build.
- Making argument delivery a first-class `CommandDispatch::Gameplay` instead of
  smuggling args through the snapshot kept `nova_os` ECS-free AND gave the future
  queued-job model a clean seam - one decision that served both the task and the
  DECISION fork-4 extension.
- The out-of-context review earned its keep: it found a BLOCKER a shared-session
  reviewer would likely have inherited my "it works" assumption on.

## What went wrong

- R1.1 (BLOCKER): blip projection used the section WORLD position while the
  scene, blocks and camera were in ship-LOCAL space anchored at the origin. Root
  cause: I copied the map app's projection call without re-checking its
  coordinate-frame assumption. The map scene IS world-space, so projecting the
  world position is correct there; my scene is local/origin-anchored, so the same
  call was wrong. The API shape transferred; the frame assumption did not.
- The BLOCKER was invisible to every test and the example because they all spawn
  the ship at the world origin with identity rotation - the one pose where
  world == local. A coordinate bug needs a fixture off the trivial point to show.
- R1.2 (MAJOR): the in-app `L`/`P` action path was untested. Root cause: it
  shares `apply_action_to_section` with the CLI path, and I let "the mutation is
  covered" stand in for "each entry point is covered" - the message handler's
  wiring, target resolution and note-flash were unpinned.

## What to improve next time

- When reusing a rendering/spatial pattern, verify the COORDINATE FRAME it
  assumes (world vs local vs screen), not just that the call compiles.
- Give spatial/transform tests a non-trivial fixture: put the root off-origin and
  rotated so a world-vs-local mismatch actually surfaces. An origin fixture proves
  almost nothing about placement.
- Pin each CALLER of a shared mutation helper, not just the helper - a shared core
  does not cover per-entry-point wiring.

## Action items

- [x] Fixed R1.1 (project local/scene space; removed `world_pos`) + regression pin.
- [x] Fixed R1.2 (in-app message-handler test).
- [x] Recorded 3 lessons in LESSONS.md (see below).
- [ ] Follow-up (already noted in DECISION.md, not filed yet): factor the shared
  RTT/orbit/blip scaffolding out of `nova_os_map.rs` + `nova_os_ship.rs`; queued/
  over-time actions + hull-resource costs + ship inventory panel. To be filed at
  the next backlog grooming, not this task.
