# Story draft review

## Intake: visual direction accepted

The user approved the four-page PoC's visuals and requested:

- Fuller dialogue.
- Cinematic-style orientation cards inside Baikal and on entering Kaveri.
- One story-development task containing the comic artifacts and future work.
- Confirmation that the other `web/design/` studies are website inputs.

The location/year example was illustrative. It did not choose a moon, orbit,
calendar, or year. Kaveri retains its established spelling.

## Artifact move

All seven PoC files moved byte-for-byte from `web/design/season-1-opening/` to
`comic-opening-poc/` before this revision. The generator still resolves the
repository root at the same relative depth and reads the existing portrait
source without changing it. Regeneration commands and documentation links were
updated for the new location.

[Baseline evidence](proof/baseline/README.md) retains the accepted visual pass.
[Current preview](comic-opening-poc/index.html) is the working revision. The
shared outline remains in lore; durable source documents use active task TODOs
rather than links to task artifacts.

## Dialogue and orientation pass

The revision keeps the palette and page sequence. It gives the characters room
to explain the repair tradeoff, the pickup, and the water-order dependency, while
retaining a small everyday exchange around the mug. It does not add combat,
new ships, an incompetent captain, or a residential supply crisis. The last
exterior remains silent.

Two dark green orientation cards contrast with the cream speech balloons:

| First appearance | Place | Draft time | Context |
| --- | --- | --- | --- |
| Page 1, first interior | Baikal / Saturn system | Opening day / calendar date TBD | Clearwell Waterworks' main base |
| Page 4, aboard Kaveri | Kaveri / departing Baikal | Later that day / calendar date TBD | Clearwell Waterworks' workship |

The relative times are draft staging, not a chosen calendar or final timing.
The exact year is explicitly unresolved. Cards remain visible in Art only mode
and appear in the plain-text transcripts.

## Implementation checks behind the answers

- `web/webpack.config.js:301-324` explicitly copies the menu, HUD, and NOVA OS
  design studies to `nova-menu/`, `nova-hud/`, and `nova-os/`. The directory is
  not universally throwaway. The comic PoC was not one of those copy inputs.
- `crates/nova_scenario/src/actions/cinematic.rs:141-164` defines the game's
  `CinematicTitle` as location, free-form date/stamp, note, corner, and duration.
  The comic borrows the where/when/context treatment, not a new shared runtime
  schema. The example game dates are not story canon.
- `web/src/lore/places/baikal.md` leaves the exact orbit open. The shared season
  outline establishes relative settlement age, not an absolute calendar date.

## Verification

Post-move checks passed:

- Generator `--check`; SVG parsing, unique IDs, and paint references.
- Exactly two orientation cards, in the page 1 and page 4 interiors.
- Four inspected full-size page renders and eight page/viewport views at 1440
  and 390 px; local-file loading, navigation, keyboard input, Art only, contact
  sheet, and transcripts.
- Browser lettering bounds: one overlong comms line was found and wrapped.
  Reinspection passed. The cockpit console was lowered locally to keep Tomas's
  face visible beneath the longer dialogue.
- Lore tests: 28 pages and 708 source links. Comic catalog tests passed.
- Production/task Markdown rendering and source links. No task-artifact links
  remain in durable lore documents; active TODOs identify this work.
- Existing public portrait outputs remain unchanged.
- Website build under `/nova-protocol/`: only the Reader demo is in the archive;
  no task pages or draft date text are published. The three existing UI design
  studies still reach their build destinations byte-for-byte.
- Protected source hashes report no changes outside this request.

Spoken dialogue increased from 46 to 130 words across the four pages. Counts
exclude signs and orientation cards; the generated transcript uses the same
source strings as the lettering.

Evidence: [browser checks](proof/revised/browser-checks.json),
[revision metrics](proof/revision-metrics.json),
[publication and scope](proof/publication-and-scope.json), and
[build log](proof/build.log). The screenshots are in `proof/revised/`; baseline
images remain separate. Run `node tasks/20260908-161328/proof/inspect.mjs` from
the repository root to repeat the local browser check with Chromium on PATH.
Only affected checks ran, not full website CI or workspace Rust checks.

## Follow-up review

The user gave feedback on this pass and requested a clean-line trial, shared
asset sources, color schemes, and a central palette. See the current
[style and library review](STYLE-REVIEW.md). The richer dialogue and card
treatment remain a draft, not a final script. Then choose the date convention: Earth-calendar dates or an epoch
such as years since Keystone's founding. Earth-calendar dates are recommended
for a shared Earth-Saturn chronology; a settlement epoch would emphasize the
colony's age but need more explanation. No option has been adopted yet.

The task remains OPEN. No publication or commit was requested.
