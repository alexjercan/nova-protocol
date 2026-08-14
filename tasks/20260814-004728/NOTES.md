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

## Findings from live verification (Xvfb + NOVA_MENU_BACKDROP)

Everything below was discovered by RUNNING the scenes, not by review.

- The app's debug log filter does not include nova_ship; run with
  `RUST_LOG=info,nova_ship=debug` when diagnosing AI/weapons.
- Passive re-engage: a passive AI only leaves its routine for hostiles
  inside 800 u, and the batteries idle whenever their target respawns.
  This produced the `engage_range` authoring knob: the battery watches
  from 1600 while the racer's default 800 never pulls it back.
- Avoidance v1 crashed the weave runner after 66 s: the held corner's
  own leg was never re-validated (a neighbor rock on the way to the
  corner was flown into blind), and waypoint-hugging rocks were skipped
  as blockers. v2 adds corner-leg hop + goal-outside-clearance; a 3 min
  live run has ZERO ship impacts in the dense band.
- Engage-state flight has NO avoidance (by design). Duel v1 staged the
  fight on the y=0 plane: the head-on chase line crosses the planetoid,
  both ships pin against it, LOS holds both triggers - permanent
  stalemate. Fix: the whole duel is staged at y=110 (above the ~91 u
  worst-case geometric radius).
- Gravity: the arena sits 110-180 u out; at the default mass 45 000 the
  424 u SOI drags the dogfight onto the rock. The duel's planetoid is
  authored at mass 6 000 (SOI ~155) - per-scene mass is exactly the
  knob the waystation already used.
- Menu framing: the panel owns the frame's right half; both batteries
  park on -X so torpedo runs and PD intercepts cross the OPEN left half.
- Verified cycles (logs in this folder's shots/ + scratchpad): gauntlet
  18 launches / 12 shot down / racer alive over ~3 min; weave 0 impacts
  over ~3 min; duel 4 full cycles in ~5.5 min, one siege torpedo per
  cycle, PD never stops it (0 shootdowns vs 5000 hp ordnance).

## Retention

- `shots/gauntlet-pd-intercept.png` - tracer stream vs inbound torpedo.
- `shots/weave-threading.png` - the runner inside the rock band.
- `shots/duel-dogfight.png`, `shots/duel-finisher-window.png`.
