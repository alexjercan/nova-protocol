# Retro: CTRL press alone fires the target cycle

- TASK: 20260711-173237
- BRANCH: fix/ctrl-alone-target-cycle
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The mechanism was verified in the dependency's source BEFORE any fix
  attempt (Chord::evaluate ignores `_value`), so the task file shipped
  with a correct diagnosis and the user report was reproducible in one
  test assertion.
- The planned fix (Down+Chord) was falsified by the test BEFORE landing:
  the first run failed on "plain scroll must not cycle targets", which
  exposed the second-layer subtlety (combiner caps at Ongoing; Start
  triggers on None -> Ongoing). Writing the regression before trusting
  the planned one-liner saved a second shipped bug.
- Fail-first A/B with committed fix + checkout-from-master sabotage,
  recorded numbers (pin Some(4.001216) vs None), revealing the bug was
  worse than reported (both directions fired).

## What went wrong

- The original 20260708-165705 e2e test passed coincidentally on buggy
  code: it counted action EVENTS and did not re-assert between the
  modifier press and the scroll, so the bare-CTRL misfire pre-incremented
  the exact count the later assertion expected. Root cause: asserting the
  wiring layer (event counts at gesture end) instead of the behavior
  layer (lock/pin/component state at every gesture step).
- The 165705 review praised that test as the load-bearing verification;
  a reviewer step through the gesture timeline (what is the expected
  state after EACH input event?) would have caught the missing
  intermediate assertion.

## What to improve next time

- Tests guarding modal/chorded input must assert the affected STATE after
  every step of the gesture (press modifier, gesture, release), not count
  events at the end.
- When an input library's condition DSL fights a modal gesture, route in
  an observer reading the modifier action's state - game semantics in
  game code - instead of stacking conditions whose combination semantics
  need a source dive to predict.

## Action items

- [x] Added `assert-each-gesture-step` and `modal-input-observer-dispatch`
  to LESSONS.md.
