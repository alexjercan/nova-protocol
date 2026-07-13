# Retro: AI threat-tiered target selection

- TASK: 20260709-225727
- BRANCH: feature/ai-target-selection (squash-merged as a8f4438)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Fourth task of the AI combat arc. The consumer swap (five systems off the
player Single onto AITarget) was near-mechanical because the state-skeleton
task had already funneled every system through one (state, anchor) shape.

## What went well

- **Tier-as-Ord-enum kept the picker honest.** Deriving Ord on
  AITargetKind and using a lexicographic (tier, distance) min_by avoided a
  weighted score entirely - nothing to tune, nothing to explain, and the
  "ship beats nearer torpedo" rule is structural rather than numeric.
- **Deferring threat-memory scoring was checked, not assumed.** Grepped
  bcs and nova for HealthApplyDamage sources before planning: `source`
  exists but nothing real populates it. The deferral is recorded in the
  task Notes with its blocker and landing spot (225731) instead of
  silently shrinking scope.
- **Each arc task keeps paying the next one.** requires -> allegiance ->
  state gate -> now AITarget: the fifth consumer swap took minutes, and
  existing tests upgraded to the REAL acquisition pipeline (run
  update_ai_target first) rather than accumulating hand-set fixtures.

## What went wrong

- Nothing of substance; one round, no findings. The only friction was
  updating four existing test call-sites for the new pipeline - expected
  fallout of replacing a Single, and the compiler found them all.

## What to improve next time

- Keep the "check the blocker before deferring" habit: a deferral with a
  verified reason and a landing task reads as engineering; one without
  reads as scope-shaving.

## Action items

- [ ] 20260709-225731 (evade) also owns: damage attribution
  (HealthApplyDamage.source population) + threat-memory scoring joining
  pick_ai_target - noted in both tasks.
