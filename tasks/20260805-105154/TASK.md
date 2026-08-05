# Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot
- ACTIVITY: UNDERSTANDING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-093910

## Context

Replaces `20260724-082856`, which tried to refresh the shipped web imagery in
place. The shipped captures were deleted instead: every game-rendered PNG is
gone from `web/src/assets/` (only the authored `banner.png`, the five
`icon-*.png` and the font remain), so this task starts from an empty slate and
a full worklist rather than from "which of these 31 files is stale?".

The goal is not parity with what was deleted. The `screenshots/` examples are
rebuilt (`20260804-093910` reduced them to capture-only) and the shots they
pose are dull: default camera, no action, no lighting intent. Redo them so the
site's imagery reads as a game worth playing, one shot at a time.

The advisory coverage report is the input and the progress meter:

    python3 scripts/gen-web-screenshots.py --report

Its `capturable` class is this task's scope: 27 images. The `manual` (7 post-card
thumbnails) and `historical` (25 old-version news figures) classes are NOT in
scope and stay outstanding.

## Inputs

Every `capturable` gap, grouped by the example that can produce it - one group
is one working session, one image is one step.

| Producer | Images |
| --- | --- |
| `screenshot_combat` | `feature-combat.png`, `feature-hud.png`, `feature-autopilot.png`, `tutorial-combat-lock.png`, `tutorial-radar-lock.png` |
| `screenshot_ui` | `feature-editor.png`, `tutorial-menu.png`, `wiki-settings.png` (no manifest slot yet) |
| `screenshot_reel` | `feature-gravity.png`, `wiki-gravity.png`, `wiki-sections.png` |
| `screenshot_sections` | `wiki-section-hull.png`, `wiki-section-controller.png`, `wiki-section-thruster.png`, `wiki-section-turret.png`, `wiki-section-torpedo-bay.png` |
| `screenshot_juice` | `feature-juice.png` |
| `screenshot_orbit` | `tutorial-orbit.png` |
| `screenshot_nova_os` | `news-090-nova-os-terminal.png`, `news-090-nova-os-apps.png` (no manifest slots yet) |
| ALIASES (reuse today) | `wiki-combat.png`, `wiki-hud.png`, `wiki-flight.png`, `wiki-radar.png` - each wants its own framing, so each wants its own capture |
| No producer yet | `news-090-combat-readability.png`, `news-090-contextual-hud.png`, `news-090-scenario-campaigns.png` |

Deleted and deliberately not replaced: `devlog5-target-viewfinder.png` and
`devlog5-radar-stance-slots.png` (the composite). The site references neither -
the report listed them "shipped but never referenced" - so the COMPOSITES entry
goes away with them unless a page asks for it again.

## Steps

Draft, to be settled in planning. Deletion is done (`web/src/assets/` holds only
authored art); the report is green-field, so order is by producer, and inside a
producer one step per image: pose it, capture it, look at it at the page crop,
adjust the example's scene/framing, recapture. Never hand-fix a generated PNG.

- [ ] Decide the look: what makes a Nova shot cool (camera framing, action beat,
      lighting, HUD state). Write it down once so 27 shots agree.
- [ ] `screenshot_combat` - redo the example's beats, then per image:
      `feature-combat`, `feature-hud`, `feature-autopilot`,
      `tutorial-combat-lock`, `tutorial-radar-lock`.
- [ ] `screenshot_ui` - redo, then per image: `feature-editor`,
      `tutorial-menu`, `wiki-settings` (new: settings pane, needs a manifest
      slot).
- [ ] `screenshot_reel` - redo, then per image: `feature-gravity`,
      `wiki-gravity`, `wiki-sections`.
- [ ] `screenshot_sections` - redo, then one step per section image (hull,
      controller, thruster, turret, torpedo bay).
- [ ] `screenshot_juice` - redo, then `feature-juice`.
- [ ] `screenshot_orbit` - redo, then `tutorial-orbit`.
- [ ] `screenshot_nova_os` - redo, then `news-090-nova-os-terminal` and
      `news-090-nova-os-apps` (new manifest slots).
- [ ] Give each of the four ALIASES its own capture step and drop it from
      `ALIASES`: `wiki-combat`, `wiki-hud`, `wiki-flight`, `wiki-radar`.
- [ ] Decide the producer for the three remaining v0.9.0 news figures
      (`news-090-combat-readability`, `news-090-contextual-hud`,
      `news-090-scenario-campaigns`) - a new example or an existing one - then
      capture each.
- [ ] Package (`python3 scripts/gen-web-screenshots.py`), re-run `--report`, and
      open the rendered landing, tutorial, news and wiki pages.

## Definition of Done

- The coverage report lists zero `capturable` gaps; what remains outstanding is
  `manual` or `historical`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Every shipped game-rendered image names one declared producer example in the
  manifest; no image is an alias of another.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Each redone `screenshots/` example runs headless and writes its shots.
  (manual: run each example under Xvfb with `NOVA_SHOT_DIR=target/reel`)
- The owner accepts the new shots as cooler than the deleted set, at their
  actual page crop. (manual: inspect the locally rendered landing, tutorial,
  news and wiki pages)

## Notes

- Scope is the `capturable` class only. Post-card thumbnails (`manual`) and
  pre-v0.9.0 news figures (`historical`) stay outstanding on purpose.
- Framing fixes land in the example's scene/pose code, never on the PNG.
- Producers are capture-only (`20260804-093910`); `nova_probe` never enters this
  path (`20260802-120045` WONTDO).
- The packaging script and its `--report` flag already exist (`0ff077ff`); this
  task consumes them, it does not build them. Wiring the report into CI as a
  warning-only job is still unowned.
