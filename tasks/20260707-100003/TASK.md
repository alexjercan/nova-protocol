# Torpedo self-detonates on spawn when target is near; add arming delay

- STATUS: OPEN
- PRIORITY: 95
- TAGS: v0.4.0,bug,torpedo

Reported in play: firing a torpedo at a target that is already close makes the
torpedo "spawn too close and just die" - it detonates on (or within a frame or two
of) spawn instead of flying out.

Cause: `torpedo_detonate_system` detonates whenever the torpedo is within
`BLAST_RADIUS * 0.5` (currently 15 units) of the target position, and it starts
checking immediately on spawn. There is no arming gate (no minimum flight time or
minimum travel distance from the muzzle) before detonation is allowed, so a
short-range shot triggers instantly. The torpedo also spawns essentially at the
spawner transform (`projectile_position + spawner_exit_velocity * 0.01`), so it is
never given room to separate.

Expected: a freshly fired torpedo flies out and arms only after it has cleared the
firing ship, then detonates on proximity to the target. A point-blank shot should
still travel a little before it can blow up.

## Steps

- [ ] Add an arming gate to the torpedo: it cannot detonate until it has either lived
      for a small arming time or traveled a minimum distance from its muzzle/origin.
      Prefer a component (e.g. `TorpedoArming { min_time, min_distance }` or a simple
      timer) rather than another hardcoded constant.
- [ ] Make the arming parameters config-driven on `TorpedoSectionConfig` (fits with the
      "unhardcode blast parameters" work in task 20260706-162913).
- [ ] Verify against the near/mid/far gates in the torpedo test range (task
      20260707-100001): the near gate should no longer cause instant self-detonation,
      and mid/far shots should still detonate on arrival.
- [ ] Consider spawning slightly further ahead of the muzzle so the torpedo never
      starts inside the firing ship's own colliders.

## Notes

Source: `crates/nova_gameplay/src/sections/torpedo_section.rs`
(`shoot_spawn_projectile`, `torpedo_detonate_system`, `BLAST_RADIUS`).
