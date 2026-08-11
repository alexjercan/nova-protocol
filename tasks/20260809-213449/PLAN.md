# Plan: Wesnoth-style modding reference (web/ only)

Inputs: RESEARCH-wesnoth-reference.md, RESEARCH-wesnoth-learning-path.md,
CATALOG-constructs.md, CATALOG-base-content.md (agent artifacts in this dir),
plus benchmark/results/after/modder/tier3/GAPS.md as the evidence list.

## Wesnoth -> Nova mapping

| Wesnoth | Nova page (slug) | Role |
|---|---|---|
| Create | `modding` (rewritten) | Front door: routes by goal, explicit "new? start here" |
| BuildingScenarios* ladder | existing guides (kept) | Task tutorials: author-scenario, author-section, make-a-mod |
| ReferenceWML hub | `modding/reference` (new) | Hub: intro + how RON works + category lists + flat A-Z construct index |
| ScenarioWML | `modding/scenario` (new) | Scenario + Campaign content items, handler shape |
| EventWML | `modding/events` (new) | Every event kind: fires-when, payload, per-event subject table |
| FilterWML + StandardUnitFilter | `modding/filters` (new) | The 3 filter kinds documented once; fail-closed rule |
| ActionWML + purpose sub-pages | `modding/actions` (new) | Every action, grouped by purpose (flow/mission/ship/spawn/view), anchor per action |
| (UnitWML-ish) | `modding/objects` (new) | Every ScenarioObjectKind field-by-field |
| VariablesWML | `modding/expressions` (new) | Literal types + full expression AST grammar + type rules |
| (no equivalent; GAPS #3/#10) | `modding/base-content` (new) | Base prototype ids, scenario ids, dep://base asset manifest |

## Structure decisions

- Sidebar: "For creators" band splits into two categories: "Learn to mod"
  (modding, the three guides) and "Modding reference" (`modding/reference`
  parent + 7 children, then modding-ron, mod-portal). Children nest under the
  hub via the manifest `parent` field; hub renders the child grid.
- Every construct name goes into the manifest `headings` of its page, so the
  wiki search doubles as the construct glossary (GAPS: "filter quickly to the
  right construct").
- Reference page skeleton (from Wesnoth): intro -> RON shape -> mandatory
  fields -> optional fields -> semantics -> named gotcha sections ("Traps for
  the unwary") -> worked snippet -> cross-refs. Difficulty rises down the page.
- Field lists as tables (Field | Type | Default | Meaning), "required" spelled
  out in Default - the site's .prose table CSS already fits; better visuals
  than Wesnoth's definition lists and matches the task's "required vs
  serde-defaulted" ask.
- Auto "Contents" box: markdown.js already collects h2/h3; add `toc: true`
  per-page option rendering a MediaWiki-style contents box (reference pages
  only). Keeps anchors un-stale by construction.
- Guides keep their content; gain "see X for the complete list" handoffs into
  the reference (Wesnoth linking discipline) - no gutting.
- modding.md rewrite: Create-style routing + explicit start-here + the ladder
  (scenario -> section -> publish) + reference shelf.
- GAPS fixes to land in reference prose: fail-closed unset-variable filters,
  arm-inside-area OnEnter miss, count-gate `> n-1` idiom, Outcome/linger
  semantics, scenario-level field list, section kind list, prototype id
  catalog, dep://base asset manifest, "base as dependency" contradiction
  (align prose with the gates: implicit, but declaring it resolves too).

## Mechanical changes

- web/src/wiki/modding/*.md (8 new pages), modding.md rewritten.
- web/src/wiki-pages.ts: category split + 8 manifest entries (+ headings).
- web/webpack.config.js: 8 WIKI_DOC_PAGES entries (children before `modding`
  for rewrite ordering), crumbParent = Modding reference for the 7 children.
- web/markdown.js: `toc` option -> contents box from collected headings.
- web/src/style.css: .wiki-toc box.
- dev/keeping-docs-in-sync.md + related links updated to the new layout.

Out of scope (per owner): Rust-side codegen of catalog data; changes outside
web/ (the task's "generate the catalog" idea is noted as a follow-up
direction, pages are hand-synced against crates/ for now).
