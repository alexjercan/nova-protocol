# One Story library

## Delta

The user approved one browsing page instead of repeated library and season
indexes. `/story/` now lists seasons and episodes in reading order. Each episode
has one clickable row and one thumbnail. Draft status reports illustrated pages
against the complete count.

Removed Preview, Start, Latest, Season contents, the extra Read label, and the
large repeated season cover. Selected season URLs are small, chunk-free redirects
to `/story/#<season-id>`. The reader returns there through Story; Pages opens the
existing page chooser. Library anchors account for the responsive header height.

Panels, Transcript, exports, page/episode controls, publication selection, and
story/art sources are unchanged. Documentation and the single unreleased Story
entry describe the new layout, rather than keeping the superseded design.

## Verified

Evidence is under [proof/navigation/](proof/navigation/). Earlier proof is frozen;
this batch uses new build and browser harness copies, not earlier proof writers.

- Targeted format, lint, type, catalog/route, and DSL/build tests pass.
- Tests cover multiple seasons, numeric season order, authored episode order,
  one entry per episode, one thumbnail per row, escaping, and redirects without
  reader chunks or a duplicate list.
- Root and `/nova-protocol/` local builds pass browser checks at 1440 and 390 px.
  An episode opens directly by keyboard. Story returns to its library section;
  season URLs redirect there; Pages still opens and closes its chooser.
- An eight-section browser fixture checks that later season anchors stay below
  the responsive header. These cloned fixture sections are not authored seasons
  or build output. The test waits for the site's smooth scroll to finish.
- All seven full-page exports still match the accepted renders pixel for pixel.
  Panel exports, modal focus/scrolling, reload position, measured layout results,
  and safe SVG rejection remain covered. Layout checks UI stays absent.
- Root and prefixed public builds retain the empty archive. No draft episode,
  season redirect, asset, or script data enters them.
- The mdBook build passes with the existing Mermaid 0.5.0/0.5.2 warning.

The redirect initially caused a browser request for an absent `favicon.ico`.
It now names the existing prefix-aware SVG favicon; the browser checks pass.

No web server or browser debugging port was started. Full website CI, Rust
checks, commit, push, publication, and live server reload were not requested or
performed. The episode remains a seven-page illustrated draft of eighteen pages.

## Next

Restart an existing development server to load the build-template changes. The
remaining story and illustration work stays open under the same task.
