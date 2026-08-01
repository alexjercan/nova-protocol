# KISS: nova_gameplay input layer - player, AI, targeting

- STATUS: CLOSED
- PRIORITY: 41
- TAGS: v0.9.0, refactor, chore, input
- KIND: STORY
- FLOW STEP: DONE
- PLAN STATUS: APPROVED
- PARENT: 20260731-170222
- DEPENDS ON: 20260731-170335

## Story

As a maintainer I want the gameplay input layer (player, AI, targeting) to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/input/
Current size: ~12k lines. Largest file: ai.rs at 5427 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
      None found - the pass turned up no defect.
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/input/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_gameplay/src/input/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What/why.** All three oversized files became folder modules -
`ai.rs` (5427) into 8 files, `targeting.rs` (3666) into 7, `player.rs` (2727)
into 6 - cut by cohesion, largest survivor 1076 lines. Comment axis applied
across the whole directory: provenance stripped to zero remaining tatr-ID
references, eleven constraint comments promoted to `NOTE:`, twelve stale
section separators deleted. Full file table and rationale in NOTES.md.

**Alternatives.** Splitting `ai.rs` by line count would have left the
behavior state machine straddling two files; splitting by cohesion put the
transition rules, the passive routines and the combat maneuvering in three
files that each read alone. `targeting/gesture.rs` was a second cut after the
first left `radar.rs` at 1693 lines - the live search and the CTRL gesture
that drives it are genuinely two concerns. `player/test_support.rs` was
preferred over duplicating a test rig into two modules.

**Difficulties / diagnosis.**

- The test modules cross-reference production items freely (`use super::*`),
  so the split needed a visibility pass. Resolved with the narrowest widening
  that compiles - `pub(super)` for systems and constants, `pub(crate)` for
  fields of public components - and `#[cfg(test)]` on the imports only tests
  need, so the lib target stays warning-free.
- One self-inflicted bug: an early edit to `player.rs` added a line BEFORE
  the split ran, shifting every recorded boundary by one and slicing a test
  module mid-body. Caught by rustc's unclosed-delimiter error; fixed by
  reverting the file and re-splitting from the pristine copy. Lesson: freeze
  the file before computing line-addressed cuts.
- The provenance stripper initially mangled prose (`Self::leg` -> `Self:leg`,
  `...` -> `.`, dropped leading articles). Caught by a lint sweep over the
  rewritten comments before any commit; the rules were tightened and the four
  damaged spots repaired by hand.

**Evidence.** `cargo check --workspace --all-targets` clean; `cargo fmt
--check` clean; `cargo doc -p nova_gameplay --no-deps` leaves no unresolved
link into the moved modules; `cargo test -p nova_gameplay --lib input::` 180 passed, 0 failed;
`#[test]` count conserved exactly (179 before, 179 after); the ID grep
returns one deliberate `TODO:` hit, listed in NOTES.md; no file over 1500
lines. A multiset comparison of non-comment lines against the pre-split files
shows every difference is a visibility keyword or a moved import - no
executable line changed.

**Round 1 follow-up.** Review round 1 was right on both MAJORs: the first cut
declared the submodules without re-exporting their items, so the direct
`input::<mod>::Name` paths silently moved one level down (three call sites
were repointed to compensate, and one rustdoc link broke). `mod.rs` now
re-exports every previously-visible name at the parent path and all three
repoints are reverted. The stripper also left two damaged comments and
dropped a live backlog ID; all three are repaired, and the stale
pre-split file pointers across five other crates were swept.

**Reflection.** Line-addressed slicing plus a compiler loop handled 12k lines
far faster than reading each file, but it only stays safe if the source is
frozen and the output is checked before it is trusted. The two guards that
were missing are both cheap and both caught real damage in review: a
mechanical comment-text diff base-vs-branch (a lint over the RESULT cannot
see a dropped word or a duplicated line), and a symbol-reachability check -
"the crate compiles" is not the same as "the paths still resolve", because
repointing the call sites hides the regression from the compiler.
