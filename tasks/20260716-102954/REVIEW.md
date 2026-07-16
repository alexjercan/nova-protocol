# Review: Build in-web-app changelog/release-notes section

- TASK: 20260716-102954
- BRANCH: changelog-revamp

## Round 1

- VERDICT: APPROVE (with three non-blocking fixes applied before landing)

Independent out-of-context review (fresh-eyes subagent) cross-checked all 11
release pages against CHANGELOG.md and the 5 devlogs, and audited links, wiring,
consistency, and ASCII. No BLOCKER/MAJOR. Confirmed clean: no fabricated
features or wrong numbers; all 5 devlog cross-links resolve and the 6
no-devlog releases link none; all changelog.html card hrefs match RELEASE_PAGES
slugs and the history rewrites (anchored, no slug is a prefix of another); the
webpack/markdown wiring is correct; ASCII clean; all 11 H1s and the shared
template shape are consistent; only 0.6.0 and 0.3.1 carry a breaking callout.

Findings addressed:

- [x] R1.1 (MINOR) web/src/releases/0.5.0.md - "This changelog, the devlogs,
  the tutorial, and the wiki all live here now" attributed later features to
  0.5.0 (wiki/tutorial landed in 0.5.2; the changelog itself is this 0.6.0
  commit). Fixed: reworded to "The devlogs and the game's new web home live
  here now (the wiki and tutorial follow in v0.5.2)."
- [x] R1.2 (MINOR) web/src/releases/0.3.1.md breaking callout had no
  **(breaking)** counterpart in CHANGELOG.md. Fixed at the source of truth:
  tagged the 0.3.1 Bevy 0.19 upgrade **(breaking)** in CHANGELOG.md, so the
  terse file and the web callout agree.
- [x] R1.3 (NIT) web/src/releases/0.3.1.md callout omitted `rand 0.10` that the
  page prose and CHANGELOG both list. Fixed: added rand 0.10 to the callout.

Rebuild green after fixes; both edits verified in the emitted HTML.
