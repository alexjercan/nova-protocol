# Torpedo still vanishes on target loss: second despawn in player targeting

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.4.0,bug,torpedo

Follow-up to task 20260707-100004. That fix stopped `update_target_position` from
despawning a torpedo when its target died, but the torpedo still vanishes for any
target that *fully* despawns - a bullet expiring on its lifetime, another torpedo,
or a spaceship. It only appears fixed for asteroids because an asteroid's targeted
entity is the `RigidBody` parent husk, which lingers after the collider child is
destroyed (task 20260706-212910), so the target link is never actually dropped.

Root cause: a second despawn path in `update_torpedo_target_input`
(`crates/nova_gameplay/src/input/player.rs`):

```
let Some(target_entity) = target_entity else {
    // TODO(20260706-162913): Maybe think of something better then just despawning the torpedo?
    for (torpedo, _) in &q_torpedo {
        commands.entity(torpedo).despawn();
    }
    return;
};
```

When the aim resource `SpaceshipPlayerTorpedoTargetEntity` is `None`, this despawns
**every** un-targeted player torpedo. The sequence for a despawning target:

1. Target dies -> `update_target_position` drops the `TorpedoTargetEntity` link
   (the 100004 freeze fix), so the torpedo is now `Without<TorpedoTargetEntity>`.
2. The aim cast (`update_spaceship_target_input`) no longer hits the dead target,
   so `res_target` becomes `None`.
3. `update_torpedo_target_input` hits the `None` branch and despawns all
   un-targeted torpedoes -> the torpedo blinks out.

Asteroids skip this because the husk keeps the link alive (step 1 never happens).

Expected: a torpedo with no current lock should keep flying (freeze-and-continue,
consistent with 100004), not be despawned.

## Steps

- [x] In `update_torpedo_target_input`, stop despawning un-targeted torpedoes when
      `res_target` is `None`: the no-lock branch now just returns, leaving them flying
      toward their frozen `TorpedoTargetPosition`. Targets are assigned only when one
      is locked. Removed the stale TODO and despawn loop.
- [x] Added tests in `player.rs`: `no_lock_does_not_despawn_untargeted_torpedo` (None
      lock -> torpedo survives, no target assigned) and `lock_assigns_target_to_owned_torpedo`
      (Some lock -> owned torpedo gets `TorpedoTargetEntity`).
- [x] Verified the remaining despawn paths are only the intended ones: torpedo
      detonation (`torpedo_detonate_system`) and `TempEntity` lifetime. No other
      despawn-on-target-loss remains.
- [x] Range (`06_torpedo_range`) autopilot smoke still green (3 fired/armed/detonated,
      no panic); the range doesn't reproduce the bug on its own because its
      `range_autotarget` keeps torpedoes locked, so the deterministic unit tests are the
      authoritative coverage.

## Resolution

The 100004 freeze fix removed the despawn in `update_target_position`, but a second
despawn in `update_torpedo_target_input` (player.rs) deleted every un-targeted player
torpedo whenever the aim lock (`SpaceshipPlayerTorpedoTargetEntity`) was `None`. After a
target fully despawns, the freeze fix drops the link, the aim cast misses, the lock goes
`None`, and this branch blinked the torpedo out. Asteroids were masked by the lingering
husk (212910). Fixed by making the no-lock branch a no-op return. 15 nova_gameplay tests
pass; clippy clean; range smoke green.

## Notes

Source: `crates/nova_gameplay/src/input/player.rs` (`update_torpedo_target_input`).
Pairs with 20260707-100004 (the first half of this fix) and 20260706-212910 (the
asteroid husk, which masks the bug for asteroids).
