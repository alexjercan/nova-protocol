# Wiki nav: preserve drawer scroll position across navigations/refresh

- STATUS: OPEN
- PRIORITY: 38
- TAGS: feature,web

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

- [ ] Add a `persistNavScroll(nav)` helper in `wiki.ts` (restore + throttled
      save + pagehide save, try/catch); call it after `renderSidebar`.
- [ ] Verify: `npm run ci` green; serve + manually confirm the drawer holds its
      scroll across a link click and a refresh (scroll down, click a page, and
      reload - the drawer stays put); no console errors; check the active link is
      still reachable.

## Notes

Small client-only change to `web/src/wiki.ts`; no manifest/CSS change expected.
Keep it dependency-free (plain sessionStorage + rAF).
