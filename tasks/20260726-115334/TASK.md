# NOVA OS app runtime

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.9.0,feature,ui,hud

## Story

As a player using NOVA OS, I want some commands to launch full-screen monitor
apps and then return to the terminal, so that GUI-heavy tools like the map and
ship viewer can use the same screen without permanent drawer panels.

## Steps

- [ ] Add a NOVA OS mode/state model for terminal mode vs active app mode,
      including the active app id and restored terminal scrollback.
- [ ] Add app registration/plumbing so a command can launch an app and hand
      keyboard/mouse input ownership to it.
- [ ] Implement app exit behavior: explicit app chrome close plus the chosen
      keyboard chord (`Ctrl+C` or `Ctrl+[`), returning to terminal mode with
      scrollback intact.
- [ ] Keep Escape as drawer close, not app close, unless implementation uncovers
      a hard input conflict that must return to planning.
- [ ] Add a placeholder/sample app in tests or behind test-only code to verify
      lifecycle without waiting for the map app.
- [ ] Add tests for app launch, input ownership, app exit, drawer close while an
      app is active, and restoration on reopen/teardown.
- [ ] Add/update `tasks/20260726-115334/NOTES.md` with the app lifecycle
      contract, input decisions, and self-reflection.

## Definition of Done

- A terminal command can launch an app that replaces terminal content inside the
  same monitor. (test: `terminal_command_launches_registered_app`)
- Closing the app returns to terminal mode with scrollback and prompt state
  preserved. (test: `nova_os_app_close_restores_terminal_state`)
- App mode owns its input, while Escape still closes the whole drawer by the
  drawer route. (test: `nova_os_app_mode_routes_input_and_escape_closes_drawer`)
- Reopening the drawer restores or resets according to the chosen scenario-local
  persistence rule, and teardown clears stale app state. (test:
  `nova_os_app_state_resets_on_teardown`)
- Touched drawer/app-runtime tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer terminal`)

## Notes

- Depends on: `20260726-115324`.
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- This task unblocks the `map` app task `20260724-102320` and the stretch ship
  viewer task `20260726-115339`.
