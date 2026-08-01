# KISS: nova_ui, nova_os, nova_editor, nova_debug

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.9.0, refactor, chore, ui
- KIND: STORY
- FLOW STEP: DONE
- PLAN STATUS: APPROVED
- PARENT: 20260731-170222

## Story

As a maintainer I want the shared UI, terminal, editor and debug crates to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_ui/ crates/nova_os/ crates/nova_editor/ crates/nova_debug/
Current size: ~9k lines across 27 files. Largest file: widget.rs at 2265 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
      (None found - see NOTES.md.)
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_ui/ crates/nova_os/ crates/nova_editor/ crates/nova_debug/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_ui/ crates/nova_os/ crates/nova_editor/ crates/nova_debug/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

### What and why

Two files exceeded 1500 lines and each held several concerns, so both became
folder modules: `nova_ui/src/widget.rs` (2265) split one-module-per-widget-family
plus a shared `paint.rs`, and `nova_os/src/terminal.rs` (1579) split into
state / edit / view. Everything else in the four crates is single-concern and
stayed put. The comment rubric then ran over all four crates. The module table
for both splits, the visibility changes they needed, and the full comment
accounting are in NOTES.md.

### Alternatives considered

- Per-module `register(app)` fns instead of one `widget::register`. Rejected:
  the ordering edge between the slider rebuild and the value sync is
  cross-family, so splitting the schedule site would spread one constraint
  across two files for no gain.
- Splitting `nova_debug/src/harness.rs` (650) and `nova_editor/src/lib.rs`
  (499). Rejected by the epic's own rule - long is not the trigger, more than
  one concern is, and neither has a second concern.
- Keeping `NOVA_OS_PROMPT_PREFIX` `pub(crate)`. It had exactly one use site, so
  it moved into `edit.rs` as a private const.

### Difficulties and diagnosis

The `NovaOsTerminal` impl block spans both halves of the terminal split, so its
fields became `pub(super)` (visible inside `terminal/` only) rather than the
type gaining accessors it does not need. The same applied to a handful of
free functions the reconcilers and row builders call across the new boundary:
`pub(super)`, never `pub`, so the crate's public surface is byte-identical.

Two doc-link classes broke in ways `cargo check` cannot see, both caught by
`cargo doc`: module docs linking to now-private sibling modules, and field docs
whose `[`CommandDispatch::Gameplay`]` resolved only because the pre-split file
imported that type. Both are the standing `split-must-re-export-not-repoint`
lesson from the other direction.

### Evidence

- `cargo check --workspace --all-targets` green; `cargo fmt --check` clean.
- `grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the four crates returns nothing.
- No file in the four crates over 1500 lines (largest: `widget/button.rs`, 863).
- `cargo test -p nova_ui -p nova_os -p nova_editor -p nova_debug --lib`: 65
  passed, 0 failed. Per-crate `#[test]` counts identical to the base commit.
- The sorted `pub` item sets of `widget.rs` vs `widget/` and of `terminal.rs`
  vs `terminal/` diff empty.
- `cargo doc -p nova_ui -p nova_os --no-deps`: 4 warnings before, 2 after (both
  pre-existing).

### Reflection

The cheap public-path proof from the prior splits (dump the `pub` name set
before and after, diff, then run `cargo doc`) paid off again and is now the
first thing to run after a split, not the last. Worth noting for the epic: a
`#[cfg(test)] mod fixtures` per split keeps hoisted test helpers honest, and
both splits here needed one - the shared-fixture question is per split, not
per task.
