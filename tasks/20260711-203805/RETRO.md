# Retro: F1 back-to-editor is Sandbox-only

- TASK: 20260711-203805
- BRANCH: fix/f1-sandbox-only (squashed to master c9bf494)
- REVIEW ROUNDS: 1 (APPROVE; in-session review with re-derivation,
  proportionate to a one-condition diff)

## What went well

- Smooth, small cycle: the mode-gate pattern from the menu family applied
  directly, and the ledger's would-it-fail-without-it rule shaped the test
  from the start (both directions provable, null branch guarded).
- Filing the user's report as its own task instead of widening the
  in-flight pause branch kept both diffs reviewable and avoided a
  same-file conflict.

## What went wrong

- One test-rig detour: entering Playing under Sandbox routes the inner
  state into Editor, whose scene setup needs GameAssets and panics
  headless. Known constraint from earlier cycles, forgotten for one
  iteration; the fix (enter via NewGame, flip the mode at press time) is
  recorded in the close record.

## What to improve next time

- The editor test fixtures could use a shared comment or helper naming the
  "never apply ExampleStates::Editor headless" constraint; three tests now
  dance around it independently.
