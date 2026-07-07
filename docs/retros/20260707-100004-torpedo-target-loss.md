# Retro: Torpedo target-loss fix (freeze instead of vanish)

- TASK: 20260707-100004
- BRANCH: feature/torpedo-target-loss
- PR: #29 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, one intended-behavior note)

See `tasks/20260707-100004/TASK.md`; this retro is about how the working went.

## What went well

- Small, surgical fix (swap `despawn()` for `remove::<TorpedoTargetEntity>()`) with
  a deterministic unit test that reproduces the exact bug: target alive -> tracked,
  target despawned -> torpedo survives, position frozen, dead link dropped. The test
  is the authoritative proof, independent of physics timing.
- Verified the thing being removed was not load-bearing. Before deleting the
  `despawn()`, checked whether it was doing scenario cleanup - it was not, because
  torpedoes get `ScenarioScopedMarker` via `on_add_entity_with::<TorpedoProjectileMarker>`
  and are torn down on scenario change anyway. That check is what separates a safe
  removal from a slow entity leak.
- Reused the range (`06_torpedo_range`) as an end-to-end smoke check and confirmed
  the old per-frame `not found in q_target` spam is gone.

## What went wrong

- The range smoke run did not actually exercise the freeze path (0 freeze events):
  the three torpedoes detonated before their shared target outlived them, so no
  torpedo was mid-flight when its target died. The range confirmed "no regression",
  not "freeze works" - that came from the unit test. Fine here, but the range as
  built does not deterministically create a mid-flight target-loss.

## What to improve next time

- When a fix's key scenario depends on timing (target dying while a projectile is
  in flight), lean on the deterministic unit test for the assertion and don't expect
  the interactive range to reproduce it by luck. If range-level proof is ever
  required, add a scripted trigger (e.g. destroy a gate on a timer) rather than
  hoping the timing lines up.

## Action items

- [ ] NIT R1.1 (intended, no change): dropping the target link makes the torpedo
      eligible to re-acquire the ship's current target rather than strictly freezing.
      Deliberate and documented.
- [ ] Optional, not filed: a scripted "kill a gate mid-flight" hook in the range
      would let it demonstrate freeze-and-continue on demand. Low value; skip unless
      the range needs it.
