# OnDestroyed event fires inconsistently

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, bug

From the TODO sweep (task 20260525-132954). A FIXME notes that an event (in the
integrity/destruction path) is not fired consistently. Investigate why and make it
reliable.

Source: crates/nova_gameplay/src/integrity/plugin.rs (FIXME near the destruction event).

## Root cause

The FIXME was on `on_blast_collision_deal_damage`: blast damage "fires only for (object,
blast) but not (blast, object)". avian raises `CollisionStart` once per collider in the pair
that carries `CollisionEventsEnabled`, with that collider as `body1` ("self"). The blast sensor
(`blast_damage`) never enabled events - it has no `Health`, so the
`on_collider_of_spawn_insert_collision_events` observer skipped it - so the blast relied
entirely on the *target* having events enabled. That is the inconsistency: a body only takes
blast damage if it independently opted into collision events. `area.rs` does not have this
problem because the area itself carries `CollisionEventsEnabled` (via `ScenarioAreaMarker`), so
it raises the event against everything it overlaps.

## Fix

Mirror the `area.rs` pattern: make the blast own its events.

- `blast_damage` (blast.rs) now includes `CollisionEventsEnabled`, so avian raises
  `CollisionStart` with the blast as `body1` against every collider it overlaps.
- `on_blast_collision_deal_damage` (plugin.rs) is rewritten so the blast is the "self" side
  (`body1`/`collider1`) and the target is `body2`/`collider2`; damage is applied to the
  target's collider. The swapped ordering (raised when the target also has events, e.g. a ship
  section) is ignored - `q_blast.get(body1)` fails when `body1` is the target - so each overlap
  deals damage exactly once and never double-dips.

## Steps

- [x] Diagnose why the blast collision event was one-directional (avian's per-`CollisionEventsEnabled`
      dispatch; blast never enabled events).
- [x] Make the blast own `CollisionEventsEnabled` and flip the observer to treat the blast as self.
- [x] Regression tests (plugin.rs physics_tests): the blast reaches a target with no events of
      its own; it deals damage exactly once when the target *does* have events; falloff and
      out-of-range behaviour preserved.
- [x] Full check suite green: `cargo test --workspace` (50 nova_gameplay, incl. 2 new;
      examples_smoke under Xvfb), `cargo clippy --workspace --all-targets`.

## Notes

Built directly on the physics-level integrity test harness from task 20260707-170001. The
impact observer is unaffected: for a (blast, object) event it early-returns because the static
blast has no `LinearVelocity`/`ComputedMass`, and for an (object, blast) event it still excludes
`BlastDamageMarker` - the tests assert exactly the falloff damage, confirming no spurious impact
contribution.
