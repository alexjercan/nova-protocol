# Retro: Main menu - MainMenu state, nova_menu crate, mode wiring

- TASK: 20260711-180426
- BRANCH: feature/main-menu (squashed to master 8504948)
- REVIEW ROUNDS: 2 (round 1: 1 BLOCKER, 1 MAJOR, 2 MINOR, 3 NIT)

## What went well

- Verify-first before coding: reading nova_debug/src/harness.rs answered how
  the 09_editor smoke would survive a new state before any design was
  committed, and the resulting "autopilot clicks the real Sandbox button"
  approach doubles as e2e menu coverage for free.
- The review skill's two mandated habits each caught a shipping bug the
  implementation had missed: independently re-deriving one load-bearing
  claim surfaced the R1.1 BLOCKER (input sets gated on the editor's private
  state), and the out-of-context review agent caught the R1.2 MAJOR
  (BCS_SHOT force-advance yanked backwards by the Loaded hook) plus the
  honest observation that the close record's screenshot claim was
  consistent with a mixed-state capture.
- Running the A/B even though the fix "obviously worked": it exposed that
  the harness pass criterion (speed > 0.1) would also have passed pre-fix
  (2.88 residual drift vs 51.40 under thrust). Without failing it first,
  the check was a placebo.

## What went wrong

- R1.1 (New Game unflyable): the implementation added a second entry route
  into GameStates::Playing but only exercised the route the implementer
  expected (Sandbox, via the existing smoke). Root cause: no audit of
  configure_sets/run_if conditions keyed on sibling state machines when the
  new path was added - the editor's ExampleStates gating of
  SpaceshipInputSystems was read during the spike, but its interaction with
  a NewGame that bypasses the editor was never traced.
- The original close record claimed verification while covering exactly one
  of the two modes the feature exists to provide. A mode switch needs a
  delivery proof per branch of the mode, not per app.
- tatr same-second collision again (five tatr new calls in one command
  produced one task); second occurrence, already a ledger entry.

## What to improve next time

- When a change adds a new route into an existing state (or a new variant
  to a mode enum), grep configure_sets/run_if/in_state across the workspace
  for that state and audit each hit against the new route before calling
  the work done - and give every enum variant its own delivery-proved
  verification, not just the one the demo happens to exercise.

## Action items

- [x] LESSONS.md: new `audit-state-gates-on-new-entry-path`, bumped
  `fail-first-regression-ab` and `tatr-same-second-collision`, noted the
  out-of-context review pass as a positive pattern occurrence.
