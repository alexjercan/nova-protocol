# Review: convert player wiki HTML pages to markdown

- TASK: 20260715-205825
- BRANCH: 20260715-205825-wiki-md

## Round 1

- VERDICT: APPROVE

Format-only conversion of 15 player pages (10 top-level + 5 section children) to
markdown, rendered through the existing pipeline. Verification:

- All 15 pages emit under dist/wiki/; `npm run ci` (format + eslint + build)
  green; the old `.html` are removed and the `WIKI_SLUGS`/`wikiPage` html
  plumbing is gone (no dead code).
- Headless render parity: keybinds renders its four control tables with `<kbd>`
  keys AND the PromptFont gamepad glyphs (kept as raw HTML); the sections parent
  renders its child grid via the markdown-embedded `<div id="wiki-children">`;
  a child (hull) shows the two-level crumb "Wiki / Ship sections / Hull" with the
  parent linked; figures upgrade to real images.
- Every internal `](../slug/)` link resolves to a real WIKI_DOC_PAGES slug (or
  the tutorial page); no leftover `<%= basePath %>` tokens.
- Content parity: article word counts match within noise (targeting-radar
  412->407, hud 600->605, combat-weapons 436->426; deltas are markup, not lost
  prose).

Issues found and fixed during work (pre-review), recorded for the trail:

- [x] W1 (MAJOR, fixed) One conversion agent dropped the `../` prefix on
  cross-wiki links in flight-autopilot / gravity-wells / factions / scenarios
  (e.g. `](flight-autopilot/)` -> would 404 under `/wiki/<slug>/`). Fixed all to
  `](../slug/)` and re-verified; also corrected one `../tutorial/` ->
  `../../tutorial/` depth error in scenarios.md.

Intentional behavior change (documented in NOTES):

- [x] W2 (NIT) Heading anchor `id`s now derive from heading TEXT (via
  markdown-it-anchor), replacing the pages' hand-curated ids (e.g. the old
  `id="radar-locking"` on "Holding to sweep" becomes `id="holding-to-sweep"`).
  Confirmed no internal wiki link targets a heading `#anchor` (the only
  `#`-anchor on the site is `#features` on the landing page), so no deep link
  breaks. Accepted as part of the markdown move.
