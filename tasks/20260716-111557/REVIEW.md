# Review: Merge blog + changelog into a unified /news/ section

- TASK: 20260716-111557
- BRANCH: changelog-revamp

## Round 1

- VERDICT: APPROVE

Independent out-of-context reviewer cross-checked all 6 merged posts against
CHANGELOG.md and the deleted devlog/release sources (via `git show 5db64177^:`),
and audited the video companions, point-releases folds, wiring, redirects, nav,
stale references, and ASCII. No BLOCKER/MAJOR/MINOR.

Confirmed clean:
- FABRICATION: every claim in all 6 posts traces to CHANGELOG.md (feature + its
  folded patches) or the matching devlog. 0.6.0 numbers (17-24%, 500-5000) exact.
- DUPLICATION removed / COMPLETENESS: each post is one coherent merged page, not
  devlog+changelog pasted back-to-back; nothing important lost from the retired
  sources (e.g. 0.5.0 keeps the editor-rebind detail that lived only in the
  release notes). 0.6.0 is genuinely expanded into a feature-by-feature writeup.
- VIDEO: only 0.1.0 (AJcAMyJ0S3Y) and 0.2.0 (NBpRYDvL-jM) embed a companion
  iframe; 0.3.0-0.6.0 none.
- WIRING/REDIRECTS: `news` chunk + NEWS_POSTS (6 feature versions) + rewrites
  correct; blog/post/changelog fully removed; REDIRECTS covers all 5 devlog URLs
  + both indexes + all 11 /changelog/<version>/ URLs, patches -> parent post.
- STALE REFS: none; nav, landing card, and dev-guide authoring section updated.
- ASCII: clean.

Nits (one applied, one intentionally left):

- [x] R1.1 (NIT) web/src/news/0.5.0.md:110 - "The devlogs ... live here now"
  read anachronistically post-merge (devlogs are retired into News). Reworded to
  "This news feed and the game's new web home start here...".
- [ ] R1.2 (NIT) web/src/news/0.6.0.md - drops the bevy-common-systems rev hash,
  the `cargo bench` invocation, and docs/modding-perf-report.md that CHANGELOG
  carries. Intentional narrative-vs-reference trimming; the footer points to
  CHANGELOG.md for exhaustive detail. Left as-is by design.
