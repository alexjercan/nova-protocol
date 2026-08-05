# Retro: Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- TASK: 20260805-105154
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

- Scene-at-a-time with an owner gate between scenes. Every scene was RUN plainly
  and shown before the next was started, and no capture ran until all six had
  passed. Nothing had to be recaptured for framing at the end.
- Shots assert the state they claim. `screenshot_ui` checks the Settings panel
  is laid out, that the clicked chapter carries `Selected`, and that five
  sections sit on the preview ship - a missed click would otherwise leave the
  PREVIOUS state on screen and still look plausible.
- The advisory report was the progress meter. `capturable` went 29 -> 0 and told
  us when the task was done, without ever gating a build.
- Framing lives in the example. Every fix landed in a beat's pose or scene, so
  the whole set is reproducible from a clean tree.

## What went wrong

- The editor build placed one section instead of four, silently. The free-fly
  WASD controller rewrites the camera Transform every frame (removing the
  component does not stop it - its private state survives), so a one-shot pose
  was gone by the next frame, and from the editor's default dead-on pose every
  SIDE face is edge-on, so each placement landed on the face the ORIGINAL camera
  saw. Fixed by pinning with `ScriptedCameraPose`. It cost three diagnostic
  passes because the count assert said "wrong number", not "wrong face"; the
  per-gesture POSITION log is what named it.
- Four fidelity captures had been shipping to nobody. `nova-os-welcome/active/
  map/ship` were referenced in no page, only in a closed task's records. Nothing
  in the pipeline reports a manifest entry with no reader - the report only
  looks the other way, from page reference to asset.

## What to improve next time

- When a scripted walk drives a controller-owned Transform, pin it; do not set
  it. The pattern is already documented in `crates/nova_scenario/src/actions/
  view.rs`, and reading that first would have saved the three passes.
- Make a failing gesture assert say WHICH gesture. A final count is a poor
  witness; the per-step count log added mid-debug should have been there from
  the start.

## Action items

- [ ] File the screen-indicator label collisions: WAYPOINT over its own distance
      readout (`wiki-radar`, `tutorial-radar-lock`), FLIP under the blip
      (`feature-autopilot`), SURVEY over the debug fps/version chip
      (`wiki-flight`). In-game HUD layout, not a capture problem.
- [ ] File the keybind-row tofu box between the keyboard and gamepad columns,
      seen in `wiki-settings` and in game.
- [ ] Wiring `--report` into CI as a warning-only job is still unowned (carried
      from this task's Notes).
