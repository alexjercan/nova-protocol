# Write documentation for nova_gameplay public API

- STATUS: OPEN
- PRIORITY: 40
- TAGS: docs,v0.8.0

## Story

As a contributor extending gameplay (a new section kind, a weapon behavior, an
autopilot verb), I want nova_gameplay's public API documented in rustdoc, so
that I can find the right component/system/plugin from `cargo doc` or the IDE
instead of reading the whole crate.

nova_gameplay is the largest gameplay crate (sections, integrity/damage,
weapons, targeting, autopilot, AI) and the one modders and contributors touch
first. Retagged from the old backlog where it had no body; part of the
rustdoc strand coordinated by 20260525-133033 - follow the conventions that
task establishes.

## Steps

- [ ] Crate-level `//!` doc: what nova_gameplay owns, its main plugins, and
      how it relates to nova_core / nova_scenario / bevy_common_systems (one
      screen, linking to the architecture wiki page for depth).
- [ ] Document every public module with a one-paragraph `//!` header (what
      lives here, when you would touch it).
- [ ] Document the public items contributors actually reach for first:
      plugins, the SectionKind surface and section config types, damage/
      resistance types, targeting/lock components, autopilot verbs and AI
      state types. Prefer doc comments that state units, invariants and
      cross-references over restating names.
- [ ] Intra-doc-link related items (config type <-> runtime component <->
      plugin that registers it).
- [ ] Verify `cargo doc -p nova_gameplay --no-deps` is warning-free; consider
      `#![warn(missing_docs)]` if the crate comes out clean (per the 133033
      enforcement decision).

## Definition of Done

- Crate and every public module have doc headers; the main public types and
  plugins have doc comments with units/invariants where applicable.
- `cargo doc -p nova_gameplay --no-deps` builds warning-free and the rendered
  docs let a reader navigate from crate root to any major system in two
  clicks.

## Notes

- Do not duplicate the wiki (sections.md, architecture.md): rustdoc explains
  items, wiki explains systems - link across.
- Skip local full test/clippy runs per repo policy; CI covers them.
