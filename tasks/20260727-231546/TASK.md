# Unify NOVA OS commands: one TerminalCommand model (app-launch + CLI subcommands)

- STATUS: OPEN
- PRIORITY: 47
- TAGS: v0.9.0,refactoring,ui,hud,nova_os

Unify the NOVA OS command surface so there is ONE command concept - a
`TerminalCommand` - instead of today's split between the static
`TERMINAL_COMMANDS` builtin table (nova_os/src/shell.rs), the separate
`NovaOsAppCommand`/`NovaOsAppRegistry` app-launch surface (nova_os/src/app.rs),
and the hardcoded `match name { "log" => ... }` dispatch inside
`NovaOsTerminal::submit` (nova_os/src/terminal.rs). A command may launch an app
when run bare (`map`), and may own CLI subcommands that print to the terminal
(`map view`). Builtins (help/log/objectives/ship/version/clear/exit) are just
`TerminalCommand`s whose action prints to the CLI.

## Problem

The NOVA OS command surface is split across three places that must agree:

1. `shell.rs` - a `&'static` `TERMINAL_COMMANDS` table of builtins, plus the
   flat two-word string `"map view"` sitting next to unrelated builtins.
2. `app.rs` - a parallel `NovaOsAppRegistry` / `NovaOsAppCommand` surface for
   app launch words (`map`), mirrored into the terminal separately.
3. `terminal.rs::submit` - a hardcoded `match name` that maps each builtin
   string to its behavior (append `snapshot.log_rows`, enter App mode, etc).

So the `map` app and its `map view` CLI counterpart are defined in three
different files with no structural link; adding a new app (e.g. the planned
ship viewer) means editing all three. The goal is a single unified model where
`map`'s launch word, its `view` subcommand, and its app runtime are declared
together, and `submit` dispatches generically off the resolved command's action
with no per-command arm.

## Target model (confirmed - Shape A: pure terminal + keyed snapshot)

Decision recorded in DECISION.md. The terminal stays PURE (no ECS): it keeps
receiving pre-built output via a snapshot filled by `nova_gameplay`.

- `TerminalCommand` becomes a single descriptor type (in nova_os):
  `name`, `summary`, `arity`, `action`, and recursive `subcommands`.
- `CommandAction` enum: `LaunchApp { app_id }` | `Cli(CliOutput)` | `Clear`
  | `Exit`. `CliOutput` = `Snapshot(key)` (rows looked up in the snapshot by a
  stable key such as `"log"`, `"ship"`, `"map view"`) | `Help` | `Version`
  (produced in-terminal, as today, so the snapshot need not carry them).
- ONE registry (`NovaOsCommandRegistry`, a Bevy resource) holds the command
  tree. Core builtins are seeded by nova_os. App-bearing command trees (`map`
  with its `view` subcommand + `LaunchApp{app_id:"map"}`) are registered by the
  owning gameplay plugin (`NovaOsMapPlugin`), co-located with the app.
- `NovaOsAppRuntime` (spawn_body/handle_key/hints, the UI) STAYS as-is,
  referenced by `app_id` from the command's `LaunchApp` action.
- The terminal mirrors the whole command tree in (replacing today's
  `app_commands` mirror) for parsing/dispatch, staying pure.
- `TerminalCommandSnapshot` output becomes keyed by command path
  (e.g. a `HashMap<&'static str, Vec<TerminalRow>>` or equivalent) filled by
  `nova_gameplay`; `submit` dispatches `Cli(Snapshot(key))` by lookup.
- `submit`'s per-command `match name` is DELETED and replaced by a generic
  dispatch over the resolved command's `action`.
- The matcher (`resolve_command`, longest-prefix, arity, `<cmd> help`/`-h`,
  `<cmd> version`/`-v`, did-you-mean, `subcommands_of`) operates over the
  unified tree. Existing behavior is preserved exactly.

## Behavior parity (must not regress)

- `help` lists every command (builtins + apps) as today.
- `map` (bare) launches the map app; `map view` prints the local-space
  contacts; both defined in nova_os_map.rs.
- `log` / `objectives` / `ship` print their snapshot rows; `clear` resets to
  welcome; `version` prints the banner; `exit` requests the animated close.
