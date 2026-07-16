# Spike: rework the changelog (concise CHANGELOG.md + in-web-app release notes)

- DATE: 20260716-102940
- STATUS: RECOMMENDED
- TAGS: spike, docs, web

## Question

Two linked uncertainties:

1. `CHANGELOG.md` entries are too verbose (2-4 line paragraphs each). How
   should the file be tightened, and is the `Added` / `Changed` / `Fixed`
   axis still the right way to group entries?
2. We want a richer, in-web-app changelog - closer to how Bevy presents its
   migration guides and release notes and how Wesnoth presents its player
   changelog: more descriptive entries, room for screenshots, and section
   groupings that mean something. What should that experience be, how does it
   relate to the existing devlog, and how does it fit the web build?

A good answer picks a concrete section taxonomy for each artifact, decides
how the web changelog differs from the devlog so we do not duplicate it, and
lands as coarse tasks a planner can expand without re-litigating the shape.

## Context

What exists today:

- **`CHANGELOG.md`** (repo root, 207 lines). Keep-a-Changelog format,
  Semantic-Versioning header, `### Added` / `### Changed` / `### Fixed` under
  each version. Hand-maintained; the version bump lands in the
  `chore(release): vX.Y.Z` commit. No automated tooling touches it. The pain
  point: each bullet is a dense multi-clause paragraph, so the file does not
  scan.
- **Web app** (`web/`). Webpack build. Markdown is compiled to HTML at build
  time in `web/markdown.js` (markdown-it + highlight.js + `markdown-it-anchor`
  + a `mermaid` fence, `html: true` so raw HTML/embeds pass through). Two page
  shells exist: `wikiDocPage` (with the manifest-driven sidebar) and
  `blogPostPage` (standalone `.prose` article with a date/version meta line and
  a Discussions footer). Pages are registered by hand in
  `web/webpack.config.js` - a `WIKI_DOC_PAGES` list and a `BLOG_POSTS` list,
  each also needing a `historyApiFallback` rewrite. Content lives in
  `web/src/wiki/**` and `web/src/posts/*.md`. Images live in `web/src/assets`
  and are copied verbatim.
- **Devlogs** already exist (`web/src/posts/`, one narrative essay per release,
  first-person "here is what I built and why"). The blog index (`blog.html`)
  is hand-authored HTML cards; the wiki index is manifest-driven.
- **Image placeholder convention**: `figure__placeholder` / `post-card__ph`
  blocks render an "Image needed" card naming the screenshot to capture. This
  lets a page ship before its art exists - directly reusable for a changelog.
- **Site nav** (`web/src/_header.html`): Home / Tutorial / Wiki / Blog /
  GitHub / Play.
- Nova has **modding with real format breaks between versions** (e.g. 0.6.0
  moved mod metadata into `*.bundle.ron` - a catalog format break). That makes
  a Bevy-style "breaking changes for modders" callout genuinely useful here,
  not just an imported idea.

## Prior art studied

**Bevy** authors migration guides and release notes as many small per-PR
markdown fragments under `_release-content/` (`migration-guides/`,
`release-notes/`), each with YAML frontmatter (`title`, `pull_requests`,
`authors`). PR labels (`M-Migration-Guide`, `M-Release-Note`) drive a CI check
that nags when a fragment is missing. At release the fragments are compiled and
moved to the website repo for a copy-edit pass and media. Two published
artifacts result: **migration guides** grouped **by subsystem** (ECS,
Rendering, Assets, ...), terse and Ctrl+F-navigable with a fixed
**what -> why -> how to migrate** shape and `// 0.15` / `// 0.16` before/after
code blocks (no images); and a **release-notes blog post** grouped **by feature
significance** (not per-PR - "users don't care if it took 17 PRs"), prose,
image/gif/video heavy.

**Wesnoth** keeps a technical `changelog.md` grouped **by subsystem** (Campaigns,
Editor, Multiplayer, Lua API, Terrain, Units, User interface, WML Engine,
Miscellaneous and Bug Fixes, ...) with terse one-line bullets, stat deltas
written as `X -> Y`, and bug IDs. PRs drop fragments into `changelog_entries/`
(same news-fragment pattern as Bevy) collapsed at release. A separate
player-facing changelog / wiki Release Notes strips the API/packaging/internal
noise and keeps only gameplay-relevant changes, grouped by impact
(New Content / Changes / Fixes / Known Issues) with 1-3 descriptive sentences
each.

The common instinct across both: **one terse subsystem-grouped technical
changelog, plus a richer audience-facing artifact grouped by significance**,
fed from **per-PR fragments** so entries are written while context is fresh.

## Options considered

### For the terse CHANGELOG.md

- **A1. Keep Added/Changed/Fixed, just shorten the prose.** Cheapest. But the
  three KAC buckets are the thing the user explicitly called out as not making
  sense - a modding-portal line and a shader line land in the same "Added"
  pile, so the file never groups by what a reader cares about.
