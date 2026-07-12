# Retro: Sticky ship locks (B5)

- TASK: 20260712-203353
- BRANCH: feature/sticky-focused-lock (landed 652184e)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES - one MAJOR; round 2 APPROVE)

Process notes only; behaviour + evidence in TASK.md, spike 20260712-203235.

## What went well

- Review caught a real regression the whole test suite (44 green) and both
  autopilots missed: universal stickiness broke NAV re-designation, because the
  same lock resource is also the GOTO/torpedo designator and CTRL+scroll cycles
  only hostile ships. The catch came from the review skill's "trace one
  load-bearing claim independently" rule - following how the lock is CONSUMED
  (into `AutopilotAction::Goto`), not just how it is produced. Ship-only `held`
  (`is_ship`) fixed it in one condition.
- Rewrote, rather than deleted, the test that encoded the old behaviour
  (`pinned_lock_holds_..._until_expiry` asserted an expired pin re-aims). The
  failing test was correct to fail - it was pinning the OLD contract - so it
  became `an_expired_pin_leaves_the_lock_sticky_not_re_aimed` with a delivery
  guard, instead of being weakened away.
- Reused the existing `pinned` gate's shape (`if !pinned && !held`) and the
  candidate tuple's `is_ship` flag, so the whole feel change is a few lines on
  machinery that already existed - the spike's central bet.

## What went wrong

- The plan/spike framed stickiness as a pure combat-feel change and did not
  account for the lock's SECOND role (nav designator). Root cause: reasoned
  about the lock from the reported symptom (torpedo steals combat lock) without
  enumerating every CONSUMER of `SpaceshipPlayerTargetLock` first. The
  `verify-first`/consumer-gate lesson again: follow the data into every consumer
  before changing a shared resource's behaviour.
- The green test suite was falsely reassuring: no test exercised nav
  re-designation while locked, so 44 passing tests hid a MAJOR. A behaviour
  change to a SHARED resource needs a test per consumer, not just per producer.

## What to improve next time

- Before changing the behaviour of a shared state resource, list its consumers
  (grep the type) and confirm the change against each one - here GOTO was the
  missed consumer. Add a test per affected consumer, not just for the producer's
  own logic.

## Action items

- [x] Lessons ledger updated: bumped `verify-first-plan-steps` to x7 with a
  shared-state-consumer variant (enumerate a shared resource's consumers before
  changing its behaviour; test per consumer).
- Playtest items recorded on TASK.md / spike: the "must CTRL+scroll to switch"
  feel and the residual "held ship lock blocks aiming at a nav target" gap
  (aim-away-release remedy).
