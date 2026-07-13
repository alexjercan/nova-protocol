# Retro: AI behavior state machine skeleton

- TASK: 20260709-225726
- BRANCH: feature/ai-behavior-state (squash-merged as 7a2bba1)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Third task of the AI combat arc; the smoothest cycle so far - plan, one
work pass, clean review.

## What went well

- **Idle-as-explicit-zero was the one real design decision, and it was
  made deliberately.** Gating by SKIPPING systems would leave a thruster
  stuck at its last written value (the pre-existing accidental behavior
  when the player despawned mid-fight). Writing zeros makes the state
  observable, testable, and correct. The test flips a fully-lit ship to
  Idle and asserts every actuator drops.
- **The require-with-default pattern carried a third task.** Marker
  requires Allegiance (203708), now AIBehaviorState (default Engage):
  spawn behavior unchanged, every test world participates for free, no
  scenario wiring. This is now the house pattern for per-ship AI state.
- **Unreachable enum variants with task pointers beat a growing enum
  later.** Patrol/Evade/Retreat exist, documented against their tasks, so
  four future tasks slot into a stable enum instead of each reshaping it.

## What went wrong

- Nothing of substance. One design wobble mid-plan (skip vs zero) was
  settled before code by asking "what does the thruster hold after the
  flip?".

## What to improve next time

- Keep asking the actuator-state question for every new gate: "what value
  does the output hold when this system stops writing it?" It found the
  stale-thrust trap here and the frozen-command semantics before review
  had to.

## Action items

- None; the arc's next tasks (225727 target selection onward) consume the
  skeleton as planned.
