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
