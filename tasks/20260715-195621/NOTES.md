# NOTES - markdown-first developer wiki

## What changed and why

The durable developer docs (build, architecture, ship-section internals,
scenario engine, RON format, mod portal, Bevy migration) moved out of `docs/`
and into the website as public wiki pages under `/wiki/dev/`. `docs/` is now
only the transient SDLC store (LESSONS.md, plans/, per-task folders, the
task-scoped perf report).

The web app already had a manifest-driven wiki (spike 20260713-225157, Option
B): hand-authored HTML article pages + client-side chrome (sidebar, search, tag
chips, see-also) rendered from `wiki-pages.ts`. Rather than replace it, we added
**markdown as a second page source** rendered at build time into the same
`article.wiki__body.prose` shell, so all the existing chrome keeps working with
no change to `wiki.ts`.

Approach chosen: markdown-first over HTML-first. The content already lived as
markdown that the workflow keeps editing; hand-HTML would fork it and pile on
`id=`-anchor boilerplate. Markdown does not cap the UI: fenced ```mermaid gives
diagrams, build-time highlight.js gives code coloring, and `html: true` leaves
raw inline HTML as an escape hatch.

## How it works

- `web/markdown.js` - a markdown-it renderer (anchor ids on h2/h3, highlight.js
  per fence, ```mermaid -> `<pre class="mermaid">`), plus `wikiDocPage()` which
  renders a `.md` to a full page. The rendered body rides on the HtmlWebpackPlugin
  `docBody` option and is injected into a `#doc-body` placeholder by
  `HtmlPartialsPlugin` at beforeEmit - after templating - so lodash never runs
  over code samples (and a function-form `.replace` keeps `$` in RON/Rust safe).
- `webpack.config.js` - a `WIKI_DOC_PAGES` list (slug -> md -> title) mapped to
  doc-page plugins + history-fallback rewrites, parallel to the existing
  `WIKI_SLUGS`. Dev slugs are `dev/`-prefixed so they never collide with player
  slugs; `wiki.ts` already handles multi-segment slugs.
- `web/src/wiki.ts` - `initMermaid()` dynamically imports mermaid ONLY when a
  `.mermaid` element exists, themed from the site CSS variables, so pages without
  a diagram never pull the (large) mermaid bundle.
- `web/src/style.css` - prose furniture for code blocks (a compact highlight.js
  theme in the site palette), tables, mermaid frames, admonitions.
- `web/src/wiki-pages.ts` - three new categories (Contributing, Architecture,
  Modding (technical)) and one manifest entry per dev page (with search
  headings + cross-links to the player pages).

The moved docs were public-ized (task-id citations and sibling-repo paths
stripped, pure work-log sections dropped from modding-ron) and given Mermaid
diagrams: crate-dependency graph + state machine + frame-flow (architecture),
integrity cascade (sections), event/filter/action pipeline (scenario), mod
catalog/bundle graph (modding-ron), publish/install sequence (mod-portal).

## Difficulties / decisions

- `templateContent` STRING is not run through lodash (unlike a `template`
  FILE), so `<%= basePath %>` survived verbatim in the first build. Fixed by
  inlining `publicPath` into the shell at config time (it is already known) -
  simpler than a plugin pass, and keeps lodash away from the body entirely.
- Injecting the body via a placeholder (not `<%= bodyHtml %>`) deliberately
  avoids lodash choking on `<%`/`${}` inside code samples.
- `npm install` reformatted `web/src/site.ts` via a prettier patch bump pulled
  in with the new deps; kept, because `npm run ci`'s `format:check` now requires
  it.

## Verification

## Review amendments (Round 1)

The out-of-context review caught two real link breakages the `docs/`-prefixed
sweep missed: bare `[name.md](name.md)` intra-doc links in architecture.md /
scenario-system.md (would 404 under `/wiki/dev/...`), and `assets/mods/demo/
README.md` still pointing at the old `docs/` path. All rewritten to wiki URLs /
the new path. Also hardened mermaid to reveal its source on load/parse failure
instead of leaving it invisible, and framed `pre.mermaid` as a figure.

Per user redirect, the Bevy migration notes moved back to
`docs/bevy-0.19-migration.md` (historical one-off, not a living reference), so
the wiki ships 6 `dev/*` pages, not 7.

## Verification

`npm run ci` green (format:check + eslint + build); all 6 `dev/*` pages emit.
Beyond the build (which does not exercise client render), served the site and
headless-screenshotted pages: the crate-dependency graph, the portal sequence
diagram, and the sections flowchart all render to themed SVG; Rust/RON code is
syntax-highlighted; tables styled; the sidebar shows the three new categories
and their pages; tag chips render. Repo swept for references to the moved doc
paths - live pointers in AGENTS.md, CHANGELOG.md and `nova_assets/src/portal.rs`
updated; historical `tasks/*` records left as-is.
