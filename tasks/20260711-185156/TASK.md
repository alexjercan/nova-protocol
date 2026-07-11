# Pause menu: ESC overlay in game and editor with Back to Main Menu

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.5.0,ui,menu,spike

Goal: a pause/escape menu so the player can get back out. Requested by the
user 2026-07-11: needed from both the game (a running scenario, New Game or
sandbox play) and the editor, with at least Back to Main Menu; Resume and
Exit are the obvious companions, and Settings can join once 20260711-180511
has content.

Direction notes for /plan (verify against the code, not from memory):
- Going back to the menu means re-entering GameStates::MainMenu. Nothing
  hooks OnExit(Playing) today (the state machine was one-way until the menu
  landed), so the plan must audit what a Playing -> MainMenu transition
  leaves behind: trigger UnloadScenario, reset the editor's private
  ExampleStates (it has no Loading-return path), reset GameMode, and check
  DespawnOnExit usage in gameplay/editor.
- Editor "back" likely means: from ExampleStates::Scenario, F1 already
  returns to Editor; the pause menu adds Editor -> MainMenu.
- Pausing a running scenario also implies freezing physics/time
  (avian/virtual time pause) while the overlay is up - decide scope: a
  true pause or just an overlay with the sim running.
- Whether ESC conflicts with existing bindings must be checked against the
  current binding table (LESSONS: concrete key assignments quote the
  binding table).

Notes:
- Spike: docs/spikes/20260711-180500-main-menu.md (menu family)
- Parent task: 20260711-174915
- Depends on: 20260711-180426 (main menu, CLOSED - landed 8504948)

