# Source/create real variety-pack art (skyboxes, asteroid/planet textures, alt hull) to replace placeholders

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.7.0,art,assets


## Goal (human/art task - not an agent code task)

This is the ART half of the "Asset variety pack" (20260716-123544). The
PIPELINE half (mods can ship their own binary resources, mod-relative asset
refs) is being implemented separately on feature/mod-binary-assets, which also
ships a small placeholder mod that dogfoods the pipeline. This task is the
actual art sourcing/creation, which needs a human decision + hands.

Produce real assets to replace the placeholders:

- at least one THEMED skybox (something distinct from the existing
  cubemap/cubemap_alt/cubemap_alt2 style) so SetSkybox has an interesting
  destination;
- asteroid texture variant(s) and planet(oid) texture variant(s);
- at least one alternate hull or section model variant.

## Sourcing decision (owner: user)

Pick the pipeline before making the art (the original task keeps its spike tag
for exactly this fork):

- procedural generation (e.g. space-skybox generators, noise-based rock/planet
  textures);
- curated CC0 packs (Poly Haven, Kenney, ambientCG, OpenGameArt CC0);
- hand-made in Blender (art/ already holds .blend sources today).

Record art licensing/attribution: cargo-about / credits/ cover CODE only. Add
an art-credits answer (a credits entry or an ATTRIBUTION file per asset pack)
so CC0/CC-BY sources are attributed correctly.

## Where the art goes

- The placeholder mod shipped by feature/mod-binary-assets shows the exact
  layout: resource files live next to the mod's content and are referenced by
  mod-relative AssetRef paths. Drop the real textures/skyboxes/models in the
  same place (replace the placeholder PNG/GLB files, keep the paths) OR add a
  new themed mod and point a scenario at it.
- Base-owned art (used by the base game itself, not a mod) still lives in
  assets/.

## Consumers waiting on this art

- Story campaign mod (20260716-123535) - wants its own look.
- Gauntlet Run 2.0 (20260716-124722) - wants its own look.
- Playable vertical slice (20260708-203659).
- Per-scenario picker thumbnails (20260715-220011).

## Notes

- Split out of 20260716-123544 by user direction 2026-07-16: agent implements
  the pipeline + placeholders, user sources the real art.
