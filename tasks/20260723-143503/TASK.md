# Goal: ch3 speed-provocation - wake the Magpies when the player burns too hot

- STATUS: OPEN
- PRIORITY: 0
- TAGS: goal,v0.8.0,content,scenario

## Story

In The Ledger chapter 3 (THE QUIET CHANNEL), add a new stealth provocation:
burning too hot wakes the two NEUTRAL Magpie pickets. Today the pickets only
wake on a picket-watch zone entry or a combat-lock paint. Chapter 3's fantasy
is "run dark and slow" (player speed cap is 25 u/s), so going fast should be
noise the pickets hear.

This requires a small, reusable ENGINE capability first - the player's live
speed is not currently visible to scenario content - then the ch3 CONTENT that
consumes it.

Design decisions (confirmed with the user):
- Threshold: 8 u/s.
- Behavior: WARN, then trip. The first overspeed while sneaking fires a Vesh
  warning and does NOT wake the pickets; a FRESH breach (after slowing back
  under a rearm band) wakes both. Hysteresis so one continuous burn does not
  warn-and-trip in consecutive frames.
- Reusable design: expose the player's speed as a reserved scenario variable
  `player_speed` (mirroring `scenario_elapsed`), so the ch3 trigger is pure
  content (an OnUpdate handler + Expression filter), and any scenario can react
  to player speed.

See GOAL.md in this folder for the done-definition and live task queue.
