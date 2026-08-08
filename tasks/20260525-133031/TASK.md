# Write documentation for bevy_common_systems public API

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: docs, backlog

## Closed (2026-07-24, wrong repo)

Closed during v0.9.0 planning triage. Every step is work in the separate
bevy-common-systems repo (~/personal/bevy-common-systems); nova only bumps the
git pin. Not a nova deliverable - track it there if/when picked up.

## Story

As a contributor (to nova or to any other consumer of the library), I want
bevy_common_systems' public API documented, so that the reusable layer nova
consumes (integrity, health, blast, mesh slicing, event dispatch, autopilot
harness) is understandable from its docs instead of from nova's usage sites.

NOTE: this work lives in the SIBLING REPO, not nova-protocol.
bevy_common_systems is an external git dependency
(https://github.com/alexjercan/bevy-common-systems, pinned by tag in
`crates/nova_gameplay/Cargo.toml` - currently v0.19.1). Retagged from the old
backlog where it had no body; part of the rustdoc strand coordinated by
20260525-133033 - reuse its conventions across repos.

## Steps

- In the bevy-common-systems repo: crate-level `//!` doc describing the
      library's scope and its Bevy version contract (the tag scheme tracks
      Bevy releases).
- Document every public module and plugin: integrity/health/blast, mesh
      slicer, event dispatch (including the indexed-dispatch behavior nova's
      perf notes reference), the BCS_AUTOPILOT test-driving harness, and the
      debug helpers.
- Doc comments on the public components/systems nova imports (grep nova's
      `use bevy_common_systems::` sites to build the priority list).
- Verify `cargo doc --no-deps` warning-free in that repo; consider
      `#![warn(missing_docs)]` per the 133033 enforcement decision.
- Cut a tag/rev with the docs and bump nova's pin in the same cycle if any
      doc work required code-visible changes (it should not).

## Definition of Done

- In the bevy-common-systems repo: crate + module docs exist, the
  nova-consumed surface is fully documented, `cargo doc` is clean.
- Nova's pin points at a version containing the docs.

## Notes

- Keep the library docs consumer-neutral (no nova-specific narrative); nova's
  architecture wiki page already explains how nova composes it.
- Coordinate with 20260525-133030/133032 so shared conventions (units,
  intra-doc links) match across repos.

## Grooming (2026-07-20): RETAGGED out of v0.8.0 (v0.8.0 -> backlog, p0)

Every step of this task is "in the bevy-common-systems repo" - it is
cross-repo work in a separate project (~/personal/bevy-common-systems), not a
nova-protocol v0.8.0 deliverable. Keeping it tagged v0.8.0 inflated the
release's doc scope with work nova cannot land. Retagged to backlog and left
as a pointer: when the bcs public-API docs are written in that repo, bump
nova's pin. The rustdoc CONVENTIONS still come from 20260525-133033 so the two
repos stay consistent.
