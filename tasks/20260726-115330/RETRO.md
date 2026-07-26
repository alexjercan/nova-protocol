# Retro: NOVA OS terminal output commands

- TASK: 20260726-115330
- DATE: 2026-07-26

## What went well

- The implementation kept the new commands read-only and reused the same
  sources that already back the drawer: `DrawerFlightLog`, `GameObjectives`
  and live player ship section components.
- The command output is generated at submit time, so `log`, `objectives` and
  `ship` do not need permanent panes or cached UI state beyond scrollback.
- Teardown now resets both the flight log and terminal session, which matches
  the player-facing "new run, fresh computer" behavior and gives the tests a
  concrete stale-state guard.

## What went wrong

- The first focused DoD command was invalid: `cargo test -p nova_gameplay
  drawer terminal` passes two filters to Cargo. The proof had to be rewritten
  as two cargo invocations under `bash -lc`.
- The first test pass proved the formatting functions more strongly than the
  live ECS path. Review correctly caught that prebuilt snapshots did not prove
  commands read current resources/components through the keyboard submission
  system.
- A doc sweep was initially run with double-quoted backtick patterns, which
  triggered shell command substitution. It was rerun with single quotes before
  using the result.

## What to do differently

- For terminal/output commands that promise current game state, include at
  least one App-driven submit test that mutates the underlying resource or
  component and proves the next command reflects the mutation.
- When converting a human-readable DoD command into an executable command,
  run it exactly once early enough that syntax mistakes are found before the
  closeout stage.
- Keep shell search patterns with backticks or `$` in single quotes by default.
