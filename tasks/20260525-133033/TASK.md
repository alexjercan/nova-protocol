# General documentation pass

- STATUS: CLOSED
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

- [x] Crate-level `//!` docs for every workspace crate. Survey found 4 with
      NONE (nova_core, nova_events, nova_info, nova_scenario), 3 too thin
      (nova_assets, nova_debug at one line; nova_gameplay vague) - all 7
      written/expanded to a one-paragraph "what it owns + main plugin +
      neighbors" header distilled from the architecture wiki. The other 8
      (editor, ui, menu, modding, mod_format, portal_gen, meta_gen, probe)
      already had adequate headers; left as-is.
- [x] Rustdoc conventions recorded in AGENTS.md ("## Conventions"): crate-level
      `//!` per crate; `///` on public items saying what/why not how; intra-doc
      links for reachable types, wiki links for concepts; runnable examples NOT
      required per item; missing_docs enabled per-crate-as-clean; keep
      `cargo doc --workspace --no-deps` warning-free.
- [x] Sequenced the per-crate tasks: 133030 (nova_gameplay API) and 133032
      (inline docs on public items across crates) now reference the AGENTS.md
      convention and drive the missing_docs rollout crate by crate; 133031
      (bcs) lives in the sibling repo (retagged out of v0.8.0 during grooming).
      Not executed here - they are the large per-item push, out of this
      umbrella's scope.
- [x] Enforcement decision (recorded in AGENTS.md + here): `#![warn(missing_docs)]`
      is enabled PER CRATE AS IT COMES CLEAN, not workspace-wide - a blanket
      turn-on would demand every public item documented at once (that IS 133032).
      Wired `nova_info` (tiny, now fully documented) as the exemplar so the
      enforcement path is proven; other crates opt in via 133032.
- [~] Verify `cargo doc --workspace --no-deps` (--features debug). It BUILDS
      (exit 0) and all NEW crate-level docs are link-clean, but a
      RUSTDOCFLAGS="-D warnings" run surfaced 108 PRE-EXISTING per-item
      broken-link warnings (see the finding below) - NOT introduced here.
      `nova_info` is fully clean and carries `#![warn(missing_docs)]`.

## Finding (2026-07-20): 108 pre-existing broken intra-doc links

A strict `cargo doc` run (`RUSTDOCFLAGS="-D warnings"`) found the workspace's
rustdoc surface already carries 108 broken/ambiguous intra-doc-link warnings in
EXISTING `///` docs (none from this task's crate-level headers):

- 88 "public documentation links to PRIVATE item" - a public item's `///`
  intra-doc-links a private fn (e.g. `ScreenshotActionConfig` -> private
  `resolve_capture_path`). Fix: un-link (plain backticks) or make the target
  public; un-link is right for implementation-detail references.
- 11 unresolved links, 5 fn-vs-module ambiguous, 4 redundant explicit targets.

By crate: nova_gameplay 78, nova_scenario 17, nova_assets 10, nova_modding 2,
nova_probe 1.

Re-scope (this is exactly the umbrella's "sequence per-crate work" job): the
per-ITEM link cleanup is NOT this umbrella's deliverable - it belongs with the
per-crate passes that document each crate's items. nova_gameplay's 78 go with
20260525-133030 (nova_gameplay API docs); the remaining 30 with 20260525-133032
(inline docs across crates). Each crate turns on `#![warn(missing_docs)]` (and
becomes broken-link-free) as it comes clean; `nova_info` is the first. The
"cargo doc warning-free" DoD is therefore the STRAND's end state, driven by the
per-crate tasks against the AGENTS.md convention - not achievable in the
crate-level umbrella alone without pre-empting those tasks.

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
