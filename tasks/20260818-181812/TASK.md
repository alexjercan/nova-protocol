# Wiki visuals pass: fill capture slots and replace text walls

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.11.0,docs,web,capture

Replace the wiki's remaining walls of text with visuals: diagrams (widgets),
screenshots, and webm loops.

Epic: `20260818-220812`.

## Wait on

Carving LANDED (`0ee9cbb0`), so the appearance gate is cleared and the re-shoot
pass on existing captures is now due. Two things still gate the capture itself:

- `20260818-221021` - the sandbox is unplayable at the current head. A capture
  from a stuttering build records the stutter.
- `20260818-221044` - the destruction pages get rewritten. Capturing a page
  that is about to change is a wasted capture, so shoot after the words settle.

Nothing else is waiting. The widget work below can start now.

## What already exists

- Widget layer: `data-widget` blocks in wiki/create .md, hydrated by
  `web/src/widgets.ts` (flagship bar: the controller stacking curve).
- Figure auto-light contract: a `figure__placeholder` naming an asset under
  `web/src/assets/` swaps to the real img/video when the file exists. No HTML
  edit needed.
- Loop pipeline: `loop_start`/`loop_end` in nova_autopilot, encoded to 720p30
  VP9 webm; `scripts/capture-web-media.sh` packages into
  `web/src/assets/loops/`. Fails loudly on unclosed loops / frame cap / ffmpeg.
- Still pipeline: `scripts/gen-web-screenshots.py` + capture examples.

## Slots already authored and waiting

- `assets/loops/loop-section-{hull,controller,thruster,turret,torpedo-bay}.webm`
  (section pages)
- `assets/loops/goto-arrival.webm` (flight-autopilot)
- `assets/loops/lock-dwell.webm` (targeting-radar)
- `assets/wiki-sandbox-range.png` (getting-started)
- `assets/wiki-scenarios-picker.png` (scenarios)
- `assets/catalog-*.png` variant thumbs (section catalog tables)
- older `assets/wiki-section-*.png` figure slots

## Scope

1. Fill every authored slot with a real capture.
2. Sweep every wiki/create page for prose a visual replaces better; add the
   figure/loop (or widget) and fold or cut the prose.
3. Re-shoot existing captures invalidated by the damage/carving rework.
4. Keep each loop <= 3 MB; deterministic capture examples so re-shoots are
   one script run.
