# Scaffold the standalone nova_autopilot crate

- PRIORITY: 99
- TAGS: v0.10.0, tooling, autopilot, crates
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-120019

## Story

Create an empty, standalone `crates/nova_autopilot` so later ports have a home.
The crate depends on `bevy` only: no `nova_*` crate, no `bevy_common_systems`,
no `avian3d`. Nova-shaped choices (env names, defaults, API vocabulary) live
here; Nova TYPES do not. The state-machine generic `S: States +
FreelyMutableState` is what keeps the crate free of `nova_gameplay::GameStates`,
so it stays.

## Steps

- [x] Add `crates/nova_autopilot/Cargo.toml`: `name = "nova_autopilot"`,
      `version`/`edition`/`license` from `workspace = true`, `publish = false`,
      a one-line `description`, `[lints] workspace = true`, and exactly one
      dependency, `bevy = { version = "0.19.0" }` (no features; matches
      `crates/nova_events/Cargo.toml`).
- [x] Register the crate in the root `Cargo.toml` `[workspace] members` list at
      line 303, alphabetically between `crates/nova_assets` and
      `crates/nova_core`. Do NOT add `default-members` (the comment at line 325
      explains why it stays absent).
- [x] Add `crates/nova_autopilot/src/lib.rs` with `#![warn(missing_docs)]` and
      crate docs stating the ownership boundary: the crate owns the automation
      drivers and the completion protocol and depends on `bevy` only; Nova
      adapters (scenario presets, camera posing, rigid-body freezing, overlay
      hiding) stay in `nova_debug` and reach in through caller hooks. Name the
      `S: States + FreelyMutableState` generic as the reason no
      `nova_gameplay::GameStates` appears here.
- [x] Add the empty module files `completion.rs`, `autopilot.rs`,
      `screenshot.rs`, `reel.rs` plus an empty `pub mod prelude` in `lib.rs`,
      each declared `pub mod` in `lib.rs` with a doc comment saying what will
      land there (completion protocol; scripted autopilot driver; settled-frame
      screenshot driver; screenshot reel driver behind caller hooks; the
      glob-import surface). No items, no `use`, no re-exports yet.

## Definition of Done

- The crate builds in the workspace.
  (cmd: `nix develop --command cargo check -p nova_autopilot`)
- The dependency list names no Nova or third-party game crate. The pattern is
  anchored so the crate's own `name = "nova_autopilot"` line does not match,
  and `test -f` keeps a missing manifest from passing vacuously.
  (cmd: `test -f crates/nova_autopilot/Cargo.toml && ! rg -n '^(nova_|bevy_common_systems|avian3d)' crates/nova_autopilot/Cargo.toml`)
- Rustdoc is clean and the crate docs state the boundary.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)
- The workspace still resolves with the new member.
  (cmd: `nix develop --command cargo metadata --format-version 1 --no-deps > /dev/null`)

## Notes

- Parent: `20260802-120019`. Nothing is ported in this task; it only lands the
  shell so the port tasks stay small.
- Source to port from: `/home/alex/personal/bevy-common-systems/src/debug/harness/`
  (`autopilot.rs`, `screenshot.rs`, `mod.rs`) and `src/completion.rs`, plus
  `crates/nova_debug/src/harness.rs` (reel driver, presets).
- Base-red verified 2026-08-02: `cargo check -p nova_autopilot` exits 101
  (`did not match any packages`); the guarded dependency grep exits 1. The
  rustdoc proof fails by the same package-not-found mechanism.
- Proof fix carried in this task: the epic's version of the dependency check
  (`! rg -n "nova_|..."`, unanchored, unguarded) is unsound twice over - it
  matches the crate's own package name, and on a missing file `rg` exits 2 so
  `!` reports success. Epic `20260802-120019` should adopt the guarded,
  anchored form when its own DoD is run.
- No feature flags on the crate: nothing in this task or the port tasks names a
  conditional-compilation requirement, and `nova_debug` is already gated behind
  its own `debug` feature.

## Close-out

**What/why.** Landed `crates/nova_autopilot` as an empty, `bevy`-only crate:
manifest, workspace membership, `lib.rs` with the ownership-boundary docs, and
four empty module files (`completion`, `autopilot`, `screenshot`, `reel`) plus
an empty `prelude`. No behaviour ported; the shell exists so the port tasks stay
small and the boundary is asserted by the dependency proof from day one.

**Alternatives.** Considered declaring the modules with `#[path]`-less stubs
only in `lib.rs` (no files) - rejected: the port tasks want a file to land into,
and per-file doc comments are where the boundary story reads best. Also
considered a `debug` feature flag mirroring `nova_debug`; skipped per the Notes -
nothing in the port set names a conditional-compilation requirement (YAGNI).

**Difficulties.** None. The `version = { workspace = true }` table form (rather
than `version.workspace = true`) was copied from `crates/nova_events/Cargo.toml`
to match the existing house style.

**Evidence.** Base-red confirmed before implementation: `cargo check -p
nova_autopilot` -> `package ID specification 'nova_autopilot' did not match any
packages`. After: all four DoD proofs pass - `cargo check -p nova_autopilot`
finished clean; the guarded anchored dependency grep exits 0; `RUSTDOCFLAGS=-Dwarnings
cargo doc -p nova_autopilot --no-deps` generated docs with no warnings; `cargo
metadata --no-deps` exits 0. `cargo fmt --check` clean. Full suite not run per
the standing local-test policy; CI covers it.

**Reflection.** The guarded/anchored dependency grep is the load-bearing part of
this scaffold - it is what will fail loudly if a port task reaches for a `nova_*`
type instead of a caller hook. Epic `20260802-120019` still carries the unsound
unanchored form and should adopt this one.
