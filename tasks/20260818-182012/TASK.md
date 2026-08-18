# Gamepad support and a real settings menu with rebinding

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: archive,duplicate

Add real gamepad support and turn the Settings menu into an actual settings
menu.

## Gamepad

- First-class gamepad play: flight, autopilot verbs, radar locking, weapons,
  camera, interface - the full keybinds surface, not a partial mapping.
- The keybinds wiki page already documents a gamepad column; make the game
  match it (or correct the page where the design differs).

## Settings menu

- Today: master volume, graphics preset, and a READ-ONLY keybind reference.
- Wanted: rebinding (keyboard + gamepad), plus room for the fun extras
  ("cool things in there").
- Persistence already exists (settings survive restarts) - reuse it for
  bindings.

## Notes

- Update the wiki pages that this touches: settings.md, keybinds.md
  (read-only claim goes away), getting-started.
- Rebind UI should follow the shared NOVA keybind conventions (the editor and
  NOVA OS share bindings; keep one source of truth).

## CLOSED 2026-08-18 - merged into `20260714-001140`

Duplicate. `20260714-001140` covers the same gamepad ground in more detail and
adds the mobile virtual pad; it now carries the settings-menu and rebinding
half of this task too. Both are on the backlog.
