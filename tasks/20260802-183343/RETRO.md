# Retro: Port the scripted autopilot driver into nova_autopilot

- TASK: 20260802-183343
- BRANCH: feat/autopilot-driver-port
- REVIEW ROUNDS: 1

## What went well

- Breadth matched the plan exactly: one module, one lib.rs line, one task
  record. The epic's "small children so each lands on its own" split held.
- The plan named the test rig's three seams (`InputPlugin`, `ManualDuration`,
  the `Once` arming helper) before any code, and all three survived contact.
  Two of the four DoD tests passed on the first compile.
- Mutation testing carried the review: six mutations, each killing exactly one
  test, is what turned "the tests pass" into "the DoD is pinned".

## What went wrong

- Two first-run test failures, both rig-side. `Messages<AppExit>` is
  double-buffered, so an exit written mid-run was gone before a later frame
  drained it; and `Update` never observes the default state because
  `StateTransition` applies the driver's first set within the same frame. The
  plan predicted the timeline and the ordering but not Bevy's own timing.
- The review's R1.1 found a guard no test exercised - the plan listed four DoD
  tests and stopped there, so the branch that only fires when the timeline
  starts in the CURRENT state was never reached. The from-scratch challenge in
  `plan` asks whether the design is right; nothing asked which BRANCHES of the
  ported code the named tests actually reach.
- A `git checkout --` intended to revert a one-line falsification mutation
  wiped the whole file back to its committed stub, costing a full rewrite. The
  file was tracked-but-stub, so the revert was silent and total.

## What to improve next time

- Porting a module with N conditional branches: enumerate the branches first,
  then check each DoD test against them. The gap is cheap to find at plan time
  and cost a review round here.
- For the remaining driver ports (`20260802-183346`, `20260802-183349`), lift
  the rig wholesale: the per-frame exit-draining `run()` helper, the
  `ManualDuration` clock and the `Once` arming helper are all reusable, and
  the state assertions start one transition in.
- Falsification mutations belong on a scratch copy (`cp` to the scratchpad,
  `cp` back), never on `git checkout --`, whenever uncommitted work sits in the
  file.

## Action items

- None requiring a task. The rig-reuse note lands with the sibling ports
  already tracked under the epic.

## Process signals

- Knowledge: the falsification-revert lesson and the Bevy test-timing lesson
  are recorded here; the central knowledge repository at
  /home/alex/personal/agent-knowledge was not written to in this session.
- Context: no compaction warning and no handoff; the working set stayed within
  one focused pass.
