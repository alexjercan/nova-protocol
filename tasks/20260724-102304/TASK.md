# Drawer shell + interaction model + objectives section (Tab, PauseStates::Drawer, slide anim, tab-handle anchor)

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.9.0,spike,feature,ui,hud

## Goal

The Tab ship-computer drawer's SHELL and interaction model, plus its first
section (expanded objectives). This task GATES the drawer's other sections and
211520's diegetic tuck-target. Design fixed by Spike:
tasks/20260721-211512/SPIKE.md - implement its recommendation, do not
re-litigate the architecture.

Scope (direction-level; /plan breaks into steps at pickup):

- Tab keybind that opens/closes a right-side drawer, hard-coded KeyCode::Tab in
  the spirit of nova_menu toggle_pause (runs in GameStates::Playing regardless
  of pause substate so it can also CLOSE while frozen); NOT in the Unpaused-gated
  flight input rig. O stays ORBIT; Tab avoids the collision.
- Pause + cursor via a new PauseStates::Drawer variant (option A2 in the spike):
  the variant carries overlay identity; generalize pause_clocks/release_cursor
  (and exit partners) to fire on any non-Unpaused state; flight/section gating
  (already in_state(Unpaused)) is unchanged. ESC from Drawer closes to Unpaused.
- Slide-in animation from the right edge via bevy_common_systems TweenPlugin
  (already wired for comms), with a backdrop fade; a collapsed tab HANDLE on the
  right edge.
- Expose the tab handle's screen anchor (component/resource holding its screen
  rect) as the tween TARGET for 211520's diegetic objective hand-off.
- A section framework the later sections (comms log, map, ship) slot into.
- First section: EXPANDED objectives, rendering bevy_common_systems
  GameObjectives (data already exists).

## Notes

- Spike: tasks/20260721-211512/SPIKE.md (RECOMMENDED). This task carries the
  load-bearing DECISION.md for the A2 pause-axis + Tab-keybind choice, citing
  the spike as context.
- Builds on the state-driven cursor from 20260721-211500 (CLOSED).
- Gates 20260721-211520 (needs the tab-handle anchor) and the comms-log section.
