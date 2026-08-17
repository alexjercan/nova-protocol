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

## Round 2 (owner feedback, 2026-08-14)

Feedback: (a) remove the big central rock from all three scenes but keep
the camera pose; (b) weave drifts offscreen - tighter loop, more
waypoints, more rocks; (c) gauntlet racer should hold near-station while
multiple dumb "torpedo-bay ships" on BOTH flanks fire scripted shots;
(d) the duel read as lame because the planetoid owned the center.

Design:

- New `Anchor` object kind: invisible `GravityWell` with an AUTHORED
  `body_radius` (deterministic camera pose - a rock's radius comes off
  the noise mesh and jitters per load), optional mass, no collider, no
  `BodyRadius` (avoidance must fly through it). All three scenes anchor
  `menu_planetoid` at radius 80 -> camera (0, 90, 300) every load; the
  visible half-frame at origin depth is ~230 u (tighter than the old
  rock average - scene geometry shrank to fit).
- New `ForceTorpedoLaunch` action + `ScriptedTorpedoOrder` in nova_ship:
  holds the bay trigger until the bay's own cooldown/ammo fire it, then
  commits the projectile (the commit is what PD acquisition requires).
  Missing target skips the launch (no duds during respawn gaps).
  Batteries become `SpaceshipController::None` + authored Enemy.
- Gauntlet v2: racer holds a ~60 u-leg diamond left of center
  (`engage_range: 300` so the 700+ u batteries never pull it; parks also
  beyond leash 250 + turret 450 so a damage-memory lunge is safe), four
  single-bay batteries on both flanks fire on staggered self-restarting
  timers (15/18/21/24 s), torpedoes cross both frame edges into the PD.
- Weave v2: ten waypoints on a 140 u ring (legs ~87 u - the autopilot
  decelerates/turns instead of cruising), loop center nudged to -40 x,
  40 rocks in a tighter shell (ring 100-190, y -60..-28) - ~1.5x the
  old volume density; whole loop + worst-case detours < the half-frame.
- Duel v2: empty center (no rock, no gravity) - chase lines through the
  middle are clear and the fight owns the frame; entrances/patrols carry
  a small y-split for depth. Finisher is scripted: rival's defeat arms a
  4 s beat, the beat launches at the victor and re-arms every 12 s (a
  miss retries), the victor's defeat cancels the clock. No allegiance
  flip anymore - the battery sits Enemy and simply never fires unless
  told to.

## Round 3 (owner feedback, 2026-08-14)

Feedback: (1) weave camera further back - ship escapes the frame; (2)
duel: winner should hold the middle of the frame, exactly one finisher
torpedo at a time, stray torpedoes must not survive into the next wave,
clear everything on reset, bring destroyed rocks back - "would full
reset make more sense?"; (3) gauntlet: intercepts should happen ON
screen, racer should move a bit more, more scene dressing.

Design:

- `backdrop_anchor(body_radius)` is now the per-scene framing knob (the
  camera math makes radius the zoom): weave authors 115 -> camera
  (0, 116, 388); gauntlet/duel stay at the reference 80.
- New engine knob `pd_range` (`AIPointDefenseRange`), the PD sibling of
  `engage_range`: gauntlet racer authors 160 so the tracer stream opens
  up mid-frame instead of at the 400 u default (frame edge).
- Gauntlet: six-point hold circuit (~75 u legs, slight y wander), 26
  rocks in a wider band, two amber nav beacons and a Neutral drifting
  cargoa wreck (all off every torpedo lane - dressing must not eat
  ordnance).
- Duel: victor's patrol is a tight ring on the frame center (fight's
  leash anchor AND the victory-lap park where the torpedo lands);
  finisher re-arm 12 s -> 20 s (> the ~16 s flight, so exactly one
  torpedo is ever in the air - 12 s doubled up, the reported bug).
