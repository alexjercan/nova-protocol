# Convert the player wiki HTML pages to markdown (unify on the pipeline)

- STATUS: OPEN
- PRIORITY: 45
- TAGS: feature,docs,web

User request: unify the whole wiki on the markdown pipeline (from 20260715-195621)
so the hand-authored player pages are markdown too, not HTML. Feasible - the
pipeline already passes raw HTML through (`html: true`), so figures and `<kbd>`
carry over verbatim; the parent/child grid is a `<div id="wiki-children">` the
existing `wiki.ts` fills; parent/child manifest relationships already exist.

## Scope

15 player pages under `web/src/wiki/` (10 top-level + 5 `sections/` children,
~1900 lines). The `wiki.html` index stays HTML (it hosts `#wiki-index`). The
`modding` page is coming-soon in the manifest - convert only if it has real HTML.

## Pipeline extensions (web/markdown.js + webpack.config.js)

1. Generalize `wikiDocPage` to take an explicit `mdPath` (any path under
   `src/wiki/`, not just `dev/`) and an optional `description` (rendered as
   `<meta name="description">`, sourced from the manifest `summary`) and an
   optional `crumb` parent `{ title, slug }`.
2. Extend `docShell` to render a two-level crumb when a parent is given:
   `Wiki / <parent title> / <title>` (child pages like `sections/hull`). Link the
   parent segment to `<basePath>wiki/<parentSlug>/`.
3. The sections parent page keeps its child grid: put `<div id="wiki-children">`
   in `sections.md` (raw HTML); `wiki.ts renderChildrenGrid` already fills it for
   the parent slug.

## Conversion (per page)

- Convert the `<article class="wiki__body prose">` inner content to markdown:
  prose -> markdown, headings keep their `id`s via the anchor plugin (verify the
  slugify matches the existing `id`s used by in-page/other-page anchors; if not,
  keep explicit anchors), lists/emphasis -> markdown.
- Keep as RAW HTML inside the markdown: every `<figure class="figure">...` block
  (the `.figure__placeholder` capture divs - site.ts upgrades them) and inline
  `<kbd>...</kbd>`.
- Drop the per-page `<!doctype>`/head/crumb/h1/tags/seealso scaffolding - the
  doc shell provides it. The first `# Title` becomes the page title.
- Cross-links: rewrite `<%= basePath %>wiki/<slug>/` anchors to relative wiki
  links (`../<slug>/` etc.) so they are base-path independent.

## Wiring

- Move each converted slug from `WIKI_SLUGS` (html) to the doc-page list with its
  `mdPath` (+ description from the manifest summary, + crumb parent for the five
  `sections/*` children). Remove the `wikiPage()` html registration and the html
  `historyApiFallback` entries the doc-page list does not already cover.
- `git rm` the old `.html` files once the `.md` renders identically.

## Steps

- [ ] Extend `markdown.js` (`wikiDocPage`/`docShell`: mdPath, description, crumb
      parent) and `webpack.config.js` (move player slugs to the doc-page list;
      keep child-before-parent ordering for the history rewrites).
- [ ] Convert the 10 top-level player pages to markdown (raw HTML for figures +
      kbd; relative cross-links).
- [ ] Convert the 5 `sections/*` children; put the child grid div in `sections.md`.
- [ ] `git rm` the old HTML; confirm `WIKI_SLUGS` is empty (or only the index) and
      remove now-dead html plumbing.
- [ ] Verify: `npm run ci` green; serve + headless-eyeball a converted top-level
      page, the sections parent (child grid) and a child (two-level crumb),
      figures still upgrade, `<kbd>` renders, search/see-also intact; check at the
      deploy subpath.

## Notes

Pure consolidation - no visible change to the player pages, they just render from
markdown. Watch heading `id` parity (deep links) and the two-level crumb. No new
runtime code beyond the shell crumb tweak.
