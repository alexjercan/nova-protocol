# Non-combatant ships hold station instead of falling into gravity wells (Ceres Queen floats)

- STATUS: OPEN
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

- [ ] Verify-first: write a failing headless/harness check that spawns a
      controller:None ship inside a gravity well's SOI and asserts it stays
      near its spawn position after N seconds (today it drifts inward). Use
      the project's scenario/headless harness (nova_probe / a gravity test).
- [ ] Decide and implement the hold rule. A ship that cannot act on gravity
      (no controller AND/OR no thrusters) should not be dragged into the well:
      either exclude it from GravityAffected at spawn, or give it a
      station-keeping hold that arrests drift. Prefer the design that keeps
      gravity visible where it makes sense but never sinks an uncontrolled
      bystander. Record the alternatives + choice in the Fix note.
- [ ] Make sure this does not change combat ships or the player (they keep
      full gravity), and does not break the ORBIT/GOTO autopilots that the
      lifeline loiter task will rely on.
- [ ] Regression pin: the failing check from step 1 now passes (ship floats).
- [ ] Docs sweep: gravity / ship-behavior dev wiki note on how
      non-combatant / thrusterless ships behave in wells. CHANGELOG Fixes.

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
- The auto opt-in uses try_insert already (despawn-race safe); keep that.
