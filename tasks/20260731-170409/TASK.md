# KISS: nova_assets

- STATUS: CLOSED
- PRIORITY: 37
- TAGS: v0.9.0, refactor, chore, assets

## Story

As a maintainer I want the assets crate to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_assets/
Current size: ~16.1k lines across 18 files. Largest file: shakedown.rs at 2843 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
- [x] Verify: check, fmt, and the existing tests for this area.

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

## Close-out

**What/why.** Split the three multi-concern files (`lib.rs`, `portal.rs`,
`scenario/shakedown.rs`) into cohesive modules and stripped the crate's
provenance comments. Nothing in the crate now exceeds 1500 lines and the 275
HUID-bearing comment lines are down to 6 deliberate `TODO`s. Moves, renames and
deletions only - no signature, no behavior, no public path changed. The full
module map, the marker table and the largest-file justification are in NOTES.md.

**Alternatives.** For `portal.rs` the cheaper option was moving only its 420
lines of tests out, which lands the file at 1353 and clears the DoD threshold.
Rejected: the epic splits on concerns, not line counts, and the file genuinely
held four (URL config, transport seam, catalog fetch, install state machine).
For `shakedown.rs` the reverse call: its 1221-line production half is one
concern - the script - so it stayed whole and only the tests split.

**Difficulties / diagnosis.** The comment pass was scripted (join a comment
block, strip provenance, rewrap only what changed) and needed three rounds. The
first skipped any block containing a blank comment line, which is most module
docs; the second skipped any block containing a list item, which swallowed the
module docs that happen to precede a numbered list. Fixing the skip to be
per-PARAGRAPH rather than per-block cleared both. Six sentences still came out
mangled where the HUID was the grammatical object ("real art is task X" ->
"real art is.") - those were exactly the deferred-work comments, so they became
`TODO(...)` markers, which is where the rubric wanted them anyway. Blocks with
```` ``` ```` fences and list indentation were left to hand edits.

**Evidence.** `cargo check --workspace --all-targets` and `cargo fmt --check`
clean. The DoD grep returns 6 hits, all `TODO(...)`. `cargo test -p nova_assets
--tests --no-fail-fast`: all 24 integration binaries green, lib 95/96.

The one failure, `an_early_derelict_kill_skips_to_the_fight`, is PRE-EXISTING -
reproduced on master at `e038c34e` before any edit here - and is filed as
20260801-122138 rather than fixed in a refactor task.

**Reflection.** Rewrapping comment blocks makes a large diff for a small
semantic change; scoping the rewrap to paragraphs that actually changed kept it
reviewable. The lesson worth carrying: when a mechanical text pass has a skip
guard, make the guard as narrow as the unit being edited - a block-level guard
on a paragraph-level edit silently skipped a third of the work twice.
