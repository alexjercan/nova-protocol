# Retro: Pause menu (ESC overlay, Back to Main Menu)

- TASK: 20260711-185156
- BRANCH: feature/pause-menu (squashed to master d327f04)
- REVIEW ROUNDS: 2 (round 1: 3 MAJOR, 2 MINOR, 1 NIT - heaviest round of
  the menu family)

## What went well

- The plan's verify-first steps resolved the real unknowns cheaply before
  code: avian's pause API, the free ESC binding, the PlayerSpaceshipMarker
  proxy for scenario-play-vs-editor, and the clean Back path riding the
  existing ambience LoadScenario teardown.
- The e2e harness pattern matured: the round-2 rerun asserted the exact
  regression review flagged (G under the overlay engages nothing, before
  AND after resume) - review findings became executable within the cycle.

## What went wrong

- R1.1: the pause gate covered system sets but the entire input layer is
  observers, which sets do not touch - autopilot could be engaged and the
  scenario script advanced while "frozen". Root cause: the gate was
  designed against the schedule mental model (sets) without enumerating
  the other execution paths (observers, hooks). 14 observers across two
  crates needed guards.
- R1.3: a checked-off test step did not exist, and the e2e's frozen-ship
  assertion could not have failed without the gate (paused virtual time
  stops FixedUpdate regardless) - the third cycle in a row where a
  verification would have passed with the mechanism deleted (180426's
  speed threshold, 180455's identical screenshots, now this). The
  discriminating question - "would this check fail if I deleted the
  thing?" - is now a ledger entry at x3, pending promotion.
- R1.2: the overlay copied the main-menu root's non-blocking Pickable into
  a context where interactive UI sits beneath. Copying a pattern must
  re-check the assumption that made it correct at the source.

## What to improve next time

- A global gate (pause, disable, mode switch) is only done when every
  execution path is enumerated: systems in sets, OBSERVERS, hooks, and
  out-of-crate consumers of the same input layer. Grep add_observer before
  claiming coverage.
- For every verification step, ask "would it fail without the change?"
  before ticking it - and name the committed artifact in the close record.

## Action items

- [x] LESSONS.md: new `set-gates-miss-observers`; new
  `would-it-fail-without-it` (x3, moved to Pending promotions); bumped
  `out-of-context-review-pass` to x4.