- `map v` -> "unknown sub-command" + usage naming `map view`; `ship help`,
  `map -h` -> that command's usage; did-you-mean on a typo of any command or
  app word; arity rejection unchanged.

## Steps

- [ ] Write DECISION.md recording Shape A (pure terminal + unified descriptor
      registry + keyed snapshot; nested subcommands; app UI stays in
      NovaOsAppRuntime referenced by id) and why (preserves the pure,
      unit-tested terminal and the crate boundary; smallest blast radius).
- [ ] Introduce the unified `TerminalCommand` descriptor + `CommandAction` /
      `CliOutput` enums in nova_os (shell.rs or a new command.rs), with
      recursive `subcommands`.
- [ ] Add `NovaOsCommandRegistry` (Bevy resource) with a registration API for
      command trees; seed the core builtins (help, log, objectives, ship,
      clear, version, exit). Remove the standalone `TERMINAL_COMMANDS` table
      and the flat `"map view"` string (absorbed into the map command tree).
- [ ] Rework `resolve_command` and helpers (`subcommands_of`, `command_meta`,
      `terminal_command_names`/`specs`, did-you-mean) to operate over the
      unified command tree; keep longest-prefix, arity, and the universal
      help/version sub-verbs.
- [ ] Mirror the unified command tree into `NovaOsTerminal` (replace the
      `app_commands` mirror + `set_app_commands`); keep the terminal pure.
- [ ] Rekey `TerminalCommandSnapshot` output by command key; replace the
      per-command `match name` in `submit` with generic action dispatch
      (LaunchApp -> App mode; Cli(Snapshot) -> lookup+extend; Cli(Help/Version)
      -> in-terminal rows; Clear; Exit).
- [ ] In nova_gameplay: register the `map` command tree + MapApp from
      nova_os_map.rs (co-located); update the snapshot builder in nova_os.rs to
      fill the keyed output map (log/objectives/ship + map view via
      terminal_map_rows); update `sync_nova_os_app_commands`/app-UI wiring to
      the unified registry mirror.
- [ ] Update the shell.rs and terminal.rs unit tests to the unified model
      (resolve over the tree: map bare = LaunchApp, map view = Cli subcommand,
      longest-prefix, arity, help/version sub-verbs, did-you-mean; submit
      dispatches each action against a snapshot).
- [ ] Verify: `cargo check`/`fmt` clean across the workspace; run the newly
      written/updated tests; manual in-game pass of every command.

## Definition of Done

1. `submit` in nova_os/src/terminal.rs contains NO per-command name arms
   (no `"log" =>`, `"ship" =>`, `"map view" =>`, etc); dispatch is generic over
   `CommandAction`. (cmd: `! grep -nE '"(log|ship|objectives|map view|clear|version|exit)"\s*=>' crates/nova_os/src/terminal.rs` returns nothing)
2. The standalone `TERMINAL_COMMANDS` static builtin table and the separate
   flat `"map view"` builtin string are gone, replaced by the single
   registry-backed command tree. (cmd: `! grep -n 'TERMINAL_COMMANDS' crates/nova_os/src/shell.rs`)
3. The `map` launch word, its `view` CLI subcommand, and the MapApp runtime are
   all registered from crates/nova_gameplay/src/hud/nova_os_map.rs.
   (test: a unit/integration test asserts the map command tree resolves `map`
   to LaunchApp and `map view` to the CLI subcommand)
4. Unified-matcher unit tests pass in shell.rs: bare `map` = app launch, `map
   view` = CLI subcommand via longest-prefix; arity, `<cmd> help`/`-h`,
   `<cmd> version`/`-v`, did-you-mean, and `map v` -> unknown-sub-command all
   behave as before. (test: `cargo test -p nova_os`)
5. `submit` dispatch tests in terminal.rs: each command's action produces the
   right effect against a snapshot (log/objectives/ship/map view print keyed
   rows; map enters App mode; clear/version/help/exit). (test: `cargo test -p nova_os`)
6. Workspace builds and formats clean. (cmd: `cargo check --workspace` and
   `cargo fmt --check`)
7. manual: in-game NOVA OS - help, log, objectives, ship, map, map view, clear,
   version, exit, a typo (did-you-mean), `map v`, and `ship help` all behave
   exactly as before the refactor.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
