# Wiki infrastructure: manifest, sidebar, search, tags, see-also, index

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: web,wiki,design

## Goal

Build the reusable wiki chrome so the ~10 sub-pages stay consistent and adding
a page is a one-line manifest edit. Per the spike's Option B (manifest-driven,
JS-injected chrome).

Deliver:

- `web/src/wiki-pages.ts` - the manifest: an array of
  `{ slug, title, category, tags[], summary, related[], headings[] }`, one entry
  per planned sub-page (Sections, Keybinds, HUD, Flight & autopilot, Targeting &
  radar, Combat & weapons, Gravity wells, Factions, Scenarios, Modding). Modding
  flagged coming-soon.
- `web/src/wiki.ts` extended to render, into placeholder divs on each wiki page:
  a category-grouped **sidebar** (active page marked), a **search** box that
  live-filters the manifest (title / summary / tags / headings), **tag chips**
  (clickable -> filter search), and a **See also** block (explicit `related`
  plus auto "shares a tag", deduped/capped).
- Wiki layout CSS (sidebar + content two-column; collapses on mobile), in the
  sharp style; reuse `.figure` placeholders for images.
- The wiki **index** (`wiki.html`) rendered from the manifest as category
  sections of cards (replacing the current one-page reference dump).
- A `wikiPage()` helper in `web/webpack.config.js` that registers a sub-page
  (HtmlWebpackPlugin entry + historyApiFallback rewrite) in one line.
- Prove it end to end with 1-2 real sub-pages wired through the manifest (e.g.
  Sections + Keybinds) so the pattern is demonstrated for the content tasks.

Done when: the wiki index lists all planned pages by category, the sample
sub-pages show the sidebar / search / tags / see-also, search filters live,
`cd web && npm run ci` is green, and a build render confirms it.

## Notes

- Spike: tasks/20260713-225157/SPIKE.md (source of truth for the architecture,
  the manifest shape, and the rejected options). Read before planning.
- Precedent: header/footer are injected into `#header`/`#footer` placeholder
  divs by `HtmlPartialsPlugin`; the wiki chrome follows the same
  placeholder-injection idea but from the manifest via `wiki.ts`.
- Keep the initial tag taxonomy small and controlled (flight, combat, ships,
  world, ui, modding).
- Foundation task: the two content tasks (225338, 225353) depend on this.
