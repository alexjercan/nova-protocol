# Ambient menu background scenario (live scene behind the menu)

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.5.0,ui,menu,scenario,spike

## Goal

Game scenes playing out behind the main menu, Factorio-style, per the
user's 2026-07-11 direction: a `menu_ambience` scenario where an AI ship
visibly does something (orbit a big asteroid), displayed through a WASD
camera with player inputs disabled, and with the HUD fully off so the shot
is cinematic.

## Steps

- [x] Add a `menu_ambience` ScenarioConfig builder in
      `crates/nova_assets/src/scenario.rs` and register it in
      `register_scenario`: cubemap skybox, a big central asteroid (reuse
      the `asteroid_grav` pattern: large radius + surface_gravity so a real
      gravity well exists), a modest scatter of smaller rocks, no player
      ship, no objectives, no areas.
- [x] Put an AI ship in orbit around the big asteroid - the scene's
      "something happening". Verify first which mechanism can hold an
      orbit without a player present: the ORBIT autopilot verb (task
      20260709-193339, landed) driven by an AI controller, or an initial
      tangential velocity at v_circ for a ballistic orbit (the well math in
      crates/nova_gameplay/src/gravity gives v_circ ~ sqrt(g_surface *
      r_surface^2 / r)). Pick whichever holds a stable orbit for minutes;
      record the choice and evidence in this file.
- [x] In nova_menu, replace the standalone menu camera from task
      20260711-180426 with `LoadScenario(GameScenarios["menu_ambience"])`
      on `OnEnter(GameStates::MainMenu)`. The loader spawns its own WASD
      camera + skybox (crates/nova_scenario/src/loader.rs:162); frame it on
      the orbiting ship / big asteroid.
- [x] Disable player input into the WASD camera while in MainMenu (the user
      explicitly wants the camera as a fixed cinematic viewpoint, not
      flyable): gate the WASD controller's input systems off in MainMenu,
      or spawn the camera without the controller and position it directly.
- [x] Cinematic HUD-off: the ambient scene has no player ship, so
      player-HUD widgets should not spawn at all (they hang off
      PlayerSpaceshipMarker) - verify that, and also hide the remaining
      chrome (status bar fps/version) while in MainMenu. If task
      20260711-180501 (HudVisibility ALL/MINIMAL/NONE) has landed by then,
      set HudVisibility::None on entering MainMenu and restore on exit
      instead of a bespoke hide; otherwise do the minimal status-bar hide
      and leave a pointer for 180501 to absorb it.
- [x] Verify teardown on both exits: New Game fires LoadScenario (loader
      tears down the previous scenario); Sandbox enters the editor scene -
      confirm the editor's OnEnter(Editor) copes with a loaded scenario or
      trigger `UnloadScenario` from the Sandbox button.
- [x] Run check/fmt and any newly written tests; eyeball with the
      screenshot harness that the menu renders over the ambient scene with
      the ship in frame.
- [x] Docs: CHANGELOG.md entry; append a Fix record line to
      tasks/20260711-180500/SPIKE.md.

## Notes

- Spike: tasks/20260711-180500/SPIKE.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (main menu state + panel)
- Related: 20260711-180501 (HUD visibility levels) - the NONE level is the
  intended long-term mechanism for the cinematic look here.
- User direction (2026-07-11, verbatim intent): factorio-like = a scenario
  where an AI ship does something, e.g. orbits a big asteroid; display it
  with a WASD camera but disable player inputs; use the NONE HUD level for
  the cinematic look. This supersedes the earlier "static asteroids +
  slow-orbiting camera" minimum - the ship IS the scene now, so the
  camera can be fixed.
- LoadScenario is dynamic and self-cleaning: the observer at
  crates/nova_scenario/src/loader.rs:111 despawns ScenarioScopedMarker
  entities from the previous scenario before spawning the new one.

## Close record (2026-07-11)

What changed:
- `menu_ambience` scenario (nova_assets): a 20u-nominal planetoid with an
  authored 6 u/s^2 gravity well at the origin, 14 depth rocks in a ring at
  170-240u below the orbit plane, and a passive `menu_orbiter` ship.
- nova_menu: OnEnter(MainMenu) loads the scenario instead of spawning its
  own camera; Update systems (MainMenu only) strip the WASD controller and
  hold a cinematic pose derived from the well's runtime geometry, re-stage
  the orbiter onto body_radius+40 and seed tangential v_circ from the
  well's real mu (shared circular_orbit_speed helper); the status bar hides
  on entry and restores on exit; the Sandbox button triggers UnloadScenario
  (the editor does not tear scenarios down itself).

Mechanism decision (the task's verify-first step): ORBIT autopilot / AI
flying was ruled out for the menu because the editor gates the spaceship
input/section system sets on its private Scenario state - in MainMenu no
thruster can fire. Ballistic orbit (gravity + seeded velocity) chosen;
task 20260711-185440 tracks real AI orbit behavior.

Evidence rig (throwaway harness 99_ambience_check, not committed: default
app under BCS_AUTOPILOT, sampling orbiter kinematics + well inventory per
second): final run travelled=84.1u over ~4.7s of orbit, radius band
[118.9, 140.0] (the 140 is the pre-restage spawn), velocity vector rotating
at constant ~17.8 u/s (= sqrt(mu/r) for the observed mu 37350), well alive,
camera at (0, 89, 297) with no WASD controller. Xvfb screenshots differ
across 8s intervals and show planetoid + menu + hidden status bar.
Longer-horizon stability (the "holds for minutes" requirement) rests on
the gravity integration test in crates/nova_gameplay/src/gravity.rs
(bounded 70s v_circ orbit under the same fixed-step semi-implicit
integrator) plus the absence of any damping on ships; the observed 4.7s
run verifies the seeding geometry, not the long horizon itself.

Bugs found and fixed along the way (all diagnosed from the harness trace,
not theory):
1. Camera pose write was overwritten by the WASD controller running later
   the same frame the removal was queued - fixed by writing the pose only
   on frames where the controller is already gone (producer/consumer
   ordering, the two-clocks family again).
2. Orbit radius was hardcoded at 50u, inside the planetoid's GEOMETRIC
   body radius (~80-91u across seeds; the well derives mu/SOI from the
   generated collider, not the nominal radius) - ship spawned inside the
   collider, was flung by penetration resolution. Fixed by deriving orbit
   radius and camera framing from the runtime GravityWell.body_radius.
3. The depth-rock ring (60-95u) also spawned inside the planetoid mesh;
   the penetration-resolution collision damage destroyed the planetoid
   (well vanished within a second, twice). Fixed by moving the ring to
   170-240u below the orbit plane.

Reflection: all three bugs came from reasoning about the scene with
NOMINAL sizes while the gravity/collider system operates on GEOMETRIC
runtime sizes - the second trace sample (well inventory per second) found
each one immediately. Instrument first, stare at screenshots later.
