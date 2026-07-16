# Devlog post-card thumbnails: thumb-devlog-3/4/5 (source decision + package)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,web,screenshot,devlog

Split out of the screenshot follow-up 20260715-004216 (which now covers only the
`devlog5-radar-stance-slots` composite). Deferred at the user's request: the
thumbnail source images / content choices are not decided yet, so this is blocked
on the user, not on capture tech.

The site references `thumb-devlog-3/4/5.png` under `web/src/assets/` (16:9,
displayed at ~300px on the devlog post cards). `scripts/gen-web-screenshots.py`
already lists them in `THUMBNAILS` (example=None), so they report as pending and
the pages fall back to placeholders until each file lands.

These are content choices tied to each devlog's subject, not a generic capture.
Suggested reuse (from the original follow-up's analysis):
- devlog 3 (zones, torpedoes, blast damage) -> `feature-juice.png` (a section
  blown off).
- devlog 4 (guided torpedoes, targeting, an enemy that fights back) ->
  `feature-combat.png` / `tutorial-combat-lock.png`.
- devlog 5 (radar locking, tutorial, the web site) -> `tutorial-radar-lock.png`
  or `tutorial-menu.png`.

## Steps

1. Decide the `thumb-devlog-3/4/5` -> source mapping with whoever owns the devlog
   content.
2. Easiest wiring: add each to the `ALIASES` map in
   `scripts/gen-web-screenshots.py` pointing at the chosen source (all sources
   are 16:9 already, displayed at 300px, so no resize needed). A distinct capture
   dropped into the stage dir always wins over an alias.
3. Re-run `python3 scripts/gen-web-screenshots.py`; confirm 0 pending for the
   thumbnails; check the devlog post cards render the images
   (`cd web && npm run build`, hard-refresh).
</content>

## Rename note (20260716, v0.7.0 planning pass)

Stays backlog (blocked on the user choosing source images), but note: after
the news merge the site now references thumb-news-0.3.0/0.4.0/0.5.0/0.6.0.png
while scripts/gen-web-screenshots.py still lists thumb-devlog-3/4/5.png -
reconcile the naming (and add the 0.6.0 thumb) when picked up.
