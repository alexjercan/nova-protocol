# Notes

## What landed

One commit on master, web/ plus this task folder. The modding docs rebuilt as
a Wesnoth-style reference, web/ only.

### Research artifacts (this dir)

- RESEARCH-wesnoth-reference.md / RESEARCH-wesnoth-learning-path.md - live
  fetches of wiki.wesnoth.org (ReferenceWML, EventWML, ActionWML, FilterWML,
  StandardUnitFilter, ScenarioWML, Create, the BuildingScenarios ladder).
  Distilled patterns: hub page with category lists + flat A-Z index; fixed
  construct-page skeleton (intro -> mandatory -> optional -> semantics ->
  named gotcha sections -> examples); document cross-cutting filters ONCE and
  have consumers link; tutorials teach a subset and hand off ("see X for the
  complete list"); difficulty rises down every page.
- CATALOG-constructs.md - mechanical inventory from Rust source: 9 events,
  3 filters, 22 actions, 5 object kinds, 15 expression nodes, all file:line
  cited, RON spellings confirmed against shipped content.
- CATALOG-base-content.md - 123 section prototypes, 10 scenarios, 1 campaign,
  140 dep://base resources, overlay rules, naming grammar, all cited.
- PLAN.md - the Wesnoth -> Nova mapping and structure decisions.

### New pages (web/src/wiki/modding/)

`reference` (hub: RON spelling rules, child grid, vocabulary by family, flat
A-Z construct index), `scenario`, `events`, `filters`, `actions`, `objects`,
`expressions`, `base-content`. All are children of the hub in the sidebar
(manifest `parent`), all carry a build-time "Contents" box, and every
construct name is in the manifest `headings`, so the wiki search doubles as
the construct glossary.

### Restructure

- "For creators" band split into "Learn to mod" (modding front door + the
  three guides, in ladder order) and "Modding reference" (hub + 7 children,
  then modding-ron, mod-portal).
- modding.md rewritten as the Create-style front door with an explicit
  "new? start here" line (the one thing Wesnoth's Create lacks).
- Guides kept whole; they gained "the complete list is X" handoffs into the
  reference and lost nothing.
- keeping-docs-in-sync.md dependency map now routes scenario-engine and
  content-builder changes into the reference pages.

### Machinery

- markdown.js: `toc` option on wikiDocPage - a MediaWiki-style Contents box
  built from the same markdown-it-anchor headings, so TOC links cannot drift.
- style.css: `.wiki-toc` (nested-counter numbering, recessed panel).
- webpack.config.js + wiki-pages.ts: 8 page registrations, children before
  the `modding` prefix for dev-server rewrite ordering.

### GAPS.md coverage (benchmark evidence list, all 11 items)

Section kind list + worked sections (guide + base-content tables); prototype
id catalog with kinds (the controller-cube failure); expression AST grammar
including the asymmetric Add arms, three-term sums, subtraction
associativity, no-NotEqual; full event/filter/action/object reference;
fail-closed filters, count-gate `> n-1`, and the spawn-inside-area rule (the
CURRENT pinned behavior - area.rs:209 test - is spawn-inside FIRES OnEnter;
the old mod-comment claim was real and got fixed, documented as such);
base-as-dependency contradiction resolved in prose (implicit, never
required, declaring resolves too); scenario-level field table; dep://base
asset manifest; HudReadout Integer format for plain counts.

## Tradeoffs

- The catalog pages are hand-synced against crates/, not generated (owner
  scoped this to web/). Mitigations: keeping-docs-in-sync rows route engine
  changes here; the task's "generate catalog data like `content -- gen`"
  idea stays open as a follow-up direction.
- cargob/cargoa hull cubes are compressed to id-set shorthand instead of
  120 table rows; the non-hull specials (the load-bearing info) are tabled
  in full, racer fully tabled (it is the ship modders rebuild).
- Anchors depend on markdown-it-anchor's slugifier (whitespace runs collapse
  to ONE dash, unlike GitHub); all cross-page anchors were link-checked
  against the built dist.

## Verification

- `cd web && npm run ci` green (format, lint, tests, build).
- Link checker over the 14 new/edited built pages: every href and #anchor
  resolves (only /play/ flagged - the sibling game build, absent from a
  local dist by design).
- Crumbs render two-level (Wiki / Modding reference / Events); 22 action
  anchors, 9 event anchors present in dist.

## Next time

- A future Rust `content -- catalog` emitter could generate the field tables
  and the base-content id lists as JSON for the site to consume - the page
  prose is already separated from the tabular data with that in mind.
- The child-grid cards on the hub show the hatched placeholder icon frame;
  per-family icons could be captured later (same pattern as the ship
  sections).

## Annotation follow-up

- Moved the scenario tutorial from `dev/guide-author-scenario.md` to
  `modding/author-a-scenario.md`; the old URL now redirects.
- Rebuilt it as one short, playable Example Mod path: file shape, three-beat
  story, one event/filter/action loop, one expression gate, objective, victory,
  exact lint/run/menu steps, screenshot placeholder, and a short mistakes list.
- Removed the construct catalogs, internals, balance essay, and ship-geometry
  detail. The exhaustive pages now own those details.
- Repointed the manifest, related pages, and docs routing map to the new page.
- Verification: `cd web && npm run ci`; rendered local-link check for the new
  guide, modding front door, and reference hub (`/play/` excluded).
