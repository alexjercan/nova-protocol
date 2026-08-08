# KISS: small crates and root binary

- STATUS: CLOSED
- PRIORITY: 33
- TAGS: v0.9.0, refactor, chore

## Story

As a maintainer I want the small crates and the root binary to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_core/ crates/nova_events/ crates/nova_info/ crates/nova_modding/ crates/nova_mod_format/ src/
Current size: ~1.9k lines. Largest file: lib.rs at 622 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable. NONE qualified - largest file in scope is 446
      lines and every file holds one concern; see NOTES.md for the per-file
      justification.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_core/ crates/nova_events/ crates/nova_info/ crates/nova_modding/ crates/nova_mod_format/ src/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_core/ crates/nova_events/ crates/nova_info/ crates/nova_modding/ crates/nova_mod_format/ src/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What / why.** Comment-only pass over `nova_core`, `nova_events`, `nova_info`,
`nova_modding`, `nova_mod_format` and the root binary. The epic's STRUCTURE
axis turned out to be a no-op for this scope: the header's "largest file:
lib.rs at 622 lines" is wrong - the real maximum is `nova_modding/src/lib.rs`
at 446, the whole scope is 2036 lines, and every file holds exactly one
concern. Splitting anything here would have been line-count-driven, which the
epic rubric forbids. So the deliverable is the COMMENT axis, applied file by
file, plus the inventory that justifies not splitting (NOTES.md).

**Alternatives considered.** (1) Split `nova_core/src/lib.rs` into
`app_builder.rs` + `config.rs` anyway, to match the shape of the sibling
tasks. Rejected: at 337 lines with one concern it fails the epic's own "a
900-line file with one concern stays" test, and it would have put a real
move-diff between the reviewer and a change that is otherwise provably inert.
(2) Keep the task HUIDs inside rustdoc, on the reading that the rubric's "keep
rustdoc" outranks DoD 3's grep. Rejected after checking the crates the earlier
children landed - `nova_ui`, `nova_os`, `nova_probe` and `nova_scenario` all
return ZERO hits on that grep, so the established precedent is that doc
comments lose their HUIDs too; the surviving guard is rewritten to read without
the history.

**Difficulties / diagnosis.** The only real judgment cost was the ~30 comments
that sit on the line between "narration" and "explains a non-obvious value".
Resolved by the rubric's stated default (burden is on KEEPING) plus one test:
does the comment survive deleting the code it describes? The `deps.rs` test
comments were the hardest call - three of them justify non-obvious EXPECTED
values (post-order output, a cycle-tolerant result, and a deliberately weak
assert whose weakness is load-bearing) and were kept as `NOTE:`; six others
restated the `graph(&[...])` literal or the assert message immediately below
and went. Two rustdoc DEFECTS surfaced while reading: `nova_events`'s module
doc never listed `OnNeutralizedEvent`, and `nova_info`'s carried a stale
paragraph calling itself the workspace's `missing_docs` exemplar for a rollout
that has since finished. Both fixed in place per "improve if wrong".

**Evidence.** A comment-stripped line multiset over every touched `.rs` file
(base vs branch) has a residue of exactly ONE line, and that line differs only
by a trailing `// no edges` - a comment. No statement, literal, signature,
import, `mod` line or visibility keyword changed anywhere in scope, so no
`Plugin::build` body, loader or algorithm can have moved. `cargo check
--workspace --all-targets` green, `cargo fmt --check` clean, the DoD 3 HUID
grep returns nothing at all, largest file 439 lines. All tests in scope ran to
COMPLETION here (this scope fits the box's RAM, unlike the earlier children):
12 lib tests plus the 3 cubemap integration tests, 0 failures. `cargo doc
--no-deps` over the five crates emits no warning from any crate in scope, so no
intra-doc link broke when the rustdoc blocks were compacted.

**Reflection.** The planned size figure was off by ~40%, which mattered: had it
been right, this task would have been scoped as a split and the reviewer would
be reading a move-diff instead of a two-line proof. Worth checking the inventory
against the plan BEFORE choosing an approach on the remaining epic children.
The comment-stripped-multiset check is now cheap enough (one throwaway script,
reused across five sibling tasks) that it should just be the default proof for
any comment-only pass - it converts "I read the diff and it looks inert" into
"the diff IS inert". One defect was found in scope and it was already tracked:
the four `ambiguous import visibility` warnings from `nova_gameplay`'s NOVA OS
split, filed as 20260801-005057. No new backlog task was needed.
