# Retro: Integrity physics-level tests

- TASK: 20260707-170001
- BRANCH: test/physics-integrity-tests
- PR: #39 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260707-170001/TASK.md`. The counterpart to 133008: that task tested the
avian-free core, this one tests the avian-dependent inputs. Spiking the physics harness up
front is what kept it clean.

## What went well

- Spiked the risky mechanism before writing real tests, exactly the 133005/133008 lesson. A
  throwaway `physics_spike.rs` answered every unknown cheaply and empirically: which plugin set
  boots headless avian, whether `ColliderOf`/`ComputedMass` appear and when, and whether a
  simulated collision produces damage. Each failure taught something concrete instead of
  being guessed around.
- Let the failures redesign the tests. Three spike failures each flipped a decision:
  "Message not initialized" -> add `MeshPlugin` (avian's `collider-from-mesh` reads
  `AssetEvent<Mesh>`); simulated impact dealt 0 damage -> the solver zeroes contact velocity
  before the observer reads it, so *inject* the event instead; `ComputedMass` was `NaN` ->
  read mass after a few steps, not the first. None of these were reasoned out in advance;
  measuring found them.
- Split the two damage paths by their physics instead of forcing one style. Impact is injected
  (solver eats the velocity), blast is sim-driven (a sensor has no solver response and fires
  deterministically). Trying to make both use the same mechanism would have made one of them
  flaky or fake.
- Caught the double-count myself. The first blast test injected an event *and* got a real
  sensor overlap, dealing 120 instead of 60. The instrumentation (printing health with/without
  the inject) made it obvious and led to the cleaner sim-only blast test.
- Recomputed expected impact damage from the real mass + the observer's own constants rather
  than hard-coding 75.4, so the test survives an avian mass-model change and still checks the
  wiring.

## What went wrong

- Two self-inflicted compile/logic detours in the spike: adding a plugin after `App::finish()`
  (panics), and spawning the two impact bodies touching (solver explosion -> `NaN` health).
  Both were caught in seconds by the spike, but both were avoidable by thinking about avian's
  init/overlap semantics first. Root cause: treated the spawn positions as cosmetic when they
  are load-bearing for a physics sim.
- Redundant imports. `use super::*` in the test mods already re-exports the parent's
  `avian3d`/`bevy`/`bcs` globs, so my explicit `use` lines were dead - only caught by
  `clippy --all-targets`, not the plain build. Worth remembering that a co-located test mod
  inherits the parent module's glob imports.

## What to improve next time

- For a physics-sim test, treat initial transforms/velocities as part of the setup contract,
  not decoration: bodies must not spawn overlapping, and "read after N steps" applies to any
  avian-computed component (`ComputedMass`, `ColliderOf`, synced `Transform`), not just mass.
- When co-locating tests, prefer `use super::*` alone and add explicit imports only for what it
  does not already provide; run `clippy --all-targets` (not just `test`) to catch dead imports.

## Action items

- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro) - trivial, for whoever next edits that file.
- [x] Reusable `integrity_physics_app` + `settle` harness now exists for any future
      physics-level integrity test.
