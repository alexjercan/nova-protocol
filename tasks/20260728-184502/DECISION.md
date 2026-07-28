# DECISION: how usage lines name a command's argument

- STATUS: ACCEPTED
- DATE: 2026-07-28 (owner approved at the plan gate)

## Context

To make `<cmd> help` and the wrong-argument error read like a shell, the usage
line must name the real argument, e.g.

```
Usage: map goto <label>
Usage: ship reload <section>
```

Today the command model (`TerminalCommand` / `TerminalCommandSpec` in
`crates/nova_os/src/{command,shell}.rs`) carries only a `CommandArity`
(`None` / `UpTo(n)`) - a COUNT, not a name. `command_help_rows` therefore emits
a generic `usage: <name> <arg>`. There is no place to hang the word `label` or
`section`.

## Options

1. Keep the generic `<arg>` placeholder. Zero model change, but the usage lines
   stay unlike a real shell (`<arg>` reads like a stub) - fails the goal.
2. Derive a placeholder from the arity number alone (`<arg1>`, `<arg2>`).
   Still nameless; no better than option 1 for a 1-arg command.
3. Add an optional argument-placeholder string to the command model
   (`arg_hint: Option<&'static str>`), set at the registration site
   (`.with_arg_hint("<label>")`), threaded through the flattened spec, and
   rendered verbatim in the usage line. Falls back to `<arg>` when a
   command declares an arity but no hint, so nothing regresses.

## Decision (proposed)

Option 3. It is the minimal, maintainable addition that lets each command
declare a meaningful argument name next to its arity, keeping the authoring
form as the single source of truth (consistent with how `summary` already
lives on the command). The arg-bearing commands set their hints at their
registration sites:

- `map goto`     -> `<label>`   (`crates/nova_gameplay/src/hud/nova_os_map.rs`)
- `ship section` -> `<section>` (`crates/nova_gameplay/src/hud/nova_os_ship.rs`)
- `ship reload`  -> `<section>`
- `ship repair`  -> `<section>`

No-argument commands carry no hint and render a bare `Usage: <name>`.
