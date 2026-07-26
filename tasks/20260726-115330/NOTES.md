# Notes: NOVA OS terminal output commands

- TASK: 20260726-115330
- BRANCH: feature/nova-os-terminal-output-commands

## Data Sources

- `log` reads `DrawerFlightLog`, the same backing stream the removed permanent
  Flight Log pane used. That keeps comms rows from `StoryFeed` interleaved with
  objective post/completion rows in observed order, and preserves the existing
  objective update behavior that edits an active posted row instead of appending
  duplicates.
- `objectives` reads the live `GameObjectives` resource at submit time. The
  command is a snapshot, not a subscribed app view, which matches the terminal
  output model: run the command again to print current state.
- `ship` reads the current player ship root with `PlayerSpaceshipMarker` plus
  direct `SectionMarker` children. It uses `SectionDamageClass` as the primary
  section kind source, with section kind markers as a fallback, and prints
  `Health`, `SectionAmmo`, `SectionInactiveMarker` and
  `HealthZeroMarker` where present.

## Reset And Teardown

`remove_drawer` now resets both `DrawerFlightLog` and `NovaOsTerminal` when the
player ship marker is removed. Clearing only the data source was not enough once
command results were printed into scrollback: old rows would remain visible even
if the next run had an empty source. The reset returns the terminal to the boot
welcome block and clears prompt/history/mode.

## Tradeoffs

- Command output is generated from a snapshot passed into `NovaOsTerminal::submit`
  instead of letting the terminal resource query the world itself. This keeps
  the terminal model mostly pure while the Bevy system owns ECS reads.
- `ship viewer` is explicitly left unknown even though `ship` is now a real
  command. That preserves the planned app command for a later task instead of
  reporting it as accidental arguments to `ship`.
- The `ship` command does not walk nested entities under weapon sections. The
  current production section entities are direct children of the ship root, and
  the task asked for available section/status data rather than the later GUI
  viewer's full hierarchy.

## Difficulties

- The task's original proof command used two Cargo test filters:
  `cargo test -p nova_gameplay drawer terminal`. Cargo rejects the second filter,
  so the DoD was corrected to run drawer and terminal filters as two commands.
- The first implementation returned a borrowed ship name from a query helper,
  which crossed ECS query lifetimes. The snapshot now owns the ship-name string.
- The initial parser treated `ship viewer` as unexpected arguments to `ship`;
  the registry test caught that, and `ship viewer` is now an explicit deferred
  unknown command.

## Self-Reflection

- The existing tests around deferred commands were useful, but they needed to be
  updated at the same time as the registry change. For future terminal command
  tasks, start by splitting "implemented one-word command" from "deferred
  multi-word app command" in the parser tests before adding output behavior.
