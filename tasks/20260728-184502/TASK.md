# NOVA OS shell: shell-like help + wrong-command usage messages

- PRIORITY: 20
- TAGS: v0.9.0, nova_os, ui, ux, feedback
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Problem

The NOVA OS terminal already resolves commands, suggests typos and prints a
`help` list, but the help and error output does not read like a real shell.
The player asked for the `help` / wrong-command usage to look more like shell
tools (reference: `tatr help`). Gaps against that reference:

- The `help` list has no `Usage:` synopsis line and no grammar for how a
  command is invoked; it opens straight into `Available commands:`.
- Per-command help (`<cmd> help`) prints a lowercase `usage: <name>` and, for
  argument-taking commands, a generic `usage: <name> <arg>` - it never names
  the real argument (`map goto <label>`, `ship reload <section>`), because the
  command model only carries an arity count, not an argument name.
- The unknown-command path prints `command not found: X` (+ optional
  did-you-mean) but never points the player at `help` the way a shell points
  at its usage.
- The wrong-argument path prints a bare `help takes no arguments` /
  `map: unknown sub-command` line whose casing and shape do not match the
  `Usage:`-led shell convention.

Reference shell shape (`tatr help` and `tatr <bad>`):

```
Usage: tatr [-r ROOT] <subcommand> [options]

Subcommands:
  help         Show this help message
  ...
```
```
Unknown subcommand: notacommand
Usage: tatr [-r ROOT] <subcommand> [options]
...
```

Scope is the NOVA OS shell/terminal in `crates/nova_os` (help + error
rendering in `terminal.rs`, the command model in `command.rs`/`shell.rs`). No
gameplay behavior changes; this is UX/wording plus a small model addition (a
per-command argument placeholder) so usage lines can be meaningful.

## Definition of Done

All proofs are pure-terminal tests in `crates/nova_os` (the `terminal.rs`
test module) plus the compile of the registration sites; no ECS/gameplay
behavior changes.

1. `help` output opens with a shell synopsis and a sectioned, aligned command
   list, and ends with a pointer to per-command help.
   (test: a `nova_os` test asserts the first row is `Usage: <command>
   [arguments]`, a `Commands:` header row follows, and the last row is
   `Type '<command> help' for details.`)
2. Per-command help names the real argument in a capital-`Usage:` line.
   (test: `map goto help` renders `Usage: map goto <label>`; `ship reload help`
   renders `Usage: ship reload <section>`; `help help` renders `Usage: help`
   with no arg.)
3. A command that owns subcommands shows a `Subcommands:` section listing each
   child with its summary, aligned.
   (test: `map help` renders `Usage: map [subcommand]`, a `Subcommands:` header,
   and aligned `map view` / `map goto` rows carrying their summaries.)
4. An unknown command prints a shell-style not-found line and points at `help`;
   a near-typo still yields the did-you-mean row.
   (test: `xyzzy` -> `command not found: xyzzy` + `Type 'help' for a list of
   commands.`; `hlep` -> also `did you mean help?`.)
5. A wrong-argument command prints a `command: reason`-style error naming the
   offending input, then the command's usage block.
   (test: `help garbage` -> `help: takes no arguments` then the `help` usage
   rows; `map v` -> `map: unknown subcommand 'v'` then the `map` usage rows.)
6. The argument placeholder lives on the command model and is set at each
   arg-bearing registration site (per DECISION.md), falling back to `<arg>`
   when an arity-carrying command declares no hint.
   (cmd: `cargo check -p nova_os -p nova_gameplay` compiles the new
   `.with_arg_hint(..)` calls; test: an arity-`UpTo` spec with no hint still
   renders `Usage: <name> <arg>`.)
7. Every existing `nova_os`/`nova_gameplay` test and doc string asserting the
   OLD help/error wording is updated to the new shell-like wording; no stale
   copy of the retired strings remains in the crate's live source.
   (cmd: `rg -n "Available commands:|takes no arguments\"|unknown sub-command|subcommands: " crates/nova_os crates/nova_gameplay` returns only intended new-format lines - the `tasks/` history tree excluded.)

## Steps

- [x] Add `arg_hint: Option<&'static str>` to `TerminalCommand`
      (`command.rs`) and `TerminalCommandSpec` (`shell.rs`); default `None`,
      thread it through `flatten_into`. Add a `.with_arg_hint(&'static str)`
      builder on `TerminalCommand`. (DECISION.md option 3.)
- [x] Set the hints at the registration sites: `map goto` -> `<label>`
      (`nova_os_map.rs`); `ship section`/`ship reload`/`ship repair` ->
      `<section>` (`nova_os_ship.rs`).
- [x] Extend `command_meta` (or add a lookup) so the renderer can read a
      command's `arg_hint`; extend `ResolvedCommand::UnexpectedArguments` to
      carry the trailing arg words so the error can name the offending input.
- [x] Rewrite `terminal_help_rows` (`terminal.rs`) to emit: `Usage: <command>
      [arguments]`, a `Commands:` header, the aligned name/summary rows, and a
      trailing `Type '<command> help' for details.` hint. (write the test
      first, watch it fail.)
- [x] Rewrite `command_help_rows` to emit the `{name} - {summary}` title, a
      capital `Usage:` line using `arg_hint` (`[subcommand]` when the command
      owns subcommands, `<hint>`/`<arg>` for an arg command, bare for none),
      and an aligned `Subcommands:` section (name + summary) when present; drop
      the redundant version footer. (test-first.)
- [x] Rewrite the `Unknown` render to append `Type 'help' for a list of
      commands.` after the `command not found` / did-you-mean rows. (test-first.)
- [x] Rewrite the `UnexpectedArguments` render to a `command: reason` line -
      `command: takes no arguments` with no subcommands, `command: unknown
      subcommand '<word>'` when the command has subcommands - then the usage
      block. (test-first.)
- [x] Update every existing `nova_os`/`nova_gameplay` test asserting the old
      strings; run `cargo fmt`, `cargo check -p nova_os -p nova_gameplay`, and
      the new `nova_os` tests. Run the DoD-7 `rg` sweep.
