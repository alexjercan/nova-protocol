# Gamepad navigation, and a real playthrough on hardware

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.13.0,input,gamepad

Rescoped 2026-08-31 for v0.13.0. The mobile virtual pad (old Part B) split
to `20260831-145917` and stays backlog, so its layout targets interactions
proven with a pad first. This task is the gamepad, and the point is a REAL
gamepad in hand - the pad support so far shipped against the harness, not
against hardware. The settled surface this task waited for exists:
v0.12.0 landed the editor rework and the settings rebinding
(`20260824-120527`). Current-state audit:
`tasks/20260815-231945/INPUT-AND-PROCESS.md` (it corrects this task's
older stale paths: the flight rig is
nova_ship/src/input/player/flight_rig.rs, the HUD crate is nova_hud).

## Part A - Gamepad navigation for menus and the editor

Make the whole out-of-cockpit UI operable with a gamepad (no mouse):

- Menus (main menu + ESC pause menu, `nova_menu`): directional focus
  movement (D-Pad / left stick), confirm (South), back/cancel (East), with a
  visible focus ring.
- Editor (`nova_editor`): navigate the gallery and rail, place and rebind
  sections, trigger play-test from the pad.
- Prefer Bevy's UI focus/navigation primitives if they fit; otherwise a
  small focus-ring + gamepad-driven focus system.
- The existing raw pad reads are inventoried in INPUT-AND-PROCESS.md
  section 5 (pause Start, HUD Select, editor L3, placement capture, NOVA OS
  RightThumb/Start). They should be registry actions (`20260820-174148`,
  landed v0.12.0) or documented fixed chords.

## Part B - The hardware playthrough

With a physical pad plugged in, play the game end to end: boot, navigate
the menus, rebind in settings, build in the editor, fly a campaign
chapter, pause, quit. Fix what the harness never caught - dead zones,
stick response, chord conflicts, focus traps, prompts that show keyboard
glyphs to a pad player.

- Record the playthrough findings with this task; each fix is evidence.
- Record which pad(s) were tested.

Done when: the menus and the editor are fully operable with a gamepad, a
full hardware playthrough completes without touching the keyboard, and
the findings list is closed or explicitly deferred.
