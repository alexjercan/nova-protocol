# Write the v0.12.0 news post and capture its media

- STATUS: OPEN
- PRIORITY: 15
- TAGS: v0.12.0,docs,web,capture,release

## Goal

Write the v0.12.0 News post and capture the media it names, to the standard
`web/src/news/0.11.0.md` set: a narrative lead, `##` feature sections, live
`data-widget` explainers, and a figure for every claim worth seeing - with
**every figure resolving to a real captured asset**. v0.11.0 shipped 16
figures over 12 assets and 8 widgets, and all 12 assets exist.

Owner's ask: "first step as always is to create the news post; make sure to
include visuals, gifs, screenshots etc; if we need to add new examples in the
screenshots to capture more interesting things go ahead and do that too".

## The release

v0.12.0 is the editor release. 130 `[Unreleased]` entries; the Interface & HUD
group alone holds ~67. The story is that the editor stopped being a ship-part
placer and became a scenario authoring tool whose output is an ordinary mod.
Secondary arcs: the controls became rebindable and gamepad-complete, and combat
VFX became credible in vacuum.

## Done when

- `web/src/news/0.12.0.md` written, sourced from the cycle's CHANGELOG, not
  restating it.
- Registered in `web/webpack.config.js` `NEWS_POSTS` and carded in
  `web/src/news.html`; `assets/thumb-news-0.12.0.png` exists.
- Every `.figure__placeholder-name` in the post resolves to a file under
  `web/src/assets/`.
- New capture examples added under `examples/screenshots/` where an existing
  one cannot show the thing, and wired into `scripts/capture-web-media.sh`
  (loops) or `scripts/gen-web-screenshots.py` (stills).
- `cd web && npm run ci` green.

## Notes

- The `.figure__placeholder` block IS the embed: `site.ts` `upgradeFigures`
  swaps it for a real `<video>`/`<img>` once the named asset exists, and
  leaves the placeholder when it 404s. So the post is authored the same way
  either side of the capture.
- Baseline is v0.11.0. Anything added and revised inside the cycle gets one
  description - where it ended up. Anything added and removed never happened.

## Media captured

Stills (`python3 scripts/gen-web-screenshots.py` after running
`screenshot_editor`, `screenshot_menu`, `screenshot_section_frame` armed):
`feature-editor.png`, `feature-editor-events.png` (new FIGURE),
`wiki-sandbox-range.png`, `wiki-controls.png`, `wiki-section-thruster.png`,
plus `thumb-news-0.12.0.png` as an alias of `feature-editor.png`. Every editor
asset on the site was v0.11.0-era - the old single-rail editor with `[SOON]`
chips - so the refresh is the substance of the media work, not a nicety.

Loops (`nix develop -c scripts/capture-web-media.sh`, full fresh capture at
`d11bfb04`): `news-0120-release-lead`, `news-0120-editor-events` (new LOOP in
`screenshot_editor`), `news-0120-cold-launch` (new LOOP in `loop_vfx_range`),
`news-0120-point-defense` and `news-0120-blast` (aliases of the re-captured
`news-0110-point-defense` and `torpedo-blast`, which now carry the new VFX).

## Open at release time

The menu-backed stills print the workspace version in their corner, so
`wiki-controls.png`, `wiki-settings.png` and `tutorial-menu.png` read `v0.11.0`
until the release bump lands. Re-run `screenshot_menu` and
`gen-web-screenshots.py` after step 2 of "Cutting a release".

## Rejected

- A figure on the wide `vfx-range` loop. The pose is sized for the frame-time
  capture - both ships plus the ejecta thrown past the target, at a little over
  one RANGE out - so the ships are 60 px blocks in a black frame. Re-framing it
  would move the measurement baseline the release reports. The loop is no
  longer packaged; the three close figures carry the combat section instead.
- `news-0120-world-objects.png` and `news-0120-inspector.png`. Nothing captured
  could fill them honestly, and every figure has to resolve.
