# Retro: Merge blog + changelog into a unified /news/ section

- TASK: 20260716-111557
- BRANCH: changelog-revamp
- REVIEW ROUNDS: 1 (APPROVE, one NIT applied)

## What went well

- Exemplar-first fan-out held up a second time: hand-expanding the 0.6.0 post,
  then fanning out the 5 merged posts against it, produced six consistent pages
  merging two sources each, and the independent review came back with zero
  BLOCKER/MAJOR/MINOR.
- A short targeted research pass (Bevy /news/ specifics) before building settled
  the two real forks - patches get no post, video is a companion not the lead -
  so the design did not wobble mid-build.
- Retiring the old sections behind meta-refresh redirect stubs (generated from a
  single REDIRECTS table) kept every old /blog/ and /changelog/ URL alive with
  ~30 lines, so the restructure did not break bookmarks or inbound links.
- The parallel drafters reused the already-written web/src/releases/*.md as the
  structured half of each merge, so the earlier /changelog/ work was not wasted
  even though its standalone pages were retired.

## What went wrong

- Churn: task 20260716-102954 built a standalone /changelog/ section that this
  task merged away one cycle later. Root cause: the original spike
  (20260716-102940) scoped "terse CHANGELOG + rich web changelog" without
  checking that a devlog/blog section already covered the same per-release
  ground - the duplication only surfaced when the user saw the two sections
  side by side. The standalone changelog shell, index, and registration were
  thrown away (the release-page prose survived as merge input).

## What to improve next time

- When a spike proposes a new user-facing SECTION, enumerate the existing
  adjacent sections first and explicitly ask "does this duplicate or belong
  merged with one of them?" before designing it standalone. A one-line
  overlap check in the spike would have proposed /news/ from the start and
  skipped building the throwaway /changelog/ shell.

## Action items

- [x] Recorded `check-adjacent-sections-for-overlap` in docs/LESSONS.md.
- [x] Applied the review NIT (0.5.0 "devlogs live here now" phrasing).
- [ ] Screenshots for the figure__placeholder / thumbnail slots across the 6
  news posts remain to be captured (tracked by the placeholders themselves).
