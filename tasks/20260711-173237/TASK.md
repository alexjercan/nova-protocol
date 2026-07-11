# Fix: CTRL press alone fires the target cycle

- STATUS: OPEN
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

- [ ] In `flight_input_rig()` (crates/nova_gameplay/src/input/player.rs),
      add `Down::default()` (Explicit, actuation 0.5 - wheel notches are
      1.0/line, brackets are bool 1.0) to every binding that carries
      `Chord::single(modifier)` (4 bindings: wheel up/down, BracketRight,
      BracketLeft).
- [ ] Extend `ctrl_scroll_cycles_targets_and_blocks_the_component_cycle`:
      after pressing ControlLeft and updating (no scroll), assert neither
      counter moved. Prove it fails against the unfixed rig (fail-first:
      commit the fix, then temporarily revert the Down additions and record
      the failing numbers here).
- [ ] Full check suite (check/fmt + touched test filters per project
      policy).

## Notes

- A/B safety: commit the fix BEFORE the sabotage revert (lessons ledger
  `commit-before-sabotage`).
- Do NOT switch to native Binding mod_keys instead: without consume_input
  the modifier-less component bindings still fire under CTRL, so BlockBy
  and the modifier action remain necessary (see 20260708-165705 notes).
