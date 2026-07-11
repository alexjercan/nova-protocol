# Retro: Multi-target tracking + subtarget cycle HUD

- TASK: 20260708-165705
- BRANCH: feature/multi-target-cycle
- REVIEW ROUNDS: 2 (round 1 APPROVE with two MINORs, both fixed and
  verified in round 2)

## What went well

- Verify-first on the dependency paid off twice. Reading
  bevy_enhanced_input 0.26's source BEFORE writing bindings established
  (a) conditions attach to binding entities, not just actions - putting
  Chord on the wheel/bracket bindings kept the gamepad binding
  modifier-free, where the obvious action-level Chord would have silently
  gated the pad; (b) native Binding mod_keys would NOT have sufficed (a
  modifier-less binding still fires while CTRL is held without
  consume_input), killing a tempting "simpler" design before it shipped.
- The one genuinely fiddly piece (input exclusivity) got an end-to-end
  test through the real plugin pipeline - real rig bundle, simulated
  keyboard/wheel/gamepad - instead of a mocked-action test that would have
  bypassed exactly the binding-level behavior under test. It also served
  as the review's independent verification of the load-bearing claim.
- Mirroring existing patterns (component-lock pin, reconcile-style HUD
  sync, resource-per-targeting-fact) made most of the diff mechanical;
  zero review findings landed in those parts.

## What went wrong

- The plan bound target-cycle prev to GamepadButton::DPadDown, which has
  been ORBIT since the autopilot work. Caught only while writing the
  bindings. Root cause: the plan named concrete buttons without citing
  the current binding table - a grep for `GamepadButton::` at plan time
  would have surfaced it in seconds.
- Review R1.1: the new hint rows carried always-non-empty fixed labels,
  breaking keybind_hints' documented "no rig, no keys, no hints"
  invariant. Root cause: extended `FlightVerbHints` without re-reading
  the consumer module's header doc where the invariant is stated.
- Two small test-rig potholes cost a compile-fix cycle each: BEI
  finalizes its context registry in `App::finish`, so an input test must
  run `app.finish()`/`app.cleanup()` before spawning a rig; and Bevy
  0.19's `MouseWheel` grew a `phase: TouchPhase` field.

## What to improve next time

- A plan step that names concrete key/button assignments must quote the
  existing binding table (same rule as formulas and orderings - it is a
  claim about current system state, not a free choice).
- When extending a resource/struct, re-read the header docs of its
  consumers for stated invariants before adding fields they render.

## Action items

- [x] Bumped `verify-first-plan-steps` in LESSONS.md (the dpad collision
  is the same failure class: a plan step encoding system state without
  citing the verifying file).
- [x] Bumped `reread-after-insert` (consumer-invariant variant noted).
- [x] Recorded the BEI `App::finish` test gotcha as a domain lesson.
