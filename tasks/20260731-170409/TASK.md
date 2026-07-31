# KISS: nova_assets

- STATUS: OPEN
- PRIORITY: 37
- TAGS: v0.9.0, refactor, chore, assets
- KIND: STORY
- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED
- PARENT: 20260731-170222

## Story

As a maintainer I want the assets crate to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_assets/
Current size: ~16.1k lines across 18 files. Largest file: shakedown.rs at 2843 lines.

## Steps

- [ ] Read the parent epic's comment and structure rubrics.
- [ ] Inventory: per-file line counts and the concerns each file holds.
- [ ] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [ ] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [ ] Open backlog tasks for any defect the pass uncovers; do not fix here.
- [ ] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_assets/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_assets/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.
