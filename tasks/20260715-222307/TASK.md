# Wiki nav: preserve drawer scroll position across navigations/refresh

- STATUS: CLOSED
- PRIORITY: 38
- TAGS: feature, web

User request: the wiki sidebar "drawer" resets to the top on every page
navigation and refresh. Keep it scrolled where the reader left it.

## Why

Each wiki page is a full HTML document; the drawer (`#wiki-nav`, the
`.wiki__nav` aside - its own `overflow-y: auto` scroll container) is re-rendered
from scratch by `wiki.ts` on every load, so its scrollTop starts at 0. Browsers
do not restore scroll for an inner overflow element.

## Approach

Persist the drawer's `scrollTop` in `sessionStorage` (per-tab; survives same-tab
navigations and reloads, clears on tab close - the right lifetime) under a single
key (the drawer is identical on every wiki page). In `web/src/wiki.ts`, after
`renderSidebar` populates the nav:
- restore `nav.scrollTop` from the saved value;
- save it back on scroll (rAF-throttled) and on `pagehide` (covers a nav that
  happens between throttled saves);
- wrap storage access in try/catch (private mode / disabled storage).

Mobile layout makes `.wiki__nav` static (no inner scroll), so `scrollTop` stays 0
and restore is a harmless no-op there.

## Steps

- [x] Add a `persistNavScroll(nav)` helper in `wiki.ts` (restore + rAF-throttled
      save + pagehide save, try/catch); call it after `renderSidebar`.
- [x] Verify: `npm run ci` green; a puppeteer-core e2e drove the real chromium -
      scrolled the drawer, navigated to another wiki page, and reloaded; the
      drawer restored its position in both cases.

## Close notes

Added `persistNavScroll(nav)` to `web/src/wiki.ts`, called right after
`renderSidebar`. It restores `#wiki-nav.scrollTop` from `sessionStorage` on load,
saves it back on scroll (rAF-throttled) and on `pagehide`, all wrapped in
try/catch for private-mode / disabled storage. One key (`wiki-nav-scroll`) - the
drawer is identical on every wiki page. sessionStorage is deliberate: it survives
same-tab navigations and reloads but clears on tab close, which is the right
lifetime for "where I left the drawer this session".

### Verification (e2e, can-fail)
A puppeteer-core script against the dev server: scroll `#wiki-nav` to 320, let the
rAF save fire, then (a) navigate to `/wiki/keybinds/` and (b) reload. Result:
`{set:320, stored:"320", afterNav:320, afterReload:320}` -> PASS. Without the
persistence code the drawer renders at 0, so the test genuinely fails-without-it.

### Self-reflection
Reached for a real browser driver (puppeteer-core over the existing chromium, no
download) instead of eyeballing - the right call for a behavior that a screenshot
can't show. Worth keeping that puppeteer-core recipe handy for future
client-behavior tasks.

## Notes

Small client-only change to `web/src/wiki.ts`; no manifest/CSS change expected.
Keep it dependency-free (plain sessionStorage + rAF).
