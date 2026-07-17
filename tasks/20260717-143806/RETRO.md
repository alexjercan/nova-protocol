# Retro: Auditor armament + audit acknowledgments

- TASK: 20260717-143806
- BRANCH: fix/auditor-armament (landed ce189482)
- REVIEW ROUNDS: 1 (APPROVE; 1 MINOR + 3 NIT)

## What went well

- The envelope math was checked BEFORE implementing, which surfaced that
  the user's literal ask (worse gun => audit happy) was unsatisfiable
  while the tube exists - and the resolution (an acknowledgment
  mechanism) honored all three user signals instead of silently dropping
  one. The fork was resolved with a design, not a question.
- Writing the ack semantics tests ALONGSIDE the partition caught the
  one-ack-per-finding bug on the first run (position() ignored the used
  flag) - the test demanded the semantics before the code had them.

## What went wrong

- R1.1: a hand-summed hp number in NOTES dropped the thruster (660 vs
  730). Root cause: arithmetic done in prose while the code that sums
  sections correctly sat one file away - derived numbers in docs should
  come from running the tool (balance_audit prints the hp), not mental
  re-addition. Same class as measure-before-writing-the-number.

## What to improve next time

- When a design record quotes a derived number, copy it from the tool's
  output, not from re-derivation by hand.

## Action items

- [x] docs/LESSONS.md: bumped measure-before-writing-the-number (x2).
