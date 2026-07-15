# Review: preserve wiki drawer scroll

- TASK: 20260715-222307
- BRANCH: feature/wiki-nav-scroll

## Round 1

- VERDICT: APPROVE

Small, dependency-free client change in `web/src/wiki.ts`: `persistNavScroll`
restores `#wiki-nav.scrollTop` from sessionStorage on load and saves it on scroll
(rAF-throttled) + pagehide, guarded by try/catch. Wired after `renderSidebar`.

- `npm run ci` green.
- End-to-end (can-fail) test with puppeteer-core driving the real chromium:
  scrolled the drawer to 320, navigated to another wiki page, and reloaded ->
  `{set:320, stored:"320", afterNav:320, afterReload:320}` PASS. Removing the code
  would leave the drawer at 0, so the test is not vacuous.
- Correct storage choice: sessionStorage survives same-tab navigations + reloads
  and clears on tab close - the right lifetime; a single key is fine because the
  drawer is identical on every wiki page.
- Degrades cleanly: storage guarded for private mode; on mobile the nav is not a
  scroll container so scrollTop stays 0 (harmless no-op).

No findings.
