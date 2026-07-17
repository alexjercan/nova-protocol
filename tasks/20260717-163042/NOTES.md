# Arrival telegraphs - design record

Task 20260717-163042, spike tasks/20260717-155740/SPIKE.md (option C).

## What shipped

- AIEngageGrace { timer } (prelude-exported, reflected): while unfinished
  and the ship undamaged, the pure next_behavior_state holds the passive
  routine against the engage pull (a leash-style early return; grace and
  leash compose - passive either way). Damage ends the grace immediately
  AND permanently: the ticking system pins the timer via tick(remaining)
  - set_elapsed alone does NOT set Bevy's finished flag, only tick()
  does, which the first test run caught.
- AIControllerConfig.engage_delay: Option<f32> (serde default; strict
  RON engage_delay: Some(8.0); non-positive = no grace, documented);
  spawn inserts the component only for positive delays, next to the
  leash insert.
- Point defense is deliberately untouched: the PD path bypasses behavior
  states, so a graced ship still swats inbound ordnance - pinned by its
  own test.
- The decision-table contract change re-pinned at the boundary: 29 pre-existing call
  sites gained the grace argument (the system call plus 28 test callers;
  27 by a top-level-comma-aware script sweep, 1 multiline by hand), plus new table rows (grace holds, damage
  overrides, grace+leash compose, ungraced delivery guard).

## The authoring pattern (documented in the wiki)

announce (clock-gated StoryMessage, e.g. elapsed > T) -> spawn far with
engage_delay covering the approach -> marker attach -> the fight starts
after the player has read the warning. The comms queue (163033) makes
the announce line survivable; this task makes the arrival passive.

## Verification

- cargo test -p nova_gameplay input::ai:: 92/92 (4 new grace tests: hold
  then engage with an ungraced delivery-guard twin, damage-now +
  pinned-forever, PD-through-grace, plus the table rows).
- cargo test -p nova_scenario --features serde: spawn wiring (positive /
  zero / omitted) + strict-RON parse tests green.
- content_lint clean; workspace --all-targets green; fmt last. Full
  suite on CI per standing instruction.

## Post-review addenda (Round 1)

- R1.1 (MAJOR, mutation-proven): the grace return's Engage-demotion path
  is THE production path (AIBehaviorState defaults to Engage, so every
  graced spawn's first tick demotes through it) and was unpinned - a
  mutation restricting the return to passive states survived all tests.
  Now pinned three ways: a table row (Engage + grace -> Patrol), the
  system rig seeding the DEFAULT state, and a comment on the return.
- R1.2: the guide gained the telegraphed-arrivals section the ticked
  step had claimed.
