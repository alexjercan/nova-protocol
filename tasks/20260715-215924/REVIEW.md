# Review: wiki nav audience bands

- TASK: 20260715-215924
- BRANCH: feature/wiki-audience-bands

## Round 1

- VERDICT: APPROVE

UI/IA change: a three-band audience tier over the categories, plus a manifest
re-bucketing. Verified:

- No orphaned pages: every distinct `category` used in `wiki-pages.ts` (9) is one
  of the categories listed in `WIKI_SECTIONS`; all 26 pages emit. A page dropped
  into a category outside the bands would silently vanish from the nav - checked,
  none did.
- Headless render: the sidebar shows For players / For creators / For developers
  as amber band headers with the categories nested under each; the creators band
  is guides-first (Modding, Author a scenario, Make and publish a mod, Modding RON,
  Mod portal); the index shows the same bands over its card grids.
- `npm run ci` green (format + eslint + build); WIKI_CATEGORIES is now derived, no
  dangling import (wiki.ts imports WIKI_SECTIONS).
- Search band-hiding: `renderSidebar`'s filter sets each band header hidden unless
  one of its categories is still visible - logic-verified against the per-category
  visibility it already computes.

Content correctness caught in-scope:

- [x] R1.1 (MAJOR, fixed) The player `modding` page asserted the RON modding
  format is "planned, not shipped yet" - false; the format shipped (the whole
  creators band documents it). Rewrote the page as the creators-band front door
  and fixed the matching stale "authoring is on the way" line in `scenarios.md`.
