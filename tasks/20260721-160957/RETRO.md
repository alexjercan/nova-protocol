# Retro: Lifeline (ch3a) - convoy defense

- TASK: 20260721-160957
- BRANCH: content/lifeline-ch3a (landed 4a1c0274)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINORs + NIT fixed in-round)

## What went well

- Verify-first on the spawn path (controller None ships still carry
  SpaceshipRootMarker) replaced the planned AI-flown convoy - and its whole
  leash/chase problem-space - with a one-line design choice that the
  narrative absorbed cleanly. The rig task's verdict held; only the
  controller kind adapted, openly recorded.
- The probe example's clock fast-forward (jumping the accumulated
  scenario_elapsed variable) let a 4-minute clock-driven scenario prove its
  entire defeat/retry/waves/victory arc in a ~15s real-time walk - a
  pattern the finale can reuse.
- The out-of-context reviewer traced the act machine against source
  dispatch semantics and found the one real race (R1.1): last-write-wins
  CurrentOutcome + an every-pulse clock gate widens the defeat-overwrite
  window that kill-gated scenarios never had.

## What went wrong

- The terminal-act discipline was applied where it was DERIVED (the
  both-haulers loss, whose doc comment even states the principle) but not
  swept across every terminal handler - the player-death path missed it.
  Root cause: pattern-by-motivation instead of pattern-by-class; the same
  pre-existing shape sits in broadside (filed as 20260721-182034).

## What to improve next time

- When any handler writes a terminal state, sweep EVERY handler that
  declares an outcome and make each one close the gates itself - the
  outcome resource is last-write-wins, so a single unguarded path can
  overwrite a settled result.

## Action items

- [x] tatr 20260721-182034 (p47): broadside terminal-act fix, same class.
- [x] LESSONS.md: appended `outcome-is-last-write-wins-close-the-act` (x1).
