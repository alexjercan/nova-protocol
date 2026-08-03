# Retro: Make nova_autopilot predicate-driven: a generic scripted state machine

- TASK: 20260802-120025
- BRANCH: refactor/predicate-autopilot
- REVIEW ROUNDS: 1

## What went well

- The caller-inventory-first plan paid off. DECISION.md required a named
  caller for every item of the proposed vocabulary, which cut `or`,
  `entity_count`, `drag` and `observe` before they became public items the
  prelude test would pin. Round 1 found no speculative knob, only two items
  (`move_cursor`, `state_is`) that the Steps named explicitly.
- Red-on-base proofs were chosen honestly. The `nova_probe` run proof is
  vacuous on the base branch (migrated sources do not compile against the old
  driver), and the plan said so, pinning the three absence greps as the real
  red-on-base evidence instead. Both greps are empty on the branch.
- Review round 1 was APPROVE with nine MINOR/NIT findings and no BLOCKER or
  MAJOR - the from-scratch challenge had already been answered in DECISION.md,
  so the reviewer had nothing structural left to dispute.
- Driving `click_at` against a real `bevy_ui` widget instead of settling for
  state-level assertions caught a genuine bug: `bevy_picking` reads
  `WindowEvent`, not the concrete `CursorMoved` / `MouseButtonInput` messages,
  so state-only assertions would have passed while every synthesized click
  resolved at the origin. The plan left that depth as an open question with a
  recorded fallback; taking the deeper option was the right call.

## What went wrong

- The driver reports `AUTOPILOT` done unconditionally after the last step, but
  the five callers migrated at the construction site end their wrapping step on
  `script_reports_done()` - i.e. after the script already cleared it. Every
  broadside / lifeline / menu_scenarios / screenshot_nova_os run now logs
  `done(autopilot) but it is not pending` (R1.1). The decision that led here
  was sound at the time: `script_reports_done()` was the minimum-diff way to
  wrap five scripts that own their own completion, and DECISION.md records that
  `20260802-120029` retires it. What was missed is that "the script already
  reported done" and "the driver reports done at the end" are two claims on the
  same one-shot registration.
- Two DoD proofs are weaker than their wording. The loop-point test scripts a
  single `hold`, so `loop_from` resolves to index 0 and cannot distinguish a
  jump-to-named-step from a restart-from-zero (R1.3); the stall test asserts
  only a non-success exit, not the three fields the DoD promises the message
  carries (R1.4). The plan wrote each criterion as a behavior sentence and
  named a test, but never asked what the test would have to script for the
  sentence to be falsifiable.
- A comment in `menu_scenarios.rs` still credits a function this diff deleted
  (R1.2), and the CHANGELOG overstates `hud_range`'s conversion - one of its
  thirteen steps waits on the world, the rest are dwells (R1.6). The example's
  own module doc got this right; the CHANGELOG did not.

## What to improve next time

- Breadth: the diff is large (24 files, +2787/-1552) and that was decided
  deliberately, not discovered. DECISION.md rejected splitting driver from
  callers because deleting `self_completing` leaves no compiling intermediate
  tree, and the offsetting cut - only three scripts rewritten, five touched at
  the construction site - held. The recorded consequence (a bisect across this
  commit cannot separate a driver bug from a migration bug) is the price, and
  R1.1 is exactly that shape of bug. No split was missed; the boundary was
  correctly identified as inseparable.
- Churn: one round, and its findings are polish. The plan-time question that
  would have prevented R1.3 and R1.4 is not the from-scratch challenge but a
  narrower one: for each `test:` proof, name the input the test must construct
  for the criterion to be able to fail. A one-step script cannot falsify "jumps
  to the named step".
- When a criterion says a diagnostic "names X, Y and Z", the proof has to read
  the message. An exit-code assertion proves the abort, not the diagnostic -
  and the diagnostic is the entire point of this task.
- Context: no compaction warning, checkpoint or handoff is recorded on this
  task; the work ran to WORK_DONE in one pass. Review round 1 was delegated to
  an out-of-context reviewer as the skill requires, which is the only context
  split here and it worked - the reviewer re-derived the duplicate-`done`
  warning from a live probe run rather than from the record.

## Action items

- R1.1 through R1.9 stay open on REVIEW.md as non-blocking polish. R1.1
  (duplicate `done(autopilot)`) and R1.3/R1.4 (weak DoD proofs) are the ones
  worth carrying forward; `20260802-120029` already owns retiring
  `script_reports_done()`, which removes R1.1's cause at the source.
- Failed observation writes: none.

## Landing message

```
refactor(autopilot): drive scripts by predicates, not wall-clock

AutopilotPlugin is now a list of named steps, each advancing when a
predicate over the world holds. elapsed() is one predicate among
frames, state_is, resource_where, any_entity, and, not, plus any
closure. Steps carry enter, on_enter, each (step-relative elapsed),
until and an optional deadline; loop_from(name) + on_loop(f) replace
loop_while_pending and the per-step deadline replaces self_completing.
A run logs its beats and a stall error-exits naming the beat that
stalled instead of reporting that a runway expired.

nova_autopilot::input adds the gestures a predicate-driven script
needs (press_key, release_key, press_mouse, release_mouse,
move_cursor, click_at), writing both WindowEvent and the concrete
messages so bevy_picking resolves a synthesized click where it landed.
nova_debug::harness adds the Nova-typed predicates
(scenario_variable_is, section_gone, player_ship_present).

hold(state, secs) and input(f) survive as constructors over the step
model, so the six pure-timeline callers are untouched; the other eight
example binaries migrate in this commit because deleting
self_completing leaves no compiling intermediate tree.
```
