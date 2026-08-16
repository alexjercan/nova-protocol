# Notes

## The mechanism

Not entity-id reuse. The blast ENTITY itself outlives the scenario.

1. `torpedo_detonate_system` (`crates/nova_ship/src/sections/torpedo_section/projectile.rs`)
   fuzes the torpedo and spawns a free-standing blast: `nova_blast(...)` (a
   Static sensor sphere with `CollisionEventsEnabled`), a `Transform` at the
   detonation point, and `TempEntity(0.1)`. It is parented to nothing and
   carried no scenario scope.
2. The blast kills the player. `CurrentOutcome` becomes `Defeat`, and
   `nova_menu`'s `sync_outcome_pause` -> `pause_clocks` pauses `Time<Virtual>`
   and `Time<Physics>`. `update_temp_entities` reads `Res<Time>`, so the blast's
   0.1 s countdown STOPS. The volume is now immortal for as long as the overlay
   is up.
3. Retry triggers `LoadScenario`. `teardown_scenario_entities` despawns every
   `ScenarioScopedMarker` entity. The blast has no marker, so it stays.
4. The reloaded scenario spawns its asteroid inside the surviving sensor. On
   unpause avian finds the new pair and raises `CollisionStart`;
   `on_nova_blast_collision` applies the full undiminished falloff and the rock
   dies.

The blast never names a stale entity. It damages whatever physically overlaps
it, which is why the fix is OWNERSHIP, not timing: a volume with no owner
cannot be made safe by shortening its fuse.

## Ruled out

- Entity-id reuse. `on_nova_blast_collision` damages `collision.collider2` from
  a LIVE avian event; it never resolves a stored id.
- The cross-frame spawn drain from `b5523a23`. `NovaEventWorld::clear()` -
  called by `teardown_scenario_entities` - discards every undrained queued
  command (and logs the count), so the settling queue cannot straddle an unload.
  The report also predates that commit.
- A missed despawn. The fuze's `try_despawn` of the torpedo lands normally; it
  is the blast it spawns, not the torpedo, that survives.

## Why the scope was missing

Scenario scoping was an allow-list of three marker types
(`MeshFragmentMarker`, `TurretBulletProjectileMarker`,
`TorpedoProjectileMarker`). Every runtime transient had to be remembered and
added by hand. The blast was not.

## The fix

Scope on the LIFETIME, not on a list of markers:

    app.add_observer(on_add_entity_with::<TempEntity>);

`TempEntity` is what every transient the game spawns already rides, and it is
exactly the set with no other owner - a projectile, a blast or a debris chunk is
parented to nothing, so only scenario scoping can delete it. The one observer
subsumes all three markers (torpedoes, turret rounds and fragments all carry
`TempEntity`) and closes the class: a future transient is scoped the moment it
declares a lifetime.

The blast radius visual (`torpedo_section/render.rs`) was the one transient that
timed itself out privately, with its own `duration` field and despawn branch. It
now rides `TempEntity(BLAST_VISUAL_DURATION)` like everything else, and its
one-off material is freed by an `On<Remove, BlastRadiusVisual>` observer, so
there is exactly one despawn owner whether the timer or the teardown wins.

## What else had the same shape

Closed by the same rule:

- the torpedo detonation blast (the bug)
- the hanabi "Blast Effect" burst, `TempEntity(2.0)` - a stale explosion drawn
  over the next scene
- the "Blast Radius Visual" expanding shell - same, and it did not even have a
  lifetime component until this change

Checked and NOT affected:

- muzzle flashes, launch bursts and skin surfaces are `ChildOf` a section, so
  they die with the ship
- `ScenarioAreaConfig` already spawns with `ScenarioScopedMarker`
- the render-scale upscale camera/sprite and the thruster/RCS audio loops are
  app-level singletons, deliberately outliving a scenario

Still open, reported not fixed:

- The scoping insert is deferred, so a transient spawned in the SAME command
  flush as the teardown is not yet in the world when the sweep queries, and gets
  scoped to the incoming scenario instead. Pre-existing and unchanged by this
  work (it is a property of `on_add_entity_with`, which the three markers used
  too), and unreachable on the reported path - a Retry is many frames after the
  detonation that killed you.
- `nova_gameplay/src/audio/sfx.rs` one-shots (`AudioPlayer` +
  `PlaybackSettings::DESPAWN`) carry no lifetime and no scope, so a detonation
  heard the frame before a switch keeps playing over the new scene's opening.
  Audible only - no damage, no physics, and the engine retires the entity when
  the clip ends.

## Verification

`crates/nova_scenario/src/loader/lifecycle.rs`:
`a_detonation_blast_cannot_damage_the_next_scenario` walks the whole chain on
production pieces - the real `TorpedoSectionPlugin` fuze, the real clock freeze,
the real `LoadScenario` teardown, a real avian collision pipeline.

Before the fix it fails on the reported symptom:

    assertion `left == right` failed: the previous scenario's blast damaged the
    reloaded scenario's asteroid (health left: Some(0.0))
      left: Some(0.0)
     right: Some(100.0)

After the fix it passes. `register_scenario_scoping` was factored out of
`ScenarioLoaderPlugin` so the test drives the PRODUCTION wiring rather than a
hand-registered copy of it - registering the observer in the test rig would have
made the test pass against the broken build.

Live: `examples/sections/torpedo_section.rs` grew invariant 7, "assert the
switch took the ordnance". The step snapshots every transient alive when the
range switch is ordered and asserts each one is gone once the crossing range is
up. Ran under Xvfb on both builds:

    fixed:   range: the switch took all 4 transient(s) ... / cycle complete, no panic
    pre-fix: range: the switch took all 20 transient(s) ... / cycle complete, no panic

BE CLEAR: that live check does NOT discriminate this bug, and it is not claimed
as before/after proof. The gate round's last detonation is ~9 s before the
switch (the beat holds `LAUNCH_SETTLE_SECS` = 10 s after a torpedo arms, and the
auto-targeted gate is dead by then, so nothing else fuzes), so no blast or blast
cosmetic is airborne at the teardown on that timeline - the snapshot is all
torpedoes and debris, which the old markers already scoped. What the live run
DOES prove is that scoping every transient does not break the real scene switch,
and the invariant catches the class from now on whenever ordnance is in the air.
The discriminating evidence is the headless test above, which is a full physics
repro of the reported symptom.

`examples/systems/outcomes.rs` cannot carry the repro either: it kills the
player with overkill on the ship root, so no blast is ever spawned.

## Docs

`web/src/wiki/dev/scenario-system.md` listed "be a self-expiring `TempEntity`"
as an ACCEPTABLE cleanup route - the exact loophole the blast slipped through.
The cleanup contract now says the opposite, and says why: a lifetime that runs
on `Time<Virtual>` stops when the overlay stops the clock, so the lifetime is
what SCOPES a transient, never what retires it.

`examples/stress/many_projectiles.rs` claimed rounds are not scenario-scoped.
They were (via `TurretBulletProjectileMarker`) and still are; the doc was already
wrong and now contradicts the rule outright, so it is corrected.
