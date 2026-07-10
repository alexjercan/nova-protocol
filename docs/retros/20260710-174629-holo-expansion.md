# Retro: World-space holo instruments (ribbon, SOI shell, flip gate)

- TASK: 20260710-174629
- BRANCH: holo-expansion (squash-merged to master as 5692536)
- REVIEW ROUNDS: 1 (APPROVE with 3 MINOR + 3 NIT, all fixed in-round)

Closes the diegetic-instruments arc (spike 20260710-174523): fourth
spike-seeded task shipped against the same substrate.

## What went well

- **First cycle of the arc with zero MAJORs.** The two AGENTS.md-bound
  lessons were applied at design time: the ribbon renders the plan the
  computer actually flies (straight segments, explicitly disclaiming
  curvature until the gravity-aware solve exists - task 20260710-193500),
  and the STOP rest point reuses the flip math's exact terms rather than
  re-deriving them.
- **Overturning a documented exclusion explicitly worked.** The
  instruments task had recorded "STOP has no spatial goal"; this task
  overturned it in the plan (the rest point IS a spatial goal), which got
  STOP coverage for the readout chip and the ribbon for one helper
  function.
- **The sync-system pattern is now a house idiom**: ribbon, gate, and
  shell all follow the orbit ring's shape (state-keyed spawn/update/
  despawn, plain Assets access, headless lifecycle tests) and none needed
  harness invention.

## What went wrong

- **R1.2: "shared" assets were not shared.** `Local<HoloAssets>` reads
  like a cache but is per-system, so three (four with the ring) identical
  materials shipped while the doc claimed one. Root cause: Local's
  semantics were assumed from its name, and the doc was written from the
  intent rather than the code - the same drift-from-the-owning-truth
  shape, this time between code and its own documentation.
- **R1.1: the STOP publish gate had no hysteresis** even though this
  exact failure (UI strobing at a threshold) had been solved twice before
  in this codebase (align gate, Hold phase). Pattern-recall, not
  knowledge, was the gap.
- The Resource switch broke the bare-World tests until they init'd it -
  caught immediately, but it shows the tests were (correctly) sensitive
  to the system signature.

## What to improve next time

- When writing a threshold that turns a visible thing on/off, add
  hysteresis by default; this codebase now has four instances of the
  pattern (align gate, Hold phase, dominant-well switch, STOP telemetry).
- Treat `Local<T>` caches as per-system by definition; anything called
  "shared" must be a Resource.

## Action items

- [ ] Playtest the whole arc together: ribbon thickness/visibility at
  range, gate size (4u), shell legibility vs the velocity sphere, and
  whether SHELL_APPROACH_FACTOR 1.5 announces wells early enough.
- [x] Arc complete; the one seeded follow-up that remains is
  20260710-193500 (gravity-aware arrival solve, spike-first).
