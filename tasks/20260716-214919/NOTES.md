# Notes: pause the sim while the Victory/Defeat outcome frame is up

- TASK: 20260716-214919
- BRANCH: feature/pause-on-outcome

## What changed and why

The outcome overlay (task 20260716-125856) deliberately shipped WITHOUT a
clock pause: it drew on top of a still-live world. This task makes it freeze
the sim using the SAME mechanism the pause menu uses, rather than a second
pause path (the task's explicit direction).

The pause menu's freeze is: enter `PauseStates::Paused`, which fires
`OnEnter(Paused)` -> `pause_clocks` (pauses `Time<Virtual>` + `Time<Physics>`)
and gates the spaceship input/section SystemSets on `in_state(Unpaused)`.
Clocks alone are not enough - `SpaceshipInputSystems` run in `Update` and do
not consume time, so a held trigger would still spawn projectiles into the
frozen world (the reason the menu gates the sets too). So reusing
`PauseStates::Paused` is the correct "same mechanism".

Changes (all in nova_menu + one in nova_scenario):

1. `sync_outcome_pause` (nova_menu): a new system next to
   `sync_outcome_overlay`/`sync_outcome_cursor`, gated the same way
   (`in_state(Playing)` + `resource_exists::<CurrentOutcome>`). It mirrors
   `CurrentOutcome` into `PauseStates`: set -> `Paused`, cleared-while-Paused
   -> `Unpaused`. `CurrentOutcome` stays the single source of truth (same
   pattern as the overlay and cursor syncs).

2. `setup_pause_ui` guard: skip spawning the pause-menu panel when an outcome
   is set. The outcome enters `Paused` too, so `OnEnter(Paused)` would
   otherwise stack the pause panel under the outcome overlay. The outcome
   overlay is the modal here.

3. `toggle_pause` guard: ESC/Start is inert while an outcome is up. Without
   this, ESC would toggle `Paused -> Unpaused` and resume the sim behind the
   still-open overlay (the exact bug), or stack the pause menu over it.

4. `decide_advance` (nova_scenario/loader.rs): the scenario-advance input
   (Enter/DPadDown) previously ignored ALL paused states. Now
   `paused && !has_outcome -> Ignore`: a plain pause-menu pause still swallows
   Enter, but the outcome frame's own pause keeps Enter/Continue/Retry live
   (the overlay is a paused modal you act on).

## Old-element-survives decision (task direction bullet 4)

On Continue/Retry the queued switch releases, teardown clears
`CurrentOutcome`, and `sync_outcome_pause` unpauses on the next frame - the
new scenario plays unpaused. On Main Menu / Enter-to-menu the app leaves
`Playing`, where the existing `force_unpause` (OnExit Playing) already resets
the pause. No explicit unpause was added to the button handlers: the single
source of truth is `CurrentOutcome`, which every advance path clears.

## Deadlock check (why unpause is reliable)

The queued-scenario switch that clears `CurrentOutcome` is processed by
`GameEventsPlugin`'s `state_to_world_system` in `PostUpdate`, gated on
`not(is_queue_empty) OR resource_changed<W>` - NOT on pause. So even while
`Paused`, `release_lingering_next` mutates the event world, the switch fires,
teardown clears the outcome, and we unpause. No deadlock.

## Fixtures re-pinned (sibling-change / pin-the-fix-at-its-boundary)

Two existing tests encoded the OLD behavior and were rewritten to the new
contract (fail-first: they fail against the old code path this task removes):

- `advance_decision_table` (nova_scenario): asserted `paused -> Ignore` for
  all outcome values; now asserts the outcome case advances under pause.
- `outcome_overlay_sits_below_the_pause_overlay` (nova_menu, review R1.7):
  pinned the pause overlay stacking on top of the outcome via ESC. The two are
  now mutually exclusive; renamed to
  `esc_over_a_shown_outcome_never_raises_the_pause_overlay` and pins that.
  The R1.1 cursor test was likewise updated (ESC no longer cycles over the
  outcome; the outcome holds its own pause and the cursor stays free).

Stale comments swept: the outcome overlay's `GlobalZIndex(9)` rationale and
`restore_cursor`'s outcome-guard both described the now-impossible
ESC-over-outcome path; updated to say the guard/ordering is defensive.

## New tests

- `a_shown_outcome_freezes_the_sim_like_the_pause_menu`: across
  Victory(queued)/Defeat(queued)/Victory(unqueued) - baseline live (delivery
  guard), outcome -> Paused + both clocks frozen, clear -> Unpaused + clocks
  resume.
- `the_outcome_pause_does_not_spawn_the_pause_menu_panel`: no pause panel
  under the outcome; delivery guard = a plain ESC pause DOES spawn it.
- `a_shown_outcome_keeps_the_cursor_free_and_esc_cannot_regrab_it` and
  `esc_over_a_shown_outcome_never_raises_the_pause_overlay` (rewrites).

## Verification

- `cargo check --workspace --all-targets`: clean (only the pre-existing
  proc-macro-error2 future-incompat warning).
- `cargo fmt --all --check`: clean.
- Tests: full nova_menu lib suite (55 pass) + nova_scenario loader tests
  (run with `-p nova_menu` sibling per `crate-solo-tests-miss-unified-features`).
- No Xvfb eyeball: the freeze is the ABSENCE of motion (clocks stop), which is
  behavioral and fully covered by the clocks_paused assertions across
  variants; there is no new pixel to look at.
