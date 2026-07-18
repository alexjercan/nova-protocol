# Fix horizontal scroll overflow on wiki guide-make-a-mod page

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.7.0,bug,web

## Context

The wiki page `/wiki/dev/guide-make-a-mod/` overflows to the right and forces a
page-level horizontal scrollbar (scroll-X). Other wiki pages do not do this.
The user wants this page to look like the others: content constrained to the
column, no page scroll-X.

## Diagnosis

- The wiki layout is a CSS grid: `.wiki { grid-template-columns: 232px 1fr }`
  in `web/src/style.css` (~line 1001). The article column is the `1fr` track.
- The article element `.wiki__body.prose` (`<article>` in `web/src/wiki.html`)
  is a direct grid child, so it defaults to `min-width: auto`. A grid/flex item
  with `min-width: auto` refuses to shrink below its content's intrinsic size.
- `guide-make-a-mod.md` has RON code blocks with very long unbreakable lines
  (e.g. a 164-char `description:` string). `.prose pre` has `overflow-x: auto`,
  but because the column will not shrink below the `pre`'s intrinsic width, the
  `1fr` track blows out past the viewport instead of the `pre` scrolling
  locally. That is what forces page scroll-X.
- Other pages have no such long code lines, so their text wraps naturally and
  the column stays within the viewport - which is why only this page breaks.

## Fix

Add `min-width: 0` to the `.wiki__body.prose` grid item so the `1fr` column is
constrained to its track. Then `.prose pre { overflow-x: auto }` works as
intended: the code block gets its own local scrollbar and the page no longer
scrolls horizontally. This is the canonical fix for the grid/flex
`min-width: auto` overflow trap and needs no content changes.

## Steps

- [x] In `web/src/style.css`, add `min-width: 0;` to the `.wiki__body.prose`
      rule (~line 1132).
- [x] Build the web app (or run its dev/lint/check) to confirm no CSS/build
      regressions. `npm ci && npm run build` -> webpack compiled successfully.
- [~] Verify on `/wiki/dev/guide-make-a-mod/` that the page no longer forces
      horizontal scroll and the code block scrolls locally, and spot-check a
      normal wiki page (e.g. getting-started) is unchanged. No headless browser
      available in this environment, so pixel-level browser verification was not
      run. Confidence is high: root cause is the classic grid-item
      `min-width: auto` overflow trap, the fix mirrors the existing
      `.wiki-child__body { min-width: 0 }` guard (style.css ~1285), and the
      build is green. Needs a quick human eyeball on localhost after rebuild.
