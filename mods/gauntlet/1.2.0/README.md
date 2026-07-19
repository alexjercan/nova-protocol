# Gauntlet Run

The first PORTAL mod: published to the static mod portal by `nova_portal_gen`,
NOT shipped inside the game's `assets/`. A mod is the same shape as the base
game: a folder with a `*.bundle.ron` manifest listing its `*.content.ron` files.

This mod adds one scenario, `gauntlet_run` - a parkour course (v1.1.0, was a
thin four-gate slalom in v1.0.0): six ordered gates across three escalating acts
(warmup / slalom / hazard), invulnerable asteroids crowding the racing line, and
an act-3 gravity well to sling or avoid. Crossing FINISH declares Victory;
wrecking your hull on the rocks declares Defeat with a Retry. Pure flying skill,
no combat. It is also a worked example of the data-driven scenario vocabulary -
`ScatterObjects`, `Asteroid` (`invulnerable`, `surface_gravity`), ordered
`OnEnter` gates, `SetSkybox` per act, and `Outcome` frames - all in RON.

## The copy-me publish example

Portal mods live under the repo-root `webmods/<id>/`, outside `assets/`, so they
never ship in the game build. The directory name IS the id. To publish your own,
copy this folder as a template:

- `gauntlet.bundle.ron` - the manifest: a `content` list plus a `meta`
  self-description (`name` and `version` are required by the portal generator).
- `gauntlet.content.ron` - the content: a `[Content]` list of `Scenario` /
  `Section` items (see the authoring guides under
  `web/src/wiki/dev/guide-author-scenario.md` and `guide-author-section.md`).

## How it publishes

`nova_portal_gen` copies this folder to `site/mods/gauntlet/<version>/` and lists
it in `catalog.json` with per-file `size` + `sha256`. The full flow - the
generator invocation, the validation gates, and the deploy - is in
`web/src/wiki/dev/guide-make-a-mod.md` (Publish to the portal).

Naming note: the manifest must be stemmed (`gauntlet.bundle.ron`, not
`bundle.ron`) - see `web/src/wiki/dev/modding-ron.md`.
