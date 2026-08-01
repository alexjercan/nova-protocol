# Retro: KISS: nova_gameplay input layer - player, AI, targeting

- TASK: 20260731-170340
- BRANCH: refactor/kiss-gameplay-input
- REVIEW ROUNDS: 2

## What went well

- Cutting by cohesion rather than line count produced 21 files whose concerns
  can be named in one clause each, and the largest survivor (1076) sits well
  under the threshold with no DoD 4 exception needed.
- Conservation was proved rather than asserted: a multiset comparison of
  non-comment lines against the pre-split files, a per-`Plugin::build`
  comparison, and an exact `#[test]` count (179 -> 179) together showed no
  executable line moved. That evidence survived two review rounds untouched.
- Scaling the cut a second time was the right call: the first pass left
  `radar.rs` at 1693 lines, and carving the CTRL gesture out of the live
  search put both halves under 1100 without inventing anything.
- Test-only cross-module imports were gated `#[cfg(test)]` from the start, so
  the lib target stayed warning-free through a large visibility widening.

## What went wrong

**The split satisfied the compiler instead of preserving the paths.** Each
`mod.rs` declared `pub mod <concern>;` and stopped there, so
`input::targeting::CombatLock` and its ~40 siblings quietly moved one level
down. Three call sites were repointed to make it compile, which is exactly
what hid the regression: `cargo check --workspace --all-targets` was green,
`cargo fmt` was clean, and 180 tests passed. The decision seemed sound at the
time because the preludes - the surface everything in this repo actually
imports through - were preserved and verified, and the epic rubric's phrase
"crate preludes keep their exports" was read as the operative clause. It is
not: the sentence before it says public paths must not change, and the
already-landed `hud/nova_os_ship` split had the re-export pattern sitting
right there as precedent. The out-of-context reviewer caught it via a
`cargo doc` unresolved-link warning, which is the one check that notices.

**A mechanical comment pass damaged prose in three ways.** A scripted
provenance strip turned "a `.chain()`" into `a.chain`, duplicated a rustdoc
line, and dropped a live backlog task ID out of a deferred-work note - the
last one inverting the epic rubric's own "keep the TODO with its ID" row. A
lint sweep over the RESULT was run and passed; it could not see any of them,
because each is only visible as a difference from the base text.

**Line addresses were computed against a file that then changed.** An
unrelated one-line import fix landed in `player.rs` between the inventory and
the split, shifting every recorded boundary by one and slicing a test module
mid-body. Cheap to fix (revert, re-split from the pristine file) and it
announced itself as an unclosed delimiter, but only because the damage
happened to land at end-of-file.

**Record numbers went stale mid-review.** The round-2 fixes changed five
files, silently falsifying five rows of the line-count table that round 1 had
verified.

## What to improve next time

- A module split is done when the paths still resolve, not when the crate
  compiles. Re-export at the parent path first; if a call site outside the
  module needs repointing, that is the signal the split broke a path.
- A mechanical comment pass owes a comment-text diff base-vs-branch, not a
  lint over the result. Every one of this pass's three text defects is
  trivially visible in that diff and invisible to everything else.
- Freeze the file before computing line-addressed cuts, or re-derive the
  boundaries immediately before slicing.
- Re-measure recorded numbers after the LAST edit of the round, not after the
  edit that motivated them.

## Diagnose

- **Breadth.** The diff is large (12k lines restructured) because the Story
  scoped a whole directory, and the three files were genuinely one concern
  each at the task level. Not a missed split: the epic already owns the
  per-area breakdown, and cutting this task further would have produced
  children that cannot be verified independently (the conservation proofs
  compare against the pre-split files as wholes).
- **Churn.** One plan-time question would have prevented round 1's MAJOR:
  "what does 'public paths must not change' mean concretely for this split,
  and which existing split in the repo already answers it?" The rubric was
  read once at the start and not re-read against the mechanism actually
  chosen. The `hud/nova_os_ship` precedent was one grep away and would have
  supplied the pattern before the first file was written.
- **Context.** No context-pressure event was recorded - no checkpoint, no
  compaction warning, no handoff. The volume was handled by line-addressed
  slicing plus a compiler loop rather than by reading 12k lines, which is what
  kept the pass inside one window; the two mechanical failures above are the
  price of that technique and both have cheap, specific guards.

## Action items

- Ledger: bumped `doc-comment-rewrap-changes-the-render` (x4),
  `re-measure-records-after-the-last-edit` (x3, now pending promotion),
  `provenance-vs-deferred-work-check-the-status` (x2),
  `generated-links-need-real-targets` (x5), `keep-docs-in-sync-with-code`
  (x10); added `split-must-re-export-not-repoint` (x1).
- No follow-up task: the pass uncovered no defect in the code it moved.
