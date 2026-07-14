# Finish the last 4 web screenshots: devlog5-radar-stance-slots composite + thumb-devlog-3/4/5

- STATUS: OPEN
- PRIORITY: 25
- TAGS: v0.6.0,example,screenshot,web

Follow-up from the screenshot-showcase pipeline (task 20260714-210131). 22 of the
26 web-referenced screenshots are now generated in-engine, packaged by
`scripts/gen-web-screenshots.py`, and wired live on the site (figures upgrade
placeholder -> `<img>` in `web/src/site.ts`, wiki icons in `web/src/wiki.ts`).
These 4 were deliberately left; each needs a decision or a dependency, not more
in-engine capture tech.

The site references these under `web/src/assets/`; the packaging script already
lists them (the composite as pending, the thumbnails in `THUMBNAILS`). Drop a
file into the stage dir (`target/reel`, or `--stage-dir`) and re-run the script
and it is copied in; a distinct capture always wins over an alias.

## 1. devlog5-radar-stance-slots.png (16:9)

A side-by-side comparison: the weapons-lowered NAV crosshair (white) next to the
weapons-raised combat reticle (red), each mid-sweep. Both halves already exist as
captures:
- left  = `tutorial-radar-lock.png` (nav sweep on the beacon), from
  `examples/15_screenshot_combat.rs`.
- right = `feature-combat.png` (combat lock), same example.

It is a **composite** (scale each to half width, place side by side into one
1920x1080). Blocked in this environment: `gen-web-screenshots.py` is stdlib-only
(the project convention, matching `gen-placeholder-sounds.py`) and there is no
Pillow/numpy/cairo installed to resize+composite. Options:
- Add an optional Pillow path to the script (documented dependency), guarded so
  the stdlib path still runs without it; compose the two shots.
- Or author the composite once by hand and commit it.
- Or capture it directly: a bespoke example that renders both stances in one
  frame (two RTT viewports), heavier than it is worth.

## 2. thumb-devlog-3/4/5.png (16:9, ~300px display)

Devlog post-card thumbnails - these are content choices tied to each devlog's
subject, not a generic capture:
- devlog 3: zones, torpedoes, blast damage -> reuse `feature-juice.png` (a section
  blown off) is a good fit.
- devlog 4: guided torpedoes, targeting, an enemy that fights back -> reuse
  `feature-combat.png` / `tutorial-combat-lock.png`.
- devlog 5: radar locking, tutorial, the web site -> reuse `tutorial-radar-lock.png`
  or `tutorial-menu.png`.

Easiest: add them to the `ALIASES` map in `gen-web-screenshots.py` pointing at the
chosen source (they are 16:9 already, displayed at 300px, so no resize needed).
Decide the mapping with whoever owns the devlog content, then alias.

## Steps

1. Decide the thumb-devlog-3/4/5 -> source mapping; add to `ALIASES` in
   `scripts/gen-web-screenshots.py`; re-run the script; the site picks them up.
2. For `devlog5-radar-stance-slots`: either add a guarded Pillow compose step to
   the script (compose `tutorial-radar-lock` + `feature-combat` side by side) or
   commit a hand-made composite into `web/src/assets/`.
3. Re-run `python3 scripts/gen-web-screenshots.py`; confirm 0 pending; check the
   pages render the images (`cd web && npm run build`, hard-refresh).
