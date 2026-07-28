# Retro: Fix dead-code warning ShipBlock.section

- TASK: 20260728-124443
- BRANCH: fix/ship-block-section-dead-field
- REVIEW ROUNDS: 1 (in-session, trivial; APPROVE)

## What went well

- Small, targeted fix that also improved the design: section identity now lives
  once (on `ShipBlock`), with the outline deriving it, instead of two copies.

## What went wrong

- The defect existed only because the PARENT task (`20260728-115435`) was verified
  with `cargo test`, under which the test-only reader of `ShipBlock.section` made
  the field look live, so the `dead_code` lint never fired. Root cause: no
  non-test build in that task's verify step.

## What to improve next time

- After a refactor that moves where a field/marker is read, run a plain
  `cargo check` (non-test cfg) before declaring done - not just `cargo test`. The
  test build hides fields that only `cfg(test)` code reads.

## Action items

- [x] Ledger: added `dead-code-hides-under-cfg-test-reader` (-> work skill).
