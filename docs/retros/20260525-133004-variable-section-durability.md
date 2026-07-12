# Retro: Variable damage by section type

- TASK: 20260525-133004
- BRANCH: v0.5.0/combat-depth (squash-landed to master as e1b949f)
- REVIEW ROUNDS: 0 (autonomous flow, self-review only)

## What went well

- Reading Bevy 0.19's observer source BEFORE designing killed a whole dead-end.
  The intuitive design for "variable damage" was a nova observer on
  `HealthApplyDamage` that scales the amount before bcs subtracts it. Grepping
  bevy_ecs's observer docs found the flat statement that observer execution order
  is arbitrary ("make no assumptions"), so that design would have raced the
  subtractor and lost half the time. Finding this in the source cost minutes;
  finding it after shipping a flaky damage system would have cost a debugging
  saga. The spike then turned the wall into the recommendation (own the trigger,
  pre-scale) instead of fighting it.
- Facing a genuine architectural fork (per-type health vs a resistance mechanism
  that would have touched bcs), I stopped and asked with concrete, costed options
  rather than guessing. The answer ("per-section health now; nova-side
  DamageResistance once damage types exist; don't touch bcs") reshaped both this
  task AND the task-3 spike direction - a guess would have built the wrong thing
  twice.
- Expressing the durability scheme as named baseline constants + an ordering
  regression test turned "variable by type" from loose magic numbers into a
  checked invariant that cannot silently drift back to uniform.

## What went wrong

- The implemented direction (thrusters fragile / turrets tough, per the task
  title) is the INVERSE of the example the user gave when answering the fork
  (turret 60, thruster 140). Root cause: the task's recorded text and the user's
  offhand "for example" numbers disagreed, and I picked the recorded spec without
  surfacing the contradiction until the final report. It is a one-line swap and
  flagged as a playtest knob, but it should have been an explicit confirm at the
  moment I noticed the conflict, not a footnote after the fact.
- Process/infra, spanning both tasks of this cycle: my spike commit leaked onto
  master. A parallel /flow session (its own sprout worktree) landed its round to
  master by checking out master IN THIS SHARED in-place checkout, silently moving
  my session off the feature branch; my next `git commit` landed on master. This
  is `landing-checkout-not-yours` a second time (first was a near-miss). Recovered
  non-destructively - cherry-picked the commit onto the branch via a temporary
  worktree, then removed it from master with a compare-and-swap-guarded reset - but
  the whole episode was avoidable.

## What to improve next time

- When the authoritative source (task text) and a fresh user aside disagree on a
  directional choice, confirm the direction at the moment of noticing, not in the
  wrap-up. A cheap one-line question beats a shipped-then-flipped default.
- In a shared in-place checkout, verify `git branch --show-current` is the
  intended branch immediately before EVERY commit, not just at session start -
  another session can move the shared HEAD between commits. Prefer a real
  sprout/worktree when the user asks for a branch. (Saved as a memory too.)

## Action items

- [ ] Confirm the durability direction with the user (thrusters-fragile as
      shipped, or flip to the turret-fragile example) - one-line swap in
      nova_assets/sections.rs.
- [x] LESSONS.md: added verify-engine-guarantees-in-source; bumped
      landing-checkout-not-yours to x2.
- [x] Memory: shared-checkout-branch-leak recorded for future in-place sessions.
