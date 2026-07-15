# Refactor dev wiki: intent-based IA + Extend-the-game guides + project tour

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: feature, docs, web

Follows spike 20260715-204133 (user picked Option A: intent-based regroup +
guides, and all guides). Builds on the wiki pipeline from 20260715-195621.

## Goal

Reorganize the developer wiki by reader intent and add how-to/extension guides
so a contributor can learn "how to add new things" and "how the complex parts
work", grounded in the real code (extension maps in the spike's exploration).

## Information architecture

Regroup the sidebar categories (in `web/src/wiki-pages.ts` `WIKI_CATEGORIES`
and each dev page's `category`) from `Contributing / Architecture / Modding
(technical)` to:

- **Get started**: `development` (Building & running), NEW `project-tour`.
- **Understand**: `architecture`, `sections`, `scenario-system`, `modding-ron`,
  `mod-portal`.
- **Extend the game** (NEW): the four guides below.

Each Understand page gets a short "To add/author one, see <guide>" pointer at
the top linking its guide; each guide links back to its reference page.

## New pages (markdown under web/src/wiki/dev/, + WIKI_DOC_PAGES + manifest)

- `dev/project-tour` (Get started) - fast onboarding: crate map at a glance,
  where each kind of thing lives, the boot path, and a "want to change X? start
  in file Y" table.
- `dev/guide-add-section` (Extend the game) - the ordered checklist to add a
  ship-section kind (SectionKind + SectionDamageClass/resistance + spawn arm +
  section Plugin + editor placement + card tint/glyph + asset prototype +
  example), ending at a runnable example.
- `dev/guide-extend-scenarios` (Extend the game) - add an event kind / filter /
  action / scenario-object kind: the enum-variant + trait-impl + prelude recipe,
  one worked example each; the NovaEventWorld state/command seam.
- `dev/guide-author-scenario` (Extend the game) - author a scenario in RON end
  to end, built up from asteroid_field / shakedown_run: objectives via
  variables + filters + actions.
- `dev/guide-make-a-mod` (Extend the game) - the mod-author lifecycle: bundle
  layout (stemmed *.bundle.ron / *.content.ron), local testing, nova_portal_gen
  publish, what the player sees; call out the sharp edges honestly.

## Steps

- [ ] Regroup categories + reassign page categories in `web/src/wiki-pages.ts`.
- [ ] Author the 5 new markdown pages under `web/src/wiki/dev/`, grounded in the
      code (verify every file:line and RON example against the tree; use mermaid
      where a flow/checklist reads better as a diagram).
- [ ] Register the 5 pages in `web/webpack.config.js` WIKI_DOC_PAGES + manifest
      entries (with headings + cross-links) in `web/src/wiki-pages.ts`.
- [ ] Add "to extend this, see <guide>" pointers atop the reference pages.
- [ ] Verify: `npm run ci` green; serve + headless-eyeball the new pages (render,
      diagrams, sidebar groups, search, see-also); check at the deploy subpath.

## Notes

The wiki markdown pipeline (markdown-it + mermaid + highlight.js), the
`wikiDocPage()` helper, and the manifest-driven chrome already exist from
20260715-195621 - this task is content + one manifest/category reshuffle, no new
infrastructure. Do not invent facts or numbers; every extension step and RON
snippet must be checked against the code.
