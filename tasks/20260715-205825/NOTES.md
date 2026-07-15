# NOTES - player wiki HTML -> markdown

## What changed and why

Unify the whole wiki on the markdown pipeline (from 20260715-195621): the 15
hand-authored player pages (10 top-level + 5 section children) are now markdown
under `web/src/wiki/`, rendered by the same `wikiDocPage` shell as the dev pages.
The `wiki.html` index stays HTML (it hosts `#wiki-index`).

## Pipeline extensions (web/markdown.js)

- `docShell(title, basePath, opts)` gained `opts.description` (optional meta) and
  `opts.crumbParent` ({slug, title}) for the two-level child crumb
  ("Wiki / Ship sections / Hull").
- `wikiDocPage` passes those through. `mdPath` was already a free parameter, so
  `docPage` in webpack.config.js just maps `src/wiki/${md}` (dev entries gained a
  `dev/` prefix in their `md`).
- webpack.config.js: `WIKI_SLUGS` + the `wikiPage()` html helper are gone; all
  pages live in `WIKI_DOC_PAGES` (children before the sections parent so the
  history-fallback prefix match hits the specific path first). The player pages
  carry no `description` (the originals had none) and the five children carry a
  `crumbParent`.

## Conversion approach

Each page: took only the `<article class="wiki__body prose">` inner content,
dropped the crumb/tags/seealso scaffolding (the shell re-provides them), turned
the `<h1>` into the leading `# Title`, and converted prose to markdown. Kept as
RAW HTML (html: true passes them through): every `<figure class="figure">`
placeholder block (site.ts upgrades them), inline `<kbd>`, and the
`<table class="controls">` blocks in keybinds/combat-weapons (they carry `<kbd>`
+ PromptFont gamepad glyph spans with no markdown equivalent). The sections
parent keeps `<div id="wiki-children">` where the child grid renders.

## Difficulties

- Heading anchor `id`s: the HTML pages had hand-curated ids that differ from the
  heading text (e.g. `id="radar-locking"` on "Holding to sweep"). The anchor
  plugin regenerates ids from text. Verified nothing links to those anchors (only
  `#features` exists, on the landing page), so this is a safe, intentional change.
- One conversion agent omitted the `../` on cross-wiki links (would 404); caught
  by a link sweep and fixed with an explicit slug-allowlist sed, then re-verified.

## Verification

`npm run ci` green; all 15 pages emit; headless render confirms tables + kbd +
gamepad glyphs, the sections child grid, child crumbs, and figure upgrades;
internal links all resolve to real slugs; content word-count parity within noise.
