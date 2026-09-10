# Complete episode one: A useful job

The user accepted the complete episode and requested task completion, removal
of its draft mentions, and a commit. This record accompanies that source commit,
starting from `88a7445b416575f16ca7ab16fddaaa7735a6fad6`.

## Completed scope

- All eighteen pages, fifty panels, and 730 spoken words are approved.
- The commit includes the previously uncommitted pages 11-18, their shared poses,
  episode-owned equipment, continuity tests, documentation, and frozen reviews.
- [Episode metadata](../../web/src/comics/season-1/episode-1/episode.json) now says
  `released`. Normal released-only builds include the episode. No build or reader
  exception hides its former draft state.
- [Episode framing](../../web/src/comics/season-1/episode-1/episode.ts) now reads
  `SEASON 1 / EPISODE 1` and `A USEFUL JOB`. The former private/provisional text is
  gone from displayed and exported pages. The library and reader no longer show
  draft labels or the draft-only Art only control.
- All page purposes, actions, dialogue, lettering, layouts, and raw art are exact
  relative to the accepted eighteen-page version. No new story choice is made.
- [The task](TASK.md) records the episode's completed acceptance checks. It stays
  OPEN because its scope is the whole season and campaign, not just this episode.
  The seven whole-season writing requirements remain unfinished.
- The comic README and staging notes record completion. One concise changelog
  entry describes the complete episode's inclusion in released-only builds;
  neither game version nor release notes are promoted.

Historical draft reviews remain unchanged. Their counts, snapshots, and lack of
publication were true for those revisions. They are not current reader content.

## Verification

New evidence lives in [proof/completion/](proof/completion/), with
[build-completion.cjs](proof/build-completion.cjs),
[inspect-completion.mjs](proof/inspect-completion.mjs), and
[check-completion.py](proof/check-completion.py).

- Full `npm --prefix web run ci` passes: formatting, lint, unit tests, isolated
  story/site deployment fixtures, mocked publisher checks, and the real website
  production build. Story-only production builds emit large-SVG asset-size
  recommendations, not build errors. No loading-speed claim or asset reduction
  is made. Two trailing spaces in the CLI build log were normalized for the
  repository whitespace check; the warnings remain recorded.
- Forty shared illustration tests, eight docking tests, six transfer tests, and
  eight homecoming tests pass. Both public lore exporter checks pass.
- Source type checking passes with `--skipLibCheck`. Plain dependency checking
  was not repeated; its previously noted dependency declaration issue is not
  claimed fixed.
- Static browser checks pass for local development, production story-only at
  root and `/nova-protocol/`, and the full production website at root. Each has
  36 clean measured layouts, eighteen native page exports, and fifty panel
  raw-art/lettered-export checks. Library, season list, reader, history, hash
  reload, modals, focus, wrapping, and unsafe SVG rejection are covered.
- Each of the eighteen page exports matches the accepted image pixel for pixel
  after exactly the two approved heading/footer text replacements in its golden
  SVG. Nothing else in the golden is edited or masked. Actual public exports
  contain neither the old framing nor a draft badge.
- All fifty raw scenes regenerate byte for byte, without a draft override.
  Earlier sources, shared artwork, public lore art, and historical proof remain
  exact. Future-draft exclusion and incomplete-release rejection still pass in
  the isolated CI fixtures; finalizing this episode does not release later work.
- Generated HTML retains all eighteen no-script transcripts. Production story
  output stays wholly under `/story/`, with the expected fifty scene SVGs and
  no task paths or old private framing in HTML, JavaScript, or source maps.
- Browser checks use intercepted static files and CDP pipes. No HTTP server or
  debugging port is started; recorded owned browser processes have exited.
- The index is empty before preparation. Only reviewed explicit paths enter the
  commit. Unrelated task edits are preserved, including the independent story
  priority change from 50 to 62, which is left out of the committed task blob.

The Tatr listing command encountered the existing unrelated incomplete directory
`tasks/20260909-230022/`, which has no `TASK.md`. Nothing there was repaired or
removed. The existing story task body was updated directly, as the Tatr skill
permits; no extra task or false whole-season closure was created.

## Publication boundary

`released` means selected by builds, not already deployed. No push, `comic-*`
tag, live publication, server startup, game version bump, or campaign runtime
implementation is part of this request.

The first live rollout still needs the empty-library bootstrap before site-only
cleanup of legacy root reader files. That tag must use an earlier reviewed
commit containing the publisher and no released episodes, not this completed
library snapshot. Tag choice, push, deployment, and live checks need separate
authorization.
