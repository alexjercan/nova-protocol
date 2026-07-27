# DECISION: unify the NOVA OS command surface into one TerminalCommand model

- STATUS: ACCEPTED
- DATE: 2026-07-27
- TASK: 20260727-231546

## Context

The NOVA OS command surface was split across three places that had to agree:

1. `nova_os/src/shell.rs` - a `&'static TERMINAL_COMMANDS` table of builtins,
   including the flat two-word string `"map view"`.
2. `nova_os/src/app.rs` - a separate `NovaOsAppCommand` / `NovaOsAppRegistry`
   surface for app launch words (`map`), mirrored into the terminal on its own.
3. `nova_os/src/terminal.rs::submit` - a hardcoded `match name { "log" => ... }`
   that mapped each builtin string to its behavior.

So `map` (app) and `map view` (its CLI counterpart) lived in three unrelated
files; adding an app meant editing all three.

## Decision (Shape A - pure terminal + keyed snapshot)

One command concept - `TerminalCommand` - with an app-launching body OR a
CLI-output body, and nested subcommands. Confirmed with the owner at the plan
gate over the alternative (Shape B: behavior-trait commands whose `run` takes an
ECS context, moving dispatch out of the pure terminal).

- `TerminalCommand` (new `command.rs`) is the authoring form: `name`, `summary`,
  `arity`, a `CommandBody` (`Cli(CliOutput)` or `App(Box<dyn NovaOsAppRuntime>)`)
  and nested `subcommands`. Subcommand names are the FULL multi-word name
  (`"map view"`), so the existing longest-prefix matcher is unchanged and every
  name stays `&'static str` (needed by `TerminalMode::App { id: &'static str }`).
- ONE registry, `NovaOsCommandRegistry` (a bevy `Resource`), replaces both
  `TERMINAL_COMMANDS` and `NovaOsAppRegistry`. Its `Default` seeds the core
  builtins (help/log/objectives/ship/clear/version/exit); gameplay plugins
  `register` their command trees (the `map` tree, carrying `MapApp` + the `view`
  subcommand, is registered once from `nova_os_map.rs`).
- The registry flattens to a `Copy` `TerminalCommandSpec { name, summary, arity,
  dispatch }` list. That flat list is what the pure terminal mirrors in and what
  `shell.rs`'s matcher (resolve/help/did-you-mean/subcommands_of) operates on -
  so the terminal stays pure (no ECS) and fully unit-testable.
- `submit` dispatches generically on the resolved command's `CommandDispatch`
  (`App` -> enter app mode; `Cli(Snapshot)` -> extend from the snapshot by the
  command's name key; `Cli(Help|Version|Clear|Exit)` -> the in-terminal action).
  The per-command `match name` is deleted.
- `NovaOsAppRuntime` (the UI seam: `spawn_body`/`handle_key`/`hints`) STAYS,
  owned by the command's `App` body and looked up by id for the app-UI systems.
  Its `summary()`/`arity()` methods are dropped - the command descriptor owns
  those now, killing the old duplication.
- `TerminalCommandSnapshot` output becomes keyed by command name
  (`command_output: HashMap<&'static str, Vec<TerminalRow>>`), filled by
  `nova_gameplay` exactly as before (log/objectives/ship + `map view` rows).

## Why not Shape B

Shape B (each command a trait object whose `run` reaches into the world) is more
"OO" but destroys the pure `submit`/snapshot model and its heavy unit-test
surface, and forces command dispatch across the `nova_os` -> `nova_gameplay`
crate boundary. Shape A keeps the tested pure core, respects the crate split,
and still delivers the exact requested model (bare `map` = app, `map view` = CLI
subcommand, both declared in one place).

## Consequences

- `map view` moves OUT of the core builtin table into the `map` command tree in
  `nova_os_map.rs`; the pure default terminal now carries only the 7 core
  builtins, and `map`/`map view` appear only once the map tree is registered
  (mirrored in-game, injected in the unit tests).
- Test rigs that registered a bare app runtime into `NovaOsAppRegistry` now
  register a `TerminalCommand` (App body) into `NovaOsCommandRegistry`.
