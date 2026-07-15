# Retro: wiki nav audience bands

- TASK: 20260715-215924
- OUTCOME: shipped (landed a8166fbf); review APPROVE, 1 MAJOR (stale page) fixed.

## What went well

- Confirming the design (band tier + labels + the exact re-mapping) with the user
  BEFORE building meant the implementation was mechanical and landed in one pass -
  the preview they picked was the spec.
- The data model (WIKI_SECTIONS = ordered bands of categories, WIKI_CATEGORIES
  derived) kept the change small: the sidebar/index renderers just gained an outer
  loop, and the search filter one extra pass to hide empty bands.
- Re-homing the modding page surfaced a real content bug (it still claimed RON
  modding was "not shipped yet"). Moving a page is a good moment to re-read it -
  the audience change forced the re-read that caught the stale premise.

## What went wrong

- The category re-bucket + reorder was one large manifest-tail Edit. It worked,
  but the safety check that would have caught a mistyped category instantly (every
  page.category in WIKI_CATEGORIES -> otherwise the page silently drops from the
  nav) was run at the END. On a category-driven nav, that check is the cheap guard
  and belongs first.

## Lessons

- `orphaned-category-check-first`: on a nav/index driven by a category (or any
  key) field, a page assigned a key outside the known set silently disappears -
  no build error. After any re-bucketing, assert every page's key is in the
  canonical set (`grep -oE 'category: "[^"]+"' | sort -u` vs the band list) as the
  first verification, not the last. 20260715-215924.
- `re-home-is-a-re-read`: moving a doc to a new audience/section is the moment its
  premise most often turns out stale - the player modding page still said modding
  "isn't shipped yet". Re-read a page when you move it, not just when you rewrite
  it. 20260715-215924.
