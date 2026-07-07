# Retro: test/build peak-memory fix (dev profile debuginfo)

## Context
While running `cargo build --features dev --all-targets` in worktrees, background builds
were being killed. Root cause turned out to be memory, not the harness.

## Diagnosis
The sibling project bevy-common-systems had already hit and documented this exact issue
(`../bevy-common-systems/docs/2026-07-03-test-memory.md`): `cargo test` / `--all-targets`
links one Bevy 0.19 + avian 0.7 binary per target (lib unittest, each of the 6 examples,
doctests). With default embedded DWARF each binary is ~1.5 GB, and cargo links many in
parallel, so `rust-lld` peaked near 40 GB. This box has 31 GB, so it swap-thrashed and
the OOM killer took the build (the `status: killed` notifications).

## Fix
Added to `[profile.dev]` in the root Cargo.toml:

```toml
split-debuginfo = "unpacked"
debug = "line-tables-only"
```

DWARF stays in the per-object `.o` files instead of being copied through the linker, and
only line tables are kept (panic backtraces survive; local-variable debug info is
dropped, which is fine since we debug via bevy-inspector-egui, not gdb/lldb).

## Measured impact
A full `--all-targets` rebuild peaked at **20.2 GB** (sampled summed RSS of the rust
toolchain), exit 0, no swap - down from the ~40 GB that was OOM-killing builds.

## Lessons
- When background builds get "killed" with no compiler error, suspect OOM before blaming
  the harness. Sample toolchain RSS to confirm.
- Sibling projects with the same engine/toolchain are worth checking for retros before
  re-diagnosing - this fix was lifted almost verbatim from bevy-common-systems.
- Worktrees have their own `target/`; always build them with
  `CARGO_TARGET_DIR=<main>/target` so they reuse the warm cache (and so a profile change
  only rebuilds once). A worktree sprouted before a Cargo.toml/profile change must merge
  the base branch in before building, or it builds under the old (heavy) profile.
