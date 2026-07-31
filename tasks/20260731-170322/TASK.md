# KISS: nova_gameplay HUD - NOVA OS drawer surfaces

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.9.0, refactor, chore, hud
- KIND: STORY
- FLOW STEP: DONE
- PLAN STATUS: APPROVED
- PARENT: 20260731-170222

## Story

As a maintainer I want the NOVA OS drawer HUD surfaces to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/hud/nova_os*.rs
Current size: ~14.3k lines across 4 files. Largest file: nova_os.rs at 8274 lines.

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
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/hud/nova_os*` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `find crates/nova_gameplay/src/hud/nova_os* -name '*.rs' | xargs wc -l` - no file over 1500 lines, or
   NOTES.md justifies the exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

The DoD 3 and 4 globs originally read `nova_os*.rs`, which stopped matching
once the split turned the three files into folder modules. Widened to
`nova_os*` (the same surface, now including the folders). No proof was
weakened - the greps cover strictly more files than before.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What and why.** Three files (8274 / 3208 / 2422 lines) became folder
modules split by concern; `nova_os_pointer_rig.rs` (396, one concern) stayed
put. Nothing exceeds 1500 lines now. Every tatr-ID provenance clause is gone
from the comments, with four load-bearing constraints promoted to `NOTE:`.
The per-file map, the visibility scheme and the comment judgements are in
NOTES.md.

**Alternatives.** The obvious alternative for the 3815-line `nova_os.rs` test
module was to leave it whole and justify it in NOTES.md as "one cohesive
concern", which DoD 4 permits. Rejected: it is not one concern - it covers
sound, input, the CRT pipeline, the app runtime and the spawned tree, and at
3815 lines it was the single worst file for the epic's fit-in-one-context
goal. It split into nine modules over a shared rig with no test changes.

**Difficulties.** Two worth recording.

*Slicing by line range cuts doc comments.* The first ship/map split ended
several slices mid-doc-comment, orphaning the docs onto whatever item started
the next slice - which compiles fine in three cases and silently mis-documents
the wrong function. Diagnosed from `expected item after doc comment` errors,
but only the cases that landed at end-of-file were caught that way; the ones
that landed on a following item were silent. Fixed by making the slicer snap
each boundary back over any leading `///` / `#[...]` block, then auditing the
already-split `nova_os/tests/` by hand - which turned up two swapped helper
docs (`press_pad`'s doc had landed on the pad test and vice versa,
`chin_controls_app`'s on an unrelated CRT test).

*`cargo fix` deleted re-exports that only `#[cfg(test)]` code uses.* It
stripped the seven `crt`/`style` names `nova_os_pointer_rig` imports, breaking
the test build; re-adding them and re-running just deleted them again. Root
cause: `nova_os_pointer_rig` is `#[cfg(test)]`, so a plain lib build genuinely
sees the re-exports as unused. Fixed by gating that re-export block behind
`#[cfg(test)]` with a comment saying who it serves.

**Evidence.** `cargo check --workspace --all-targets` green with no warnings;
`cargo fmt --check` clean; `cargo test -p nova_gameplay --lib hud::nova_os`
102 passed / 0 failed; the HUID grep returns nothing; largest file 1334 lines.
The test set was diffed by name against the pre-split tree and is identical -
no test added, removed, renamed or weakened.

**Reflection.** Slicing a file by line range is fast but it does not know
where items begin. Next time, snap boundaries to item starts from the outset
rather than after the compiler complains - and remember that the compiler only
catches the end-of-file cases, so a silent-mis-attribution audit is not
optional. Separately, `cargo fix` is unsafe on a crate whose modules are
cfg-gated; prefer reading its diff to accepting it.

**Defect found.** `20260731-174911` - the objective / flight-log row lists are
dead in production (nothing but tests spawns their containers). Left unfixed
here per the moves-only scope.
