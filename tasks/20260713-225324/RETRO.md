# Retro: wiki infrastructure (20260713-225324)

- DATE: 20260713-225324
- VERDICT: APPROVE (2 review rounds)

## What went well

- The spike's Option B (a `wiki-pages.ts` manifest as single source of truth)
  paid off immediately: the sidebar, live search, tag chips, see-also, and the
  index are all just views of the one array, so they cannot drift, and adding a
  page is a one-line manifest edit + its HTML + a `wikiPage()` registration.
- The `comingSoon` flag turned an awkward "manifest lists 10 pages, only 2
  exist" problem into a feature: unbuilt pages show as non-navigable stubs, so
  the wiki already reads as a complete map and the content tasks just flip the
  flag as each page lands. No throwaway scaffolding.
- Flow's "update the branch from master before landing" step earned its keep: a
  parallel job advanced master mid-task; merging it into the branch (an
  unrelated Rust examples refactor, no conflict) and re-verifying kept the land
  clean.

## What went wrong / difficulties

- Review caught a real 404 bug: the first cut rendered navigable sidebar and
  see-also links to the eight not-yet-authored pages. Caught before landing, but
  it would have shipped dead links across the whole wiki.
- The wiki chrome is entirely client-rendered, and `npm run ci` only checks
  build/lint/format - it never exercises the DOM logic (slug parsing, sidebar
  render, search filter). With no headless-DOM tooling installed, runtime went
  hand-reviewed, not machine-verified; recommend an eyeball on the served site,
  or adding jsdom/Playwright if the wiki JS grows.

## Lessons

- `generated-links-need-real-targets`: links rendered from a data manifest must
  be gated on the target actually existing (or explicitly marked unavailable),
  or they 404. Here the manifest listed planned pages before their HTML existed;
  the fix was a `comingSoon` flag that renders those as non-links.
- `ci-skips-client-render`: a build-only CI proves the bundle compiles, not that
  client-rendered UI works; client-side DOM logic needs a runtime check (headless
  DOM or a manual eyeball), which the green build does not provide.

## Follow-ups

- Content tasks 20260713-225338 (gameplay pages) and 20260713-225353 (world/meta
  pages) fill the eight coming-soon stubs; each flips `comingSoon` off, adds the
  HTML, and appends its slug to `WIKI_SLUGS` in webpack.
- Task 20260713-225333 trims the tutorial (its keybind/targeting/weapons tables
  now have a home in the wiki).
