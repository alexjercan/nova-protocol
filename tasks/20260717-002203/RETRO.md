# Retro: Docs - canonical scheme model / base as a normal mod

- TASK: 20260717-002203
- OUTCOME: landed (squash 999a881a), review APPROVE round 1.

## What was built

Updated the five author-facing docs + CHANGELOG to teach the canonical scheme
model (self:// / dep://<id>/ / dep://base/, no bare paths) and base-as-normal-mod,
retiring the "bare == base game" convention.

## What went well

- The completeness grep (bare-teaching prose AND stale moved-path references)
  caught a stale instruction the prose edits missed: "copy
  assets/textures/cubemap.png.meta" still pointed at the pre-migration location.
  Sweeping for the OLD paths (not just the old prose) is what found it.
- Docs-only task = no build, so it dodged the resource-pressure build kills that
  slowed tasks 2-3. Fast to land.

## What went wrong / difficulties

- Nothing significant. The main care was choosing the RIGHT scheme per example:
  base-mesh reuse examples became dep://base/, own-art examples stayed self://.
  A blind sed would have mis-schemed a mod's own-art example as dep://base.

## What to improve next time

- When docs describe file LOCATIONS (copy this .meta, ship under that dir), a
  path-relocation task must sweep the docs for the old paths, not only the old
  prose - same lesson as sweep-content-repo-wide-not-just-assets, applied to docs.
