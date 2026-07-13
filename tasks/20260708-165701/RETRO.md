# Retro: Turret lead/intercept pip (HUD)

- TASK: 20260708-165701
- BRANCH: weapons-hud (shared arc branch)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 1 NIT, addressed; round 2 APPROVE)

A smooth consumer cycle: the substrate did its job and the pip was a
~150-line module plus tests. What shipped is in the task's Resolution.

## What went well

- **The substrate paid off on its first fresh consumer.** No projection,
  visibility, or sizing code in the pip module at all - just a reconcile
  system and a 15-line driver. The widget's `Point` anchor design (decided in
  the spike for exactly this case) fit without modification.
- **Deviating from the plan and saying so.** The planned add/remove observers
  would have missed sections attached after `PlayerSpaceshipMarker`; the
  reconcile system covers every ordering in one idempotent pass. The plan
  step was rewritten to match reality and the rationale recorded, so the
  TASK.md stays truthful.
- **Reviewing the data source, not just the rendering.** Checking who
  actually writes `TurretSectionTargetVelocity` exposed that the game never
  feeds it - the pip renders a degenerate lead today. Scoped correctly as a
  follow-up gameplay task (20260709-173700) instead of blocking a rendering
  task on a gameplay gap.

## What went wrong

- **The plan encoded a lifecycle mechanism before checking spawn ordering.**
  The observer design was written at plan time on the assumption that turret
  children exist when the player marker lands; implementation-time reading
  showed that is not guaranteed. Harmless here because the deviation was
  cheap, but the plan could have said "reconcile" from the start had it
  checked the spawn path.
- **The example's "loses its target" stage needed a rethink.** Player-input
  turrets never lose their target input (they always aim down the camera
  ray), so the planned assertion was impossible as written; the disabled-
  section path (`SectionInactiveMarker`) is the real-world way a pip dies.
  Root cause: the plan step borrowed the phrase from the task description
  without checking what "losing a target" means for the player input path.

## What to improve next time

- When a plan step names a lifecycle mechanism (observers vs reconcile vs
  polling), verify the spawn/attach ordering it depends on before writing it
  down.
- When an assertion is phrased from a feature description ("disappears when
  it loses its target"), trace the actual code path that would cause it
  before planning the test around it.

## Action items

- [x] tatr 20260709-173700: feed locked-target velocity into turret lead
  (filed during review; makes the pip a real intercept marker).
