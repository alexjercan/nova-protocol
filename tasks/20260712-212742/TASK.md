# Combat target set includes committed torpedoes (sticky + CTRL+scroll point defense)

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.5.0, targeting, spike

## Goal

Playtest bug after the sticky-ship-lock change (task 20260712-203353): with a
sticky ship lock you can no longer engage a torpedo - aiming is blocked (the
picker stands down while a ship is held) and CTRL+scroll only cycles hostile
SHIPS, so an incoming torpedo is unreachable and the auto-turrets (which fire at
the lock) cannot down it.

User's model (2026-07-12): the game maintains the list of available COMBAT
targets and the current one; with no target it auto-picks the best; with a
target it does NOT auto-switch (sticky); CTRL+scroll steps to the next best.
This already exists for ships - just extend the combat target set to include
committed hostile torpedoes so CTRL+scroll reaches them and a selected torpedo
stays locked while the PDC downs it. Nav designation (asteroids/beacons for
GOTO) stays aim-driven and separate (do NOT make it sticky - review R1.1 of
20260712-203353).

## Steps

- [ ] In `update_spaceship_target_input` candidate collection
      (input/targeting.rs), add a `is_combat_target` flag to the collected
      tuple = `is_ship || (committed hostile torpedo:
      TorpedoProjectileMarker + TorpedoTargetChosen)`. Keep the local `is_ship`
      for the range-gate branch; replace the tuple's 4th field (currently
      `is_ship`, only consumed by the ranked-candidate filter) with
      `is_combat_target`.
- [ ] Rank filter (`rank_ship_candidates` caller): `is_hostile &&
      is_combat_target` so the candidate `entries` (the CTRL+scroll cycle set,
      the candidate HUD, edge indicators) include committed hostile torpedoes.
      Rename `rank_ship_candidates` -> `rank_combat_targets` (+ doc) for clarity.
- [ ] Sticky `held`: gate on `is_combat_target` (not just `is_ship`), so a
      cycled/acquired committed torpedo is also sticky (holds while you shoot
      it) and reverts to the aim pick when it dies / leaves range. Nav bodies
      stay non-combat -> non-sticky.
- [ ] Tests (delivery-guarded): CTRL+scroll from a ship lock reaches a
      committed hostile torpedo; a committed-torpedo lock is sticky (a closer
      body does not steal it); a passing torpedo still does NOT auto-steal a
      held ship lock; a non-combat nav lock (asteroid/beacon) stays aim-driven
      (re-designates). Reuse the `multi_target_world` rig style.
- [ ] Verify: `cargo test -p nova_gameplay targeting`; 12_hud_range +
      10_gameplay autopilots green.

## Notes

- Spike: docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md
  (this completes the sticky-lock direction per the user's clarified model).
- Auto-turrets fire at `SpaceshipPlayerTargetLock` (input/player.rs turret aim
  feed); there is no independent point defense, so locking the torpedo IS how
  you shoot it down.
- The candidate collection already tracks `is_torpedo` (TorpedoProjectileMarker)
  and `torpedo_committed` (TorpedoTargetChosen) and `is_hostile` - the flag is a
  local combine, no new query.
- Depends on: 20260712-203353 (CLOSED).
