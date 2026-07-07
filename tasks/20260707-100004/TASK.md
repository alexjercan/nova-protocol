# Torpedo despawns silently when its target dies mid-flight

- STATUS: OPEN
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

- [ ] Decide the intended behavior and implement it: freeze on the last known target
      position and keep flying (detonate on arrival), fly straight until the
      `TempEntity` lifetime expires, or detonate in place. Freeze-and-continue is the
      most intuitive; confirm against the torpedo test range.
- [ ] Stop the per-frame warning spam for a legitimately-dead target.
- [ ] Add coverage in the torpedo test range (task 20260707-100001): kill a moving
      target after firing and confirm the torpedo behaves as designed rather than
      disappearing.

## Notes

Source: `crates/nova_gameplay/src/sections/torpedo_section.rs`
(`update_target_position`). Pairs with the arming fix (task 20260707-100003) and the
guidance work (task 20260525-133021).
