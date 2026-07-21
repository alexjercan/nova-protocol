# Review: fix cargo-run-launches-probe + ambiguous-glob-reexport regressions

- TASK: 20260721-151934
- BRANCH: fix/cargo-run-and-glob

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (a small, mechanical, fully-verified regression revert -
  no design judgment to blind-check; both DoD proofs re-run with real output.
  Substantive-diff out-of-context default waived per the recorded exception)

The diff is a 2-line functional change (remove the redundant `default-members`
block; drop one name from a module prelude) + 2 doc-row updates + comments. Both
regressions were introduced THIS session (default-members in 6f41f47a; the
HudReadoutFormat mirror enum in c6e2138c) and both fixes were verified by running
the exact failing commands:

- `cargo run -- --help` -> `Running target/debug/nova-protocol --help` printing
  the GAME's clap help ("Simple spaceship editor scene..."), NOT probe's usage.
  Bare `cargo run` targets the game again (DoD 1).
- `cargo build -p nova_core` -> no `ambiguous glob re-exports` /
  `warning: nova_core (lib) generated 1 warning` (DoD 2).
- No stale `default-members` doc claim remains (AGENTS.md + README.md rows
  reworded; the removed key is documented in the Cargo.toml comment as an
  anti-regression note).

Rationale for the fixes over alternatives: `default-members` was redundant for
its stated goal (meta_gen is not a game dependency, so bare `cargo build` of the
root package never compiled it) and its own retro flagged it as a footgun with a
drop-it escape hatch; removing it is that escape hatch. Dropping the gameplay
`HudReadoutFormat` from its prelude (rather than renaming) is minimal and keeps
the authoring surface (nova_scenario's enum) canonical.

No open findings.
