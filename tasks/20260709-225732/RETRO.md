# Retro: AI torpedo usage from Engage: launch envelope + cooldown

- TASK: 20260709-225732
- BRANCH: feature/ai-torpedo-usage (local branch by user request, merged)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR and one NIT, both addressed in-cycle)

A one-round cycle on wave 3's torpedo task: launch envelope, per-bay
cadence, and the AI-side commit-on-launch sibling of the player path.

## What went well

- **Reading the consumer before designing the producer.** Before choosing
  the held-trigger model, shoot_spawn_projectile was read end to end:
  fire_state.reset() on launch plus the input-before-sections set ordering
  is what makes "hold the trigger, reset the cadence on the NEXT frame"
  safe. The alternative (predictively resetting the cadence when pulling
  the trigger) was rejected on evidence, not taste: a disabled bay ignores
  the pull, and a predictive reset would burn the cooldown on a launch
  that never happened.
- **Prior retro lessons applied deliberately.** The Add-observer race
  (sections spawn before their root's AISpaceshipMarker command applies)
  was reasoned about at design time and sidestepped with a lazy insert -
  the 225731 lesson about trusting the real event/command order, not the
  convenient one. And the review round ran in the honest order this time:
  findings written first, fixes second, responses and ticks last.
- **Pure-function seam, fourth cycle running.** ai_torpedo_envelope kept
  the geometry unit-testable; the 11 new tests run the real systems for
  everything stateful (lazy insert, trigger release, commit, cadence).

## What went wrong

- **R1.1 (commit unfiltered): the launch side gated on "target is a
  ship", but the commit side attached the owner's current AITarget
  unfiltered - one frame apart, so a target that died in exactly that
  frame could flip the pick to a hostile torpedo and send fresh ordnance
  chasing ordnance.** Root cause: the trigger system was written as "the
  decision" and the commit system as "bookkeeping", so the decision's
  preconditions were never re-checked where the effect lands. When a
  decision and its effect are separated by a frame, the effect site must
  re-validate the preconditions.

## What to improve next time

- When adding a gate to a decision, grep for every downstream consumer of
  the same input and check whether the gate must hold there too -
  especially across frame boundaries, where the input can change between
  decision and effect.

## Action items

- Playtest knobs noted in code, not tasks: AI_TORPEDO_COOLDOWN_SECS, the
  range band constants, and the alignment gate (launches only open on
  approach legs, since the standoff orbit points the hull off the
  bearing - watch whether that reads as intended "attack runs" or as the
  AI hoarding torpedoes).
- The consolidated v0.4.0 CHANGELOG task (20260710-093420) now also
  covers this task.
