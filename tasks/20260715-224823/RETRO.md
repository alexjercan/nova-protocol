# Retro: Sync modding wiki guides + mod metadata to the now-playable scenarios

- TASK: 20260715-224823
- BRANCH: docs/mod-scenarios-sync (landed as master eaf45654)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- The sweep up front (grep the whole `web/src/wiki` + `docs` tree for the mod
  ids and their describing words) turned a fuzzy "update the docs" into a short
  concrete list: only `guide-make-a-mod.md` actually embedded stale
  descriptions. Everything else that matched (`modding-ron.md`,
  `guide-author-section.md`) was accurate on inspection, so the diff stayed
  small and honest instead of a speculative rewrite.
- Verified the fix by grepping BOTH sides (the shipped `*.bundle.ron` /
  `*.content.ron` and the guide snippet) for byte-equal description strings -
  the doc claim is now checkably true, not just plausibly updated.

## What went wrong

- Nothing. Small, well-scoped, landed in one round.

## What to improve next time

- When docs EMBED code/config (this guide pastes the mod RON), a code change
  silently drifts them - the dependency is invisible to the compiler. The
  planning habit that paid off: land the gameplay tasks first, then a docs-sync
  task that depends on them, rather than editing docs speculatively alongside
  code. Keep filing the docs-sync as its own dependent task.

## Action items

- No new ledger lesson: this is `sweep-then-delete` (grep describing words in
  docs) working as intended. No follow-up tasks.
