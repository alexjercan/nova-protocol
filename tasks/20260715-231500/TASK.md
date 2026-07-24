# Annotate the HUD and radar wiki screenshots with callout labels

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: docs, web, assets, backlog

## Closed (2026-07-24, folded into the frontend-images refresh)

Closed during v0.9.0 planning triage. The stale wiki-hud/wiki-radar captures and
their annotation are folded into a single consolidated frontend-app image refresh
task (missing + stale images across web/). See that task.

## Story

As a new player reading the HUD and radar wiki pages, I want the screenshots
labeled so I can map each named instrument in the prose to the thing on the
image, so that the visual pages explain themselves without cross-referencing
paragraphs against an unlabeled picture.

Split out from task 20260715-224013 (player docs front door). The player docs
task delivered the getting-started page, glossary, tutorial surfacing and
jargon trim, but deferred the optional image-annotation step because it needs
image tooling, not prose.

## Steps

- Annotate `web/src/assets/wiki-hud.png` with callout labels for the named
      instruments (velocity sphere, speed/mode chips, ORBIT ring, keybind hint
      cluster) so `web/src/wiki/hud.md` is self-explanatory.
- Annotate `web/src/assets/wiki-radar.png` with callout labels (sweep box,
      combat vs nav lock, fine-lock section marker) matching
      `web/src/wiki/targeting-radar.md`.
- Keep the annotation style consistent between the two images (font, arrow
      style, color that reads on the dark HUD). Plain ASCII labels only.
- Verify: `npm run ci` green; serve + eyeball that both figures render
      with legible labels at the wiki figure size and on the deploy subpath.

## Definition of Done

- Both figures carry legible, consistent callouts for every instrument their
  page's prose names, readable at the rendered wiki size.
- If re-capture was needed, the new screenshots still match what the current
  build actually shows (v0.7.0 HUD: check the ammo gauge and RCS-violet
  velocity sphere states are not accidentally in frame contradicting the
  prose).

## Notes

- The screenshots already exist and auto-upgrade via `site.ts`; this task only
  overlays labels, it does not re-capture. If re-capture is easier than
  editing the PNGs, that is acceptable as long as the labels land.
- Consider `scripts/gen-web-screenshots.py` if re-capturing; keep source
  screenshots reproducible.
- Low priority: the pages read fine without labels; this is a polish pass.

## Re-scope (2026-07-21): needs re-capture first, not just an overlay

Finding when picked up: the existing screenshots are STALE and cannot carry the
callouts the prose names, so this is no longer a pure label-overlay task:
- `web/src/assets/wiki-hud.png` shows the **v0.5.2** HUD (version chip in frame),
  predating v0.7.0's ammo gauge + RCS-violet velocity sphere the DoD says to
  check for.
- The instruments to label are CONDITIONAL and are not in the current frame:
  `hud.md` describes the ORBIT ring "only while you hold an orbit", the mode chip
  ("AP GOTO - BURN") "only while the autopilot is engaged", and the CYAN sphere
  "when the autopilot is flying". The shot has no active orbit / engaged
  autopilot, so those instruments cannot be labeled on it.
- `gen-web-screenshots.py` only PACKAGES staged captures; the capture itself is
  game-render work (the screenshot harness in the right scene state).

Revised approach (do these in order when picked up):
- RE-CAPTURE `wiki-hud.png` at the current build in a scene state that shows
      every named instrument at once: manual + autopilot states as needed
      (velocity sphere, speed + mode chips with the autopilot engaged, the ORBIT
      ring while orbiting, the keybind cluster). May need 1-2 shots or a staged
      capture; keep the source reproducible via the screenshot harness.
- RE-CAPTURE `wiki-radar.png` at the current build showing the sweep box, a
      combat vs nav lock, and a fine-lock section marker.
- THEN annotate both with consistent ASCII callouts (ImageMagick is
      available in the devshell; 1920x1080 source) and verify `npm run ci` +
      eyeball at the rendered wiki figure size.

Left OPEN at P20 (optional polish; the pages read fine without labels). The
re-capture is the real cost - a text agent cannot reliably drive the game to the
exact HUD states and judge label placement blind, so this suits a session that
can iterate on the rendered game or a human with the capture harness. (User
call 2026-07-21: re-scope + defer.)
