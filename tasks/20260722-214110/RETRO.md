# Retro: Ledger ch4 diverging endings (20260722-214110)

## What went well

- The owner's "one ending avoids the fight" call turned a flavor-only fork
  (both endings converged on the same Auditor brawl, differing by message) into
  a real risk/reward divergence: BURN = safe but broke (no Auditor, no payday),
  SELL = payday at a price (survive the telegraphed Auditor). The structure now
  carries the choice, not just the text.
- The act-latch discipline established in ch2/ch3 transferred exactly: the burn
  buoy sets `act = 3` SYNCHRONOUSLY (before any death window) and defers only the
  player-facing Victory overlay behind `burn_gate`/`burn_said`. That closes the
  `act < 3` Defeat window immediately, so `defer-opens-a-consumer-race` cannot
  bite - the reviewer confirmed no death race. This is the same rule ch2's win
  handler and ch3's `arrive3_said` breather follow; it is now the mod's standing
  pattern.
- Deleting the ~650-line burn-branch Auditor block also fixed the chapter's
  worst pacing gap for free: the climactic spawn that had NO `engage_delay` now
  only exists on the sell path, where it gained the 8.0 telegraph + a warning
  line. The one remaining fight ARRIVES instead of materializing.
- Pruning the orphaned lint ack was the honest move: removing a spawn orphans its
  `close-spawn` ack, so the ack had to go with it (leaving it would silence a
  finding that no longer exists / mislead the next reader). The surviving ack
  still matches the live sell-branch spawn - verified by the reviewer.

## What went wrong / was tricky

- The engine has only `Victory`/`Defeat` outcome kinds (confirmed at
  `actions.rs:471`) - no neutral/bittersweet/escape kind. So "distinct terminal
  outcomes" could not be carried by the KIND; it rests on distinct terminal
  MESSAGES plus the structural fact that only one path has a fight. The test
  pins the structural no-spawn fact (not just the messages), so a regression to
  convergence would fail the rig rather than silently pass. A richer outcome-kind
  taxonomy would be an engine change (out of scope for this data-only task).

## Lessons / what to do differently

- "Distinct terminal outcomes" with a two-value outcome enum means the test must
  assert the STRUCTURAL divergence (one path spawns the boss, the other does
  not; both reach a terminal act; neither chains), not only the banner text - or
  a future edit that re-converges the fights would false-green on matching
  messages. (`review-rig-can-false-green`.)
- When a content change REMOVES a spawn/finding, sweep its acks in the SAME task
  (`balance_acks.ron`): an ack outlives its finding silently. Grep the ack's
  target id after removing the producer.

## Follow-ups

- Owner playtest question 1 (burn tone: shipped bittersweet SAFE-BUT-BROKE vs a
  clean escape) is a one-line message swap in the deferred overlay; batched for
  Finish.
- A neutral/bittersweet `ScenarioOutcomeKind` (so endings can diverge by kind,
  not just message) is a possible future ENGINE task - noted, not filed, pending
  owner interest at Finish.
