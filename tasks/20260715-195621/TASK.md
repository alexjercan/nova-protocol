# Publish technical/onboarding docs as a markdown-first wiki

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: feature, docs, web

## Context

The web app (`web/`) has a manifest-driven wiki of hand-authored HTML pages
covering **player** topics (keybinds, HUD, combat, ships). Separately, `docs/`
holds ~2500 lines of **developer** markdown (build, architecture, scenario
engine, modding format, portal) that the SDLC workflow keeps editing. We want
that technical/onboarding content published in the wiki, with real diagrams and
schemas, so a new contributor can find their way around the code fast.

Two approaches were weighed:

- **Markdown-first** (chosen): add a build-time markdown pipeline that renders
  `.md` into the existing wiki article shell. Diagrams come from fenced
  ` ```mermaid ` blocks, code gets build-time syntax highlighting, and raw inline
  HTML stays available as an escape hatch for custom widgets/animations. So
  markdown raises the floor (easy edits, stays in the format the workflow uses)
  without capping the ceiling.
- **HTML-first** (rejected): hand-authoring 2500 lines of frequently-edited
  prose as HTML is heavy `id=`-anchor boilerplate and would fork the content
  away from the markdown the workflow writes.

Decisions confirmed with the user:
1. Approach: **markdown-first**.
2. Source of truth: **move** the durable docs into `web/src/wiki/`; `docs/` keeps
   only transient tatr/agent output (LESSONS.md, plans/, task records). Single
   source, no duplication.
3. Placement: **new developer categories in the same wiki** (one unified
   sidebar/search), not a separate docs site.

## Approach

Keep the existing manifest-driven wiki exactly as-is (spike
`tasks/20260713-225157/SPIKE.md`, Option B). The player HTML pages do not change.
Add markdown as a **second supported page source** that renders at build time
into the same `article.wiki__body.prose` shell, so the sidebar, search, tag
chips and "see also" from `wiki.ts` + `wiki-pages.ts` keep working unchanged.
Developer pages are namespaced under a `dev/` slug prefix and grouped in new
categories.

## Build pipeline (markdown -> HTML at build time)

Rendering happens in Node (webpack config), so there is no runtime markdown cost
and no-JS/SEO readers still get full content -- matching the site's existing
"content in HTML, chrome via JS" split.

- **Deps** (`web/package.json`, devDependencies): `markdown-it`,
  `markdown-it-anchor` (heading `id`s for deep links + crumb anchors),
  `highlight.js` (build-time code highlighting), `mermaid` (client-side diagram
  render).
- **Shared template** `web/src/wiki/_doc.html`: an EJS template mirroring an
  existing wiki page (`src/wiki/targeting-radar.html`) -- `#header`/`#footer`
  placeholders, `aside#wiki-nav`, `article.wiki__body.prose` with the crumb,
  `<h1>`, `#wiki-tags`, the injected body via `<%= bodyHtml %>` (lodash `<%=` is
  unescaped, same mechanism the templates already use for `basePath`), and
  `#wiki-seealso`. Uses the `wiki` chunk.
