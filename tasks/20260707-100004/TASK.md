# Torpedo despawns silently when its target dies mid-flight

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.4.0,bug,torpedo

`update_target_position` looks up the torpedo's target entity every frame and, if it
is gone (target destroyed, scenario churn), immediately despawns the torpedo:

```
let Ok(target_transform) = q_target.get(**target_entity) else {
    warn!(...);
    commands.entity(torpedo).despawn();
    return;
};
```

So an in-flight torpedo vanishes the instant its target dies, which reads as a bug
(the torpedo should not just blink out) and contributes to the "torpedoes feel
flaky" impression. It also logs a warning every such frame.

Expected: losing the target mid-flight should degrade gracefully, not delete the
torpedo mid-air.

## Steps

- [x] Decide the intended behavior and implement it: **freeze-and-continue**. When the
      target entity is gone, `update_target_position` now drops the dead
      `TorpedoTargetEntity` link (instead of despawning the torpedo) and leaves
      `TorpedoTargetPosition` frozen at its last value, so the torpedo keeps flying
      toward the last known position and detonates on arrival (or expires via its
      `TempEntity` lifetime). If the ship has a live target selected, the torpedo also
      becomes eligible to re-acquire it (a bonus, not a regression).
- [x] Stop the per-frame warning spam: removing the link means the torpedo no longer
      matches the `With<TorpedoTargetEntity>` query, so the failing lookup (and its log)
      does not repeat. The remaining message is a single `debug!` on the loss frame.
- [x] Coverage: deterministic unit/integration test
      `torpedo_survives_target_loss_and_freezes_position` (target alive -> tracks; target
      despawned -> torpedo survives, position frozen, dead link removed). The range
      (`06_torpedo_range`) autopilot smoke run still fires/arms/detonates with no vanish,
      no panic, and no despawn-spam; the range is available for manual freeze checks.

## Resolution

Root cause: `update_target_position` called `commands.entity(torpedo).despawn()` when the
target lookup failed, so a torpedo blinked out the instant its target died. Changed it to
`remove::<TorpedoTargetEntity>()` and keep `TorpedoTargetPosition` frozen - freeze-and-
continue. Covered by a new test (7 torpedo tests pass); `cargo clippy` clean; range smoke
run green (3 fired/armed/detonated, no panic, old `not found in q_target` spam gone).

## Notes

Source: `crates/nova_gameplay/src/sections/torpedo_section.rs`
(`update_target_position`). Pairs with the arming fix (task 20260707-100003) and the
guidance work (task 20260525-133021).
