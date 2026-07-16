# Build in-web-app changelog/release-notes section (blog-pattern pages + index + nav)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: spike,web,docs

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
- Seed with 0.6.0; backfill earlier releases lazily (see SPIKE open questions).

Done = `/changelog/` builds and serves, 0.6.0 page is rich and themed with a
breaking-changes section, nav link present, no duplication of the devlog.

## Notes

Spike: tasks/20260716-102940/SPIKE.md
Depends on 20260716-102950 (tightened CHANGELOG is the source to curate from).
