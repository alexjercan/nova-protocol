# NOVA OS terminal input and command shell

- STATUS: CLOSED
- PRIORITY: 48
- TAGS: v0.9.0, feature, ui, hud, input

## Story

As a player in NOVA OS, I want a focused terminal prompt with command editing,
history, autocomplete and helpful errors, so that the ship computer is an
interactive terminal rather than a static screen. This builds on the monitor
shell from `20260726-115320`.

## Steps

- [x] In `crates/nova_gameplay/src/hud/drawer.rs`, add a drawer-local terminal
      model resource for prompt text, cursor position, scrollback rows, command
      history, history cursor, completion hint, parse status and active mode.
- [x] Add private command registry/parsing helpers for simple command words and
      verb+argument forms, with `help` and `clear` as the first executable
      commands and no gameplay-mutating commands.
- [x] Split Tab handling in `toggle_drawer`: from `PauseStates::Unpaused` it
      opens NOVA OS as today, while from `PauseStates::Drawer` it leaves the
      drawer open so the terminal input system can use Tab for completion.
- [x] Add a drawer-only keyboard input system under `PauseStates::Drawer` for
      text entry, Enter submit, Backspace/Delete, Left/Right cursor movement,
      Up/Down history navigation and Tab autocomplete.
- [x] Preserve Escape close through `nova_menu::toggle_pause`, updating only the
      existing Escape drawer test if the terminal state needs setup in the rig.
- [x] Replace the prompt placeholder in the NOVA OS monitor with live terminal
      UI nodes for scrollback, prompt, cursor placement, completion hint,
      invalid-token/error coloring and nearest-command suggestions.
- [x] Add tests in the touched drawer/input rigs for the keyboard state machine,
      autocomplete behavior, typo suggestions, prompt rendering and Escape/Tab
      split.
- [x] Add/update `tasks/20260726-115324/NOTES.md` with input routing decisions,
      current code facts, tricky cases, bugs diagnosed, alternatives and
      self-reflection.

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
  `nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`)

## Notes

- Depends on: `20260726-115320`.
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Decision: `tasks/20260726-115324/DECISION.md`.
- Do not add gameplay-mutating commands in this task; it owns shell mechanics.
- Assumption for the plan gate: the concrete artifact is a drawer-owned Bevy UI
  terminal model and keyboard system in `hud/drawer.rs`, not a new pause/menu
  mode, not a shared `nova_ui` widget, and not a gameplay command bus yet.
- Current code facts: `hud/drawer.rs` owns `NovaDrawerPlugin`, `toggle_drawer`,
  scroll viewports, `DrawerFlightLog`, objective rebuilds and the NOVA OS prompt
  placeholder. `nova_menu::toggle_pause` closes `PauseStates::Drawer` on Escape.
  Flight controls are already gated inert while `PauseStates::Drawer` is active,
  so the terminal can own drawer-mode keyboard input without changing the flight
  input rigs.

## Work Record

- Added drawer-local `NovaOsTerminal` state, `TerminalRow` scrollback,
  command parsing, completion and close-match suggestions in `hud/drawer.rs`.
- Wired `KeyboardInput` handling while `PauseStates::Drawer` is active for
  typed characters, Enter, Backspace/Delete, cursor movement, history and Tab
  completion.
- Changed keyboard Tab so it opens NOVA OS from flight but no longer closes an
  open drawer. The gamepad right-stick click still toggles open/closed.
- Replaced the pending prompt placeholder with live scrollback, prompt and hint
  text nodes, including invalid-command coloring.
- Updated `CHANGELOG.md` and `web/src/wiki/hud.md` because player-facing drawer
  input behavior changed.
- Red-first proof: `nix develop --command cargo test -p nova_gameplay drawer`
  failed before implementation with missing terminal types and systems.
- Bug diagnosed during implementation: the initial Tab split also stopped the
  gamepad right-stick click from closing the drawer; the existing
  `pad_toggles_drawer_state` test caught it, and `toggle_drawer` now treats
  gamepad close separately from Tab autocomplete.
- Verification:
  `nix develop --command cargo test -p nova_gameplay drawer`;
  `nix develop --command cargo test -p nova_gameplay terminal`;
  `nix develop --command cargo test -p nova_menu escape_closes_the_drawer_to_unpaused`;
  `nix develop --command cargo fmt --check`;
  `nix develop --command cargo check`;
  `cd web && npm ci && npm run ci`.
- Local full `cargo test` and `cargo clippy` were not run, per repo guidance;
  CI owns those slower workspace-wide checks.
