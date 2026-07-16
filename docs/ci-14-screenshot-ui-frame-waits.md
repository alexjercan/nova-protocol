# CI fix: `14_screenshot_ui` never reached Playing

## Symptom

The `examples_smoke` test (`harnessed_examples_reach_playing_without_panic`)
passed locally but failed in CI on a single example:

```
example 14_screenshot_ui never reached Playing
```

Every other harnessed example passed. The stderr tail showed the run stayed in
`MainMenu` for its whole life: the `menu_ambience` backdrop loaded, the menu
orbiter logged "held orbit for 5s" twice, and no editor scenario ever loaded.
The autopilot still logged `cycle complete, no panic (t=14.0s)` - it just never
left the menu.

## Root cause

`14_screenshot_ui` drives the shipped app through its real state machine:
`Loading -> MainMenu -> Playing`. The autopilot (`AutopilotPlugin`) only forces
the *first* state (`Loading`); the menu->editor transition happens because the
example's `ui_capture_script` clicks the "Sandbox Button". Reaching `Playing`
therefore depends on that click firing inside the fixed-seconds autopilot window.

The script paced its beats with **frame-count** waits (`state.wait = 90`, then
`20`, then click) so a screenshot beat could settle before capture. Those waits
ran on the capture-less smoke path too, where they serve no purpose. On a fast
local GPU 110 frames pass in ~2s. On CI's `llvmpipe` software renderer the menu
loads its GLB backdrop at roughly 7 fps (the log shows entities spawning ~135ms
apart), so 110 frames overran the 14s window and the Sandbox click never fired.
The frame-count waits made the walk's wall-clock cost scale with frame rate,
which the fixed-seconds window does not tolerate.

## Fix

`examples/14_screenshot_ui.rs`:

- Make the settle waits conditional on `capturing` (`BCS_REEL`). The capture
  path keeps its generous 90/20/30-frame settles; the smoke path uses minimal
  waits (6 / 0 / 6) - just enough for the next button to spawn and the state
  transition to apply - so the menu -> editor -> Playing walk is short in
  *frames* and fits the window regardless of frame rate.
- Widen `UI_AUTOPILOT_SECS` from 14 to 20 for headroom on slow CI GPUs. The
  reduced waits do the real work; the wider window is cheap insurance.

## Verification

Reproduced CI's environment locally with software rendering (`Xvfb` + llvmpipe).
Before: the smoke path burned the window in the menu. After: it reaches
`nova harness: reached Playing` at ~5.5s of autopilot elapsed, loads the editor,
builds the ship (`preview_section` fires), completes the cycle with no command
errors, and exits `AppExit::Success`.

## Lesson

Autopilot/harness beats that must finish inside a fixed-seconds window should be
paced by elapsed time or state, not by frame counts. Frame-count waits couple
wall-clock cost to frame rate, so a slow CI GPU silently blows the budget while
a fast local GPU hides it.