- Duel full reset: research confirmed runtime projectiles/bullets/debris
  ARE scenario-scoped (loader observers), teardown clears the event
  world, and seeded scatter rebuilds the identical field - so the wave
  reset is `NextScenario` to the scenario's OWN id (precedent:
  asteroid_field's lingering retry). Safe shape: dedicated handler,
  `linger: false` (linger would make Enter in the menu trigger it),
  `delay: Some(1.0)` (an instant switch consumed in the same flush
  discards other handlers' queued commands). Cost: a ~2-frame camera
  blink per reset. The menu ambience holds no scenario handle, so the
  pinned backdrop survives the swap.

## Round 4 (owner feedback, 2026-08-14)

Feedback: (1) gauntlet racer should RUN OUT of bullets, get overrun and
blown up - and the second ship (the dressing wreck) is useless; (2) make
AI_WAYPOINT_SLACK authorable so the weave runner presses closer to its
waypoints; (3) duel wants more determinism and a more centered fight;
(+) failsafe: any scene whose main ship is blown up lingers 5-10 s and
restarts.

Design:

- New `SetAmmo(u32)` section modification: a HARD magazine - rounds
  overridden AND auto-reload stripped (racer PDCs shipped with a
  self-refilling 500-round magazine, so they could never be permanently
  overrun). Twin apply-on-add observers cover both build orders (weapon
  components land via deferred setup observers).
- Gauntlet arc: 1800 rounds per turret, no reload -> the stand falls by
  ammo clock; wreck dressing removed; the in-place respawn loop replaced
  by the linger-then-full-reset idiom.
- New `waypoint_slack` knob (`AIWaypointSlack`): the patrol advance gate
  is `arrival_standoff (50) + slack (default 25)`; the weave runner
  authors 5 and turns ~55 u out instead of 75. NOTE the knob's floor:
  the autopilot still brakes toward rest at 50 u from the mark, so slack
  below ~2 risks asymptoting outside the gate - the wiki row documents
  "author small, not zero".
- Failsafe restarts everywhere: gauntlet 8 s linger, weave 6 s (a rammed
  runner previously left a pilotless band forever), duel already had it.
  All three use NextScenario-to-self in a dedicated handler with
  delay 1.0.
- Duel centering: both duelists tether leash 200 to center-hugging
  patrol rings (rival mirrors the victor's tight ring); each wave now
  fights over the same ground mid-frame.

## Round 5 (owner feedback, 2026-08-14)

Feedback: the backdrop self-reset CRASHES the game (bevy_ui
BorderRadius::resolve, min 0 max -12, on every duel/gauntlet reload);
the duel winner still drifts left at the end; the gauntlet never runs
dry (and the wreck reads as a useless second ship); the weave runner
still does not reach its beacons; the owner also rejects the menu's
well-derived camera ("each scenario poses its own camera") and wants
torpedoes rebalanced to Expanse-style near-one-hit lethality.

Root causes and fixes:

- CRASH: the menu UI resolved its render target through the SCENARIO
  camera; a self-reload despawns that camera mid-frame and the UI lays
  out against a degenerate target (negative node size). The
  remembered-pose hold (round 4's attempt) kept a camera ACTIVE but
  could not keep the UI's target entity alive. Real fix: the menu owns
  a dedicated UI camera (`IsDefaultUiCamera`, order 100, no clear),
  spawned on menu entry - backdrop reloads can no longer touch the
  interface's target. The remembered pose stays, now only to keep the
  backdrop VIEW from blinking through the loader pose.
- Camera contract inverted per owner direction: every backdrop authors
  a `SetCamera` in OnStart (the existing photo-mode action - it pins
  `ScriptedCameraPose` and strips the fly controller itself). The menu
  derives nothing: blank until the pose lands, hold remembered pose
  across reloads. A poseless backdrop is a lint ERROR - erroring
  scenarios are already filtered out of the menu draw, so the failure
  mode is "not in rotation", never "blank menu". Grace-frames fallback
  machinery deleted. The three new scenes DROP their invisible anchors
  entirely (nothing orbits there); the Anchor kind stays engine
  vocabulary for orbit targets. The example mod's backdrop teaches the
  new contract.
- Duel left-drift: the battery sat authored-Enemy (round 4), and the
  freshly victorious ship - still in its combat hold, which keeps ANY
  acquired hostile - chased it left to the leash edge. The battery is
  Neutral again and the beat's handler flips it Enemy just for the kill
  window (the scripted launch still needs Enemy ordnance for PD).
- Gauntlet never ran dry: 1800 rounds/turret was ~4x too generous ->
  400 (a roughly ten-torpedo defense across the pair).
- Weave beacons: `waypoint_slack` alone could not beat the autopilot's
  own 50 u rest distance -> new `arrival_standoff` per-ship override
  (`FlightArrivalStandoff`, read by the GOTO arrival rule and the
  patrol advance gate). Weave authors standoff 10 + slack 5: turns
  ~15 u off each beacon.
- Torpedo lethality: standard blast damage 100 -> 750 (breaking). A
  hit decides a small-craft fight; the counter is PD, and the ordnance
  stays 1 hp. Balance audit: clean.

## Follow-up design: structural blast pressure

Accepted 2026-08-17 after the arena exposed one torpedo deleting a large hull.

- Preserve standard 750 damage / 30 u radius and siege 2000 / 45 u. The visible
  sphere remains the damage radius. Preserve the centre-relative proximity fuze.
- Fix radial falloff to use each target collider's world centre. The current
  compound-body path incorrectly uses the shared rigid-body root for every
  section.
- Explosive pressure travels on one centre ray from the blast to each target
  section. Only live `SectionMarker` colliders consume penetration.
- A blocker that survives its incoming pressure stops the ray. A blocker that
  is destroyed transmits 65 percent of the pressure. Existing holes transmit
  without loss. Test blockers against current health; equality destroys.
- Cladding and fixtures still take radial damage but do not count as structural
  layers. Keep one global transmission constant; guidance + authored warhead
  composition is separate future work.
- Resolve every blast in one fixed tick against the same pre-damage health
  snapshot. Same-tick warheads can combine to destroy a blocker but cannot use
  the new hole until a later tick.
- Keep ordinary section health, overkill clamping, and structural collapse. No
  blast-specific ship cap or capital immunity.

Implementation:

- `NovaDamagePlugin` now collects blast overlaps during Avian's fixed physics
  pass and resolves them after `PhysicsSystems::Last`. All damage triggers stay
  deferred until every ray has read the same health snapshot.
- Collider centres come from Avian's `ColliderOf` + `ColliderTransform` lifted
  through the body's current `Position` and `Rotation`, not render transforms or
  the compound root origin.
- `examples/systems/blast_penetration.rs` reproduced the old path before the
  fix: every child on a compound body received root-distance damage (300, or
  400 from the double blast). It now proves attenuation, shielding, atomic
  salvos, and fixture exclusion and exits cleanly under autopilot.
- Verification: `nova_gameplay` 145 passed / 1 ignored; `nova_ship` 649 passed;
  `nova_scenario` 187 passed; `nova_authoring` 78 passed; catalog drift 2
  passed; the new rendered range passed; content lint, web CI, Rust format, and
  diff checks passed.
- First arena playtest found the visible shell taking damage around the whole
  ship. Cause: only structural TARGETS ran the centre-ray traversal;
  non-structural cladding and fixtures took direct radial damage even behind a
  section. The resolver now traces every Explosive target through structural
  blockers. Only sections consume penetration, unchanged. The range pins both
  directions: a fixture cannot shield a section, while a section does shield a
  fixture behind it.

## Round 5b (owner live-testing, 2026-08-14)

Owner hit two regressions from the camera rework: the backdrop
rendered BLACK behind a working menu, and everything ran at ~10 FPS.

- 10 FPS: two full game instances were sharing the GPU - my background
  verification run plus the owner's (scene_baseline was slow too, which
  ruled out menu code). Background instance killed; single-instance
  runs pace normally.
- BLACK backdrop, root-caused by bisection (disable the UI camera ->
  scene renders): `CameraOutputMode`'s DEFAULT has `blend_state: None`,
  and per bevy_camera's own doc an unblended write "ignores the
  existing data in the final render target" - the overlay camera's
  mostly-empty view replaced the scenario camera's whole frame. Bonus
  artifact: view textures are POOLED, and the overlay's uncleared view
  inherited the boot loading screen's final frame - the "NOVA OS /
  LOADING" ghost was a stale pooled texture, not a leaked entity
  (instrumentation proved the loading screen despawns cleanly). Fix:
  the overlay clears its own view to transparent
  (`ClearColorConfig::Custom(Color::NONE)`) and writes with
  `BlendState::ALPHA_BLENDING` over the scene.
- Duel stall found in the same session: a rival lost its flight
  computer WITHOUT counting as defeated, drifted out of the victor's
  leash reach, and froze the cycle for 11 minutes (plus a per-frame
  autopilot engage/disengage churn on the computer-less hulk - engine
  wart, noted in open threads). Blunt fix: duel and gauntlet arm a
  watchdog timer at OnStart (300 s / 360 s) that reloads the scenario;
  healthy cycles reload far earlier and the reload re-arms it.
- Verified after the fix: gauntlet backdrop + menu composite correctly
  through self-reloads (2 loads, 0 panics in the capture window; the
  earlier broken-compositing run did 6 crash-free reloads, pinning the
  round-5 UI-camera fix as the crash cure).

## Round 6 (owner feedback, 2026-08-14)

Feedback: make the menu a Factorio-style carousel (backdrops switch
between each other), give the endless scenes a limit, and prune the
three planetoid-and-orbiter scenes to the single most interesting one.

Design:

- Fixed hand-off ring, pure data: each scene's terminal reset now
  targets the NEXT scenario id instead of its own (gauntlet -> weave ->
  duel -> waystation -> gauntlet). Scenes with a natural act end
  (gauntlet's fallen stand, duel's erased victor) hand off from the
  aftermath linger; the endless ones (weave, waystation) arm a 150 s
  rotation timer at OnStart; the stall watchdogs also point at the next
  scene. The menu's random draw picks only the ENTRY point.
- Kept Waystation Traffic (live freighter lanes - the most alive of the
  three lookalikes); menu_ambience and menu_scrapyard deleted (authoring
  modules, catalog entries, bundle lines, generated RON).
- Duel stall, second root cause (SetHealth(500) on the computers did
  not cure it): the cripple is integrity DISCONNECTION - a dead hull
  node disables the controller subtree at any health - and the hulk
  drifted into the dressing ring, where rocks blocked the victor's line
  of fire (the LOS gate held the trigger forever). Scene fix: the duel
  arena has NO rocks at all now, and the leash widened 200 -> 400 so
  the winner chases a drifting cripple down and finishes it - the kill
  fires the defeat chain and the finale actually plays.

## Round 7 (owner feedback, 2026-08-14)

Feedback: the rockless duel arena reads oddly; particle VFX ghost
across the menu (suspected camera leak - correct); defeat rule should
be "loses weapons OR the controller" with thrusters irrelevant; plus a
gauntlet nit - the racer parks in front of the rock band and vanishes
into the low contrast.

- Defeat rule (engine, breaking): neutralize = disarmed OR brain-dead.
  New `HadFlightComputer` history stamp mirrors `WasArmedCombatant`, so
  computer-less emplacements (the scripted batteries) are exempt from
  the brain-death half and only die by disarm. Thrusters left the rule
  entirely. This structurally cures the duel stall - a crippled hulk
  defeats itself the frame its computer (or last gun) goes - so the
  duel got its dressing rocks BACK (LOS blocking no longer wedges
  anything), leash back to 250, hardened controllers kept so the act is
  a dogfight rather than a first-burst decapitation.
- VFX ghosting: bevy_hanabi renders through the 2D pipeline too, so the
  menu's overlay camera re-drew every world-space particle burst -
  untonemapped, alpha-blended over the finished frame. The overlay now
  sits on an empty render layer (23; nova_os_ui owns 20-22): its world
  pass draws nothing, and UI is unaffected because bevy_ui routes by
  TARGET camera, not layers.
- Gauntlet contrast: station circuit raised ~25 u above the fight plane
  (against black sky instead of the far rock cluster) and the camera
  pulled in to (0, 80, 260).

## Retention

- `shots/gauntlet-pd-intercept.png` - tracer stream vs inbound torpedo.
- `shots/weave-threading.png` - the runner inside the rock band.
- `shots/duel-dogfight.png`, `shots/duel-finisher-window.png`.
