# Refresh frontend app images: fill missing + re-capture stale screenshots across web/

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,web,assets,screenshot

## Story

As a visitor to the nova-protocol web app (landing, devlog/news, wiki), I want
the images to be present and current, so the site does not fall back to
placeholders or show screenshots from old builds that no longer match the game.

Consolidates two closed tasks - 20260715-092658 (devlog/news thumbnails) and
20260715-231500 (stale wiki-hud/wiki-radar captures) - into one image-refresh
pass, plus a general sweep for other missing/stale images the user has noticed.

## Steps

- [ ] Inventory: enumerate every image referenced under `web/src/` (assets,
      thumbnails, wiki figures, landing hero) and mark each present / missing /
      stale (captured from an old build / wrong version chip in frame).
- [ ] Reconcile the thumbnail naming drift: site references
      `thumb-news-0.3.0/0.4.0/0.5.0/0.6.0.png` while
      `scripts/gen-web-screenshots.py` still lists `thumb-devlog-3/4/5.png`.
      Pick the canonical scheme, add the missing 0.6.0 (and any newer) thumb.
- [ ] Re-capture stale screenshots at the current build via the screenshot
      harness: `wiki-hud.png` (v0.5.2 in frame - needs a scene showing velocity
      sphere, speed+mode chips with autopilot engaged, ORBIT ring while orbiting,
      keybind cluster) and `wiki-radar.png` (sweep box, combat vs nav lock,
      fine-lock section marker). Keep sources reproducible.
- [ ] Fill genuinely missing images (the ones the user flagged) - source or
      capture each and wire it in.
- [ ] Verify: `cd web && npm run ci` green; serve and eyeball that no card or
      figure falls back to a placeholder and no screenshot shows an old version
      chip.

## Definition of Done

- No image reference under `web/src/` resolves to a placeholder (cmd: grep the
  gen-web-screenshots pending report shows 0 pending).
- `wiki-hud.png` / `wiki-radar.png` match the current build's HUD (manual: no
  stale version chip in frame; named instruments visible).
- Thumbnail naming is consistent between the site and the gen script (cmd:
  `npm run ci`).

## Notes

- The user will point at the specific missing images; step 1's inventory should
  surface them and any others.
- Re-capture is game-render work (screenshot harness in the right scene state),
  not a pure packaging job - `gen-web-screenshots.py` only packages staged
  captures. Best done in a session that can drive the game / with a human on the
  harness.
- ImageMagick is available in the devshell for any overlay/annotation.
