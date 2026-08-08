# Non-combatant ships hold station instead of falling into gravity wells (Ceres Queen floats)

- STATUS: CLOSED
- PRIORITY: 78
- TAGS: v0.8.0, bug, gameplay, ai, gravity

## Story

Playtest verdict (owner, 2026-07-22): the neutral Ceres Queen hauler in the
Broadside opening is meant to just float in place while the ambush plays out.
Instead it drifts and crashes into the gravity well. For that encounter a
stationary float is exactly right - it just must not fall into the well.

Root cause (from the ship-AI/gravity map): every SpaceshipRootMarker gets
`GravityAffected` auto-inserted (gravity.rs insert_gravity_affected_on_ship),
including ships spawned with `SpaceshipController::None`. A controller:None
hauler has no AI and no thrust, so the gravity well pulls it in with nothing
to resist, and it falls.

Scope of THIS task: the "float in place, do not fall" case (neutral scripted
bystanders like the Ceres Queen). The lifeline convoy, which must actively
loiter/orbit, is the sibling task 20260722-092432 and depends on this one.

## Steps

- [x] Verify-first: write a failing headless/harness check that spawns a
      controller:None ship inside a gravity well's SOI and asserts it stays
      near its spawn position after N seconds (today it drifts inward). Use
      the project's scenario/headless harness (nova_probe / a gravity test).
- [x] Decide and implement the hold rule. A ship that cannot act on gravity
      (no controller AND/OR no thrusters) should not be dragged into the well:
      either exclude it from GravityAffected at spawn, or give it a
      station-keeping hold that arrests drift. Prefer the design that keeps
      gravity visible where it makes sense but never sinks an uncontrolled
      bystander. Record the alternatives + choice in the Fix note.
- [x] Make sure this does not change combat ships or the player (they keep
      full gravity), and does not break the ORBIT/GOTO autopilots that the
      lifeline loiter task will rely on.
- [x] Regression pin: the failing check from step 1 now passes (ship floats).
- [x] Docs sweep: gravity / ship-behavior dev wiki note on how
      non-combatant / thrusterless ships behave in wells. CHANGELOG.

## Definition of Done

- A controller:None neutral ship spawned in a gravity well holds its position
  (does not crash into the well)
  (test: gravity/scenario harness assertion on position drift;
  manual: owner replays Broadside opening, Ceres Queen floats).
- Combat ships and the player still experience full gravity
  (test: existing gravity tests stay green).
- CHANGELOG entry under Fixes (cmd: `grep -ni "gravity\|float" CHANGELOG.md`).

## Notes

- Key symbols: gravity.rs GravityWell / GravityAffected /
  insert_gravity_affected_on_ship (the opt-in observer) / gravity_well_system;
  relations.rs Allegiance; scenario spaceship SpaceshipController::None.

## Fix (2026-07-22)

Verify-first turned up a partial FALSIFICATION of the literal report, which
sharpened the fix: broadside and lifeline have NO gravity wells (every
AsteroidConfig is `surface_gravity: None`); only final_tally has one, and the
ships near it are all piloted (player + AI pickets/flagship; the anchorage is
an invulnerable Asteroid, not a ship). So NO shipped scenario currently places
a `controller: None` ship inside a well's SOI - the Broadside Ceres Queen
already floats because there is nothing pulling it. The lifeline convoy's
"crash into the planetoid" the owner saw is not gravity: those haulers are at
rest with no force on them, so it is knockback drift (raider collisions /
explosion impulse) with no thrust to recover - addressed by the active-loiter
sibling task 20260722-092432.

What the fix DOES deliver: the correct, guaranteed RULE the owner asked for -
an unpiloted ship never feels a gravity well, so it can never be dragged in.

Design / alternatives:
- (A) exclude `controller: None` ships from GravityAffected - chosen.
- (B) give them station-keeping thrust to hold against the pull - rejected:
  more machinery for a ship that is meant to be inert set-dressing, and it
  would fight physics every frame.
Implementation: the gravity opt-in observer was keyed on `SpaceshipRootMarker`
(every ship root, pilot or not). nova_gameplay cannot see the nova_scenario
`SpaceshipController` (deps run scenario -> gameplay, one way), but it DOES own
the pilot markers `PlayerSpaceshipMarker` / `AISpaceshipMarker` that
nova_scenario attaches per controller (a `None` ship gets neither). So the
opt-in is now TWO observers keyed on those pilot markers: piloted ships
(player, AI) opt in; unpiloted bystanders never do. This also means a hauler
that GAINS an AI pilot (task 20260722-092432) opts back in automatically - the
coherent foundation that task builds on.

No current-content behaviour changes (verified: the asteroid_field sandbox's
one `controller: None` target sits at ~240u from its gravity rock, outside the
160u SOI, so it felt no force before OR after). This is a correctness rule +
preventive guarantee, not a visible-bug fix - stated plainly.

Coverage (gravity.rs tests, real avian physics harness):
- `an_unpiloted_ship_does_not_opt_into_gravity` and
  `an_unpiloted_ship_root_floats_in_a_well` (the behavioural pin: a bystander
  parked at r=50 inside the SOI holds its exact position, velocity stays zero).
  Both FAIL before the fix, when every ship root opted in.
- `piloted_ships_..._opt_into_gravity` and
  `a_piloted_ship_root_is_pulled_through_the_real_plugin_wiring` keep the
  player/AI-feel-gravity behaviour green.
All 18 gravity tests pass. CHANGELOG under Gameplay & Flight (a rule, not a
Fix, since no current content changes); dev wiki gravity note added.
- The auto opt-in uses try_insert already (despawn-race safe); keep that.
