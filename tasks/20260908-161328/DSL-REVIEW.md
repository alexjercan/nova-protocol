# TS script, panel art, and reader modals

This supersedes the source contract in [SOURCE-REVIEW.md](SOURCE-REVIEW.md).
Earlier proof remains revision-specific. No earlier proof writer was rerun.

## Delta

- One TS file per page owns action and dialogue. `episode.ts` gives the reading
  order. All eighteen pages are maintained there, including the eleven without
  illustrations. `NOTES.md` retains the staging constraints.
- Python exports nineteen named panel scenes under the episode's `art/` folder.
  It owns no dialogue, transcripts, location cards, or page assembly. Blank text
  slots keep screen labels in the original painter order, including behind hands
  and inside rotated equipment groups.
- Removed the six superseded episode sources after checking their exact frozen
  copies: the per-episode generator, maintained Markdown script, two page
  generators, and the two old prop locations. The props now live under `art/`.
- The shared engine validates scripts, generates art, derives Markdown and
  transcripts, and composes full-color pages in the real reader. Speaker text is
  separate from balloon placement. Accepted wrapping uses word-count breakpoints;
  new paragraphs can use automatic measured wrapping.
- Raw panel SVGs and lettered panel/page SVGs are available independently.
  Composed exports use the displayed composition, not another lettering pass.
- Per user feedback, removed Layout checks UI, reports, and overlays. Measured
  results remain available to automated browser checks.
- Per user approval, Panels and Transcript are compact top-toolbar buttons.
  Their scrollable native modals take no space from the fitted page. Close and
  Escape restore button focus; background controls are inert while a modal is
  open. The no-script transcript fallback remains.
- Updated the authoring guide, development chapter, documentation map, shared
  illustration guidance, source pointers, and the single unreleased Story entry.

## Verified

Evidence: [proof/dsl/](proof/dsl/). The baseline stores working-tree hashes and
exact source snapshots, not an assertion that the original tree was clean.

- Eighteen pages, fifty panels, and 730 spoken words. Every migrated action and
  spoken line matches its source. Layout positions, screen labels, orientation
  cards, and pages 8-18 remain unchanged.
- All seven complete page exports are pixel-identical to the accepted reader
  exports at 1500 x 1000. The test covers both root and `/nova-protocol/` builds.
  This is rendered equality, not a claim that the SVG source bytes are equal.
- Desktop and phone inspection passed for all seven pages. There is exactly one
  visible page, no duplicate inline ids, and no measured layout warnings.
- Nineteen individual art and lettered-panel exports load. Automatic paragraph
  wrapping preserves the words. Script, event-handler, and external-paint SVG
  fixtures fail safely while their transcripts remain available.
- Modal checks cover Close, Escape, focus restoration, inert background controls,
  phone keyboard scrolling, unchanged page selection, and unchanged viewport
  height while open. Reload preserves `#page-4`. Layout controls and overlays
  are absent.
- Root and prefixed deployable builds contain only `story/index.html`, no Story
  assets, draft data, source paths, or demo. Metadata tests also cover ordinary
  development-mode builds and production-mode serving as released-only.
- Actual build tests prove excluded TS and Python cannot execute. Selected
  incomplete releases fail. Unreferenced scenes are not drawn; cache writes are
  deterministic, retain unchanged mtimes, and remove stale selections.
- Targeted TypeScript checks, lint/format checks, comic/reader tests, metadata
  tests, DSL/build tests, asset-namespace checks, and 37 illustration tests pass.
- The mdBook build passes with the existing Mermaid 0.5.0/0.5.2 version warning.
  Twelve Markdown sources and 123 local link targets pass; the Story changelog
  entry is 184 characters. `git diff --check` passes.
- The source checker retains 609 protected files, including prior proof and
  public illustrations. The index remains empty; all seven writing requirements
  remain unchecked.

Browser inspection used Chromium through CDP pipes and intercepted static files.
No web server or browser debugging port was started. One profile cleanup raced
with Chromium shutdown; bounded removal retries fixed that harness-only issue,
and the prefixed check was rerun successfully.

Full website CI and Rust checks were not run for this batch. A live development
server edit/reload test was not run because the user owns server startup. The
static reload and fresh-process source-generation checks do not claim otherwise.

## Next

The episode remains `draft`, with seven of eighteen pages illustrated. This work
neither publishes the episode nor completes the season. No commit or push was
made. The next story-art work is the distress and funding-refusal sequence after
workflow feedback.
