# Add inline doc comments to all public plugin structs/components

- STATUS: OPEN
- PRIORITY: 36
- TAGS: docs,v0.8.0

## Story

As a contributor exploring the workspace in an IDE, I want every public plugin
struct and component to carry a doc comment, so that hover/completion explains
what a type is for at the point of use, across all crates - not just the big
gameplay crate.

This is the breadth pass of the rustdoc strand (20260525-133033 coordinates;
20260525-133030 does nova_gameplay in depth): a sweep across ALL workspace
crates (nova_core, nova_scenario, nova_assets, nova_modding, nova_mod_format,
nova_menu, nova_ui, nova_editor, nova_events, nova_info, nova_debug,
nova_perf, nova_meta_gen, nova_portal_gen) adding at least a one-line doc
comment to every public plugin struct, component, resource and event type.
Retagged from the old backlog where it had no body.

## Steps

- [ ] Enumerate the gap: for each workspace crate, list public
      plugins/components/resources/events without doc comments (a
      missing_docs dry run per crate produces the checklist; paste the counts
      here).
- [ ] Sweep crate by crate, smallest first: one-line minimum per item -
      what it marks/carries/configures and who inserts it; units and
      invariants where they exist. Plugins get one extra line: what systems
      they add and in which schedule.
- [ ] Follow the conventions from 20260525-133033; where a type is really the
      scenario/modding surface (config structs deserialized from RON), make
      the doc comment agree with the wiki's field tables - the RON field
      names are player-facing API.
- [ ] Verify `cargo doc --workspace --no-deps` warning-free; flip
      `#![warn(missing_docs)]` on per crate as each comes clean, per the
      133033 enforcement decision.

## Definition of Done

- Zero undocumented public plugin/component/resource/event types across the
  workspace (measured by the same missing_docs dry run that produced the
  checklist).
- Each crate that came clean has the lint on so it stays clean.

## Notes

- Mechanical but large; fine to split per-crate across sessions - keep the
  checklist in this task current so progress is visible.
- Skip local full test/clippy runs per repo policy; CI covers them.
