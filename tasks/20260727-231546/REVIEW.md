# Review: Unify NOVA OS commands into one TerminalCommand model

- TASK: 20260727-231546
- BRANCH: refactor/nova-os-terminal-command

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings produced by an out-of-context reviewer (fresh agent, no sight
of the implementing session). The in-session pass re-ran the DoD greps (both
empty) and independently re-ran the two load-bearing tests
(`command::tests::registered_app_tree_flattens_and_resolves_its_runtime`,
`shell::tests::nova_os_map_launches_app_and_map_view_prints`) - both pass - before
adopting the round.

What the reviewer verified (not trusting the Work Notes):

- DoD greps both empty: no per-command name arm remains in `submit`
  (`crates/nova_os/src/terminal.rs`); `TERMINAL_COMMANDS` is gone from
  `crates/nova_os/src/shell.rs`.
- `cargo test -p nova_os`: 16 passed, 0 failed. `cargo fmt --check`: clean.
  `cargo check --workspace --all-targets`: clean (only a pre-existing
  `proc-macro-error2` dependency future-incompat warning, unrelated to this diff).
- Matcher preserves longest-prefix, arity, `help`/`-h`, `version`/`-v`,
  did-you-mean; `map` -> `App`, `map view` -> `Cli(Snapshot)` by longest-match.
- `submit` dispatch is generic over `CommandDispatch` with no per-command name
  arm. Snapshot rekeying (`command_output` keyed by command name, `output(name)`
  empty when absent) is correct; keys match command names.
- Registry flatten is depth-first (core first, then registered trees);
  `app_runtime(id)` only returns an `App` body; `with_subcommand` debug-asserts
  the one-word extension.
- Every ticked Step delivered; `TERMINAL_COMMANDS` / `NovaOsAppRegistry` /
  `NovaOsAppCommand` / `terminal_command_specs` removed; `summary()`/`arity()`
  dropped from the runtime trait.
- Live-ECS command tests still route ECS -> keyed snapshot -> `submit`, so they
  would fail if the key lookup regressed. Tests are meaningful, not weakened.
- DECISION.md exists (ACCEPTED) and adequately justifies Shape A vs Shape B.
- Doc sweep: no stale mentions of the removed symbols in README, docs/,
  AGENTS.md or .claude/; remaining references are confined to the immutable
  `tasks/` history.

No BLOCKER/MAJOR/MINOR/NIT findings.

Pending manual acceptance (not resolved by APPROVE): DoD #7 - in-game visual
pass of every NOVA OS command (help, log, objectives, ship, map, map view,
clear, version, exit, a typo, `map v`, `ship help`). Covered by automated tests
and the headless `screenshot_nova_os` smoke run; left for owner eyeball at Finish.
