# NOVA OS app runtime

- PRIORITY: 46
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player using NOVA OS, I want some commands to launch full-screen monitor
apps and then return to the terminal, so that GUI-heavy tools like the map and
ship viewer can use the same screen without permanent drawer panels.

## Steps

See `DECISION.md` for the two accepted forks: app-as-plugin trait objects, and
context-sensitive Escape as the app-exit key.

- [x] Add a NOVA OS mode/state model for terminal mode vs active app mode
      (`TerminalMode::App { .. }`), tracking the active app id; app mode leaves
      the terminal scrollback untouched so exit restores it.
- [x] Add a `NovaOsAppRuntime` trait + `NovaOsAppRegistry` resource
      (app-as-plugin). A launch command resolved from the registry enters app
      mode and hands keyboard/mouse input ownership to the app.
- [x] Implement uniform app exit owned by the runtime: on-screen chrome close
      control plus context-sensitive Escape, returning to terminal mode with
      scrollback and prompt intact.
- [x] Make Escape context-sensitive: in app mode it exits the app (drawer stays
      open); in terminal/prompt mode it still closes the whole computer. Done in
      one place (`close_drawer_from_menu_keys`) so a single Escape read cannot
      both exit the app and close the drawer.
- [x] Add a placeholder/sample app behind test-only code, registered into the
      registry, to verify lifecycle without waiting for the map app.
- [x] Add tests for app launch, input ownership, app exit (chrome + Escape),
      terminal-mode Escape still closing the drawer, and restoration on
      reopen/teardown.
- [x] Add/update `tasks/20260726-115334/NOTES.md` with the app lifecycle
      contract, input decisions, and self-reflection.

## Definition of Done

- A terminal command resolved from the app registry launches an app that
  replaces terminal content inside the same monitor. (test:
  `terminal_command_launches_registered_app`)
- Exiting the app (chrome close or Escape) returns to terminal mode with
  scrollback and prompt state preserved. (test:
  `nova_os_app_close_restores_terminal_state`)
- App mode owns its input, and Escape exits the app to the terminal rather than
  closing the drawer; from terminal mode Escape still closes the drawer. (test:
  `nova_os_app_mode_owns_input_and_escape_exits_app`)
- Reopening the drawer restores the active app/terminal state within a scenario,
  and teardown clears stale app state back to the terminal. (test:
  `nova_os_app_state_resets_on_teardown`)
- Touched drawer/app-runtime tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay -- drawer terminal`)
  Note: corrected from `... drawer terminal` (no `--`) during work - `cargo test`
  takes a single positional filter and rejects the second, so the two filters
  must be passed through to libtest after `--`.

## Notes

- Depends on: `20260726-134738` (and transitively `20260726-115324`).
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- This task unblocks the `map` app task `20260724-102320` and the stretch ship
  viewer task `20260726-115339`.
