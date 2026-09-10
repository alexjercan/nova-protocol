# Simulation defects: severing, lock validity, projectile ownership, flush races

- STATUS: OPEN
- PRIORITY: 81
- TAGS: v0.14.0, bug, physics, combat, review

## Goal

Fix the latent defects found by the 2026-09-09 read of `nova_ship` and
`nova_gameplay`: severing, targeting, projectile ownership, destruction
observers, degenerate authored values, and command-flush races. Line
numbers are from the read and will drift.

Every item gets a `bug_` range in `examples/systems/` (or a unit test
where no live app is needed) that fails before the fix, then the fix, one
commit. Owner (2026-09-09): "create systems examples for all edge cases
and bugs we find."

## Confirmed by reading

- [ ] `crates/nova_ship/src/sections/integrity.rs:381,482,487,512` a sever
      batch whose root died before `apply_pending_sever_motion` is pushed
      back onto `PendingSeverMotion` with no retry count or expiry, and
      `recompute_pending_sever_mass` (`:439`) then updates mass properties
      on every live body in every stuck batch every tick, for the rest of
      the process. Neither resource is reset by `teardown_scenario_entities`.
      Expire the batch, and clear both resources on teardown.
- [ ] `crates/nova_ship/src/input/targeting/contacts.rs:273,363` the
      combat-lock filter never drops a lock on a `NeutralizedMarker` hull:
      the threat arrow vanishes, the reticle stays, `WeaponsHot` stays true
      and the turrets keep working a dead hulk. The AI side already checks
      it (`ai/acquisition.rs:167`).
- [ ] `crates/nova_gameplay/src/rounds.rs:1079` and
      `projectile_hooks.rs:66-74` the owner filter compares the ROOT body,
      so a piece severed off the shooter stops being filtered and the
      ship's own in-flight rounds and torpedoes bite its detached bow.
      Track the fragment's origin, or filter on the fragment marker for a
      grace window.
- [ ] `crates/nova_ship/src/sections/torpedo_section/bay.rs:81`
      `1.0 / config.fire_rate` unguarded: an authored `0.0` fires once then
      never, a negative or NaN rate fires every tick. The turret guards the
      same line (`turret_section/setup.rs:74-77`). Reject at lint, guard at
      spawn.
- [ ] `crates/nova_gameplay/src/integrity/explode.rs:313`
      `detach_destroyed_body` takes a plain `Single<&mut WyRand>`, so an
      unmatched `Single` silently skips the only owner of a destroyed
      section's despawn and zero-health sections stay standing with live
      colliders. `fixture.rs:199` `shed_dead_fixtures` has the same shape.
      Use `Option<Single>` as `turret_section/firing.rs:88` does, and
      log once.
- [ ] `crates/nova_ship/src/sections/hull_radius.rs:71-77` a root that
      loses every live section keeps its last `HullRadius`, so the
      attitude envelope tunes a surviving controller to a hull ten times
      its size. Remove or zero it when the arms map has no entry.
- [ ] `crates/nova_gameplay/src/audio/voice.rs:275,302,499` plain
      `despawn()` under a doc comment that says the writes here must be
      the `try_` variants; the Retry teardown flush is the race the
      comment names. Panic under the default error handler.
- [ ] `crates/nova_gameplay/src/lifetime.rs:137` `on_insert_despawn_entity`
      uses `despawn()` where its sibling at `:105` uses `try_despawn()`
      with the panic named. Mod-facing only today.
- [ ] `crates/nova_ship/src/physics/attitude.rs:82` and
      `controller_section.rs:461-478` a root with no `HullRadius` yet
      (the spawn frame, a stub) gets INFINITY ceilings written onto every
      `PDController` and the load clamp is disabled for that tick. Floor
      the write or skip the tick.
- [ ] `crates/nova_gameplay/src/rounds.rs:593` `TipWalk::next` stops at
      `REJECT_BUDGET` 16 even when the tip has not advanced, so a round
      crosses a dense debris cloud unharmed for that step. Advance the
      origin past the rejected set, or budget per advance.

## Plausible, verify by running

- [ ] `crates/nova_gameplay/src/integrity/neutralize.rs:146,153` plain
      `insert` on a root that `:170` writes with `try_insert` for the
      stated reason. Align.
- [ ] `crates/nova_ship/src/sections/integrity.rs:406` plain `insert` on a
      section read in the same loop while `:168,172` use `try_insert`.
      Align.
- [ ] `crates/nova_ship/src/flight/autopilot.rs:1096` raw `.normalize()`
      on a thruster direction; a degenerate authored rotation makes
      `tail_dv` NaN and the burn never completes. Use `try_normalize`
      and reject the rotation at lint.
- [ ] `autopilot.rs:757` `error / crumb_band` with no floor; 0/0 on station
      writes a NaN `RcsIntent`. Floor the band.
- [ ] `crates/nova_ship/src/sections/railgun_section/wake.rs:460`
      `owed_particles(covered)` has no per-frame cap: a hitch during a
      slug's flight requests tens of thousands of particles in one frame.
      Cap per frame.

## Judged solid, leave alone

`flight/guidance.rs` and `flight/thrusters.rs` normalisation, the PD
controller's dead-target zeroing, torpedo target-death freeze, turret
hinge rejection, occlusion endpoints, gravity hysteresis, damage speed
floors, the ship audio `Local` resets.

## Proof

- `bug_sever_batch_expires`: sever a hull, kill the root the same frame,
  assert `PendingSeverMotion` is empty two ticks later and after Retry.
- `bug_neutralized_lock_drops`: lock a hostile, strip its last weapon,
  assert the lock clears and `WeaponsHot` falls.
- `bug_own_fragment_not_shot`: fire a burst, sever the bow into the line
  of fire, assert the fragment takes no damage from its former hull.
- A lint test per degenerate authored value (fire rate, section rotation).
- The `Option<Single>` and `try_` items are unit-tested by the shape the
  sibling files already use.
