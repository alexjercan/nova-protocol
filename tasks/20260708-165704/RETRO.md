# Retro: Off-screen target/threat edge indicators (HUD)

- TASK: 20260708-165704
- BRANCH: feature/edge-indicators
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- A smooth cycle: the widget's ClampToEdge/arrow path (built two tasks
  earlier, tested then) needed zero changes for its first real consumer -
  paying for the substrate up front (the weapons-hud spike's core bet)
  delivered exactly as designed.
- The pure tracked_entities()/arrow_color() split and the reconcile
  pattern were lifted straight from targeting.rs/component_lock.rs;
  pattern reuse again produced a finding-free review.

## What went wrong

- A step was ticked before it was done (the weapons-hud spike Fix record),
  caught only during the pre-review honesty pass. Root cause: ticking the
  "verify + docs" checkbox for the verify half while the docs half was
  still pending - compound checkboxes invite half-true ticks.

## What to improve next time

- Do not bundle unrelated actions in one plan step; when a plan arrives
  that way, tick only when EVERY clause is done, or split the step in
  TASK.md first.

## Action items

- [x] Bumped `half-ticked-compound-steps` into LESSONS.md (new, x1).
- User feedback arrived mid-cycle: pressing CTRL alone cycles the target
  (bug in landed 20260708-165705). Filed as its own task per the flow
  discipline; see that task's retro for the mechanism.
