# Scaffold the standalone nova_autopilot crate

- STATUS: OPEN
- PRIORITY: 99
- TAGS: v0.10.0,tooling,autopilot,crates
- KIND: TASK
- FLOW STEP: BACKLOG
- PLAN STATUS: DRAFT
- PARENT: 20260802-120019

## Story

Create an empty, standalone `crates/nova_autopilot` so later ports have a home.
The crate depends on `bevy` only: no `nova_*` crate, no `bevy_common_systems`,
no `avian3d`. Nova-shaped choices (env names, defaults, API vocabulary) live
here; Nova TYPES do not. The state-machine generic `S: States +
FreelyMutableState` is what keeps the crate free of `nova_gameplay::GameStates`,
so it stays.

## Steps

- [ ] Add `crates/nova_autopilot` with a `bevy`-only manifest, register it in
      the workspace members, and give it crate docs stating the ownership
      boundary (driver + completion here; Nova adapters in `nova_debug`).
- [ ] Add the empty module skeleton (`completion`, `autopilot`, `screenshot`,
      `reel`, `prelude`) with module docs and `#![warn(missing_docs)]`.

## Definition of Done

- The crate builds in the workspace.
  (cmd: `nix develop --command cargo check -p nova_autopilot`)
- The dependency list names no Nova or third-party game crate.
  (cmd: `! rg -n "nova_|bevy_common_systems|avian3d" crates/nova_autopilot/Cargo.toml`)
- Rustdoc is clean and the crate docs state the boundary.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Notes

- Parent: `20260802-120019`. Nothing is ported in this task; it only lands the
  shell so the port tasks stay small.
- Source to port from: `/home/alex/personal/bevy-common-systems/src/debug/harness/`
  and `src/completion.rs`, plus `crates/nova_debug/src/harness.rs`.
