# KISS: nova_menu - split the 7705-line lib.rs

- STATUS: CLOSED
- PRIORITY: 38
- TAGS: v0.9.0, refactor, chore, menu

## Story

As a maintainer I want the menu crate to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_menu/
Current size: ~8k lines in 2 files. Largest file: lib.rs at 7705 lines.

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
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_menu/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_menu/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

### What and why

`crates/nova_menu/src/lib.rs` was 7705 lines holding the entire crate. It is now
219 lines - crate doc, prelude, module list, `impl Plugin` - over nine concern
modules and a nine-file test tree. The comment rubric ran over every file in the
crate. Largest file is now `mods.rs` at 875 lines.

Details, the per-file table and the comment-rubric decisions: NOTES.md.

### Alternatives considered

- **Split by line count into `menu_a.rs`/`menu_b.rs`.** Rejected: the epic
  rubric splits on cohesion, and the file had nine genuine seams already.
- **A folder module per concern (`pause/mod.rs` + `pause/tests.rs`).** Rejected
  as heavier than the problem: flat siblings plus one `tests/` folder is the
  simplest thing that keeps every file small.
- **Keep the tests in one `mod tests`.** Impossible under DoD 4: the test block
  alone was 3490 lines.
- **Make moved items `pub` rather than `pub(crate)`.** Rejected: it would widen
  the crate's public surface, which DoD 6 forbids. `pub(crate)` is invisible
  outside the crate, so `NovaMenuPlugin` + `prelude` remain the whole API.

### Difficulties and diagnosis

- **Slicing 7705 lines by hand was not viable.** Wrote a brace-balance item
  parser that attaches each item's leading doc/attribute lines, then assigned
  items to modules by name. Content moved byte-for-byte, so behavior
  preservation is by construction rather than by review.
- **`ScrollableList` was emitted into two files.** Two of my line ranges
  overlapped by four lines. `cargo check`'s `never constructed` warning caught
  it, which is why the crate has to end at zero warnings, not zero errors.
- **The HUID-stripping pass damaged the prose it edited.** Removing `task <id>`
  from mid-sentence glued words together ("Sincethe", "untilwires"), ate empty
  parens (`config_dir()` -> `config_dir`), flattened a bullet list onto one
  line, and left a dangling "Since ... , so ...". Six sites were caught during
  the pass by grepping for glued preposition+article pairs and diffing
  `()`-bearing token counts; that grep was too narrow, and review round 1 found
  a seventh ("untilwires" - both halves are real words, so no character-class
  heuristic sees it). Replaced with a word-level multiset diff of every comment
  in the crate against master, which reports no lost or invented token beyond
  the deliberate deletions.
- **Four pre-existing orphan docstrings surfaced** - `app()`, `mod_dep_graph`,
  `DepStatus`, `button()` - each opening with a paragraph describing the item
  above it. All confirmed against `git show master` before touching them, so
  none is a regression from this pass. Fixed here: comment-only, in scope. Two
  were found by the pass and two by review, which is the point: this defect
  compiles, tests clean, and is invisible to every automated check.

### Evidence

| Proof | Result |
|-|-|
| `cargo check --workspace --all-targets` | exit 0; zero warnings from `nova_menu` |
| `cargo fmt --check` | exit 0 |
| `cargo test -p nova_menu --lib` | 76 passed, 0 failed |
| DoD 3 grep | one hit, the deliberate `flex_shrink` NOTE |
| DoD 4 `wc -l` | largest file 873; no exception needed |
| Test-name multiset | 76 before, 76 after, identical names |
| Non-comment source text | pure move; 28 base-only lines, all import fragments or rustfmt re-wraps |

The four remaining workspace warnings are pre-existing `ambiguous import
visibility` in `nova_gameplay`, landed by 20260731-170322 and untouched here.
Filed as 20260801-005057.

DoD 6 (`manual:`) stays open for the owner.

### Reflection

The parser-plus-assignment-map approach made the structure axis mechanical and
its correctness checkable after the fact - the non-comment text diff is a much
stronger claim than "I read the diff and it looked fine", and it is cheap.

The comment axis went the other way. Regex-stripping prose is not mechanical
even when it looks it: seven of 67 sites came out subtly wrong, and not one
would have failed a compile. Worse, my own repair pass declared victory on six
and missed the seventh, because the heuristic I reached for (glued words with a
tell-tale character class) cannot see a join of two ordinary words.

The lesson is about the shape of the check, not the diligence. For a mechanical
edit to prose, the verification has to be a multiset diff against the base -
words for comments, exactly as the non-comment line diff did for code - not a
pattern hunt for the damage I happened to imagine. The line-level check found
every structural problem on the first pass; the pattern hunt found six of seven.
Same effort, different confidence. Wrote the word-diff after review round 1;
it should have been the first thing built, alongside its line-level twin.
