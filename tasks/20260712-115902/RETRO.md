# Retro: ManeuverTelemetry teardown despawn race

- TASK: 20260712-115902
- BRANCH: fix/telemetry-teardown-race (landed as aade8b0)
- REVIEW ROUNDS: 1 (APPROVE; 3 MINOR doc-accuracy findings, addressed)

## What went well

- Fail-first discipline caught three wrong theories in one task: the
  direct-despawn rig that could not reproduce the race, the
  FallbackErrorHandler swap that structurally cannot see remove-warns, and
  the cross-entity race that does not exist. Each was exposed because a
  verification refused to fail (or to pass) on cue - would-it-fail as a
  working tool, not a checklist line.
- A refuted hypothesis was converted into durable value instead of being
  quietly deleted: the two cross-entity tests became ordering pins that go
  red exactly when bevy's command ordering would make the reverted
  hardening necessary, and the pin-coverage gap became task
  20260713-203709. The branch history (harden -> refute -> revert) was
  kept honest rather than retconned.
- Two minutes in the bevy source (queue_handled(_, warn) vs unhandled
  insert) settled what the task record's borrowed pattern got wrong.
- The fresh-context reviewer re-derived the command-queue mechanics from
  bevy source, re-ran the flight sabotage itself, and still found three
  real doc errors - including a prescribed remedy (observer-side
  try_despawn) that could not fix the race it predicted.

## What went wrong

- The two cross-entity try_* fixes were implemented straight from the
  audit agent's RACY verdicts, which were reasoned from assumed
  breadth-first queue semantics bevy does not have. Root cause: accepted a
  subagent's mechanism reasoning without demanding an executable probe
  first - code was written from a model of the system, not the system.
- The plan step prescribed the 175352 pin pattern (handler swap to panic)
  from that task's record without checking which command flavors the
  pattern escalates. Borrowed rigs inherit their limits; the record's "any
  command error now fails CI" overclaimed and the plan inherited the
  overclaim.
- NOTES.md kept the refuted end-of-queue model in its Mechanism section
  after the probe overturned it (review R1.1) - prose written under the
  dead theory was not re-read when the theory died.

## What to improve next time

- When a subagent (or a task record) asserts an engine-level ordering or
  scheduling guarantee, read the engine source or write a five-line probe
  BEFORE implementing fixes on top of it.
- After a mid-task refutation, re-read every artifact written under the
  old theory (notes, comments, task record) in one pass, not just the code.
- When borrowing a rig/pattern from another task, verify its coverage
  against the new failure mode before prescribing it in a plan step.

## Action items

- [x] tatr 20260713-203709 filed (pin gap: remove/despawn warns bypass the
      handler swap).
- [x] Ledger: bumped `would-it-fail-without-it`,
      `out-of-context-review-pass`, `verify-engine-guarantees-in-source`;
      added `borrowed-rig-coverage-check`,
      `refutation-invalidates-earlier-prose`.
