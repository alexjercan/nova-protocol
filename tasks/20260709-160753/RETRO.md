# Retro: Camera twitch (fixed-tick stair-step under the smoothed camera)

- TASK: 20260709-160753
- BRANCH: fix/camera-twitch-interpolation (squash-merged as 2642f70)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 2 MINOR + 3 NIT; round 2 APPROVE)

A tight cycle: playtest report to landed two-component fix in one sitting.
Mechanism and fix in `tasks/20260709-160753/NOTES.md`.

## What went well

- **Verified the user's hypothesis before building on it.** The report
  suspected a frame-lag/transform-sync bug; checking the actual schedules
  (easing in RunFixedMainLoop, anchor in Update, camera in PostUpdate)
  disproved the lag and pointed at the missing per-body interpolation opt-in
  instead. The fix stayed two component insertions rather than a system
  reorder chasing a lag that did not exist.
- **The bug was predictable in hindsight and the precedent was in-repo.**
  Turret bullets already carried `TransformInterpolation` for exactly this
  reason; the ship never did because nothing sampled it smoothly until the
  camera got smoothing. When a change makes something newly *observable*
  (smoothing sampling the ship at render rate), audit what it now observes.
- **Proving the test against the bug produced the best evidence of the
  cycle.** Commenting the component out made the behavioral test fail with
  the exact reported symptom (repeated poses on non-tick frames, one full
  tick of motion per jump) - diagnosis, fix, and regression net confirmed in
  one run.

## What went wrong

- **I delivered presence checks where the task promised a behavioral test**
  (review R1.2) - the component-exists assertions would stay green if an
  avian upgrade changed the interpolation default while the twitch returned.
  Root cause: I wrote "if feasible" in my own plan and then treated it as
  permission to skip; the reviewer showed it was a 30-line test. If a plan
  hedges with "if feasible", the implementation owes an explicit
  feasibility answer, not silence.
- **"Physics is untouched" overstated again** (R1.1) - third docs-precision
  finding in three cycles (perception claims, optimum-vs-delivered times,
  now this). Projectile spawns and the torpedo fuze DO now read the eased
  pose; intentional, but the doc had to say so. The pattern: I write the
  summary sentence from the design intent instead of from the list of
  actual readers/consumers. Enumerate consumers first, then summarize.
- **I ran the squash-merge from inside the worktree** - the silent no-op the
  diegetic-autopilot retro documented verbatim ("run flow's merge/cleanup
  from the main checkout"). Caught immediately by the empty commit, redone
  correctly, but this is a known lesson that failed to hold under a
  compound command whose first token was `cd <worktree>`. Rule refined: the
  landing sequence gets its own command that starts with `pwd` in the main
  checkout - never appended to a command that cd'd elsewhere.

## What to improve next time

- Landing sequence: separate command, `pwd` first, never after a `cd` into
  the worktree being merged.
- Write doc summary claims from an enumeration of actual consumers, not
  from intent; the reviewer keeps finding the gap between the two.
- A plan's "if feasible" is a question the implementation must answer
  explicitly - feasible-and-done or infeasible-because.

## Action items

- [ ] User re-runs the playtest checklist (flip full vs stripped, burn lean,
  now twitch-free flight) - remaining knob feedback lands as direct retunes.
- [x] R1.4/R1.5 accepted as-is (per-frame propagation cost negligible at
  current scene sizes; redundant presence checks guard different seams).
