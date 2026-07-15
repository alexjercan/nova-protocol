# Wiki nav: audience bands (For players/creators/developers) above categories

- STATUS: OPEN
- PRIORITY: 42
- TAGS: feature,docs,web

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

- [ ] Add `WIKI_SECTIONS` + re-bucket categories in `wiki-pages.ts`; re-assign
      page categories; decide the player `modding` page's fate + repoint links.
- [ ] `wiki.ts`: render band headers in the sidebar and index; make search hide
      empty bands.
- [ ] `style.css`: band-header styles (sidebar + index).
- [ ] Verify: `npm run ci` green; serve + headless-eyeball the sidebar (three
      bands, correct pages under each) and the index; search still filters; check
      at the deploy subpath.

## Notes

UI/IA change only, on the markdown wiki pipeline (20260715-195621 / -204358).
No page content changes beyond `category` reassignments (+ maybe folding the
player modding page). Keep the sharp house style.
