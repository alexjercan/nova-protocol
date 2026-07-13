# Retro: OnTravelLock/OnCombatLock scenario events

- TASK: 20260713-140922
- BRANCH: feat/onlock-events (landed 18632a2)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Reading the NEIGHBOR's review history before implementing (the orbit
  tracker's R1.1/R1.2 comments live in the code) caught a plan error
  before it shipped: "once per acquisition" would have reintroduced the
  exact soft-lock class the orbit tracker documents. The vocabulary now
  has one consistent bridge-event contract: acquire fires, held state
  echoes, beat guards own ordering.
- The pure tick_lock_slot state machine made the semantics unit-testable
  in eight lines of asserts before the e2e even ran.

## What went wrong

- The plan step encoded the once-per-acquisition mechanism WITH a
  confident rationale (equality-skipped writers) that was true but
  answered the wrong question - dedup was never the risk, consumption
  was. Plans citing a mechanism should also ask what the neighboring
  system's review history says about the same shape.

## What to improve next time

- verify-first-plan-steps extension: when a plan adds a sibling to an
  existing pattern (second bridge event, second tracker), read the
  sibling's REVIEW/comment trail as part of the plan, not just its code.

## Action items

- [x] The 140929 stale-lock walk pin must be written against the echo
  semantics (recorded in the review).