- **A2. Regroup by subsystem/area (Wesnoth-style), terse one-liners.** Sections
  like Gameplay, Combat, Scenarios, Modding, Interface, Web & Platform, Audio
  & Visuals, Performance, Fixes, Internals. Entries become single scannable
  lines, `X -> Y` for deltas. Drops the strict Keep-a-Changelog type axis (so
  the header claim changes) but is what actually "makes more sense". Recommended.
- **A3. Drop CHANGELOG.md entirely, keep only the web changelog.** Rejected:
  the terse file is the machine/power-user/modder artifact (greppable, in-repo,
  no build), and GitHub release automation and offline readers expect it.

### For the richer web changelog

- **B1. Just keep writing devlogs.** Rejected: devlogs are narrative essays,
  not a scannable structured reference, and they already exist - this would not
  answer the request. The web changelog must be *reference-shaped*.
- **B2. New `/changelog/` section, blog-pattern pages, one page per release.**
  Per-release markdown under `web/src/releases/<version>.md`, rendered through a
  new `releaseNotePage` shell (a near-clone of `blogPostShell`), an index at
  `/changelog/` with release cards, and a nav link. Each release page groups
  changes **by feature/theme** (biggest first), 1-3 sentences per item with
  `figure__placeholder` screenshots, and - when a format breaks - a
  **"Breaking changes / migrating your mods"** callout in the Bevy migration
  spirit. This reuses the entire existing markdown-at-build-time pipeline; the
  only new machinery is one shell function and one registration list.
  Recommended.
- **B3. Full Bevy clone: per-PR fragment files compiled into the page.**
  The best long-term authoring story, but a bigger lift (fragment dir, compile
  step, CI nag) and orthogonal to *presentation*. Split out as an optional,
  lower-priority follow-up so it does not block the visible win.

## Recommendation

Ship two artifacts from a clear division of labor, in priority order:

1. **Tighten and re-section `CHANGELOG.md` (A2).** Replace Added/Changed/Fixed
   with subsystem sections; collapse each entry to one scannable line; use
   `X -> Y` for numeric deltas. This is the fast, low-risk, self-contained win
   and it stands alone. Proposed section set (drop any empty per release):
   Gameplay & Flight; Combat & Weapons; Ships & Sections; Scenarios &
   Objectives; Modding & Mod Portal; Interface & HUD; Web & Platform; Audio &
   Visuals; Performance; Fixes; Internals & Tooling. Keep the SemVer note and
   `[Unreleased]`; adjust the "format is based on Keep a Changelog" line to
   describe the new subsystem grouping honestly rather than claim strict KAC.

2. **Build the in-web-app changelog (B2)** as a new `/changelog/` section that
   reuses the blog markdown pipeline. Per-release pages grouped by feature/theme
   with screenshots and, where formats break, a modder-facing "Breaking changes"
   callout. Seed it with 0.6.0 and backfill earlier releases from the tightened
   CHANGELOG + existing devlogs. Add the "Changelog" nav link.

**Division of labor so nothing is duplicated:**

- `CHANGELOG.md` - terse, complete, in-repo, subsystem-grouped. For modders and
  power users skimming "what changed". Every change appears here.
- Web changelog - richer, curated, image-rich, theme-grouped, with a breaking-
  changes/migration section. The Bevy-migration-guide analogue. Skips pure
  internals.
- Devlog - unchanged: the narrative "story of the release". The web changelog
  page should cross-link to the matching devlog rather than retell it.

3. **(Optional, deferrable) News-fragment authoring pipeline (B3).** Adopt the
   Bevy/Wesnoth `changelog_entries/`-style per-PR fragments plus a release-time
   compile and a CI nag. Real value once the two artifacts exist and start
   drifting, but not required for the visible improvement - lowest priority.

## Open questions

- **Backfill depth for the web changelog.** Do we write rich pages for all of
  0.1.0-0.6.0, or start at 0.6.0 and only backfill on demand? Leaning: ship
  0.6.0 rich, backfill lazily. A user/product call for the implementing task.
- **URL/name: `/changelog/` vs `/releases/` vs "Release notes".** Leaning
  `/changelog/` labelled "Changelog" in nav for discoverability; confirm during
  the web task.
- **New shell vs extend `blogPostPage`.** A `releaseNotePage` clone is simplest
  and keeps the two content types independent; revisit only if it invites real
  duplication in `markdown.js`.
- **Does the terse-file re-section want SemVer/KAC-compat tooling later?** Only
  matters if we automate GitHub Releases from it; out of scope for now.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps:

- tatr 20260716-102950 (P80): Tighten and re-section CHANGELOG.md - terse
  subsystem-grouped entries. Do first; low risk, self-contained.
- tatr 20260716-102954 (P60): Build the in-web-app changelog/release-notes
  section - blog-pattern per-release pages, `/changelog/` index, nav link,
  breaking-changes callout, seeded with 0.6.0.
- tatr 20260716-102957 (P30): Optional news-fragment changelog authoring
  pipeline (`changelog_entries/` + release compile + CI nag). Deferrable.

## Fix record

(Implementing tasks append a few lines here as they land.)
