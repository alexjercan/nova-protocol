# Ambient menu background scenario (live scene behind the menu)

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.5.0,ui,menu,scenario,spike

Goal: game scenes playing out behind the main menu, Factorio-style. Register
a `menu_ambience` scenario (skybox, drifting asteroid field, later a few
passive AI ships flying by; no player, no objectives) and load it via the
existing `LoadScenario` event on OnEnter(GameStates::MainMenu). Slow-orbit
the scenario camera (transform-orbit helpers in bevy-common-systems).
Entering New Game/Sandbox loads the next scenario, which already tears the
old one down.

Start with asteroids only; add ambient ships as polish once playerless AI
behavior is verified.

Notes:
- Spike: docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (main menu state + panel)

