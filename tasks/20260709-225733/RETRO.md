# Retro: AI point-defense turret priority

- TASK: 20260709-225733
- BRANCH: feature/ai-point-defense (squash-merged, see git log)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

Pulled forward mid-arc on a live user decision ("torpedoes are the PDC's
main purpose") while the standoff-flight task was being planned.

## What went well

- **The user redirect slotted in cleanly because the arc's seams were
  real.** Target selection (225727) had separated WHO to fight from HOW
  each actuator uses it, so "guns prioritize torpedoes, hull keeps
  chasing ships" was an override at the gun layer, not a redesign of
  target selection. The spike's original open question (when does the
  torpedo-urgency flip apply) got answered by the user instead of
  guessed.
- **Recording the decision in the task before coding** kept the plan and
  the diff aligned with what was actually asked - the review could check
  the user's words against the behavior.
- **Reused the tier-then-distance lexicographic picker pattern** for the
  hunting-me preference; second use of the 225727 idiom, zero new scoring
  machinery.

## What went wrong

- Nothing of substance. The paused standoff task (225729) sat in its own
  worktree untouched, which is exactly what per-task worktrees are for.

## What to improve next time

- When a user interjects a behavior decision mid-flow, do what happened
  here on purpose every time: write it into the owning task as a "User
  decision" section first, then re-plan that task, then build.

## Action items

- Resume 20260709-225729 (standoff flight) - worktree already sprouted.
- 20260709-225731 (evade) still owns the break-off burn + damage
  attribution.
