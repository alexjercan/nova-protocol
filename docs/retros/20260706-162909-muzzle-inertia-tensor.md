# Retro: Projectiles inherit the ship's rotational muzzle velocity

- TASK: 20260706-162909
- BRANCH: feat/muzzle-inertia-tensor
- PR: #41 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260706-162909/TASK.md`. A "just enable the commented-out line" task that would
have shipped a bug if taken at face value.

## What went well

- Did not trust the pre-written sketch. The FIXME'd line
  (`ang_vel.cross(radius_vector) + lin_vel`) looked ready to switch on, but checking avian's
  `ComputedCenterOfMass` docs showed it is body-*local* while `radius_vector` subtracted it from
  a *world* position. Reading the frame of every quantity before wiring it up caught a lever-arm
  bug that "uncomment the line" would have shipped.
- Extracted the kinematics into one pure, tested helper instead of enabling two nearly-identical
  inline expressions. The physics is now verified once (four sharp cases) and both call sites
  read as intent, not arithmetic.
- Chose test cases that actually constrain the formula: pure-rotation with a hand-computed cross
  product pins direction *and* sign (easy to get backwards), on-COM pins the `p - com = 0`
  branch, translation-only pins the regression-safety (non-rotating ships unchanged).

## What went wrong

- Nothing broke, but I briefly considered an end-to-end test (spin a real ship, fire, assert
  tangential muzzle velocity) and decided against it. Root-cause reasoning: the only non-trivial
  new logic is the pure helper (tested) and a one-line world-frame conversion (visible in review
  and covered for no-panic by the example firing ranges). An integration test would have needed
  the full ship/section/spawner setup for little additional assurance over the unit tests. Worth
  noting as a deliberate scope call, not an oversight.

## What to improve next time

- When a task is "enable the code that's already there", treat the existing code as a draft to
  verify, not an answer to uncomment. Check the frame/units of every input first - local vs
  world, radians vs degrees, COM-relative vs origin-relative - because commented-out sketches
  are exactly the code that was never run and never checked.

## Action items

- [ ] Optional follow-up: an end-to-end firing test on a rotating ship (physics harness from
      170001) if muzzle-velocity behaviour ever needs regression cover beyond the unit tests.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
