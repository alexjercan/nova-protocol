# Combat target set includes committed torpedoes (sticky + CTRL+scroll point defense)

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.5.0, targeting, spike

## Outcome (CLOSED 2026-07-12)

Extended the combat target set to committed torpedoes in
`update_spaceship_target_input` (input/targeting.rs): the candidate tuple's 4th
field is now `is_combat_target = is_ship || is_torpedo.is_some()` (uncommitted
torpedoes are already filtered out before this point, so any torpedo here is
committed). The ranked candidate set (CTRL+scroll cycle, candidate HUD, edge
indicators) filters `is_hostile && is_combat_target` (renamed
`rank_ship_candidates` -> `rank_combat_targets`), and the sticky `held` gate now
uses `is_combat_target` too. So a committed hostile torpedo is a cyclable,
sticky combat target: CTRL+scroll reaches it, it holds while the auto-turrets
(which fire at the lock) down it, then reverts to the aim pick. Nav bodies
(asteroids/beacons/wells) are not combat targets, so they stay aim-driven and
GOTO re-designation is unchanged (review R1.1 of 20260712-203353 preserved).

Updated `candidates_track_hostile_ships_only` (asserted torpedoes stay OUT - the
old behaviour) to `candidates_track_hostile_combat_targets_including_torpedoes`,
and added `a_committed_torpedo_lock_is_sticky` (a closer ship does not steal a
held torpedo lock; delivery guard re-acquires the ship once the torpedo is
gone).

Verified: `cargo test -p nova_gameplay targeting` 45 pass; `12_hud_range` +
`10_gameplay` autopilots green; `fmt --check` clean.

## Steps

- [x] Add `is_combat_target` (ship or committed torpedo) to the candidate tuple.
- [x] Rank filter uses `is_hostile && is_combat_target`; renamed
      `rank_combat_targets`.
- [x] Sticky `held` gates on `is_combat_target`.
- [x] Tests: candidates include a hostile committed torpedo; a torpedo lock is
      sticky (delivery-guarded); ship-sticky + nav-aim-driven still hold.
- [x] Verify: targeting tests + autopilots.

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

- [x] In `update_spaceship_target_input` candidate collection
      (input/targeting.rs), add a `is_combat_target` flag to the collected
      tuple = `is_ship || (committed hostile torpedo:
      TorpedoProjectileMarker + TorpedoTargetChosen)`. Keep the local `is_ship`
      for the range-gate branch; replace the tuple's 4th field (currently
      `is_ship`, only consumed by the ranked-candidate filter) with
      `is_combat_target`.
- [x] Rank filter (`rank_ship_candidates` caller): `is_hostile &&
      is_combat_target` so the candidate `entries` (the CTRL+scroll cycle set,
      the candidate HUD, edge indicators) include committed hostile torpedoes.
      Rename `rank_ship_candidates` -> `rank_combat_targets` (+ doc) for clarity.
- [x] Sticky `held`: gate on `is_combat_target` (not just `is_ship`), so a
      cycled/acquired committed torpedo is also sticky (holds while you shoot
      it) and reverts to the aim pick when it dies / leaves range. Nav bodies
      stay non-combat -> non-sticky.
- [x] Tests (delivery-guarded): CTRL+scroll from a ship lock reaches a
      committed hostile torpedo; a committed-torpedo lock is sticky (a closer
      body does not steal it); a passing torpedo still does NOT auto-steal a
      held ship lock; a non-combat nav lock (asteroid/beacon) stays aim-driven
      (re-designates). Reuse the `multi_target_world` rig style.
- [x] Verify: `cargo test -p nova_gameplay targeting`; 12_hud_range +
      10_gameplay autopilots green.

## Notes

- Spike: tasks/20260712-203235/SPIKE.md
  (this completes the sticky-lock direction per the user's clarified model).
- Auto-turrets fire at `SpaceshipPlayerTargetLock` (input/player.rs turret aim
  feed); there is no independent point defense, so locking the torpedo IS how
  you shoot it down.
- The candidate collection already tracks `is_torpedo` (TorpedoProjectileMarker)
  and `torpedo_committed` (TorpedoTargetChosen) and `is_hostile` - the flag is a
  local combine, no new query.
- Depends on: 20260712-203353 (CLOSED).
