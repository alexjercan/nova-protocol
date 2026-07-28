# REVIEW: RMB-only orbit-drag in map + ship apps

Round 1, out-of-context reviewer (general-purpose agent), branch `fix/rmb-only-orbit`.

Scope: two-file bug fix removing the LMB clause from the mouse orbit-drag gate in
the `map` and `ship` NOVA OS apps, plus one pinning test per app.

## Correctness

- The fix drops `|| mouse_buttons.pressed(MouseButton::Left)` from both orbit
  gates (`nova_os_map.rs`, `nova_os_ship.rs`), leaving RMB-only, so an LMB
  press-with-motion no longer moves the camera and the blip `Button` widget's
  Primary activation lands.
- Grep confirms no remaining LMB orbit path and no other `MouseButton::Left`
  handling in these apps; RMB is used only for orbit, so no new conflict.
- Keyboard orbit (Q/E/R/F), wheel zoom, WASD pan, `[`/`]` cycle, and ship `T`
  reset are untouched. Minimal, correct.

## Test quality

- Both tests drive the real systems via `run_system_once` with genuine
  `ButtonInput<MouseButton>` state and real `MouseMotion` messages against a
  camera actually spawned with its orbit component; `PauseStates::NovaOs` +
  `enter_app` satisfy the active guard so the orbit body executes (not vacuous).
- Reversion-sensitive both ways: the LMB `assert_eq(before)` would FAIL under the
  old `|| pressed(Left)` code; the RMB `assert_ne(before)` would FAIL if RMB
  orbit were broken.

## Findings

No BLOCKER/MAJOR/MINOR issues.

- NIT (declined): tests assert RMB changes the angles but not the delta
  magnitude/sign. Not needed for this bug fix.

- VERDICT: APPROVE
