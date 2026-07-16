# Pause the game while the Victory/Defeat outcome screen is displayed (like the menu)

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.7.0,feature,ui,scenario


## Goal

When the scenario outcome frame (VICTORY/DEFEAT overlay, task 20260716-125856)
is shown, the simulation should PAUSE - freeze physics/AI/gameplay ticking the
same way the menu screens do - instead of the world continuing to run behind
the overlay. Today the overlay draws on top of a still-live scene.

## Direction

- Find how the menu/pause states already freeze the sim (the Esc pause menu and
  the main menu). There is very likely an existing "gameplay is running" run
  condition / state gate (a Pause state, a `TimeState`, or a
  `run_if(in_state(Playing))` guard on the gameplay schedules).
- Apply the SAME mechanism when the outcome frame is active, so physics, AI,
  weapons and timers stop while VICTORY/DEFEAT is up - matching the menu's
  behavior exactly rather than inventing a second pause path.
- Keep the outcome frame's own input alive (Continue/Retry/Main Menu buttons
  must still work while the sim is paused), same as the pause menu overlay.
- Decide what happens to the pause on Continue/Retry: unpause on resume/retry,
  stay paused into the menu, etc. (`does-the-old-element-survive`).

## Notes

- Reported/requested by user 2026-07-16.
- Related: scenario outcome frame (20260716-125856), pause menu Retry
  (20260716-210125).
