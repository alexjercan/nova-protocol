# Four-page clean-line adaptation

The user requested commits for the reviewed work and application of the accepted
visual style to the original four pages. This review covers that adaptation,
not new story, publication, or approval of the resulting pages by the user.

[Open the restyled pages](comic-opening-poc/index.html) or
[compare the previous version](proof/before-clean-line-opening/index.html).

## Delta

- Restaged all ten panels with closer frontal people, light shadows, fine
  linework, cropped workplace details, and full comic color.
- Page 1 keeps the inhabited exterior, coffee, game-night notice, and interruption.
- Page 2 brings the pump and people close. Rear pipes no longer cover Leila's
  face. The stopped industrial line and normal residential supplies remain clear.
- Page 3 retains the ship-through-window composition, Elena's working gesture,
  the delivery record, and the mug joke. Jonah's final tail points off-panel,
  not into the mug.
- Page 4 keeps Tomas visible above the console, then gives the silent departure
  to the shared plated Kaveri. Ebro and Baikal remain behind it.
- The same dialogue, speaker labels, display/record text, card facts, page titles,
  and departure endpoint remain. Wrapping and panel proportions can change.

The prior seven draft files were copied byte-for-byte before editing; the source
and README are retained as `.txt` evidence. Their hashes are in
[the archive manifest](proof/before-clean-line-opening/sources.json). The first
short-dialogue baseline and all earlier PNG evidence also remain untouched.

## Source ownership

The library now owns Leila, Rina, and Tomas's original frontal head data and
work-jacket busts, as well as the existing Jonah/Elena drawings. The public
portrait exporter imports those shared heads. All five public crew portrait SVGs
remain unchanged. Samir's head and the legacy template colors remain unmigrated.

The page generator owns composition, the pump, delivery record, and lettering.
`opening_props.py` shares the incidental mug-and-hand drawing between this
opening's two studies. It remains task-local, not a series-wide prop catalog.
New character and workplace colors live in the central palette. Shared ships,
scenery, the three accepted study SVGs, and the lore shader were not redrawn.

## Verified

- [Fifteen illustration tests](proof/restyled/unit-tests.txt).
- Deterministic checks for both private drafts and both public exporters.
- [Full website CI](proof/restyled/web-ci.txt): formatting, lint, all website
  tests, and root build. Lore remains 28 pages / 724 links.
- [Browser checks](proof/restyled/browser-checks.json): four raw SVGs and eight
  desktop/mobile page views; lettering bounds, navigation, keyboard, Art only,
  contact sheet, transcripts, local comparison links, and file-only requests.
  Art only keeps the cards visible. The owned browser exited.
- Inspected the four rendered pages, corrected the occluded face and misplaced
  tail, and reran the browser check. Current PNGs are in `proof/restyled/`.
- [Source and scope checks](proof/restyled/source-and-scope.json): ten panels and
  130 spoken words; all dialogue, displays, records, and cards unchanged; frozen
  and public assets unchanged; SVG IDs/references and palette ownership checked.
- [Markdown rendering and local links](proof/restyled/markdown-checks.txt).
- The built site copies the lore SVGs exactly, contains no private draft markers,
  and discovers only the Reader demo. No public article, game, wiki, other task,
  or changelog change belongs to the restyle.

Concurrent `.scufris.toml` commits were inspected and preserved. The index was
empty between commit operations. No Rust checks, new game capture, push, or
publication was performed.

## Follow-up: Baikal's first picture

At the user's request, Baikal's unchanged orientation card now appears on the
opening exterior instead of the following interior. Kaveri's card stays where
it was. No faces, scene art, dialogue, or other page layouts changed.

The [new page 1](proof/first-panel-card/page-01.png),
[browser checks](proof/first-panel-card/browser-checks.json), and
[card-only comparison](proof/first-panel-card/change-checks.json) record this
follow-up. Regeneration, lettering, first-panel placement, and desktop/mobile
checks passed. The earlier restyle evidence remains untouched. That card-move
pass used affected checks only, with no commit or publication.

The user then said "looks good", requested a commit, and asked to focus the
existing task on writing season one. The accepted art remains the working
baseline. See [COMMIT-REVIEW.md](COMMIT-REVIEW.md) for the commit checks.

## Next

Write the shared season story under [the existing task](TASK.md), starting from
the approved progression. Character-specific expressions remain a proposed
illustration follow-up, not a completed pass or a prerequisite for writing.
