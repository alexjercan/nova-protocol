# Retro: HUD projection on the frame's final camera pose

- TASK: 20260710-231928
- BRANCH: fix/hud-projection-postupdate (squash-merged as 5ba0e3c)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Fourth spoke of the twitching family; details in the task file and the
spike doc's fix record.

## What went well

- **Reading dependency schedule sources before coding killed two wrong
  designs in one task.** The plan assumed (a) a bcs change might be needed
  for ordering (it was not - the set is public) and (b) the projection
  slot was "after Propagate, before Layout" (impossible - bevy_ui lays
  out BEFORE propagation). Both were falsified by reading bcs and
  bevy_ui plugin sources in the understand-first step, not by debugging
  a mysterious one-frame lag after shipping.
- **The sweep found a latent bug bigger than the reported one**: bcs
  leaves the chase-camera move unordered against transform propagation -
  a per-build coin flip that can render EVERY frame with last frame's
  camera. Third task in this family where reading the whole seam
  surfaced an unreported defect (unordered tick-vs-shoot, frame-capped
  fire rate, now this).
- **Commit-then-sabotage A/B discipline held** (lesson from the bullet
  cycle retro applied verbatim): the Update-schedule A/B was run against
  a committed base and reverted cleanly, producing the 54 px vs
  sub-pixel evidence without risk.
- **Scope discipline on the anchor sweep**: only reads feeding PLACEMENT
  were switched to the render clock; text values and static wells were
  left alone with the reasoning documented, so the diff stayed reviewable.

## What went wrong

- Nothing that cost a cycle. Process observation: the plan encoded two
  schedule-ordering assumptions as directives ("move to PostUpdate after
  Propagate...") rather than questions, and both were wrong in detail.
  The implementation caught them because the work skill's
  update-steps-first rule was followed, but plans keep writing external
  ordering facts from memory.

## What to improve next time

- Plan steps that depend on a dependency's schedule/set ordering should
  be phrased as "verify X orders before Y, then slot accordingly" - the
  same pattern as the residual-roll retro's evidence-rig rule, now with
  a second occurrence (bullet task's avian child-pose assumption was the
  first). Third occurrence promotes it to the plan skill.

## Action items

- [ ] Optional upstream: order `ChaseCameraSystems::Sync` before
      propagation inside bcs itself (benefits non-nova consumers). File
      as a bcs-side chore next time a bcs cycle is open; nova's pin makes
      it non-urgent here.
- [x] None else; the crosshair task (20260710-231929) chains directly
      into the new PostUpdate slot.
