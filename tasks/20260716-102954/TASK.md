# Build in-web-app changelog/release-notes section (blog-pattern pages + index + nav)

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: spike,web,docs

## Outcome

Shipped the `/changelog/` web section. `changelogNotePage`/`changelogNoteShell`
added to `web/markdown.js` (standalone `.prose` article, back-to-changelog crumb
+ footer, GitHub CHANGELOG.md pointer); a `.callout`/`.callout--breaking` style
block in `web/src/style.css`; 11 theme-grouped release pages
`web/src/releases/0.1.0.md`..`0.6.0.md` with `figure__placeholder` shots,
breaking-changes callouts on 0.6.0 (catalog/bundle) and 0.3.1 (Bevy 0.19), and
devlog cross-links on 0.1.0-0.5.0; a `web/src/changelog.html` index of release
cards; `web/src/changelog.ts` entry + full webpack registration and history
rewrites; and a Changelog nav link in header and footer. `npm run build` is
green and emits `/changelog/` + all 11 `/changelog/<version>/` pages.

Independent review APPROVE (REVIEW.md): no fabrication, links/wiring correct,
ASCII clean; three non-blocking copy fixes applied before landing (0.5.0
mis-attribution, 0.3.1 (breaking) tag added to CHANGELOG.md, rand 0.10 in the
0.3.1 callout).

Eleven release pages were drafted by parallel subagents from a hand-written
0.6.0 exemplar + the tightened CHANGELOG + the devlogs, then reviewed.

## Steps

- [x] 1. `changelogNoteShell` + `changelogNotePage` in `web/markdown.js`.
- [x] 2. `.callout` / `.callout--breaking` style block in `web/src/style.css`.
- [x] 3. `web/src/releases/<version>.md` for all 11 releases (0.6.0 down to
      0.1.0), themed, with figure placeholders, breaking callouts, devlog links.
- [x] 4. `web/src/changelog.html` index of release cards, newest-first.
- [x] 5. `web/src/changelog.ts` entry + webpack registration + rewrites.
- [x] 6. "Changelog" nav link in `_header.html` and `_footer.html`.
- [x] 7. `npm run build` green; `/changelog/` + `/changelog/<version>/` emit.

## Goal

Add a richer, in-web-app changelog - the Bevy-migration-guide / Wesnoth-player-
changelog analogue - as a new `/changelog/` section that reuses the existing
build-time markdown pipeline (`web/markdown.js`, blog pattern).

Shape:

- Per-release markdown pages under `web/src/releases/<version>.md`, rendered
  through a new `releaseNotePage` shell (a near-clone of `blogPostShell`), a
  `/changelog/` index with release cards, a `historyApiFallback` rewrite, and a
  "Changelog" link in `web/src/_header.html`.
- Each release page groups changes **by feature/theme** (biggest first), 1-3
  sentences per item, with `figure__placeholder` screenshots (reuse the
  existing "Image needed" convention so it ships before art exists), and a
  **"Breaking changes / migrating your mods"** callout whenever a format breaks
  (Nova's mod bundle/catalog formats do break between versions).
- Cross-link each release page to its matching devlog rather than retell it -
  devlog stays the narrative, the changelog page is the structured reference.
- Backfill decision (user, 2026-07-16): write rich themed pages for ALL
  releases 0.1.0 through 0.6.0 up front, curated from the tightened CHANGELOG
  and the existing devlogs.

Done = `/changelog/` builds and serves, 0.6.0 page is rich and themed with a
breaking-changes section, nav link present, no duplication of the devlog.

## Notes

Spike: tasks/20260716-102940/SPIKE.md
Depends on 20260716-102950 (tightened CHANGELOG is the source to curate from).
