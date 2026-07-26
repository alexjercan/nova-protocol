# NOVA OS terminal input and command shell

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.9.0,feature,ui,hud,input

## Story

As a player in NOVA OS, I want a focused terminal prompt with command editing,
history, autocomplete and helpful errors, so that the ship computer is an
interactive terminal rather than a static screen. This builds on the monitor
shell from `20260726-115320`.

## Steps

- [ ] Add a drawer-local terminal state resource for prompt text, cursor
      position, scrollback rows, command history, completion hint, parse status
      and active mode.
- [ ] Route keyboard input while `PauseStates::Drawer` is active so text editing
      goes to the terminal prompt instead of flight controls.
- [ ] Make Tab open the drawer from flight but act as autocomplete once NOVA OS
      owns the keyboard; Escape closes the drawer from terminal mode.
- [ ] Implement Enter submit, Backspace/Delete, Left/Right cursor movement and
      Up/Down history navigation.
- [ ] Implement command registry plumbing for simple verb and verb+argument
      commands, starting with `help` and `clear`.
- [ ] Render valid prefix hints, invalid token coloring and closest-command
      suggestions in the terminal UI.
- [ ] Add tests for the keyboard state machine, autocomplete behavior, typo
      suggestions and Escape/Tab split.
- [ ] Add/update `tasks/20260726-115324/NOTES.md` with input routing decisions,
      tricky cases, and self-reflection.

## Definition of Done

- Tab opens NOVA OS from flight, but inside the terminal it completes command
  prefixes instead of closing the drawer. (test:
  `tab_opens_drawer_then_completes_terminal_command`)
- Escape closes NOVA OS from terminal mode without going through the pause menu.
  (test: existing `escape_closes_the_drawer_to_unpaused` updated/passes)
- Enter, Backspace/Delete, Left/Right and Up/Down behave like a minimal shell.
  (test: `terminal_prompt_edits_and_navigates_history`)
- Invalid commands render an error plus a close-match suggestion when available.
  (test: `terminal_unknown_command_suggests_nearest_match`)
- Touched input/drawer tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer terminal`)

## Notes

- Depends on: `20260726-115320`.
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Do not add gameplay-mutating commands in this task; it owns shell mechanics.
