# Spike: wiki architecture - sub-pages, sidebar, tags, search, see-also

- DATE: 20260713-225157
- STATUS: RECOMMENDED
- TAGS: spike, web, wiki, design

## Question

We want to split the site's reference content out of the tutorial and grow the
wiki from one long page into a real, Obsidian-brain-style wiki: ~10 sub-pages,
a persistent category sidebar, per-page tags, "see also" cross-links, and
client-side search - full content plus image placeholders, in the existing
sharp style. In a static TypeScript + Webpack + Tailwind site with hand-authored
HTML pages, what is the architecture that gives all of that without
hand-maintaining nav/search/related across 11 pages? A good answer names the
source of truth, how each wiki feature derives from it, and the page/build
wiring, concretely enough to seed the build tasks.

## Context

- Pages are separate HTML files, each registered in `web/webpack.config.js` as
  its own `HtmlWebpackPlugin` entry + a `historyApiFallback` rewrite; all share
  `style.css` and a JS entry that calls `initSite()` (`site.ts`), which marks the
  active top-level nav link.
- The shared header/footer are injected at build time by `HtmlPartialsPlugin`
  into `#header` / `#footer` placeholder divs (see `_header.html`,
  `_footer.html`). This is the precedent for "shared chrome via a placeholder".
- Today all reference content lives inline in one `wiki.html` (Sections,
  Scenarios, Gravity wells, Factions, HUD tiers), and `tutorial.html` mixes
  getting-started (Shakedown Run) with reference tables (keybinds, targeting,
  weapons). The split: tutorial keeps only "clear your first scenario and start
  playing"; everything else becomes wiki sub-pages.
- Target sub-pages (user-approved): Sections, Keybinds, HUD, Flight & autopilot,
  Targeting & radar, Combat & weapons, Gravity wells, Factions, Scenarios, and
  Modding (coming soon) - plus a wiki index.
- The sharp `.figure` placeholder component already exists for images.

## Options considered

- **A. Static everything, hand-authored per page.** Each sub-page hard-codes its
  own sidebar, tag chips, and see-also list; search is dropped or a separate
  build-time JSON index. Simple mental model, works with no JS. But 11 pages
  each duplicating the sidebar means every new page or rename edits all 11, and
  tags/related drift out of sync by hand. Fails the "real wiki that stays
  consistent" bar.

- **B. Manifest-driven, JS-injected chrome (recommended).** One data module,
  `wiki-pages.ts`, is the source of truth: an array of
  `{ slug, title, category, tags[], summary, related[], headings[] }`, one entry
  per sub-page. A shared `wiki.ts` reads it and, on every wiki page, renders the
  chrome into placeholder divs: the category-grouped **sidebar** (marks the
  current page active), the **tag chips**, the **"See also"** block (explicit
  `related` plus auto "shares a tag"), and a **search** box that filters the
  manifest (title / summary / tags / headings) live. The wiki **index** is just
  the manifest rendered as category sections of cards. Each sub-page HTML holds
  only its real article content (good for SEO / no-JS reading) plus the
  placeholder divs. Adding a page = author its HTML + one manifest entry (+ the
  mechanical webpack registration). Every wiki feature is a view of the one
  graph - which is exactly the Obsidian-brain model.
  - Search: client-side, no dependency - a small substring/fuzzy matcher over
    the manifest fields. Cheap because the corpus is ~10 pages; no index server,
    no lib.
  - Categories/tags: `category` drives sidebar grouping; `tags` render as chips
    and are clickable to pre-fill/filter search.
  - See also: `related` slugs are explicit; the renderer also appends pages that
    share >=1 tag, deduped, capped.

- **C. Build-time generation from the manifest.** A webpack step generates each
  page's HTML from the manifest + a content file. Maximally DRY but adds a
  templating layer and moves content out of the natural HTML; more infra than a
  10-page wiki needs right now. B keeps content in HTML and gets the same
  consistency for the chrome only, which is where duplication actually hurts.

- **Do nothing / keep one page.** Rejected: the user explicitly wants the split
  and the sub-page structure; one page does not scale to the content or give the
  wiki feel.

## Recommendation

Adopt **Option B**: a `wiki-pages.ts` manifest as the single source of truth,
with a shared `wiki.ts` that injects the sidebar, search, tag chips, and
see-also into placeholder divs on every wiki page, and a manifest-rendered wiki
index. Keep article content in each sub-page's HTML. This gives a genuinely
interlinked, consistent wiki where adding or renaming a page is a one-line
manifest edit, and search/tags/related come for free - the "second brain"
property the user asked for - while staying within the site's existing
static-HTML + placeholder-injection pattern and the sharp visual style.

Trade-offs accepted: the sidebar/search/tags/see-also render client-side (no-JS
users still get the full article and the header/footer, just not the wiki
chrome), and the 10 sub-pages are still 10 mechanical webpack registrations (a
small `wikiPage()` helper in the config keeps that to one line each).

## Open questions

- Search depth: manifest fields (title/summary/tags/headings) are almost
  certainly enough for ~10 pages; full body-text indexing can come later if it
  feels thin. Start shallow.
- Tag taxonomy: keep the initial tag set small and controlled (e.g. `flight`,
  `combat`, `ships`, `world`, `ui`, `modding`) so chips stay meaningful; grow
  deliberately.
- Sidebar on mobile: collapses to a top dropdown / disclosure; settle the exact
  affordance during work.
- Content accuracy: the sub-page copy must be pulled from the actual code
  (keybinds, section list, faction rules, HUD tiers, damage types) - a work-phase
  step, not an architecture question.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260713-225324: wiki infrastructure - `wiki-pages.ts` manifest, the
  `wiki.ts` chrome injector (sidebar + search + tags + see-also + active), the
  wiki layout CSS, the manifest-rendered wiki index, and a `wikiPage()` webpack
  helper. Deliver with 1-2 real sub-pages to prove it. (Foundation - do first.)
- tatr 20260713-225333: trim `tutorial.html` to first-scenario onboarding only
  (Shakedown Run + how to start playing); its reference tables move to wiki
  pages. (Independent of the infra task.)
- tatr 20260713-225338: author the gameplay-system pages (Sections, Keybinds,
  HUD, Flight & autopilot, Targeting & radar, Combat & weapons, Gravity wells),
  content pulled from the code. (Depends on 225324.)
- tatr 20260713-225353: author the world/meta pages (Factions, Scenarios,
  Modding coming-soon), content pulled from the code. (Depends on 225324.)

## Fix record

- 20260713-225324 (infra) LANDED: `wiki-pages.ts` manifest + `wiki.ts` chrome
  injector (sidebar, live search, tag chips, see-also), wiki-layout CSS, the
  manifest-rendered index, and a `wikiPage()` webpack helper. Shipped the index
  + two real sub-pages (Ship sections, Keybinds); the other eight are coming-soon
  stubs (non-navigable) for the content tasks to fill. See
  tasks/20260713-225324/TASK.md.
- 20260713-225333 (tutorial trim) LANDED: cut the four reference tables from
  the tutorial (they live in the wiki Keybinds page) and pointed to the wiki;
  the tutorial now reads as a guided first hour (intro, menu, Shakedown Run).
  Follow-on: audited the real gamepad bindings from the input code and corrected
  the Keybinds page (burn = right bumper, free look = left bumper, combat also
  binds left trigger, turret/torpedo fire is editor-rebindable, not "RT2") while
  giving every gamepad control a PromptFont glyph.
- 20260713-225338 (gameplay pages): pending.
- 20260713-225353 (world/meta pages): pending.
