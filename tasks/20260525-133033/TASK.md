# General documentation pass

- STATUS: OPEN
- PRIORITY: 58
- TAGS: docs,v0.8.0

## Story

As a contributor reading the code through `cargo doc` or an IDE, I want the
workspace's rustdoc surface to explain each crate and its public items, so that
the code-level documentation carries the same weight the wiki carries for
concepts. This is the umbrella for the rustdoc/API strand of v0.8.0 docs (the
wiki strand is 20260718-152214); it was retagged from the old backlog (legacy
#113) where it had no body.

The concrete per-crate work is split into its own tasks: nova_gameplay API
docs (20260525-133030), bevy_common_systems API docs (20260525-133031, note:
external repo), and inline doc comments on public plugin structs/components
(20260525-133032). This task is the coordinating pass: crate-level docs,
consistency, and the decision on enforcement.

## Steps

- [ ] Write or refresh crate-level docs (`//!` in lib.rs) for every workspace
      crate: what it owns, its main plugin(s), how it relates to its neighbors
      (one paragraph each; the architecture wiki page is the source to distill
      from, not duplicate).
- [ ] Establish the rustdoc conventions for the workspace (doc style, whether
      examples are expected, intra-doc links) and record them in AGENTS.md or
      the dev wiki so the per-crate tasks follow one standard.
- [ ] Sequence and, where cheap, execute the per-crate tasks (133030, 133032;
      133031 lives in the bevy-common-systems repo) against that standard.
- [ ] Decide on enforcement: is `#![warn(missing_docs)]` (or deny) worth
      turning on per crate as it comes clean? Record the decision; wire it for
      any crate that is already compliant.
- [ ] Verify `cargo doc --workspace --no-deps` builds warning-free and spot
      check the rendered output for the main crates.

## Definition of Done

- Every workspace crate has a crate-level doc explaining its role.
- A written rustdoc convention exists and the per-crate tasks reference it.
- `cargo doc --workspace --no-deps` is clean, and the enforcement decision
  (lint on/off per crate, and why) is recorded.

## Notes

- Do not duplicate the wiki: rustdoc explains the code items; the wiki explains
  the systems. Link from rustdoc to the wiki page where a concept needs more
  than a paragraph.
- Skip local full test/clippy runs per repo policy; CI covers them.
