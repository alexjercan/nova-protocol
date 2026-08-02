# Unify NOVA OS commands: one TerminalCommand model (app-launch + CLI subcommands)

- PRIORITY: 47
- TAGS: v0.9.0, refactoring, ui, hud, nova_os
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

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

- [x] Write DECISION.md recording Shape A (pure terminal + unified descriptor
      registry + keyed snapshot; nested subcommands; app UI stays in
      NovaOsAppRuntime referenced by id) and why (preserves the pure,
      unit-tested terminal and the crate boundary; smallest blast radius).
- [x] Introduce the unified `TerminalCommand` descriptor + `CommandAction` /
      `CliOutput` enums in nova_os (shell.rs or a new command.rs), with
      recursive `subcommands`.
- [x] Add `NovaOsCommandRegistry` (Bevy resource) with a registration API for
      command trees; seed the core builtins (help, log, objectives, ship,
      clear, version, exit). Remove the standalone `TERMINAL_COMMANDS` table
      and the flat `"map view"` string (absorbed into the map command tree).
- [x] Rework `resolve_command` and helpers (`subcommands_of`, `command_meta`,
      `terminal_command_names`/`specs`, did-you-mean) to operate over the
      unified command tree; keep longest-prefix, arity, and the universal
      help/version sub-verbs.
- [x] Mirror the unified command tree into `NovaOsTerminal` (replace the
      `app_commands` mirror + `set_app_commands`); keep the terminal pure.
- [x] Rekey `TerminalCommandSnapshot` output by command key; replace the
      per-command `match name` in `submit` with generic action dispatch
      (LaunchApp -> App mode; Cli(Snapshot) -> lookup+extend; Cli(Help/Version)
      -> in-terminal rows; Clear; Exit).
- [x] In nova_gameplay: register the `map` command tree + MapApp from
      nova_os_map.rs (co-located); update the snapshot builder in nova_os.rs to
      fill the keyed output map (log/objectives/ship + map view via
      terminal_map_rows); update `sync_nova_os_app_commands`/app-UI wiring to
      the unified registry mirror.
- [x] Update the shell.rs and terminal.rs unit tests to the unified model
      (resolve over the tree: map bare = LaunchApp, map view = Cli subcommand,
      longest-prefix, arity, help/version sub-verbs, did-you-mean; submit
      dispatches each action against a snapshot).
- [x] Verify: `cargo check`/`fmt` clean across the workspace; run the newly
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

## Work Notes (close-out)

### What changed and why

Implemented Shape A from DECISION.md. One command concept now spans the whole
surface:

- New `crates/nova_os/src/command.rs`: the authoring `TerminalCommand` (name,
  summary, arity, `CommandBody` = `Cli(CliOutput)` | `App(Box<dyn
  NovaOsAppRuntime>)`, nested `subcommands`), the `NovaOsCommandRegistry` bevy
  resource (Default seeds the 7 core builtins; plugins `register` trees), the
  flatten into `TerminalCommandSpec`, and `nova_os_footer_hints` (moved here from
  app.rs).
- `shell.rs`: dropped the `&'static TERMINAL_COMMANDS` table and the flat
  `"map view"` string. The matcher now takes a `&[TerminalCommandSpec]` and
  returns `ResolvedCommand::Run { name, dispatch }` (plus Usage/Version/errors);
  the `App`/`Builtin` split is gone. `CliOutput` / `CommandDispatch` /
  `TerminalCommandSpec` live here.
- `terminal.rs`: `NovaOsTerminal` holds `commands: Vec<TerminalCommandSpec>`
  (seeded from `core_command_specs()`), `set_commands`/`command_specs` replace
  `set_app_commands`/`app_commands`. `submit`'s per-command `match name` is
  DELETED - dispatch is generic over `CommandDispatch` (App -> app mode;
  `Cli(Snapshot)` -> `snapshot.output(name)`; `Cli(Help/Version/Clear/Exit)` ->
  the in-terminal action). `TerminalCommandSnapshot` is now
  `command_output: HashMap<&'static str, Vec<TerminalRow>>` keyed by command name
  (+ `with_output` builder), replacing the four typed row fields.
- `app.rs`: trimmed to the `NovaOsAppRuntime` trait (dropped `summary`/`arity` -
  the command descriptor owns those), `NovaOsAppInputOutcome` and the hints
  const. `NovaOsAppCommand` and `NovaOsAppRegistry` removed.
- `nova_gameplay`: the `map` tree (launch word `map` + `map view` subcommand +
  `MapApp` runtime) is now registered ONCE from `nova_os_map.rs`. `nova_os.rs`
  inits `NovaOsCommandRegistry`, `sync_nova_os_commands` mirrors its specs into
  the terminal, the snapshot builder fills the keyed output map, and the app-UI /
  keyboard / footer systems look runtimes up via `registry.app_runtime(id)`.

Subcommands carry their FULL name (`"map view"`), so the existing longest-prefix
matcher is untouched and every id stays `&'static str` (required by
`TerminalMode::App { id: &'static str }`). Considered Shape B (behavior-trait
commands with ECS dispatch) and rejected it at the plan gate - it would have
dropped the pure `submit`/snapshot model and its unit tests.

### Difficulties

- `map view` help rows also have `map ` as a prefix; the help-listing test's
  first-prefix match picked `map` for the `map view` row. Fixed by selecting the
  LONGEST matching name.
- The did-you-mean test typo `mep` was within Levenshtein distance 2 of `help`
  (delete `l`, sub `h`->`m`), so the "no near core command" assertion failed.
  Switched to `nap` (distance 1 of `map`, > 2 of every core command).

### Verification

- `cargo test -p nova_os`: 16/16 pass (shell matcher + command registry +
  terminal submit/completion/help).
- Targeted `nova_gameplay` tests (app launch/exit/close/bleed/footer-hints,
  live-ECS `objectives`/`ship` command tests, `map_view_rows`, map scene
  activation, app-UI chrome): 16/16 pass across two runs.
- `cargo check --workspace --all-targets` and `cargo fmt --check`: clean.
- Headless smoke (`BCS_AUTOPILOT=1` `screenshot_nova_os` under Xvfb): reached
  Playing and exited `AppExit::Success` with no panics - the autopilot script
  runs `help`, `ship`, the `lo`->`log` ghost and the `map` app launch through the
  new unified dispatch.
- DoD #1/#2 absence greps return nothing; every remaining mention of the removed
  symbols is in the immutable `tasks/` history.

DoD #7 (in-game visual pass of every command) is a manual proof: PENDING owner
confirmation. The behavior is covered by automated tests and the smoke run, but a
human eyeball of the live CRT is left for acceptance.

### Self-reflection

The keep-full-`&'static`-names decision was the key simplification - it let the
matcher and all its tests stay byte-for-byte behavioral while only the data
source changed. Next time, when writing did-you-mean / longest-prefix test
fixtures, pick the typo/collision cases by actually computing distances rather
than eyeballing, to avoid the two self-inflicted test failures here.
