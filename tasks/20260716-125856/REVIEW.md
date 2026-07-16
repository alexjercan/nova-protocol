# Review: Scenario outcome frame - Victory/Defeat action + overlay

- TASK: 20260716-125856
- BRANCH: feature/scenario-outcome-frame

## Round 1

- VERDICT: REQUEST_CHANGES
- Reviewer: out-of-context pass (fresh-context agent over the raw diff; ran
  the suite: nova_scenario 70 + nova_menu 43 + parity 2 + skybox e2e 1, all
  green; verified regeneration consistency, consumer sweep, test vacuity).

- [x] R1.1 (MAJOR) crates/nova_menu/src/lib.rs:288 (restore_cursor) vs :562
  (sync_outcome_cursor) - pause/unpause over a live VICTORY overlay bricks
  the mouse: outcome frees the cursor, Esc pauses, Esc/Resume unpauses ->
  restore_cursor sees Playing + a live PlayerSpaceshipMarker (Victory keeps
  the ship alive) and re-locks/hides the cursor; sync_outcome_cursor only
  acts on outcome.is_changed() so it never re-frees - Continue/Main Menu
  become unclickable in release builds. Defeat masks it (ship despawned),
  which is why the probe screenshot missed it. Fix: skip the re-grab in
  restore_cursor when an outcome is declared; same guard in
  regrab_cursor_on_player_spawn for symmetry; add a pause-cycle-over-outcome
  regression test (the pause test rig + a spawned PrimaryWindow/CursorOptions
  entity).
  - Response: fixed - restore_cursor and regrab_cursor_on_player_spawn skip when an outcome is declared; regression test pause_cycle_over_a_live_outcome_keeps_the_cursor_free with a cfg(not(debug)) delivery guard (the guard half ran green on the default-features suite).
- [x] R1.2 (MINOR) crates/nova_scenario/src/world.rs (clear() before the
  drain) - `Outcome` composed with `NextScenario(linger: false)` silently
  drops the outcome: the instant switch tears down and wipes
  queued_commands before the drain applies the CurrentOutcome write. Author
  gets zero feedback. Suggest a debug/warn on discarding non-empty
  queued_commands, or a docs sentence that linger: false swallows the
  outcome.
  - Response: fixed both ways - clear() debug-logs discarded undrained commands, and guide-author-scenario.md's Outcome section now states the linger rule (an instant switch swallows the outcome).
- [x] R1.3 (MINOR) crates/nova_menu/src/lib.rs:427 (sync_outcome_overlay) -
  Continue/Retry and the [Enter] hint snapshot next_scenario.is_some() only
  at outcome-change time; a NextScenario queued by a LATER event leaves
  stale UI (overlay says Main Menu while Enter actually releases the queued
  switch). Suggest also rebuilding when queued-ness flips, or hard-document
  "queue the NextScenario in the same event as the Outcome".
  - Response: fixed - OutcomeOverlay carries the queued snapshot and sync_outcome_overlay rebuilds when it goes stale; test outcome_overlay_rebuilds_when_a_switch_is_queued_later.
- [x] R1.4 (MINOR) crates/nova_scenario/src/loader.rs:663 (on_next_input) -
  the observer wiring is untested; only the pure decide_advance table is. A
  mutation in the match body (ExitToMenu dropping state.set) passes the
  suite. Also the TASK step claiming an Enter integration test is ticked
  while coverage is actually table + button-path equivalents; amend the step
  text or add the wiring test.
  - Response: TASK step text amended to state the real coverage (decide_advance table + button route); the Enter-key wiring pin deliberately lands with the slice's example 19 (20260708-203659) where the full production chain is exercised - synthesizing a bevy_enhanced_input Start<> in a unit rig is not worth the harness.
- [x] R1.5 (NIT) tasks/20260716-125856/TASK.md:27 - step 2 says
  "ScenarioScopedMarker so scenario teardown despawns it"; the
  implementation (deliberately) uses CurrentOutcome-driven despawn +
  DespawnOnExit(Playing). Amend the ticked step text to match reality.
  - Response: fixed - step text amended to the as-built lifecycle (resource-driven despawn + DespawnOnExit).
- [x] R1.6 (NIT) crates/nova_scenario/src/actions.rs
  (outcome_action_without_the_resource_is_a_warning_not_a_panic) - the
  "warning" half of the name is unasserted; either pin the warn via a log
  capture or rename the test to what it asserts.
  - Response: fixed - renamed to outcome_action_without_the_resource_does_not_panic_or_conjure_it; the warn stays unasserted by design (doc comment says so).
- [x] R1.7 (NIT) crates/nova_menu/src/lib.rs (z-order) - nothing pins the
  load-bearing outcome(9) < pause(10) relation the code comments rely on;
  overlay_roots_carry_an_explicit_z_index sweeps only the main-menu panels.
  A one-line relational assert would keep a future pause-z change from
  sinking the ESC-over-outcome ordering.
  - Response: fixed - test outcome_overlay_sits_below_the_pause_overlay pins outcome_z < pause_z relationally.
- [x] R1.8 (NIT) crates/nova_assets/src/scenario.rs:403,504,529 -
  asteroid_field/asteroid_next keep silent lingering NextScenario beats
  (incl. a death restart) with no Outcome, so the "old silent press-Enter"
  UX still exists in that chain. In-scope per the TASK (shakedown only);
  record the retrofit as follow-up work on the slice task.
  - Response: recorded on the slice task 20260708-203659 (retrofit the asteroid_field chain when the outcome vocabulary lands across base content).

Clean areas verified: consumer sweep (no exhaustive EventActionConfig match
breaks; editor matches! is non-exhaustive; portal/modding only serde-wrap),
regeneration consistency (only shakedown_run.content.ron changed; parity x2
green; committed RON with the new Outcome block parses through the
production load path), test vacuity (each new test fails with its mechanism
deleted; drain test carries its own delivery guard), lifecycle/change
detection (first-run is_changed benign; no double-despawn; Retry teardown
clears outcome before the new ship's deferred spawn), docs accuracy, spec
honesty (buttons deviation recorded).

## Round 2

- VERDICT: APPROVE

Verified each Round 1 response against the new diff (commit "fix(outcome):
review round 1 ..."):

- R1.1: both cursor functions carry the outcome guard; the regression test
  ran on the DEFAULT-features suite, so its cfg(not(debug)) delivery-guard
  half (outcome cleared -> the same pause cycle re-locks) actually executed
  and passed - the free-cursor assert cannot be vacuous. Ticked.
- R1.2: clear() traces discarded undrained commands; the authoring guide
  states the linger rule. Ticked.
- R1.3: queued snapshot lives on the marker; rebuild pinned by
  outcome_overlay_rebuilds_when_a_switch_is_queued_later (green). Ticked.
- R1.4: reasoned deferral accepted - the pure table + button route cover the
  decision and the release mechanism; the full Enter chain is explicitly
  assigned to example 19 in the slice task. Ticked.
- R1.5/R1.6: step text and test name now match reality. Ticked.
- R1.7: relational z pin green. Ticked.
- R1.8: follow-up recorded on 20260708-203659. Ticked.

Suite after fixes: nova_scenario 70 + nova_menu 46 + parity 2 + skybox e2e 1,
all green; cargo check --workspace --all-targets clean; fmt clean. Full
workspace suite + clippy run in CI per the repo's standing instruction.
No new findings introduced by the fixes. APPROVED.
