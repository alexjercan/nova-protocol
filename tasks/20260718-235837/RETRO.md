# Retro: CI red - 10_playable smoke fails post lock-dwell (guns/burn binding collision)

- TASK: 20260718-235837
- BRANCH: fix/10-playable-smoke-guns-burn (landed to master via GitHub PR #57, commit 53c0e573)
- REVIEW ROUNDS: reviewed on PR #57 (not through the /review cycle); CI run 29661054915 green on the merge

## What went well

- The decisive clue was a SIDE variable, not the failing assertion. The
  backstop reported `goto=false`, which points at the travel sweep, but
  `arrived=1.0` was impossible under the intended choreography - the ship
  should never have reached the beacon. Chasing the impossible variable
  found the burn/fire binding collision faster than staring at the lock code.
- The new-looking mechanics (lock_refire_secs, ORBIT trim, the acquisition
  dwell) were read for their semantics before being blamed. Two of them were
  innocent; reading first avoided two wrong fixes.
- Root cause was pinned to a real mechanism with real numbers (Space bound to
  both "guns" and FlightBurnInput; ship overruns the 18u trigger and leaves
  the 18-degree radar cone), not a theory.

## What went wrong

- The fix was landed via a GitHub PR (#57) but the task was left IN_PROGRESS
  and its sprout worktree/branch left behind. A later session (this one)
  found a stale branch, had to reconcile master vs branch to discover the
  code was already on master and CI already green, before it could close the
  task. Root cause: the landing-via-PR path skipped the task's close-out and
  sprout cleanup that the flow's own land step performs.

## What to improve next time

- When resuming a task that has a leftover sprout/branch, first check whether
  the change already landed on master (compare the target file, look for the
  merge commit / PR) before re-doing or re-reviewing anything. A `git diff
  master <branch> -- <the-real-file>` that comes back empty means the work
  converged; the branch is just stale.
- A latent class of bug lives here: PlayerControllerConfig `input_mapping`
  bindings silently overlay the flight rig's bindings (consume_input: false),
  so any content section mapped to W/S/Space/RightTrigger double-drives
  flight. A content lint for that overlap would catch this at author time.

## Action items

- [ ] Follow-up (surfaced to user, not yet filed): content lint for
  input_mapping sections that collide with the flight rig bindings
  (W/S/Space/RightTrigger) - could fold into the content-lint task
  20260718-152240.
- [x] Closed task 20260718-235837 and cleaned up the stale
  fix/10-playable-smoke-guns-burn sprout/branch.
