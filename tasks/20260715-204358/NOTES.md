# NOTES - dev wiki IA refactor + Extend-the-game guides

## What changed and why

Follows spike 20260715-204133 (user picked intent-based regroup + all guides).
The dev wiki was all reference ("what X is"); the code has a consistent, teachable
extension pattern that was undocumented. This task reorganizes the wiki by reader
intent and adds the missing how-to guides.

- IA regroup (`web/src/wiki-pages.ts`): replaced the dev categories
  Contributing / Architecture / Modding (technical) with **Get started /
  Understand / Extend the game**, and reassigned the six existing dev pages.
- Five new markdown pages under `web/src/wiki/dev/` (registered in
  `web/webpack.config.js` WIKI_DOC_PAGES + `wiki-pages.ts`):
  - `project-tour` (Get started) - crate map at a glance, boot path, and a
    "want to change X? start here" table.
  - `guide-add-section` - the ~9-step checklist to add a ship-section kind.
  - `guide-extend-scenarios` - add an event/filter/action/object kind (the
    enum + trait-impl + prelude recipe) and the NovaEventWorld seam.
  - `guide-author-scenario` - author a scenario in RON, built up from the
    shipped scenarios.
  - `guide-make-a-mod` - the mod-author lifecycle (bundle -> test -> publish ->
    install), with honest sharp edges.
- Each Understand reference page got a short callout blockquote linking its guide.

No new infrastructure - reuses the markdown pipeline from 20260715-195621.

## Process

Five guide pages were drafted by parallel subagents, each grounded in a
code-extension map (from the spike's sweeps) but instructed to VERIFY every
file:line and RON snippet against the tree, not trust the map. They caught real
drift in the maps (examples live at repo root not per-crate; only 3 resistance
rows needed because of the `(_, Kinetic)` wildcard; the plugin tuple line).

## Verification

`npm run ci` green. Served + headless-screenshotted: the sidebar shows the three
new intent groups with all 11 dev pages; a guide page renders its checklist,
syntax-highlighted Rust, and mermaid diagram. An out-of-context fact-check pass
verified the guides against the code (APPROVE) - paths, enum sets, RON shapes,
the portal CLI, validation gates, and overlay semantics all check out; two NITs
(a missing `Stop` flight verb, a "two arrays" overcount) fixed on the branch.

## Follow-up (user request, separate task)

The user asked whether the EXISTING player wiki HTML pages can also become
markdown. That is a distinct change (teach the doc shell about figure
placeholders and the parent/child section grid, convert ~15 pages) - filed as
its own task rather than widening this branch.
