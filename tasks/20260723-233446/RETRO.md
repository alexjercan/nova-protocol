# Retro: HUD allegiance markers

- TASK: 20260723-233446
- BRANCH: feat/allegiance-markers
- REVIEW ROUNDS: 1 (APPROVE, no MAJOR)

## What went well

- Two load-bearing engine assumptions were verified in SOURCE before any code
  was written, and both paid off:
  - The CSS-border-triangle look depends on Bevy's UI border shader mitering
    per-side wedges AND on `BoxSizing`. Reading `bevy_ui_render`'s
    `nearest_border_active` + `extract_uinode_borders` confirmed the miter,
    and `bevy_ui`'s `Node` default (`BoxSizing::BorderBox`) revealed that a
    0x0 node would collapse its borders - so `ContentBox` is mandatory. Caught
    at design time, not after a blank-screen run.
  - Tracing the real ship-spawn path (`nova_scenario` `insert_spaceship_sections`
    + the `#[require]` defaults on the controller markers) showed that
    `Allegiance` and `PlayerSpaceshipMarker` both land via a DEFERRED command
    AFTER this module's `Add<SpaceshipRootMarker>` observer runs. That
    falsified the plan's "read allegiance + skip player in the observer" step
    before it was built.
- The out-of-context reviewer independently re-derived both facts from the
  same sources and found zero MAJORs - one review round. The up-front source
  reading is exactly what made the review cheap.
- The App-driven test spawns the player via its real `#[require]` marker, which
  reproduces the deferred-spawn ordering, so it is a genuine delivery guard: an
  `Add`-observer regression on the player-skip would leave the player marked
  and fail the test.

## What went wrong

- The plan's Steps prescribed a mechanism that was racy: "on
  `Add<SpaceshipRootMarker>` -> colour from the ship's Allegiance at spawn;
  skip the player." Root cause: the plan named a spawn-time read without
  checking WHEN `Allegiance`/`PlayerSpaceshipMarker` actually become visible on
  the entity. They are `#[require]` defaults on the controller markers,
  inserted by a deferred command inside nova_scenario's own root-add observer,
  so they are absent at sibling-observer time. The redesign (grey-then-recolour
  via `Changed<Allegiance>`; player-skip via an `Added<PlayerSpaceshipMarker>`
  SYSTEM that defers past the spawn-command flush) was found during work, not
  planning.

## What to improve next time

- When a plan step reads a component inside an `Add<Marker>` observer, verify
  that component's INSERT TIMING (is it in the same bundle, a `#[require]`
  default, or a deferred command from another observer?) at plan time - the
  same discipline `verify-engine-guarantees-in-source` already asks for
  observer/ordering behaviour, extended to component availability.
- The triangle's visual rides an undocumented shader side effect and has no
  rendered evidence yet. Capture a one-frame screenshot when the perf probe
  (task 20260723-233453) runs, to close the manual DoD item with a picture.

## Action items

- [x] Both review MINORs addressed on the branch (recolor `break`; AI-hostile
      test case) - see REVIEW.md post-review note.
- [ ] Manual/visual acceptance (fly a mixed-allegiance scenario; confirm the
      triangle renders as a filled down-triangle and friend/foe reads at a
      glance) - batched for the user; pairs with the perf `cmd:` check on
      task 20260723-233453.
- [x] Lessons ledger updated (border-triangle + require-timing domain lessons;
      `verify-engine-guarantees-in-source` bumped).
