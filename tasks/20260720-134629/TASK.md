# rustdoc: fix the 108 pre-existing broken intra-doc links (cargo doc warning-free)

- STATUS: IN_PROGRESS
- PRIORITY: 57
- TAGS: v0.8.0,docs,tooling

## Story

As a contributor reading `cargo doc`, I want the workspace rustdoc to build
warning-free, so a real problem is not buried under 108 stale broken-link
warnings - and so `RUSTDOCFLAGS="-D warnings"` can gate it in CI later.

Follow-up to the rustdoc umbrella (20260525-133033), which found but did not fix
these (they are per-item, not crate-level).

## The debt (measured 2026-07-20)

`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --features debug`
reports 108 warnings in EXISTING `///` docs:

- 88 "public documentation for X links to PRIVATE item Y" - a public item's doc
  intra-doc-links a private fn/system (e.g. `ScreenshotActionConfig` ->
  `resolve_capture_path`). Fix: UN-LINK (plain backticks) - internal systems
  should not become public API just to satisfy a doc link.
- 11 unresolved links - target does not exist (typo/renamed/wrong path). Fix
  the path if the intended target is obvious, else un-link.
- 5 "X is both a function and a module" - ambiguous. Disambiguate (`` [`X()`] ``
  for the fn, `` [`mod@X`] `` for the module).
- 4 redundant explicit link target - drop the redundant `` [`X`]: Y `` form.

By crate: nova_gameplay 78, nova_scenario 17, nova_assets 10, nova_modding 2,
nova_probe 1.

## Steps

- [x] Re-measure to get the authoritative list (file:line:name per warning)
      from a strict `cargo doc` run; the fix targets that list, not memory.
- [x] Un-link the 88 private-item references: `[`name`]` -> `` `name` `` at each
      reported site (precise per file:line - the private item stays named in
      prose, just not linked). Re-read a sample of edited hunks (a scripted
      multi-edit is a hypothesis until the artifact shows it).
- [x] The 11 unresolved: inspect each; fix the path when the intended target
      is clear (renamed item), else un-link. Do NOT invent a target.
- [x] The 5 ambiguous: disambiguate with the `X()` / `mod@X` form.
- [x] The 4 redundant: remove the redundant explicit target.
- [x] Verify `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
      --features debug` exits 0 (zero warnings). Spot-check a couple of rendered
      pages still read sensibly (an un-linked reference is still readable prose).
- [x] Where a crate is now BOTH broken-link-free and missing_docs-clean, note it
      as a candidate for `#![warn(missing_docs)]` (per the umbrella's decision);
      do not force missing_docs here (that is 133032's per-item add).

## Definition of Done

- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --features debug`
  builds with zero warnings.
- No public API was widened just to satisfy a link (un-link, do not `pub`), and
  no link target was invented for an unresolved reference.

## Notes

- Purely a doc-comment sweep; no behavior change, no public-API change. CI
  compiles the crates already; this only touches `///`/`//!` text.
- Overlaps but does not block 133030 (nova_gameplay API docs) / 133032 (per-item
  docs): those ADD docs, this fixes existing links. Landing this first means
  those tasks start from a warning-free surface.

## Verification (2026-07-20)

- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --features debug`
  now exits 0 with ZERO warnings (was 108).
- Fixes: 88 private-item + 11 unresolved shortcut links un-linked (`[`X`]` ->
  `` `X` ``, scripted per exact file:line:name, no misses); 5 ambiguous
  `screen_indicator` links disambiguated to the module (`mod@super::
  screen_indicator`); 4 redundant explicit targets reduced to the shortcut.
- Pure doc-comment sweep: `git diff` is 106/106 line replacements, 0 non-`///`/
  `//!` lines changed - no code or public-API change.
- No public API was widened and no link target invented (unresolved refs to
  private/external/out-of-scope items were un-linked, keeping the name in prose).
- missing_docs: no crate newly qualifies from link-fixing alone (missing_docs
  needs every public item documented - that stays the per-item add in 133032);
  not forced here.
