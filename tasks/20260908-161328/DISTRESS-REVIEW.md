# Episode one: pages 8-10

## Delta

Illustrated the next three accepted script pages in the real reader:

- **8, A familiar voice:** Nadia's distress call interrupts the return. Samir
  records her report while Tomas starts route work and Jonah requests the
  reserve and port information. The viewpoint stays aboard Kaveri.
- **9, What it will cost:** Tomas explains the arrival ordering and the delay
  to Baikal's repair. Rina and Leila assess the transfer with the restrained
  assembly still outside the pressure window. Tomas's warning gets a serious
  answer, not a villain treatment.
- **10, The decision:** Elena asks EarthWorks to cover the diversion while
  preparation continues. Samir monitors the labelled audio exchange with
  Daniel. Elena then commits Clearwell to the cost; Jonah gives the instruction
  from a shot framed at Tomas's station.

The three TS files now own illustrated panels and measured balloon placement.
Nine named scenes live in `art/distress.py`. All action, dialogue, ids, page
purposes, and reading order remain exact. New screen labels also live in TS.
There are still eighteen pages and 730 spoken words; ten pages are illustrated.

Reused the accepted frontal heads, original expressions, shared colors, pressure
window treatment, handholds, and assembly model. No new Daniel likeness, pirate
cutaway, numerical countdown, orbit, cargo sacrifice, or engineering standard
was introduced. The arrival display is explicitly an ordering, not a scale.

This is new draft artwork for user review, not approval or publication.

## Verified

Evidence: [proof/distress/](proof/distress/).

- Static local builds and CDP-pipe inspection at root and `/nova-protocol/`, at
  desktop and phone widths. No server or listening port was started.
- All ten pages compose and export. Twenty measured layouts per prefix have no
  balloon, text-bound, or face-contour issues. All 28 raw and lettered panels
  load. Earlier seven page exports remain pixel-identical at both prefixes.
- Season/episode navigation, browser history, page hashes, exports, modal focus
  and scrolling, automatic wrapping, and unsafe SVG rejection still pass.
- First-pass inspection found obscured faces and screen labels, detached-looking
  forearms, and two new balloon/face overlaps. The new scenes were reframed,
  consoles separated, and the final instruction narrowed before the clean run.
  The shared diagnostics and all accepted earlier artwork were left unchanged.
- Targeted Prettier, ESLint, catalog/DSL tests, and 37 illustration tests pass.
  Root/prefix released builds remain empty archives with no private episode
  routes, artwork, or dialogue.
- Source checks compare every action and spoken line with the starting script,
  keep pages 11-18 scripted, validate the nine raw scenes, and verify retained
  source/proof hashes. The seven whole-season writing requirements stay open.

The user's concurrent task-priority and backlog work is outside this batch.
No full website CI, Rust check, commit, tag, push, or deployment was requested or
performed for these private illustrations. Existing proof writers were not rerun.

## Next

Review the new pages at `/story/season-1/episode-1/#page-8` in normal local
serving. Pages 11-13 continue through the intercept, damage assessment, and
port approach. The rescue remains noncombat and the spare remains aboard.