- **Helper** in `web/webpack.config.js`: `wikiDocPage(slug, mdPath, title)` that
  reads the `.md`, renders it with a configured `markdown-it` instance
  (anchor plugin + highlight.js `highlight` hook + a fenced ` ```mermaid ` rule
  that emits `<pre class="mermaid">...`), and passes `bodyHtml` + `title` +
  `crumbTitle` as `templateParameters` into `_doc.html`, output to
  `wiki/<slug>/index.html`. Register each dev slug in the `WIKI_SLUGS` list and
  the `historyApiFallback` rewrites (both already map over `WIKI_SLUGS`, so the
  `dev/...` slugs flow through the existing `.map()` calls).
- **Mermaid (client-side)** in `web/src/wiki.ts`: after boot, if
  `document.querySelector(".mermaid")` exists, `await import("mermaid")` and run
  it. Dynamic import keeps mermaid out of the bundle for pages without diagrams.
- **Styling** (`web/src/style.css`, near the existing `.prose`/`.figure` rules):
  a highlight.js theme for `pre code.hljs`, table styling, a `.mermaid` figure
  frame, and simple admonition/callout styling. Reuse the existing `.prose`
  scale so dev pages match the sharp house style.

`wiki.ts`, `wiki-pages.ts`'s interfaces, and the player pages need no structural
change -- dev pages are just more manifest entries whose HTML happens to be
generated.

## Content migration (move docs -> wiki)

`git mv` the durable reference docs into `web/src/wiki/dev/` as the wiki source,
editing each for a public audience (strip task-id refs like `20260709-131502`,
rewrite sibling-repo paths like `../bevy-common-systems/...`, and add the
diagrams the user wants):

| From `docs/` | To `web/src/wiki/dev/` | Slug | Category | Diagrams to add |
|---|---|---|---|---|
| development.md | development.md | `dev/development` | Contributing | build/deploy flow |
| architecture.md | architecture.md | `dev/architecture` | Architecture | crate-dependency graph, app state machine, Update/FixedUpdate frame flow |
| sections.md | sections.md | `dev/sections` | Architecture | integrity -> disable -> destroy damage cascade |
| scenario-system.md | scenario-system.md | `dev/scenario-system` | Architecture | event -> filter -> action pipeline |
| modding-ron-format.md | modding-ron.md | `dev/modding-ron` | Modding (technical) | mod-bundle/catalog schema |
| mod-portal.md | mod-portal.md | `dev/mod-portal` | Modding (technical) | publish/fetch/install sequence |
| bevy-0.19-migration.md | bevy-migration.md | `dev/bevy-migration` | Architecture | (reference, optional) |

Slugs are `dev/`-prefixed: no collision with player slugs, groups output under
`/wiki/dev/`, and `wiki.ts` already handles multi-segment slugs
(`currentSlug`/`bySlug`, as with `sections/hull`). No page has slug `dev`, so the
active-state and parent/child logic is unaffected.

**Stays in `docs/`** (transient/agent-generated): `LESSONS.md`, `plans/`, and
per-task records. Rewrite `docs/README.md` so it describes `docs/` as the
transient store (LESSONS + plans + task folders) and points durable reference to
the wiki; update its "After a meaningful change" guidance so future agents edit
the wiki markdown under `web/src/wiki/dev/` instead of `docs/`.

## Manifest changes (`web/src/wiki-pages.ts`)

- Append developer categories to `WIKI_CATEGORIES`: `"Architecture"`,
  `"Contributing"`, `"Modding (technical)"` (after the player categories, so
  player content stays on top in the sidebar).
- Add one `WikiPage` entry per moved doc: `slug` (`dev/...`), `title`,
  `category`, `tags` (extend the small taxonomy with e.g. `dev`, `architecture`,
  `build`), `summary`, `related` (cross-link player <-> dev pages, e.g.
  `dev/scenario-system` <-> `scenarios`), and a few key `headings` for search.

## Steps

- [x] Add deps (markdown-it, markdown-it-anchor, highlight.js, mermaid) to
      `web/package.json`; `npm install`.
- [x] Build the markdown render pipeline: `markdown-it` config (anchor +
      highlight.js + mermaid fence), the `_doc.html` shared template, and the
      `wikiDocPage()` helper wired into `web/webpack.config.js` (WIKI_SLUGS +
      history rewrites).
- [x] Add client-side mermaid init + prose CSS (code/tables/mermaid/admonitions)
      in `web/src/wiki.ts` and `web/src/style.css`.
- [x] `git mv` the durable docs into `web/src/wiki/dev/*.md`, edit each for a
      public audience, and add the mermaid diagrams.
- [x] Add new categories + `WikiPage` manifest entries in `web/src/wiki-pages.ts`.
- [x] Rewrite `docs/README.md` as the transient store; update the
      "after a meaningful change" guidance to point at the wiki markdown.
- [x] Verify: `npm run serve` (pages render with diagrams + highlighted code,
      search/sidebar/see-also include them), `npm run ci` green, `docs/` reduced.

## Critical files

- `web/webpack.config.js` -- `wikiDocPage()` helper, markdown-it config, register
  `dev/*` slugs (WIKI_SLUGS + history rewrites).
- `web/src/wiki/_doc.html` -- new shared doc template.
- `web/src/wiki.ts` -- dynamic mermaid init (only addition).
- `web/src/wiki-pages.ts` -- new categories + dev page manifest entries.
- `web/src/style.css` -- code/table/mermaid/admonition prose styles.
- `web/package.json` -- markdown-it, markdown-it-anchor, highlight.js, mermaid.
- `web/src/wiki/dev/*.md` -- moved + edited docs.
- `docs/README.md` -- rewrite to "transient store" + point reference to the wiki.

## Verification (end to end)

1. `cd web && npm install` (new deps).
2. `npm run serve`, open `http://localhost:8090/wiki/`:
   - New categories and dev pages appear in the sidebar; search finds them
     (e.g. "architecture", "crate", "deploy").
   - `/wiki/dev/architecture/` renders headings with anchor links, a Mermaid
     crate/state/frame diagram, syntax-highlighted Rust/RON code blocks, and
     tables.
   - Tag chips + "See also" render (cross-links between a dev page and its player
     counterpart resolve).
   - A page with no diagram does not pull in the mermaid bundle (dynamic import).
3. `npm run build` produces `dist/wiki/dev/<slug>/index.html` for each page.
4. `npm run ci` (`format:check && lint && build`) passes.
5. Confirm `docs/` retains only LESSONS.md, plans/, task records, and the
   rewritten README; the deploy workflow (`.github/workflows/deploy-page.yaml`)
   needs no change (dev pages build with the web app).
