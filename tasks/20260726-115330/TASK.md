# NOVA OS terminal output commands

- PRIORITY: 47
- TAGS: v0.9.0, feature, ui, hud, gameplay
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

As a player using NOVA OS, I want the first useful commands to print real ship
computer information into the terminal, so that the drawer replaces the old
objectives/log panels without losing their value.

## Steps

- [x] Implement `log` as terminal output over the existing drawer Flight Log /
      `StoryFeed` and objective-event data, preserving useful ordering.
- [x] Implement `objectives` as terminal output over active `GameObjectives`.
- [x] Implement read-only `ship` as terminal output over available player ship
      section/status data, including weapons, thrusters and critical/neutralized
      state where available.
- [x] Add empty-state output for each command so a quiet scenario still returns
      useful text.
- [x] Add tests that commands read current data, update after resource changes,
      and do not duplicate stale rows across scenario/player teardown.
- [x] Update player-facing docs surfaces that describe the drawer after the
      implementation changes land.
- [x] Add/update `tasks/20260726-115330/NOTES.md` with data-source choices,
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
  `nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`)

## Notes

- Depends on: `20260726-134738` (and transitively `20260726-115324`).
- Epic: `tasks/20260725-104330/TASK.md`.
- Spike: `tasks/20260725-104330/SPIKE.md`.
- Do not add `reload` or `repair` here; this task is read-only command output.

## Work Record

- Added `log`, `objectives` and read-only `ship` to the NOVA OS terminal
  registry in `crates/nova_gameplay/src/hud/drawer.rs`.
- `log` prints the existing `DrawerFlightLog` rows, which are still synced from
  `StoryFeed` and objective post/completion changes.
- `objectives` prints the live `GameObjectives` resource and has a readable
  empty state.
- `ship` prints a live player-ship snapshot from the current
  `PlayerSpaceshipMarker` root and direct `SectionMarker` children, including
  section kind, HP, ammo where present, critical state and neutralized state.
- Player teardown now resets both `DrawerFlightLog` and `NovaOsTerminal`, so
  printed command output does not leak into the next scenario/player run.
- Updated `CHANGELOG.md`, `web/src/wiki/hud.md` and
  `web/src/wiki/keybinds.md` after the doc sweep found future-only drawer
  command wording.
- Verification:
  `nix develop --command bash -lc 'cargo test -p nova_gameplay drawer && cargo test -p nova_gameplay terminal'`;
  `nix develop --command cargo check`;
  `nix develop --command cargo fmt --check`;
  `npm ci && npm run ci` in `web/`.
- The first attempted DoD command,
  `nix develop --command cargo test -p nova_gameplay drawer terminal`, failed
  because Cargo accepts only one test filter before `--`; the task proof was
  corrected to run drawer and terminal filters through `bash -lc`.
- `npm ci` reported existing audit vulnerabilities unrelated to this change.
- Review feedback added live-path coverage:
  `terminal_objectives_command_reads_live_resource_updates` and
  `terminal_ship_command_reads_live_player_sections` submit through
  `handle_terminal_keyboard` instead of only formatting prebuilt snapshots.
