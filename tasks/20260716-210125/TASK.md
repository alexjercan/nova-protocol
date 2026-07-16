# Pause menu: Retry button - restart the current level from the Esc menu

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.7.0,feature,menu,ui,scenario

Requested directly by the user: add the possibility to "retry" a level when
pressing Esc. Pressing Esc in-game opens the pause menu; it should offer a
Retry action that restarts the current scenario from scratch (same scenario,
fresh state), in addition to the existing options.

Requirements:
- A Retry button in the pause menu, styled like the existing buttons.
- Retry tears down the running level and relaunches the same scenario the
  player selected (works both for scenarios picked from the main menu and,
  if applicable, editor play-tests).
- Player-facing wiki pages that document the pause menu / controls get
  updated in the same task.

Steps:
- [x] Explore pause menu wiring and scenario start/teardown (how GameStates
      transitions clean up, where the selected scenario is tracked).
- [x] Implement the Retry action + button.
- [x] Verify with cargo check + fmt; run the game if feasible.
- [x] Update player wiki docs if they describe the pause menu.
- [x] Record NOTES.md with the design record.

Done 2026-07-16. A Retry button now sits between Resume and Back to Main
Menu in the pause overlay, shown only while a scenario is live; it
re-triggers LoadScenario with the CurrentScenario config (the loader's own
teardown-then-spawn path, identical to the defeat overlay's Retry) and
unpauses. Covered by two new nova_menu tests (both pass); tutorial.html and
CHANGELOG updated in the same change. Full design record, difficulties and
reflection: NOTES.md next to this file.

