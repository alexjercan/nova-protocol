# Notes

## Plan

Order of work; one commit per line unless noted.

1. Task + notes.
2. Engine: `MeshFragmentMarker` debris gets `TempEntity` lifetime.
3. Engine: asteroid silhouette seed (`AsteroidConfig.seed`) + scatter
   derives per-rock seeds from a side RNG (position stream unchanged).
4. Engine: passive-flight obstacle avoidance in `input/ai/passive.rs`.
5. Content: heavy torpedo prototype (+ bay plumbing if projectile
   durability is not authorable).
6. Content: three backdrop scenarios + catalog + bundle + gen.
7. Menu: `NOVA_MENU_BACKDROP` override for deterministic capture.
8. Docs + CHANGELOG.
9. Verify: lint, check, targeted lib tests, run under Xvfb, screenshots.

## Design decisions

### Relations drive everything

- Only Player<->Enemy is hostile. Backdrop duels need one AI ship with
  `allegiance: Some(Player)`.
- Neutral ships never acquire and are never point-defensed. The scene-3
  battery idles as Neutral and flips to Enemy via `SetAllegiance` when
  the duel resolves.

### Scene 1: torpedo gauntlet

- Racer with player-grade turrets (PDC display class), Player
  allegiance, `orbit: menu_planetoid`, small leash so it holds the ring
  while the battery sits beyond gun reach.
- Battery: thrusterless Spaceship, torpedo bays, no controller motion;
  authored rotation keeps the orbit cone inside the launch alignment.
- Battery distance ~600: inside AI engage range (800) so it launches,
  outside turret reach (450) so the duel never resolves.
- Respawn loop for robustness: `OnDefeated(ship)` -> despawn + timer ->
  single `OnTimerEnd` spawn site. `OnStart` only starts the timer, so
  each id has exactly one spawning handler (lint clean).

### Scene 2: asteroid weave

- Requires avoidance: autopilot GOTO has no obstacle awareness; the AI
  passive layer now detours around any body with `BodyRadius` that
  blocks the leg to the current waypoint.
- Deterministic rock silhouettes make the field stable across runs.

### Scene 3: duel cycle

- OnDefeated (destroyed OR neutralized) is the resolve signal: AI stops
  shooting at neutralized targets, so full destruction is not
  guaranteed; script despawns wrecks before respawn.
- Finisher: heavy torpedo with blast sized to destroy every section of
  the winner outright, high speed + armored projectile so PDC cannot
  reliably stop it. Honest sim: the battery really launches it.
- Timer keys re-arm themselves legally (expired keys removed before
  dispatch), so the cycle is a self-restarting timer chain.

### Debris lifetime

- Mesh fragments previously lived until scenario teardown; long-lived
  menu scenes accumulate dynamic convex hulls forever. Fragments now
  carry a fixed lifetime.
