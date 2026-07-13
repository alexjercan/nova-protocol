# Retro: Travel/combat lock slots + deliberate radar

- TASK: 20260713-082330
- BRANCH: feature/lock-slots-radar (landed 9e655d1)
- REVIEW ROUNDS: 1 (APPROVE; two MINORs recorded, one deliberate deviation)

Process notes only; what/why in TASK.md and spike 20260713-082207.

## What went well

- The adversarial round's implementation caveats were a literal checklist:
  the enhanced-input event mapping (Complete/Cancel/Fire), the shared
  threshold const + boundary-frame test, the drop-commit-on-pause rule and
  the provisional-candidate separation all went in first-try because they
  were specced before coding.
- Escalating verification tiers paid off: pure fns -> rig unit tests ->
  gesture e2e with real input + manual REAL-clock stepping -> and finally the
  live example DRIVING the actual gesture through the shipping binary's input
  pipeline. The live tier caught a real bug the unit tiers could not: the
  same-frame RMB+CTRL press latches the travel slot (raised derives in Update,
  the radar Start observer fires in PreUpdate). Only an end-to-end run with
  production scheduling exposes cross-schedule ordering.
- WIP commits at green checkpoints (core landed, port landed) made the huge
  diff resumable and reviewable.

## What went wrong

- A python splice keyed on `#[cfg(test)]` matched the FIRST occurrence (a
  cfg'd import at the top of the file), duplicating ~600 lines. Recovered
  surgically, but the lesson is sharp: anchor scripted splices on UNIQUE
  strings and verify with a compile/grep immediately after every splice
  (and commit before large scripted edits - the WIP-commit habit is what made
  the mistake cheap).
- Wrote a Changed<T>-dependent test against run_system_once, which rebuilds
  the system each call so EVERYTHING reads as changed - the deliberate-neutral
  case false-failed. Fixed with world.register_system + run_system (state
  persists). New rig lesson.

## What to improve next time

- Changed<T>/tick-dependent logic in tests: always register the system once
  and reuse the SystemId; run_system_once is only safe for tick-free systems.
- Scripted refactors: unique anchors, compile after each splice, WIP-commit
  before starting.

## Action items

- [x] Ledger: add `registered-system-for-change-detection`; bump
  `commit-before-sabotage` family note (splice variant).
- Recorded for 082337: the same-frame latch edge (R1.1) and the relation-tint
  deviation (R1.2, awaiting user veto); toast stack Name lookup (R1.3).
