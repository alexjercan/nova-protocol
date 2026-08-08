# Wiki nav: audience bands (For players/creators/developers) above categories

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: feature, docs, web

User request: the dev categories (Get started / Understand / Extend the game)
don't visibly signal they are developer material vs the player game-manual pages.
Segment the wiki nav by the three "views of interest" the user named, as a top
tier above the existing categories, and pull the content-authoring pages into a
creator band so the developer band is pure engine/code. Confirmed labels:
"For players" / "For creators" / "For developers".

## Target structure

- **For players**: Ships & building, Flying, Combat, Interface, World (unchanged
  player categories - Factions + Scenarios stay under World).
- **For creators**: new category "Scenarios & mods" =
  [Author a scenario (RON), Make and publish a mod, Modding data format (RON),
  Mod portal].
- **For developers**: Get started [Building & running, Project tour],
  Architecture [Architecture, Ship sections (internals), Scenario engine],
  Extending [Add a ship section, Extend the scenario engine].

The old dev categories "Understand" and "Extend the game" are replaced by
"Architecture", "Extending", and the creator "Scenarios & mods"; "Get started"
stays. The player "Modding" coming-soon page is now redundant with the creator
band - fold it (decide in-flight: remove + repoint `../modding/` links, or keep
as a lead-in under Scenarios & mods).

## Implementation

- `web/src/wiki-pages.ts`: introduce a section (band) tier -
  `WIKI_SECTIONS: { name: string; categories: string[] }[]` in band order, with
  each band's ordered categories. Derive/replace `WIKI_CATEGORIES` from it. Re-
  assign the `category` on the moved dev/creator pages per the target above.
- `web/src/wiki.ts`: `renderSidebar` iterates `WIKI_SECTIONS`, emitting a band
  header (`.wiki-nav__section`) above each band's category groups (keep the
  existing per-category group rendering + search filter, which must now also
  hide/show band headers when all their categories are filtered out).
  `renderIndex` gets the same band headers above its category card grids.
- `web/src/style.css`: a `.wiki-nav__section` style (stronger than
  `.wiki-nav__cat`: uppercase, a top rule/spacing) and an index band heading.
- Sweep for any code/doc that references the removed category names or the
  player modding page; update links.

## Steps

- [x] Add `WIKI_SECTIONS` + re-bucket categories in `wiki-pages.ts`; re-assign
      page categories; the player `modding` page moves into "Scenarios & mods"
      (kept, rewritten).
- [x] `wiki.ts`: render band headers in the sidebar and index; make search hide
      empty bands.
- [x] `style.css`: band-header styles (sidebar + index).
- [x] Verify: `npm run ci` green; headless-eyeball the sidebar (three bands) and
      index; no orphaned categories (all 9 map to a band); 26 pages emit.

## Close notes

### What changed
- `wiki-pages.ts`: introduced `WIKI_SECTIONS` (the three audience bands, each an
  ordered list of categories); `WIKI_CATEGORIES` is derived from it. Replaced the
  flat dev categories (Understand / Extend the game / player Modding) with
  "Scenarios & mods" (creators) and "Architecture" + "Extending" (developers),
  and reordered the tail so array order = nav order (guides before the RON/portal
  reference in the creators band).
- `wiki.ts`: `renderSidebar` and `renderIndex` iterate `WIKI_SECTIONS`, emitting a
  band header above each band's category groups; the search filter also hides a
  band header when all its categories filter out.
- `style.css`: `.wiki-nav__section` (amber, top rule) and `.wiki-index__band`
  (large amber heading) set the bands apart from the cyan category labels.

### The stale modding page
The player `modding` page claimed the RON modding format was "planned, not
shipped yet" - false since the format landed. Rewrote it as the creators-band
front door (author a scenario / package a mod / extend the engine, each linking
its guide) and fixed the matching "authoring is on the way" line in
`scenarios.md`. A real content bug caught while re-homing the page.

### Self-reflection
The reshuffle was one big Edit of the manifest tail; the cheap guard is an
orphaned-category check (every page.category in WIKI_CATEGORIES) - ran it at the
end, worth running first next time.

## Notes

UI/IA change only, on the markdown wiki pipeline (20260715-195621 / -204358).
No page content changes beyond `category` reassignments (+ maybe folding the
player modding page). Keep the sharp house style.
