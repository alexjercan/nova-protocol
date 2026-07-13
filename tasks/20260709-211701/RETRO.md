# Retro: Shooter-frame bullet lead

- TASK: 20260709-211701
- BRANCH: fix/turret-lead-inherited-velocity (squash-merged onto master)
- REVIEW ROUNDS: 1 (APPROVE, no findings)

## What went well

- **The user's playtest hypothesis was precise and correct** ("maybe the
  bullets inherit inertia from the spaceship") - verified in
  shoot_spawn_projectile in one read. Second time this repo's cycle starts
  from a sharp user observable; asking players what they SEE keeps paying.
- **Fix at the solve, not the feeds.** Subtracting the shooter's muzzle
  point velocity inside update_turret_aim_point corrected the player feed,
  the AI and the turret-range example at once, and reused the exact
  point-velocity + COM-lift expression from the spawn path so aim-time and
  fire-time physics share one formula.
- **The invariant test IS the bug report.** Formation flight (equal
  velocities) must aim at the target itself; the old world-frame solve
  fails it by leading a relatively-stationary target. One test encodes the
  entire complaint.

## What went wrong

- **The gap shipped in the same session that filed its prerequisite.** The
  velocity feed (173700) made turrets lead for the first time, and the lead
  was wrong the moment the shooter moved - the review of 173700 checked the
  feed values but never asked what frame lead_intercept_point assumes.
  Root cause: the solver was pre-existing tested code, so it was trusted;
  but "tested" covered a static shooter only. When new data starts flowing
  into old math, re-derive the old math's assumptions against the new data
  source.

## What to improve next time

- When wiring a new input into an existing solver, state the solver's frame
  /unit assumptions explicitly in the task and check each against the new
  feed - especially for physics (frames, inherited velocities, local vs
  world).

## Action items

- None; scroll-wheel bind (20260709-211702) is the remaining playtest item.
