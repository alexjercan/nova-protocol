# Ambient menu background scenario (live scene behind the menu)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,ui,menu,scenario,spike

## Goal

Game scenes playing out behind the main menu, Factorio-style, per the
user's 2026-07-11 direction: a `menu_ambience` scenario where an AI ship
visibly does something (orbit a big asteroid), displayed through a WASD
camera with player inputs disabled, and with the HUD fully off so the shot
is cinematic.

## Steps

- [ ] Add a `menu_ambience` ScenarioConfig builder in
      `crates/nova_assets/src/scenario.rs` and register it in
      `register_scenario`: cubemap skybox, a big central asteroid (reuse
      the `asteroid_grav` pattern: large radius + surface_gravity so a real
      gravity well exists), a modest scatter of smaller rocks, no player
      ship, no objectives, no areas.
- [ ] Put an AI ship in orbit around the big asteroid - the scene's
      "something happening". Verify first which mechanism can hold an
      orbit without a player present: the ORBIT autopilot verb (task
      20260709-193339, landed) driven by an AI controller, or an initial
      tangential velocity at v_circ for a ballistic orbit (the well math in
      crates/nova_gameplay/src/gravity gives v_circ ~ sqrt(g_surface *
      r_surface^2 / r)). Pick whichever holds a stable orbit for minutes;
      record the choice and evidence in this file.
- [ ] In nova_menu, replace the standalone menu camera from task
      20260711-180426 with `LoadScenario(GameScenarios["menu_ambience"])`
      on `OnEnter(GameStates::MainMenu)`. The loader spawns its own WASD
      camera + skybox (crates/nova_scenario/src/loader.rs:162); frame it on
      the orbiting ship / big asteroid.
- [ ] Disable player input into the WASD camera while in MainMenu (the user
      explicitly wants the camera as a fixed cinematic viewpoint, not
      flyable): gate the WASD controller's input systems off in MainMenu,
      or spawn the camera without the controller and position it directly.
- [ ] Cinematic HUD-off: the ambient scene has no player ship, so
      player-HUD widgets should not spawn at all (they hang off
      PlayerSpaceshipMarker) - verify that, and also hide the remaining
      chrome (status bar fps/version) while in MainMenu. If task
      20260711-180501 (HudVisibility ALL/MINIMAL/NONE) has landed by then,
      set HudVisibility::None on entering MainMenu and restore on exit
      instead of a bespoke hide; otherwise do the minimal status-bar hide
      and leave a pointer for 180501 to absorb it.
- [ ] Verify teardown on both exits: New Game fires LoadScenario (loader
      tears down the previous scenario); Sandbox enters the editor scene -
      confirm the editor's OnEnter(Editor) copes with a loaded scenario or
      trigger `UnloadScenario` from the Sandbox button.
- [ ] Run check/fmt and any newly written tests; eyeball with the
      screenshot harness that the menu renders over the ambient scene with
      the ship in frame.
- [ ] Docs: CHANGELOG.md entry; append a Fix record line to
      docs/spikes/20260711-180500-main-menu.md.

## Notes

- Spike: docs/spikes/20260711-180500-main-menu.md
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
