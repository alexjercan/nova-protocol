# Retro: Ambient menu background scenario

- TASK: 20260711-180455
- BRANCH: feature/menu-ambience (squashed to master 1818b57)
- REVIEW ROUNDS: 2 (round 1: 1 MAJOR, 4 MINOR, 2 NIT)

## What went well

- The pre-work research pass (ORBIT verb, spawn velocity, gravity gating,
  playerless AI) caught the decisive constraint before any code: thruster
  flight is dead in MainMenu because the editor gates the input/section
  sets, so the ballistic-orbit mechanism was chosen on evidence, not
  preference - and the infeasible AI-orbit path became a filed task
  (20260711-185440) instead of a dead end.
- Instrumentation beat screenshots decisively. Three scene bugs in a row
  were each diagnosed in one run by a throwaway harness logging per-second
  kinematics plus a well inventory; the earlier screenshot-based "verify"
  had happily captured a camera parked inside a rock.
- The out-of-context review agent again earned its cost (second cycle
  running): its MAJOR (the two bug-carrying systems had no committed
  tests) and the OnExit-teardown generalization both made the landed code
  meaningfully better.

## What went wrong

- All three scene bugs shared one root cause: reasoning with NOMINAL
  authored sizes (radius 20, orbit 50, ring 60-95) while the gravity and
  collider systems operate on GEOMETRIC runtime values (body radius
  ~80-91u derived from the generated noise mesh). The gravity module even
  documents this ("the 2026-07-10 no-stable-band regression") - reading
  the consuming system's derivation before authoring content against it
  would have prevented all three.
- The first verification pass shipped a false positive: pixel screenshots
  "confirmed" the scene while the camera pose write was being overwritten
  by the WASD controller in the same frame the removal was queued - the
  producer/consumer same-frame ordering trap again, this time with a
  component removal instead of a schedule slot.
- The evidence harness stayed throwaway while carrying all the behavioral
  proof; review had to demand committed regression tests for exactly the
  systems the harness had debugged. Convert harness assertions into unit
  tests as they stabilize, not after review asks.

## What to improve next time

- When authoring content (scenarios, placements, orbits) against a system
  that derives its own geometry or parameters at runtime, read the
  derivation first and write placement code against the runtime values
  (query the component) rather than the authored constants.

## Action items

- [x] LESSONS.md: new `authored-vs-derived-values`; bumped
  `diagnostic-first` and the `two-clocks` family note (same-frame
  component-removal variant); bumped `out-of-context-review-pass` to x2.
