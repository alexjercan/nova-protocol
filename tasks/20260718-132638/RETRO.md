# Retro: Render-scale Low breaks UI clicks

- TASK: 20260718-132638
- BRANCH: fix/render-scale-clicks (squash-landed as master fb8dc63c)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Root-caused in minutes from bevy source, not guesswork: `ui_focus_system`
  (`focus.rs:193`) resolves a cursor only for `Window`-targeted cameras, so the
  image-targeted UI camera the first cut shipped could never be clicked.
- The `ImageRenderTarget.scale_factor` trick kept the fix contained - the HUD
  moved to the window camera without touching any HUD projection code, because
  the scenario camera can be made to report window-logical coordinates while
  rendering fewer pixels.
- Added a regression test that pins the real invariant (UI camera targets a
  window), and re-shot the scene to confirm crisp HUD + aligned indicators over a
  soft world.

## What went wrong

- The original render-scale review (including a multi-agent code-review pass)
  verified RENDERING (screenshots looked correct) but never verified
  INTERACTION. An image-targeted UI camera renders its nodes perfectly and is
  completely unclickable - a screenshot cannot tell the two apart. Root cause: I
  treated "the frame looks right" as "the UI works", and the whole-frame-into-one-
  image design's headline benefit ("one coordinate space, projection just works")
  masked that it had quietly broken picking. The user found it on the first
  click.

## What to improve next time

- For any change that reparents/retargets UI or touches the camera a UI is on,
  verify a CLICK, not just a render. If a click can't be automated headlessly,
  say so explicitly and flag it for a human re-test - do not let a screenshot
  stand in for interaction.
- Treat "image-targeted UI camera" as a known trap: it renders but never picks.

## Action items

- [x] Regression test: UI camera must target a window on Low.
- [x] Corrected the design docs (settings, design log, report, CHANGELOG) from
  "whole frame incl HUD" to "world in image, UI on window".
