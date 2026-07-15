# Annotate the HUD and radar wiki screenshots with callout labels

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,docs,web,assets

Split out from task 20260715-224013 (player docs front door). The player docs
task delivered the getting-started page, glossary, tutorial surfacing and jargon
trim, but deferred the optional image-annotation step because it needs image
tooling, not prose.

## Goal

Make the visual player pages self-explanatory: overlay callout labels on the two
figure screenshots so a reader can map the prose to what they see.

## Steps

- [ ] Annotate `web/src/assets/wiki-hud.png` with callout labels for the named
      instruments (velocity sphere, speed/mode chips, ORBIT ring, keybind hint
      cluster) so `web/src/wiki/hud.md` is self-explanatory.
- [ ] Annotate `web/src/assets/wiki-radar.png` with callout labels (sweep box,
      combat vs nav lock, fine-lock section marker) matching
      `web/src/wiki/targeting-radar.md`.
- [ ] Keep the annotation style consistent between the two images (font, arrow
      style, color that reads on the dark HUD). Plain ASCII labels only.
- [ ] Verify: `npm run ci` green; serve + eyeball that both figures render with
      legible labels at the wiki figure size and on the deploy subpath.

## Notes

- The screenshots already exist and auto-upgrade via `site.ts`; this task only
  overlays labels, it does not re-capture. If re-capture is easier than editing
  the PNGs, that is acceptable as long as the labels land.
- Low priority: the pages read fine without labels; this is a polish pass.
