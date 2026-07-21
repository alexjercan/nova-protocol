# Fix two v0.8.0 regressions: cargo run launches probe (default-members) + ambiguous glob re-export (HudReadoutFormat)

- STATUS: CLOSED
- PRIORITY: 92
- TAGS: v0.8.0,bug,tooling

## Story

Two regressions from the v0.8.0 tooling/feature session surfaced when the user
ran the game locally:

1. `cargo run` (the documented "play the game" command) launched the `probe`
   binary instead of the game. Cause: the `default-members` list added in the
   nova_meta_gen relocation (6f41f47a) changed bare `cargo run`'s package
   selection from the root `nova-protocol` package to the 14-member set, which
   resolves to a nova_probe bin. `default-members` was REDUNDANT for its stated
   goal (skip the web-only meta_gen in bare builds) - meta_gen is not a game
   dependency, so a bare `cargo build`/`run` of the root package never built it
   anyway - and its own retro flagged it as a footgun with a "drop this step"
   escape hatch.
2. `warning: ambiguous glob re-exports` in nova_core: `HudReadoutFormat` is a
   distinct type in BOTH `nova_scenario::prelude` and `nova_gameplay::prelude`
   (the HudReadout feature added a mirror enum on each side), and
   `nova_core::prelude` globs both.

## Steps

- [x] Remove the `default-members` block from the root Cargo.toml (restores
      `cargo run` -> the game; bare builds still skip meta_gen because it is not
      a game dep; CI uses `--workspace`, unaffected).
- [x] Drop the gameplay `HudReadoutFormat` from `crates/nova_gameplay/src/hud/readout.rs`'s
      module prelude re-export (it is only used within readout.rs and referenced
      by full path from nova_scenario's sync) so nova_core's glob has one
      `HudReadoutFormat`, not two.
- [x] Sweep docs for the now-removed `default-members` claim: AGENTS.md +
      README.md meta_gen crate-table rows.
- [x] Verify: `cargo build -p nova_core` is warning-free (glob gone);
      `cargo run -- --help` prints the GAME's clap help (not probe's), proving
      bare `cargo run` targets the game again.

## Definition of Done

- Bare `cargo run` targets the game binary (cmd: `cargo run -- --help` shows the
  game CLI, not probe).
- `cargo build -p nova_core` emits no ambiguous-glob-reexport warning.
- No stale `default-members` doc claim remains.

## Notes

- Regressions from 6f41f47a (default-members) and c6e2138c (HudReadout mirror
  enum). Surfaced by the user's `cargo run --features dev` on 2026-07-21.

## Outcome (2026-07-21): both regressions fixed + verified

1. Removed the `default-members` block from the root Cargo.toml. The root is a
   PACKAGE (nova-protocol, the game), so bare `cargo run`/`build` targets the
   game binary again; meta_gen is still skipped by bare builds (not a game dep);
   CI uses `--workspace` (unaffected). Verified: `cargo run -- --help` now runs
   `target/debug/nova-protocol` and prints the GAME clap help, not probe's usage.
2. Dropped the gameplay `HudReadoutFormat` from readout.rs's module prelude (it
   stays `pub`; nova_scenario's sync references it by full path). Verified:
   `cargo build -p nova_core` no longer emits the ambiguous-glob-reexport warning.

Docs swept: the AGENTS.md + README.md meta_gen crate-table rows no longer claim
`default-members` exclusion.
