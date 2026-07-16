# Retro: Build in-web-app changelog/release-notes section

- TASK: 20260716-102954
- BRANCH: changelog-revamp
- REVIEW ROUNDS: 1 (APPROVE with three non-blocking fixes applied pre-landing)

## What went well

- Exemplar-first fan-out. Writing one gold-standard page (0.6.0.md) by hand,
  then having parallel subagents draft the other 10 against it plus a strict
  per-version spec, produced consistent shape/voice across all 11 pages with
  one review round. The independent reviewer confirmed no fabrication and clean
  wiring, so the fan-out did not trade quality for speed.
- Reusing the existing blog markdown pipeline meant the only new machinery was
  one shell function and one registration list - the 11 pages, the index, and
  the nav dropped into slots the codebase already had, keeping the diff small
  relative to the output.
- An out-of-context reviewer (fresh agent, no build context) caught two
  cross-release attribution errors that the authors - who each saw only their
  own version - structurally could not.

## What went wrong

- R1.1: the 0.5.0 page claimed the wiki and tutorial "live here now", which is a
  0.5.2 feature, and listed "this changelog" (the thing this very commit adds).
  Root cause: the 0.5.0 drafter inherited imprecise wording from devlog-5 (which
  really does say it), and a per-version drafter has no view of the release
  boundary - it cannot tell that a feature its source mentions actually shipped
  later. The exemplar controlled shape but not per-item factual scoping.

## What to improve next time

- When fanning out doc drafters over a shared multi-part source (per-release,
  per-module), add cross-boundary attribution to the review pass explicitly:
  "does page for vN claim anything that belongs to vN+1?". Shape consistency is
  cheap to enforce with an exemplar; factual scoping across the boundary needs a
  reviewer who sees all parts at once.

## Action items

- [x] Recorded `exemplar-first-fanout` and `cross-boundary-attribution` in
  docs/LESSONS.md.
- [x] Applied the three review fixes before landing (0.5.0 wording, 0.3.1
  (breaking) tag in CHANGELOG.md, rand 0.10 in the 0.3.1 callout).
- [ ] Screenshots for the `figure__placeholder` slots across the 11 pages remain
  to be captured (tracked by the placeholders themselves; not a code task).
