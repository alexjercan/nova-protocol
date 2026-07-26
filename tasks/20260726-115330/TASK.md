# NOVA OS terminal output commands

- STATUS: OPEN
- PRIORITY: 47
- TAGS: v0.9.0,feature,ui,hud,gameplay

## Story

As a player using NOVA OS, I want the first useful commands to print real ship
computer information into the terminal, so that the drawer replaces the old
objectives/log panels without losing their value.

## Steps

- [ ] Implement `log` as terminal output over the existing drawer Flight Log /
      `StoryFeed` and objective-event data, preserving useful ordering.
- [ ] Implement `objectives` as terminal output over active `GameObjectives`.
- [ ] Implement read-only `ship` as terminal output over available player ship
      section/status data, including weapons, thrusters and critical/neutralized
      state where available.
- [ ] Add empty-state output for each command so a quiet scenario still returns
      useful text.
- [ ] Add tests that commands read current data, update after resource changes,
      and do not duplicate stale rows across scenario/player teardown.
- [ ] Update player-facing docs surfaces that describe the drawer after the
      implementation changes land.
- [ ] Add/update `tasks/20260726-115330/NOTES.md` with data-source choices,
      reset/teardown handling, and self-reflection.

## Definition of Done

- `log` prints comms plus objective event rows from the same sources the landed
  Flight Log uses. (test: `terminal_log_command_prints_flight_log_rows`)
- `objectives` prints the current active objectives and a readable empty state.
  (test: `terminal_objectives_command_prints_active_objectives`)
- `ship` prints a read-only ship status summary from live player ship data.
  (test: `terminal_ship_command_prints_section_status`)
- Scenario/player teardown does not leak stale command output source state into
  the next run. (test: `terminal_commands_clear_on_drawer_teardown`)
- Touched drawer/HUD tests pass. (cmd:
  `nix develop --command cargo test -p nova_gameplay drawer terminal`)

## Notes

- Depends on: `20260726-134738` (and transitively `20260726-115324`).
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Do not add `reload` or `repair` here; this task is read-only command output.
