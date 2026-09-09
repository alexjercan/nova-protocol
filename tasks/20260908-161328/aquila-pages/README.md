# Aquila: episode 1, pages 5-7

**Frozen review.** The accepted SVGs and HTML remain unchanged. `generate.py.txt`
and `props.py.txt` preserve their previous sources; the commands below are
historical. Maintained sources now live in `web/src/comics/season-1/episode-1/`.
Use `npm --prefix web run serve` and the real reader for new work.

Open [the art review](index.html), or the
[complete episode script with current illustrations](../episode-1/index.html).
These three private pages extend the accepted four-page opening. Pages 8-18
remain script, not finished art. Nothing here enters public comic discovery.

The user accepted Gantry and its crew as working designs, then chose a
pressurized, non-rotating transfer hall. Both ships remain outside. Handholds,
restrained cargo, and a freight lock establish freefall handling without suits
or an open path from the hall to vacuum. This local scene does not define a
station blueprint, physical dimensions, gravity level, or transit duration.

`generate.py` owns composition and dialogue wrapping. It reads every spoken
line from `../episode-1/SCRIPT.md` and reuses the opening page frame. The script
now says the load is awkward to turn and jokes about chasing two loads rather
than picking them off a floor. It retains the arrival, Nadia meeting, Gantry-first
departure, and game-night exchange.

Shared artwork supplies the accepted faces, separate arms/bodies, industrial
ship models, and Aquila's existing two-ring/freight-spine concept. `props.py`
owns the episode-local replacement assembly and handling frame. That same solid
prop is rendered in close inspection and in Kaveri's external cradle, sharing
its painter pass rather than covering the ship with an unrelated flat image.
No prior public lore asset is regenerated into a different drawing.

```bash
python3 tasks/20260908-161328/aquila-pages/generate.py
node tasks/20260908-161328/episode-1/generate.mjs
python3 tasks/20260908-161328/aquila-pages/generate.py --check
node tasks/20260908-161328/episode-1/generate.mjs --check
node tasks/20260908-161328/proof/inspect-aquila.mjs
```

The review has Art only, full-size SVG links, print pages, and text transcripts.
Phone fitting is a landscape overview, not panel-by-panel mobile lettering.
Current checks write only `proof/aquila/`. Do not rerun historical proof writers
into their frozen directories; their page counts and unchanged-source scopes
belong to their earlier revisions.
