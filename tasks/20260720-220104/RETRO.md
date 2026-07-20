# Retro: triage the OPEN task backlog

## What went well

- The triage confirmed nova's backlog is already healthy: all 24 OPEN tasks
  carry an intentional scheduling tag (v0.8.0 scheduled or backlog deferred) and
  a priority - nothing orphaned. The honest output was an assessment + a short
  close/defer candidate list, not busywork re-tagging.
- Held the line on scope: made NO unilateral closes of product/feature work.
  Close/defer of a game's feature backlog is the user's product call; the task
  surfaces candidates (the 3 May-25 doc tasks, esp. the cross-repo bcs one) and
  leaves the ruling to the user.

## What went wrong

- Introduced a stray non-ASCII character in REVIEW.md prose; caught and fixed
  before landing (the ASCII-only rule). An edit is a hypothesis until re-read.

## What to improve next time

- A triage on a well-tended backlog is mostly confirmation; the value is the
  surfaced shortlist, not mutations. Keep it a periodic light pass, not a rewrite.

## Action items

- [x] Triage recorded in TRIAGE.md; all OPEN tasks confirmed tagged; landed 37d3e3a3.
- User ruling pending on 3 stale doc tasks (20260525-133030/133031/133032).
