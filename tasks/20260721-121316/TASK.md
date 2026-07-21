# Full missing_docs rollout on nova_scenario + nova_gameplay (non-category tail)

- STATUS: OPEN
- PRIORITY: 36
- TAGS: docs,v0.8.0

## Story

The breadth rustdoc pass (task 20260525-133032) documented every public
plugin/component/resource/event TYPE across the workspace and flipped
`#![warn(missing_docs)]` on every small/medium crate that came fully clean
(nova_core, nova_debug, nova_menu, nova_ui, nova_modding, nova_info,
nova_editor, nova_meta_gen, nova_events, nova_assets, nova_probe).

The two LARGE crates were left with only their CATEGORY types documented; the
non-category public tail (free functions, config sub-structs that are not
Component/Resource, type aliases, enum variants, associated fns) is still
undocumented, so the enforcement lint is still OFF there. This task finishes the
full `missing_docs` rollout on those two crates so the lint can go on.

Remaining non-category missing_docs tail (measured 2026-07-21 with
`RUSTFLAGS="--force-warn missing_docs" cargo build --workspace`, counting each
crate's own `src/` lines):

- nova_scenario: 233 (heaviest: actions.rs, variables.rs, objects/asteroid.rs)
- nova_gameplay: 144

## Steps

- [ ] Sweep nova_scenario's remaining undocumented public items; one-line
      `///` each per the AGENTS.md rustdoc conventions (what/why, RON-surface
      configs agree with the wiki field tables). Then add
      `#![warn(missing_docs)]` and confirm it builds clean.
- [ ] Same for nova_gameplay (its category surface + module headers are already
      done from tasks 20260525-133030/133032; only the non-category tail
      remains). Flip the lint once clean.
- [ ] Verify `cargo doc --workspace --no-deps` stays warning-free and
      `cargo build --workspace` emits zero missing_docs from the two crates.

## Definition of Done

- nova_scenario and nova_gameplay are fully documented (0 missing_docs) with
  `#![warn(missing_docs)]` enabled, so the whole workspace is lint-clean.

## Notes

- Large but mechanical; fine to split across sessions per-file. Skip local
  full test/clippy per repo policy; CI covers them.
