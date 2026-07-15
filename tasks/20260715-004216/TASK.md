# Generate the devlog5-radar-stance-slots composite screenshot

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: v0.6.0, example, screenshot, web

Follow-up from the screenshot-showcase pipeline (task 20260714-210131). 22 of the
26 web-referenced screenshots are generated in-engine, packaged by
`scripts/gen-web-screenshots.py`, and wired live on the site (figures upgrade
placeholder -> `<img>` in `web/src/site.ts`). This task finishes the one remaining
*composite* shot. The 3 devlog post-card thumbnails were split into task
20260715-092658 (deferred: their source images/choices are not decided yet).

## devlog5-radar-stance-slots.png (16:9, 1920x1080)

A side-by-side comparison for the devlog-5 post
(`web/src/posts/devlog-5-radar-locking-shakedown-and-the-web.html`): the
weapons-lowered NAV crosshair (white) next to the weapons-raised combat reticle
(red), each mid-sweep. The figure caption reads "white NAV lock lowered, red
combat lock raised", so left = NAV, right = combat. Both halves already exist as
committed captures under `web/src/assets/`:
- left  = `tutorial-radar-lock.png` (nav sweep on the beacon), from
  `examples/15_screenshot_combat.rs`.
- right = `feature-combat.png` (combat lock), same example.

It is a **composite**: scale each 1920x1080 source to half width (960x1080) and
place them side by side into one 1920x1080.

DECIDED (of the original options - guarded Pillow path, hand-author, or a bespoke
two-viewport capture): compose it in-script with a **stdlib PNG codec**. The
project convention is stdlib-only Python (matching `gen-placeholder-sounds.py`);
adding a Pillow dependency contradicts that, hand-authoring is not reproducible,
and a bespoke capture example is heavier than the shot warrants. A small
stdlib decode + area-downsample + compose is correct, reproducible, and adds no
dependency. A distinct capture dropped into the stage dir still wins over the
generated composite (same precedence as an alias).

## Steps

1. Add a stdlib PNG decoder to `scripts/gen-web-screenshots.py` (parse IHDR +
   IDAT, zlib-inflate, reverse the per-scanline filters; handle 8-bit RGB and
   RGBA, non-interlaced - the two shapes Bevy's `save_to_disk` and the icon
   encoder produce).
2. Add an area-average (box) downscaler and a side-by-side composer; wire a
   `COMPOSITES` table like `ALIASES` (staged distinct capture wins; else build
   from the two `web/src/assets/` sources; else report pending if a source is
   missing). Remove `devlog5-radar-stance-slots.png` from `FIGURES` so it is no
   longer double-reported as pending.
3. Add a `--self-test` mode that round-trips a synthetic image through
   decode/resize/compose and asserts correctness (no GPU-captured asset needed),
   so the codec is checkable in isolation.
4. Run `python3 scripts/gen-web-screenshots.py`; confirm the composite lands at
   1920x1080 and validates; confirm 0 pending except the split-out thumbnails.
   Build the web (`cd web && npx tsc --noEmit`; the figure auto-upgrades).
5. Document the composite step in the script header and `docs/development.md`.
   Commit the generated PNG (content, like the other shots).
</content>
