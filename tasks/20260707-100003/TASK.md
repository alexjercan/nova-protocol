# Torpedo self-detonates on spawn when target is near; add arming delay

- STATUS: CLOSED
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

- [x] Add an arming gate to the torpedo: it cannot detonate until it has either lived
      for a small arming time or traveled a minimum distance from its muzzle/origin.
      Done via a `TorpedoArming { min_time, min_distance, origin, elapsed, armed }`
      component (latched once armed). `update_torpedo_arming` ticks it before
      `torpedo_detonate_system`, which now skips any un-armed torpedo.
- [x] Make the arming parameters config-driven on `TorpedoSectionConfig`. Added
      `arm_time` (0.5s) and `arm_distance` (5.0) fields with defaults; the in-game
      config in `nova_assets/src/sections.rs` sets them too.
- [x] Verify. The torpedo test range (task 20260707-100001) does not exist yet, so
      verified instead with unit + integration tests in `torpedo_section.rs`: unarmed
      on spawn; arms via time even without moving (point-blank); arms via distance
      before min_time (fast shot); stays armed once armed; `torpedo_detonate_system`
      leaves an un-armed on-target torpedo alive but detonates an armed one. When the
      range lands it should re-confirm near/mid/far behaviour interactively.
- [x] Consider spawning slightly further ahead of the muzzle. Left as-is: the spawner
      is already offset from the ship (`spawn_offset`, e.g. `Vec3::NEG_Z * 2.0`), and
      the arming gate is what actually fixes the self-detonation, so no extra forward
      nudge was needed. Revisit only if torpedoes are seen clipping the hull.

## Resolution

Root cause was that `torpedo_detonate_system` fired from frame one whenever the
torpedo was within `BLAST_RADIUS * 0.5` of the target, with no arming gate. Added a
latched `TorpedoArming` component (time OR distance from the muzzle), ticked each
frame before detonation; detonation is skipped until armed. Params are config-driven
(`arm_time` / `arm_distance`). Covered by 6 tests. `cargo clippy` and full build green.

## Notes

Source: `crates/nova_gameplay/src/sections/torpedo_section.rs`
(`shoot_spawn_projectile`, `update_torpedo_arming`, `torpedo_detonate_system`,
`TorpedoArming`, `BLAST_RADIUS`).
