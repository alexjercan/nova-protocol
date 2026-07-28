# RETRO: RMB-only orbit-drag in map + ship apps

## What happened

Playtest report: clicking a blip in the `map`/`ship` NOVA OS apps often failed to
select it. Root cause: LMB was bound to TWO things on the same surface - the blip
`bevy_ui` `Button` widget (Primary click -> `Activate` -> select) AND the camera
orbit-drag. Any press with a few pixels of motion was consumed as an orbit drag,
which moved the camera, slid the blip out from under the cursor, and killed the
click. Fix: gate orbit-drag on `MouseButton::Right` only in both input systems.
One clause removed per file, plus a mirrored pinning test per app.

## What went well

- Reproduced at the input-system altitude before touching the fix: two live-tree
  tests (`ship_orbit_drag_is_rmb_only`, `map_orbit_drag_is_rmb_only`) driving the
  real `*_input` systems with real `MouseMotion` + `ButtonInput`, RED first
  (LMB moved the angles), GREEN after. Reversion-sensitive in both directions
  (LMB asserts no-orbit, RMB asserts orbit).
- The diagnosis named the exact mechanism (the `Button` widget's Primary
  activation is LMB) rather than guessing, so the fix was aimed and one-line.
- Both apps fixed symmetrically in the same pass - no half-fix for a reviewer to
  catch (kin of `pin-each-caller-not-just-shared-core`).

## What went wrong / friction

- First instinct was to `cargo test ... | tail`, which eats the exit code
  (AGENTS.md warns about exactly this); switched to writing output to a file and
  reading it. Also `run_in_background` + a poll loop was clunky for what is a
  60s compile; fine, but the pipe-swallows-exit trap is the real note.

## Lesson candidate

- `one-pointer-button-cant-both-activate-a-widget-and-drag-the-world`: when the
  same mouse button drives a `bevy_ui` `Button`/widget activation AND a
  camera/world drag on the same surface, they fight - a click-with-motion becomes
  a drag and the widget never activates. Reserve the widget's button (LMB/Primary)
  for the widget; put drag on RMB (or require a modifier). Test at the input
  system: hold the button + send a `MouseMotion`, assert the drag target did NOT
  move; hold the drag button, assert it did.

## Improve next time

- When adding a mouse-drag to any app that also has clickable UI widgets on the
  same viewport, default drag to RMB from the start.
- Reach for the file-redirect-then-grep pattern for build/test output immediately,
  not after hitting the pipe-eats-exit-code gotcha.
