# Real-reader working preview

This report describes the initial reader migration. The later
[source consolidation](SOURCE-REVIEW.md) replaces its opt-in command and source
locations; the evidence under `proof/reader/` remains frozen.

The user accepted pages 5-7 and chose the real reader, not a browser editor.
They approved the featured-current-season layout with ordered episode lists.
This batch implements that workflow. Nothing was committed or published.

## Delta

- `/story/` features the current season and its episodes in reading order.
  Earlier seasons get compact cards when they exist. Demos stay separate.
  Start/Latest links use released episodes only, without duplicate destinations.
- The phosphor interface surrounds untinted full-color art. The existing
  seven illustrated pages now use the real page controls, hashes, transcripts,
  and full-size image view.
- `npm --prefix web run story:dev` starts an opt-in loopback preview. `story:build`
  builds it once. Both use an isolated cache, not `web/dist/`. Production-mode
  previews, output overrides, and draft/public season identity conflicts fail.
- Scene composition, dialogue, registration, and transcripts derive from the
  maintained sources in `art/comics/season-1/episode-1/`. Generated JSON pages
  remove the need for individual TypeScript image wrappers.
- The browser measures dialogue, wraps paragraphs, and creates the same composed
  SVG for display and export. Existing deliberate line breaks remain. Local
  Layout checks and Art only controls assist review.
- Editing sources regenerates the preview and reloads the selected page. A real
  source-edit/restore test caught and fixed cached HTML and unchanged build-hash
  problems. Unchanged outputs retain their mtimes; stale generated files go away.

## Source ownership and retained art

The [authoring guide](../../web/src/comics/README.md) owns the new workflow.
`opening.py` owns pages 1-4 and their dialogue. `aquila.py` reads pages 5-7's
words from the then-maintained [script](proof/dsl/SCRIPT.md.txt), now frozen.
Pages 8-18 remain script, outside the generated preview. The full episode is
still 18 pages, 50 panels, and 730 spoken words, not an illustrated release.

The old task-local sources have exact `.txt` snapshots. Their SVGs and review
HTML remain unchanged and are now frozen evidence, not competing readers.
The public lore art, shared drawing code, and earlier proof are unchanged.
Both authored `pages()` functions have identical ASTs before and after the
move; pages 5-18's script body is exact.

Native 1500x1000 browser comparisons against the accepted SVGs found five
pixel-identical exports. Page 2 differed in 32 pixels and page 5 in 16 pixels,
out of 1,500,000 each. The rendered exports were inspected. No scene, face,
pose, expression, dialogue, or accepted balloon placement was changed.

The initial collision report caught a conservative whole-hair bounding box
near Leila's balloon, not a covered face. Generated metadata now marks the
actual face contour. This keeps the accepted art instead of moving it to
silence a false alarm.

## Verification

Evidence lives in [proof/reader/](proof/reader/), against its `before.json`
working-tree snapshot, not a clean-HEAD assumption.

- 31 illustration tests; both public exporters and three retained studies.
- Comic page/renderer/reader tests, manifest/JSON/preview-isolation tests,
  targeted ESLint and formatting, and source type checking with `skipLibCheck`.
- Normal and preview builds at `/` and `/nova-protocol/`.
- Real-reader browser checks at 1440 and 390 px: all seven pages, unique IDs,
  contents, keyboard/hash/reload navigation, transcripts, Art only, diagnostics,
  and composed SVG exports. All current page diagnostics are clear.
- A live source edit regenerated lettering without manual refresh and kept
  `#page-4`; restoring the source did the same. The original bytes were restored.
- A paragraph fixture verified browser-measured wrapping. Script, event-handler,
  and external-paint SVG fixtures were rejected while transcripts stayed usable.
- Public root/prefix library, demo contents, and reader were inspected. Both
  builds contain only the public demo routes and exact public story assets.
  No private dialogue or page assets entered those builds.
- Source/isolation checks retained 384 protected files, six exact source
  snapshots, deterministic outputs, an empty index, and the existing open task.

Plain `tsc --noEmit` is blocked by the existing Mermaid declaration's missing
`type-fest` dependency. Source checks pass with `skipLibCheck`; the website
builds pass. Full website CI, mdBook, and Rust checks were not run in this batch.
Concurrent release-news, media, and widget work was not part of this review.

## Limits and next step

This is a reader-based authoring preview, not an in-browser editor. Diagnostics
check text/page bounds, balloons, and face contours; they do not validate
composition, physical logistics, or every scene object. Hair and other props
still need visual review. Phone fitting remains a landscape overview.

Page additions update automatically. Adding/removing an episode or collection
requires restarting the server; a route change fails rather than emitting dead
links. Publishing a complete episode is a separate, explicitly requested step.

Review the library and reader next. Pages 8-10's distress and funding-refusal
sequence follows after that feedback. The wider season remains unfinished.
