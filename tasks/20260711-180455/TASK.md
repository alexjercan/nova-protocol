# Ambient menu background scenario (live scene behind the menu)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,ui,menu,scenario,spike

## Goal

Game scenes playing out behind the main menu, Factorio-style: a live
`menu_ambience` scenario (skybox, drifting asteroids, later passive ships)
loaded behind the menu panel, with a slow-orbiting camera.

## Steps

- [ ] Add a `menu_ambience` ScenarioConfig builder in
      `crates/nova_assets/src/scenario.rs` and register it in
      `register_scenario`: cubemap skybox, ~30 asteroids in a wider scatter,
      no player ship, no objectives, no areas. Verify first whether
      `AsteroidConfig`/spawn path supports an initial velocity or spin; if
      not, static asteroids are fine for v1 (the orbiting camera provides
      the motion).
- [ ] In nova_menu, replace the standalone menu camera from task
      20260711-180426 with `LoadScenario(GameScenarios["menu_ambience"])`
      on `OnEnter(GameStates::MainMenu)`. The loader spawns its own camera +
      skybox (crates/nova_scenario/src/loader.rs:162).
- [ ] Slow-orbit the menu camera: verify the transform-orbit helper's name
      and API in bevy-common-systems (re-exported through
      nova_gameplay::prelude), then attach it to the scenario camera while
      in MainMenu. Confirm the WASD controller does not eat mouse/keys under
      the menu; disable or remove it in MainMenu if it does.
- [ ] Verify teardown on both exits: New Game fires LoadScenario (loader
      tears down the previous scenario); Sandbox enters the editor scene -
      confirm the editor's OnEnter(Editor) copes with a loaded scenario or
      trigger `UnloadScenario` from the Sandbox button.
- [ ] Polish, only if playerless AI behaves: add one or two passive drifting
      ships (`SpaceshipController::None`). Skip without guilt otherwise.
- [ ] Run check/fmt and any newly written tests; eyeball with the screenshot
      harness (BCS_SHOT) that the menu renders over the ambient scene.
- [ ] Docs: CHANGELOG.md entry; append a Fix record line to
      docs/spikes/20260711-180500-main-menu.md.

## Notes

- Spike: docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (main menu state + panel)
- LoadScenario is dynamic and self-cleaning: the observer at
  crates/nova_scenario/src/loader.rs:111 despawns ScenarioScopedMarker
  entities from the previous scenario before spawning the new one.
