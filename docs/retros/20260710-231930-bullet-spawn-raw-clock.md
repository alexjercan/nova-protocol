# Retro: Bullet spawn on the raw physics clock

- TASK: 20260710-231930
- BRANCH: fix/bullet-spawn-raw-clock (squash-merged as 3f87977)
- REVIEW ROUNDS: 1 (APPROVE with 1 MINOR, resolved by filing 20260711-114640)

Third spoke of the twitching family. What changed and why is in the task
file; the spike fix record has the family view.

## What went well

- **Derive, then code.** Writing the tick-window algebra down before
  implementing exposed that the planned compensation formula ("advance by
  full velocity * overshoot") was itself wrong - the ship-motion terms
  cancel and the exact formula is simpler
  (`spawn = muzzle - muzzle_exit_velocity * lead`). Second cycle in this
  family where the plan encoded a subtly wrong mechanism and a
  ten-minute derivation/diagnostic corrected it before it shipped.
- **The rewrite surfaced two latent bugs beyond the reported one**: the
  tick-vs-shoot systems were UNORDERED in the Update set (random phase
  jitter), and the shipped 100 rounds/s fire rate was silently capped at
  render rate by the one-shot-per-frame structure. Both fell out of
  reading the whole subsystem before editing, not from the symptom.
- **A/B in both directions**: the stream regression was proven to fail
  against the pre-fix path AND with only the lead compensation disabled,
  so it guards the schedule move and the formula independently.

## What went wrong

- **A file-level `git checkout` nuked the uncommitted rewrite.** The
  A/B sabotage was applied before committing the fix, and the "revert the
  sabotage" reflex reached for `git checkout <file>` - which restored the
  branch base, discarding ~250 lines of uncommitted work. Root cause: the
  A/B pattern from the previous two cycles was always run against a
  COMMITTED baseline, and the reflex did not check `git status` first.
  Recovered from session context in minutes, but pure luck that the
  session held every edit.
- **The stream rig initially mixed two velocity families** (it fired
  during settle() before the test velocity was set). Caught by the test's
  own failure output in one run; the trigger-cold-until-armed rig shape is
  the reusable fix.

## What to improve next time

- A/B discipline, now explicit: COMMIT the fix first, then apply the
  sabotage, then revert with `git checkout` - never sabotage an
  uncommitted tree. (This is the destructive-git cousin of the
  off-axis-counter-torque retro's `git add -A` lesson: file-level git
  operations assume a clean, committed baseline.)
- Physics test rigs: keep stimuli cold until ALL initial conditions are
  set; settle() runs real frames and live systems will act during it.

## Action items

- [x] tatr 20260711-114640 filed (torpedo launch shares the eased-pose
      sampling; low severity, not blocking the umbrella).
- [ ] Watch the "plan encoded the wrong mechanism, derivation fixed it"
      pattern (now 2 occurrences: 231931's straight-line burn, this
      task's overshoot formula). A third occurrence promotes a plan-skill
      note: physics steps in plans must cite a derivation or mark the
      formula as unverified.
