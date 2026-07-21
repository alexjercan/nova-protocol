# Add inline doc comments to all public plugin structs/components

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: docs, v0.8.0

## Story

As a contributor exploring the workspace in an IDE, I want every public plugin
struct and component to carry a doc comment, so that hover/completion explains
what a type is for at the point of use, across all crates - not just the big
gameplay crate.

This is the breadth pass of the rustdoc strand (20260525-133033 coordinates;
20260525-133030 does nova_gameplay in depth): a sweep across ALL workspace
crates (nova_core, nova_scenario, nova_assets, nova_modding, nova_mod_format,
nova_menu, nova_ui, nova_editor, nova_events, nova_info, nova_debug,
nova_probe, nova_meta_gen, nova_portal_gen) adding at least a one-line doc
comment to every public plugin struct, component, resource and event type.
Retagged from the old backlog where it had no body.

## Steps

- [x] Enumerate the gap: for each workspace crate, list public
      plugins/components/resources/events without doc comments (a
      missing_docs dry run per crate produces the checklist; counts below).
- [x] Sweep crate by crate, smallest first: one-line minimum per item -
      what it marks/carries/configures and who inserts it; units and
      invariants where they exist. Plugins get one extra line: what systems
      they add and in which schedule.
- [x] Follow the conventions from 20260525-133033; where a type is really the
      scenario/modding surface (config structs deserialized from RON), make
      the doc comment agree with the wiki's field tables - the RON field
      names are player-facing API.
- [x] Verify `cargo doc --workspace --no-deps` warning-free; flip
      `#![warn(missing_docs)]` on per crate as each comes clean, per the
      133033 enforcement decision.

## Checklist / results (2026-07-21, branch docs/breadth-rustdoc)

Measured with `RUSTFLAGS="--force-warn missing_docs" cargo build --workspace`,
counting warnings pointing at each crate's own `src/` (all public items, not
just the 4 categories). Category-type audit = undocumented public
Plugin/Component/Resource/Event/Message structs+enums.

| Crate | before (total missing_docs) | after (total) | category types left | lint on? |
|-------|-----------------------------|---------------|---------------------|----------|
| nova_core       | 6   | 0   | 0 | YES |
| nova_debug      | 6   | 0   | 0 | YES |
| nova_menu       | 2   | 0   | 0 | YES |
| nova_ui         | 1   | 0   | 0 | YES |
| nova_modding    | 1   | 0   | 0 | YES |
| nova_info       | 0   | 0   | 0 | YES (was already on) |
| nova_meta_gen   | 3   | 0   | 0 | YES |
| nova_editor     | 1   | 0   | 0 | YES |
| nova_events     | 39  | 0   | 0 | YES |
| nova_assets     | 34  | 0   | 0 | YES |
| nova_probe      | 20  | 0   | 0 | YES |
| nova_scenario   | 252 | 233 | 0 | no (LARGE - category types done; non-category tail 233) |
| nova_gameplay   | 191 | 144 | 0 | no (LARGE - category types done; non-category tail 144) |

DoD bullet 1 MET: 0 undocumented public plugin/component/resource/event types
across the WHOLE workspace (verified by audit script).

DoD bullet 2 MET: every crate that came fully clean has `#![warn(missing_docs)]`
on. nova_core, nova_debug, nova_menu, nova_ui, nova_modding, nova_editor,
nova_meta_gen, nova_events, nova_assets, nova_probe flipped this task;
nova_info was already on.

LARGE-crate tail (non-category public items: fns, config sub-structs, type
aliases, enum variants): nova_scenario 233, nova_gameplay 144. Deferred to
follow-up task (filed this session) for the full missing_docs rollout + lint.

Verify: `cargo build --workspace` exit 0, 0 missing_docs warnings from the
lint-enabled crates. `cargo doc --workspace --no-deps` warning-free (only the
pre-existing `proc-macro-error2` future-incompat dep warning remains).

## Definition of Done

- Zero undocumented public plugin/component/resource/event types across the
  workspace (measured by the same missing_docs dry run that produced the
  checklist).
- Each crate that came clean has the lint on so it stays clean.

## Notes

- Mechanical but large; fine to split per-crate across sessions - keep the
  checklist in this task current so progress is visible.
- Skip local full test/clippy runs per repo policy; CI covers them.

## Close-out (2026-07-21, branch docs/breadth-rustdoc)

Both DoD bullets met and the task is CLOSED:

- Bullet 1 (zero undocumented public plugin/component/resource/event types
  across the workspace): DONE. Verified by an audit script that cross-references
  every `pub struct`/`pub enum` deriving Component/Resource/Event/Message or
  named `*Plugin` against the `--force-warn missing_docs` output - 0 remaining
  workspace-wide (including nova_scenario and nova_gameplay).
- Bullet 2 (each fully-clean crate has the lint): DONE. `#![warn(missing_docs)]`
  is now on nova_core, nova_debug, nova_menu, nova_ui, nova_modding,
  nova_editor, nova_meta_gen, nova_events, nova_assets, nova_probe (and
  nova_info, already on). Each builds with zero missing_docs.

The LARGE-crate non-category tail (nova_scenario 233, nova_gameplay 144 public
fns / config sub-structs / type aliases / enum variants) is genuinely large and
is deliberately left for the filed follow-up task 20260721-121316 ("Full
missing_docs rollout on nova_scenario + nova_gameplay"). The lint stays OFF on
those two crates until that lands.

Reviewer notes: a handful of `*Config` types the enumeration listed as
`[Component]` (e.g. SpaceshipConfig, PlayerControllerConfig, the HUD `*HudConfig`
structs, TorpedoTargetHudConfig) are NOT Components in the code - they are plain
RON/spawn config structs. They were documented truthfully as config structs
rather than asserting a Component role. Two gameplay lines
(TurretSectionBarrelFireState, TorpedoSectionSpawnerFireState) were written from
reading the fire/muzzle systems as per-shot cooldown timers without exhaustively
tracing every reset path - worth a glance. All cargo-doc intra-doc links the
sweep introduced were fixed (submodule-qualified paths in objects/mod.rs;
demoted private `HoloAssets` links to code text; `Self::to_markdown`).
