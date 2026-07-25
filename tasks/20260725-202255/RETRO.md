# Retro: Mirror OnNeutralized handlers in The Ledger webmod

## What went well

- The audit was bounded to The Ledger webmod and found the extra chapter one
  player retry before finish.
- Existing Ledger harness tests were a good fit, so the change was verified
  through production scenario configs instead of a synthetic parser-only check.
- The review pass caught the neutralize-then-destroy double-count risk and the
  tests now pin it.

## What went wrong

- The original mainline neutralization work stopped at generated base content
  and did not sweep the hand-authored webmods.
- The first version of the ch5 chain test still assumed only the destroyed
  Auditor path could enter chapter five; adding the neutralized path required
  the test to classify both event kinds.

## Improve next time

- Treat content-wide behavior changes as repo-wide sweeps: generated base,
  webmods, assets/mods, examples, and Rust-coded scenario fixtures.
- When mirroring an event that can precede another event for the same entity,
  add the shared idempotence gate at the same time as the sibling handler.

## Lessons

- Promoted `sweep-content-repo-wide-not-just-assets` to pending promotion at x3.
- Added `neutralized-then-destroyed-counters` for objective counters mirrored
  across `OnDestroyed` and `OnNeutralized`.
