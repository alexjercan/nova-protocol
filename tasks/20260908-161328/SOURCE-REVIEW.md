# Comic source consolidation

This records the completed consolidation step. The later
[TS/panel refactor](DSL-REVIEW.md) supersedes its source contract and review UI.
The evidence below remains specific to this earlier revision.

The user approved `web/src/comics/` as the single source home and requested
removal of the demo. Normal local serving now includes drafts. Publishing the
unfinished episode was not requested and did not occur.

## Delta

- Moved the episode's script, opening/Aquila scene sources, and props into
  `web/src/comics/season-1/episode-1/`.
- Added season `comic.json`, episode `episode.json`, and a small `generate.py`
  composition entry. Metadata is authored once, outside the drawing code.
- The episode is explicitly `draft`, with complete `pageCount: 18`; seven pages
  are illustrated. A released episode with fewer pages is a build error.
- `web/comic-sources.js` validates all metadata and selects episodes before any
  generator runs. Released episodes form a prefix of reading order. A season
  can hold released episodes alongside its next draft.
- `web/build-comics.py` derives scene SVGs, lettering data, and transcripts;
  `web/story-build.js` connects this to normal webpack builds and serving.
- `npm --prefix web run serve` includes local drafts. `npm --prefix web run build`
  is released-only even in webpack development mode. Production-mode serving
  also excludes drafts. The normal full-site wrapper uses the same behavior.
- Removed the special `story:dev`/`story:build` commands, `story-preview.js`,
  the old `art/comics/` source tree, and the demo manifest, page, cover, and
  Python drawing helpers. Generic typed renderer helpers remain available.
- Removed recursive page-module discovery. Selected definitions are embedded;
  only referenced assets are emitted. Neither comic folders nor generated
  caches are broad copy inputs.
- Local serving uses its own output cache and serves no stale disk-site files.
  Deployable builds still use `web/dist`. No sources move when an approved,
  complete episode is eventually marked released.

The [comic README](../../web/src/comics/README.md) owns the current authoring
contract. The [initial reader review](READER-REVIEW.md) and earlier proof retain
their historical commands and source scope. They are not rerunnable checks of
this later structure.

## Verification

Evidence: [proof/consolidation/](proof/consolidation/), based on its saved
working tree at `7fdd25222c3790a4430a1ff4f3071c78d1334261`.

- Website CI: formatting, lint, tests, and normal build passed.
- Metadata tests cover missing/unknown fields, ids, sequences, unregistered
  directories, publication order, missing generators, complete page counts,
  mixed released/draft seasons, escaping, prefix routes, referenced assets,
  stale cleanup, and development/output guards.
- The metadata test generator records calls: excluded drafts are not executed.
  Its unreferenced image is neither emitted nor retained as stale output.
- Normal local serving passed desktop/phone checks of the library, contents,
  seven pages, navigation, transcripts, art-only controls, layout diagnostics,
  unique inline IDs, and SVG exports. A real source edit/restore regenerated
  without manual refresh and retained `#page-4`.
- Prefix development builds passed the same reader checks plus paragraph
  wrapping and three rejected unsafe-SVG fixtures. Both public root/prefix
  builds have only `/story/`, no story images, no demo, and no private page data.
- 31 illustration tests, both public exporters, and the retained clean-line,
  expression, and Gantry studies passed.
- Source checks retained 489 protected files and all eight scene/cover SVG
  hashes. Scene source changes are only repository-root paths; script changes
  are only its outline link and local command. Dialogue and art are unchanged.
- mdBook built successfully, with the existing Mermaid preprocessor version
  warning. No Rust build, tests, or Clippy were run.
- The index remains empty. No commit, push, or deployment was made.

## Outcome

The real seven-page draft is available through normal web development. The
public archive is empty until a complete episode is approved for release.
The full script remains 18 pages / 50 panels / 730 spoken words. Pages 8-18
still need illustration; the wider season and all seven writing checks remain
open. No scene, physical detail, biography, or later outcome was added here.
