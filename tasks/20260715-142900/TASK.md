# Static mod portal: webmods/ + generator bin (catalog.json, hashed files) + deploy step; demo mod moves online

- STATUS: OPEN
- PRIORITY: 17
- TAGS: modding,web

Spike: tasks/20260714-202515/SPIKE.md (options D, H, J)
Depends on: 20260715-142849 (bundle meta - the generator reads it).

Goal: remote mods served as generated static files on the existing GitHub Pages
site. Remote mod sources live in a repo-root `webmods/` folder (same bundle
shape, outside `assets/` so they don't ship in the game). A small workspace bin
(reusing nova_modding types) scans `webmods/`, validates (parse bundle+content,
unique ids vs shipped set, deps exist), computes per-file size + sha256, and
emits `site/mods/catalog.json` (versioned `schema_version`; entries: id,
version, meta, `files: [{path, size, sha256}]`) plus copies files to
`site/mods/<id>/<version>/...`. Wire format is JSON (serde types shared between
game and generator). Add the deploy-workflow step. MOVE the demo mod online:
source relocates to `webmods/demo/`, its entry leaves `assets/mods.catalog.ron`
- it becomes the portal's first real mod (Explore dogfood). Document the portal
layout + publish flow in docs/.

