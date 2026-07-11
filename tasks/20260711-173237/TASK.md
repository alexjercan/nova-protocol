# Fix: CTRL press alone fires the target cycle

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: bug, hud, input

User report (20260711, playtest of 20260708-165705): "when I press CTRL it
instantly moves on the other target" - pressing the modifier alone cycles
the lock, without any scroll.

## Mechanism (verified in bevy_enhanced_input 0.26 source)

`Chord::evaluate` (src/condition/chord.rs) IGNORES the binding's input
value (`_value`) - it returns Fired whenever all chorded actions fire. A
binding whose ONLY condition is `Chord` therefore loses the implicit
"fires on any non-zero value" Down behavior (that default applies only
when NO conditions are attached, src/condition.rs), so the chorded
wheel/bracket bindings of `TargetCycleNextInput`/`TargetCyclePrevInput`
fire the moment CTRL is held. Chord is an Implicit-kind condition
(all-implicit-must-fire, capping others at Ongoing); the fix is to add an
Explicit actuation condition alongside it.

The existing e2e test passed coincidentally: the CTRL press itself fired
the action (count 0 -> 1) one update before the scroll, and Start does not
re-fire while held, so the post-scroll assertion saw the same count it
expected.

## Goal

Holding CTRL alone does nothing; CTRL+scroll (and CTRL+brackets) still
cycles; plain scroll still cycles components. The e2e test fails on the
pre-fix code for the CTRL-alone case.

## Steps

- [x] REPLANNED during implementation: the Down+Chord pairing is ALSO
      wrong - the combiner (bevy_enhanced_input trigger_tracker.rs) caps a
      fired-explicit/unfired-implicit binding at Ongoing, and `Start`
      triggers on None -> Ongoing too, so the UNMODIFIED scroll then
      started the target cycle (observed: "plain scroll must not cycle
      targets: left 1, right 0"). Actual fix: drop Chord/BlockBy entirely
      and route in the observers - `on_component_cycle_next/prev`
      (input/targeting.rs) read the modifier action's `TriggerState` and
      step the SHIP lock while it fires, the component fine-lock
      otherwise. The rig returns to the plain `actions!` macro (no entity
      captures needed); `TargetCycleNextInput` keeps DPadUp,
      `TargetCyclePrevInput` keeps no direct binding (reached via the
      dispatch; a dedicated key can bind later).
- [x] The e2e test moved to input/targeting.rs
      (`ctrl_routes_the_wheel_between_component_and_target_cycle`) and now
      asserts BEHAVIOR (lock / component-lock / pin resources) instead of
      action events - the old event-counting form passed coincidentally on
      the buggy code because the CTRL press itself fired the action one
      update before the scroll. Covers: plain scroll steps the component
      (delivery guard for the wheel), CTRL alone changes nothing (delivery
      guard: the modifier action is Fired), CTRL+scroll steps and pins the
      lock without touching the component, release hands the wheel back,
      DPadUp cycles unmodified. Fail-first proven: with the fix committed,
      `git checkout master -- input/player.rs` (the landed wiring, plus a
      2-line visibility shim) fails the test at "CTRL alone must not pin:
      left Some(4.001216), right None" - on the buggy wiring CTRL alone
      fired BOTH chorded cycle actions (next and prev, lock netting back)
      and pinned for 4 s.
- [x] Full check suite: cargo check --workspace green, fmt applied,
      input:: filter 128/128 green. Full suite in CI.

## Notes

- A/B safety: commit the fix BEFORE the sabotage revert (lessons ledger
  `commit-before-sabotage`).
- Do NOT switch to native Binding mod_keys instead: without consume_input
  the modifier-less component bindings still fire under CTRL, so BlockBy
  and the modifier action remain necessary (see 20260708-165705 notes).

## Close record (20260711)

What changed: dropped the Chord/BlockBy input-condition routing entirely;
the CTRL modifier is a plain action whose TriggerState the component-cycle
observers read (`cycle_modifier_held` in input/targeting.rs), stepping the
ship lock while it fires and the component fine-lock otherwise. The rig is
back on the actions! macro; the e2e test moved to targeting.rs and asserts
resource behavior.

Why not the planned Down fix: two layered dependency subtleties, both
verified in source. (1) Chord::evaluate ignores the binding value ->
fires on the bare modifier (the reported bug). (2) The condition combiner
(trigger_tracker.rs `state()`) yields Ongoing when an explicit condition
fires but an implicit does not, and `Start` triggers on None -> Ongoing
(events.rs) -> the planned Down+Chord pairing made PLAIN scroll start the
target cycle. Observer dispatch sidesteps the condition DSL for modal
gestures altogether; game semantics live in game code.

Difficulties: the original e2e test passed coincidentally on the buggy
code (Ctrl press pre-fired the counter it later asserted) - event-count
assertions on the action layer masked a behavior bug; the rewritten test
asserts on the lock/pin/component resources instead.

Self-reflection: the review's "load-bearing claim verified by e2e test"
was half-right - the test drove real input but asserted the wrong layer,
and its counters were not re-checked between the modifier press and the
scroll. When a test guards a modal gesture, assert at every step of the
gesture, not just the end state, and prefer asserting the affected state
over counting events.
