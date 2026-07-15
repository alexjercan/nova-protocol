# Retro: player docs front door (20260715-224013)

## What shipped

A new-player on-ramp into the wiki: `getting-started.md` ("Your first flight"),
`glossary.md` (terms + the `u`/`u/s` units), a "Start here" category leading the
For-players band, both pages registered in `WIKI_PAGES` + `WIKI_DOC_PAGES`, the
tutorial surfaced through getting-started as its front door, and control-theory
jargon trimmed from `flight-autopilot.md` and `sections/controller.md` (behavior
kept). `u/s` is now defined at first use in `hud.md` and `flight-autopilot.md`.

## Decisions

- Tutorial surfacing: chose option (b) - getting-started is the tutorial's front
  door - over option (a) an external-href WikiPage. The nav/index/see-also
  render paths all build links via `pageUrl(base, slug) -> wiki/<slug>/` and
  assume a matching `WIKI_DOC_PAGES` markdown entry, so an external href would
  have to thread an optional field through three render paths AND special-case
  the doc-page build to skip rendering a nonexistent md. Not a two-line change,
  so (b) won on simplicity, exactly as the task predicted.
- Deferred the optional screenshot-annotation step (needs image tooling, not
  prose) to its own follow-up, task 20260715-231500 (P30). The prose pages read
  fine without labels; splitting kept this task a clean prose+registration unit.

## What went well

- Ground-truth-first paid off: every player-facing control claim was checked
  against `crates/nova_menu/src/lib.rs` (New Game -> `shakedown_run`, Sandbox ->
  editor) and `keybinds.md` before it was asserted, and every glossary term was
  matched to how the existing pages actually use it, so nothing was invented.
- Mirroring an existing working cross-wiki link fixed the relative depth up front
  (`../glossary/`, `../../tutorial/`), avoiding the 404-depth class of bug that
  bit earlier conversion passes.

## What to do differently

- The subagent softened the controller one-liner in `wiki-pages.ts` (the page
  `summary`) as well as the page body - correct and consistent, but it was not an
  explicit step. Listing every place a piece of jargon appears (body + manifest
  summary + related-page summaries) in the task would remove the guesswork.
- Nothing needed a second review round; the single-agent-then-review-the-diff
  loop was the right weight for a prose task of this size.
