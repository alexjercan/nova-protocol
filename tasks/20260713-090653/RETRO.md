# Retro: Shakedown radar-era rework

- TASK: 20260713-090653
- BRANCH: feat/shakedown-radar-era (landed bc7f3a9)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The stale-doc sweep BEFORE implementation (user-requested) turned a
  five-item inherited scope into a two-item one: the audio blip and the
  gesture rows had already shipped through other tasks, and writing that
  down first prevented re-building them here. A task that sat open across
  five landings should always get this pass.
- The capability beat cost almost nothing because every piece existed:
  SetControllerVerb (GOTO unlock), the deny cue (F7), RADAR in ROW_VERBS,
  the contextual cluster's emphasized-early behavior. The scenario change
  is config + text; the mechanics were all pre-paid by the UX rounds.
- The beat-walk pin of the Lock lifecycle incidentally became the FIRST
  proof that SetControllerVerb executes through the real event pipeline
  (the GOTO grant was never asserted) - a free regression net.
- The landing && chain from the 121605 lesson was applied and the
  pre-committed task file prevented the stub collision entirely.

## What went wrong

- Nothing broke. One judgment call worth revisiting in playtest: LOCK
  withheld through beats 1-3 makes GOTO unusable there too (granted but
  lock-less); traced and judged deliberate at review, but if playtest
  wants GOTO earlier the grant order can swap.

## What to improve next time

- When a backlog task inherits sub-items from multiple closures, sweep it
  against the landed state BEFORE planning - this cycle got that for free
  because the user asked; make it the default for any task older than the
  family it depends on.

## Action items

- (none - the deliberate-radar spike has no open consumers left)
