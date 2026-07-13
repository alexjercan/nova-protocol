# Retro: Torpedo proportional navigation guidance

- TASK: 20260525-133021
- BRANCH: feature/torpedo-pn-guidance
- PR: #31 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, two tuning NITs)

See `tasks/20260525-133021/TASK.md`; this retro is about how the working went.

## What went well

- Designed PN against the *actual* control model first. The torpedo isn't
  free-acceleration - it orients a PD controller and thrusts forward - so PN had to
  produce a desired *heading*, not a lateral acceleration. Getting that framing right
  up front meant the implementation fit the existing sync/thrust systems cleanly
  instead of fighting them.
- Made the guidance a pure function and hand-verified the sign before trusting it. I
  worked the crossing-target case by hand (a +X-crossing target must yield a +X
  lead) and caught that the command is `Ω × V`, not `V × Ω` - in the derivation, not
  by watching torpedoes fly the wrong way in-game. The three unit tests then pinned
  it (lead / straight-pursuit / degenerate).
- One source of truth (`TorpedoSteering`) for the desired heading, so the orientation
  and thrust systems can't diverge, and target velocity via `LinearVelocity` so PN
  degrades to pursuit on target loss - reusing the behavior from the 100004/120608
  fixes rather than special-casing.

## What went wrong

- Hit Bevy's 15-element tuple-bundle limit. Adding two components (`TorpedoGuidance`,
  `TorpedoSteering`) to the projectile spawn pushed the tuple to 16, which is not a
  `Bundle` (E0277). Cost one build cycle. Root cause: I didn't count the existing
  bundle size before adding to it. Fix is standard - nest the new pair into a
  sub-tuple so it counts as one element.

## What to improve next time

- Before adding components to an existing `spawn((...))` tuple, count the current
  elements - Bevy implements `Bundle` for tuples only up to 15. If adding would
  exceed it, nest proactively rather than eating an E0277 build cycle. (This spawn
  was already at 14.)

## Action items

- [ ] NITs R1.1 / R1.2 -> range tuning: the `nav_constant` default (3.0), the
      guidance/controller turn-authority coupling, and the low-speed pursuit->PN
      handoff threshold are all feel knobs best set in `06_torpedo_range`. No code
      change; no new task.
- [ ] Blast/param unhardcoding (`20260706-162913`) is the natural next torpedo task -
      already tracked.

## Round 2 addendum (the fix that followed the user's bug report)

The user reported the torpedo still flew off randomly and never turned onto the
target - even a stationary one. Round 1's confidence was misplaced, and the miss
carries the sharpest lesson of this task:

- **Test from the system's real initial conditions, not the algorithm's happy
  path.** The Round 1 closed-loop tests initialized the torpedo already flying at
  the target at speed 60. The real torpedo leaves the bay at ~1 u/s *sideways*
  with its nose perpendicular to the velocity. Velocity-anchored PN diverges from
  exactly that state - the tests could never see it. The launch state was sitting
  in `shoot_spawn_projectile` (`spawner_transform.up() * spawner_speed`) the whole
  time; reading the spawn code when writing the sim would have caught it.
- **When a harness metric looks "close enough", ask what physical limit it is
  pinned against.** The 19-21u closest-approach plateau was not noise: it was the
  turning circle at unbounded speed vs the 15u fuze, and later the speed log
  (60 u/s vs the 35 cap) exposed thrust pumping. Each plateau value pointed
  directly at its root cause once compared against speed/turn-rate and the fuze
  radius.
- **Scalar thrust gating interacts with steering in non-obvious ways.** A
  total-speed cutoff left the torpedo ballistic (cannot steer, 21u miss); an
  along-nose gate preserved steering but let turning pump total speed to 60. The
  workable pair was along-nose gating for deliberate acceleration plus linear
  damping for a true terminal velocity - measured, not assumed: each variant went
  through the harness before judging.

Net: three empirical instruments (law tests from launch state, a closed-loop
thrust sim, and the 06/07 headless harnesses with speed + approach telemetry)
turned a vague "PN doesn't work" into two crisp root causes and a verified fix.
