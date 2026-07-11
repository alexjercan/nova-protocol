# Main menu: MainMenu state, bottom-right panel UI, mode wiring

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,ui,menu,spike

Goal: give the game a real front door. Add a `MainMenu` variant to
`GameStates` (Loading -> MainMenu -> Playing), implement the menu in a new
`nova_menu` crate with bevy_ui: a small panel anchored bottom-right with the
title "Nova Protocol" and four buttons.

- New Game: set `GameMode::NewGame`, transition to Playing, load an existing
  scenario (asteroid_field) with a canned default player ship.
- Sandbox: set `GameMode::Sandbox`, transition to Playing; nova_editor enters
  its editor state only in this mode. Register a "sandbox_field" scenario
  variant (asteroid_field without hostile spawns) for the editor's Play
  button.
- Settings: placeholder sub-panel (title + Back). Content is task
  20260711-180511 (v0.6.0).
- Exit: send AppExit; hide the button on wasm.

Existing examples must keep working (builder flag or direct Playing set).
Menu background starts as skybox only; the live ambient scene is task
20260711-180455.

Notes:
- Spike: docs/spikes/20260711-180500-main-menu.md
- Parent task: 20260711-174915
- Only two OnEnter(GameStates::Playing) hooks exist today: nova_core
  AppBuilder (status UI) and nova_editor (enters ExampleStates::Editor).
