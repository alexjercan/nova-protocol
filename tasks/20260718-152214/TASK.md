# Docs-follow-code audit: reconcile web/src/wiki/dev pages with current code, fix drift and gaps

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.8.0,docs,web

## Goal

Make the docs follow the code. The dev wiki under `web/src/wiki/dev/` is the
source of truth (per docs/README.md) but drifts as code lands. Sweep every page
against the current code, fix stale/missing bits, and close gaps. This is the
"documentation improvements where there are gaps + making the docs follow the
code" half of the v0.8.0 docs strand (the rustdoc/API side is 20260525-133033).

## Steps

- Page-by-page reconcile each `web/src/wiki/dev/*.md` with the code it
  describes; fix drift and fill gaps. Known suspects to verify:
  - `architecture.md` crate map vs the 15 real crates and plugin order.
  - `scenario-system.md` vs the current actions/events/filters (Outcome action,
    area OnEnter/OnExit, allegiance) shipped in v0.7.0.
  - `sections.md` vs render-mesh-transform + configurable colliders (v0.7.0
    tasks 20260718-113307/121205/102022) and ammo slots.
  - `modding-ron.md` / `mod-portal.md` vs the bundle model and mod-relative
    resource refs if the asset-variety pack landed.
  - `development.md` command list vs the real scripts/bins and the settings /
    RCS features added in v0.7.0.
- Verify each documented command actually runs; correct any that changed.
- Where a v0.7.0 feature has no page/section at all, add one (RCS, settings
  menu, graphics presets, render-scale lever, outcome frame).
- Record the drift found so the release-flow "keeping-docs-in-sync" step
  (20260718-152225) can be tightened to prevent recurrence.

## Notes

- Player-facing wiki (`web/src/wiki/`) is in scope where it describes shipped
  behavior; the v0.7.0 pre-release web update (20260718-152333) covers the
  release-note/news surface, this covers reference-doc accuracy going forward.

