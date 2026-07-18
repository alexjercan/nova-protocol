# Annotate the HUD and radar wiki screenshots with callout labels

- STATUS: OPEN
- PRIORITY: 20
- TAGS: docs,web,assets,v0.8.0

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

- [ ] Annotate `web/src/assets/wiki-hud.png` with callout labels for the named
      instruments (velocity sphere, speed/mode chips, ORBIT ring, keybind hint
      cluster) so `web/src/wiki/hud.md` is self-explanatory.
- [ ] Annotate `web/src/assets/wiki-radar.png` with callout labels (sweep box,
      combat vs nav lock, fine-lock section marker) matching
      `web/src/wiki/targeting-radar.md`.
- [ ] Keep the annotation style consistent between the two images (font, arrow
      style, color that reads on the dark HUD). Plain ASCII labels only.
- [ ] Verify: `npm run ci` green; serve + eyeball that both figures render
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
