# Capture round: shoot all six accepted scenes, package, and review at page crop

- PRIORITY: 65
- TAGS: v0.10.0,screenshot,web,assets
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749, 20260805-112819, 20260805-112841, 20260805-112903, 20260805-112929, 20260805-112951

## Context

Round two of the refresh (`20260805-105154`): the six scenes are accepted by the
owner, so now they get shot. Nothing is designed here - this task runs the
producers, packages the output, and looks at the result on the real pages. Any
framing complaint goes back into the producing example's beat and is recaptured;
a generated PNG is never hand-fixed.

Depends on all six scene tasks.

## Steps

- [ ] Capture each producer into staging on a real GPU (windowed, `NOVA_SHOT_DIR=target/reel`):
      `screenshot_scene`, `screenshot_flight`, `screenshot_combat`,
      `screenshot_sections`, `screenshot_ui`, `screenshot_nova_os`.
- [ ] Package: `python3 scripts/gen-web-screenshots.py`, and read what it
      reports as pending or wrong-shaped.
- [ ] Review every image AT ITS PAGE CROP, not full-frame: the site sizes
      figures `aspect-ratio: 16/9; object-fit: cover`, so the edges of a shot
      are not what a reader sees.
- [ ] For each rejected framing: fix the producing example's beat, recapture
      that shot alone, re-review. Never edit the PNG.
- [ ] Commit the accepted PNGs into `web/src/assets/`.
- [ ] Re-run the report and open the rendered site: landing, tutorial, news and
      the affected wiki pages.

## Definition of Done

- The coverage report lists zero `capturable` gaps; everything outstanding is
  `manual` or `historical`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Every shipped game-rendered image names one declared producer and no image is
  an alias of another.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The packaging run reports nothing pending and nothing wrong-shaped.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py`)
- The owner accepts the rendered pages: landing, tutorial, news and the affected
  wiki pages carry intentional crops, and any remaining placeholder is `manual`
  or `historical`. (manual: inspect the locally rendered website)

## Notes

- Capture needs a display and a GPU; headless is Xvfb + lavapipe.
- The `manual` post-card thumbnails (7) and `historical` news figures (25) stay
  outstanding on purpose - they are not this task's scope and their placeholders
  are expected to remain.
- Wiring `--report` into CI as a warning-only job is still unowned; it is not
  part of this task.
