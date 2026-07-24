# Retro: Flight objective HUD rework

- TASK: 20260724-134312
- BRANCH: feat/objective-hint
- REVIEW ROUNDS: 1 (out-of-context APPROVE, zero findings)

(What/why in TASK.md close-out; this is process only.)

## What went well

- Reading the bcs `rebuild_lines` at PLAN time (it's a `Single<..panel>` that
  skips when absent, and `ObjectivesPlugin` owns `GameObjectives`) turned "remove
  the compact panel" from a scary resource-lifetime question into a clean
  deletion: keep the plugin, drop the spawn, done. `verify-engine-guarantees-in-
  source` paid off before a line of code - the out-of-context reviewer later
  re-derived the exact same fact from the locked bcs rev and confirmed it.
- The does-the-old-element-survive removal sweep was clean because the plan
  grepped the removed symbols up front: the one cross-module consumer
  (`OBJECTIVES_PANEL_WIDTH_PX` in `objective_feedback`) surfaced at compile time,
  not review, and got a local const. Zero review findings on a 4-file rework.
- Answering the two owner design forks at the gate (gamepad button, hint content)
  with `AskUserQuestion` before cutting the worktree meant no rework from a
  guessed-wrong default.

## What went wrong

- `pad_toggles_drawer_state` failed on first run - asserted after a single
  `update()`, but a `NextState` set during Update applies on the NEXT frame, and
  without a `clear()` the stale `just_pressed` edge re-toggles. This is the SAME
  trap the Tab test hit earlier in this drawer family (`press_tab` already had the
  press+update / release+clear+update shape). Root cause: I wrote the pad test
  fresh instead of copying `press_tab`'s helper shape verbatim - the exact
  `reuse-known-good-stack` lesson, and the exact `NextState`/input-edge gotcha,
  both already on record.

## What to improve next time

- For a headless input test that drives a state transition, copy the sibling
  press-helper (press+update, release+CLEAR+update) verbatim - do not hand-roll
  the update cadence. This gotcha has now cost a rerun twice in one family.

## Action items

- [x] Lessons ledger: add `nextstate-input-test-needs-clear-and-two-updates`
  (new), bump `reuse-known-good-stack` (recurred again).
