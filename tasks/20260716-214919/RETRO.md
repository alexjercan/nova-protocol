# Retro: Pause the sim while the Victory/Defeat outcome frame is up

- TASK: 20260716-214919
- BRANCH: feature/pause-on-outcome (landed ec69507e)
- REVIEW ROUNDS: 1 (APPROVE; 3 MINOR, all addressed or routed)

See TASK.md / NOTES.md for what changed and why, REVIEW.md for the findings.
This retro is process-only.

## What went well

- Verify-first planning paid off completely: reading the whole pause path
  (PauseStates, pause_clocks, the Unpaused set-gates), the outcome overlay,
  decide_advance, AND the external `state_to_world_system` registration in the
  bevy_common_systems dep BEFORE writing code produced the load-bearing design
  insight - reuse the SAME `PauseStates::Paused` the menu uses, so the freeze
  is definitionally identical to the menu's and any incompleteness is a
  pre-existing menu bug, not this task's. Zero mid-implementation surprises.
- The independent out-of-context review agent earned its cost again: it
  re-derived the no-deadlock chain from the dep source (PostUpdate,
  pause-independent), confirmed freeze completeness (no `Time<Real>` anywhere
  in gameplay), and surfaced both the pre-existing `fire_on_update` gap and the
  dropped z-value pin that the shared-session eye would have missed.
- Applied `probe-the-adversarial-variant` preventively: the freeze test runs
  Victory(queued)/Defeat(queued)/Victory(unqueued), leading with Victory-alive
  (the case most likely to keep the sim visibly running behind the banner).
- Swept stale prose: the z-index rationale comment and restore_cursor's
  outcome-guard both described a now-impossible ESC-over-outcome path; both
  updated in the same change rather than left to rot.

## What went wrong

- `advance_decision_table` broke on the `decide_advance` contract change and I
  caught it only by running the full suite, not by sweeping for the fixture
  first. Root cause: changed a pure decision function's logic without grepping
  its test callers up front, so the fail-first re-pin was DISCOVERED rather
  than planned. Cheap here (my own run caught it, master never went red), but
  the sweep should have been a plan step.
- R1.2: the "clear unpauses" tests clear the outcome by direct write, not the
  real Continue/Retry -> teardown chain, so the end-to-end seam under pause is
  covered only by composition. Accepted as adequate (a 3-plugin rig would be
  heavier and less faithful), but it is a genuine coverage seam the reviewer
  was right to name.

## What to improve next time

- Before changing a pure decision/predicate function whose OLD contract a table
  test asserts, grep the function's test callers FIRST and make the fixture
  re-pin an explicit step - so the fail-first A/B is deliberate. (Family of
  pin-the-fix-at-its-boundary refactor-variant.)

## Action items

- [x] Filed follow-up tatr 20260716-231855: gate `fire_on_update` on Unpaused
      for both pause paths (R1.1, pre-existing).
- [x] Ledger: bumped `out-of-context-review-pass` and
      `audit-state-gates-on-new-entry-path`; sharpened `pin-the-fix-at-its-boundary`.
