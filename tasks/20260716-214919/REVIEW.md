# Review: Pause the game while the Victory/Defeat outcome screen is displayed

- TASK: 20260716-214919
- BRANCH: feature/pause-on-outcome

## Round 1

- VERDICT: APPROVE

Reviewed the diff against master with fresh eyes plus an independent
out-of-context pass that re-derived all six load-bearing claims from source
(deadlock-freedom, freeze completeness, no spurious unpause, no stuck state,
1-frame races, test non-vacuity). No BLOCKER or MAJOR: the five safety claims
hold and the freeze mechanism is the identical `PauseStates::Paused` the menu
uses. Findings below are all MINOR/NIT and left to implementer discretion.

- [x] R1.1 (MINOR) [pre-existing - route to follow-up] crates/nova_scenario/src/loader.rs:252
  - `fire_on_update` is gated only on `scenario_is_live`, not on
  `PauseStates::Unpaused`, so the `OnUpdate` scenario pulse keeps firing while
  Paused, and OnUpdate handlers' actions still apply via the ungated PostUpdate
  `state_to_world_system`. In practice the world values those handlers key on
  (positions, kills, orbit holds) are frozen, so nothing NEWLY fires; a handler
  whose predicate is already true would re-run its action per frame. This hole
  is IDENTICAL for the ESC pause menu (which also enters `Paused` without
  gating the pulse), so it is pre-existing, not a regression, and consistent
  with "freeze the same way the pause menu does". Per the review rule (review
  the diff, not the repo) this is not a blocker on this branch; filed as a
  separate task to gate the pulse for BOTH pause paths.
  - Response: Agreed, pre-existing and out of scope. Filed follow-up task
    20260716-231855 to gate `fire_on_update` on `Unpaused` for both the pause
    menu and the outcome frame. Not changed on this branch (changing it here
    would make the outcome diverge from the menu, the opposite of the goal).
- [x] R1.2 (MINOR) crates/nova_menu/src/lib.rs test rig - the "clear unpauses"
  assertions clear the outcome by writing `CurrentOutcome.0 = None` directly
  rather than driving the real Continue/Retry -> `release_lingering_next` ->
  PostUpdate switch -> `teardown_scenario_entities` -> None chain, so the
  end-to-end seam (does the switch really clear the outcome AND unpause while
  Paused) is covered only piecewise.
  - Response: Accepted as adequate by composition rather than a new heavy rig.
    The nova_menu test drives the clear via the EXACT write teardown performs
    (`teardown_scenario_entities` sets `outcome.0 = None`, loader.rs:571); the
    teardown->None half is pinned by nova_scenario's
    `teardown_clears_the_declared_outcome`; and state_to_world_system's
    pause-independence is a property of the external `bevy_common_systems`
    PostUpdate registration (not locally regressible). A combined
    ScenarioLoaderPlugin+NovaMenuPlugin+GameEventsPlugin rig would be a heavier,
    less-faithful fixture for marginal gain. Left as-is.
- [x] R1.3 (MINOR) crates/nova_menu/src/lib.rs - the R1.7 rewrite
  (`esc_over_a_shown_outcome_never_raises_the_pause_overlay`) only asserts the
  outcome overlay's `GlobalZIndex` `is_some()`, dropping the old test's pin on
  the z VALUE / above-HUD relation that the overlay comment still claims.
  - Response: Fixed - the test now asserts the outcome overlay's `GlobalZIndex`
    is above the HUD chrome (`> 0`), restoring the value pin the rewrite
    dropped.
