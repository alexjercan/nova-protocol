# KISS: nova_scenario

- PRIORITY: 36
- TAGS: v0.9.0, refactor, chore, scenario
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260731-170222

## Story

As a maintainer I want the scenario crate to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_scenario/
Current size: ~13.8k lines across 17 files. Largest file: actions.rs at 2908 lines.

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
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_scenario/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_scenario/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What/why.** Split the three files over 1500 lines into folder modules by
concern (`actions/`, `loader/`, `lint/`) and stripped every task-id provenance
clause from the crate's comments, promoting the constraints that guard a value
to `NOTE:`. Details and the per-file mapping are in NOTES.md.

**Alternatives.** Splitting `actions.rs` by line count (one file per N action
configs) was rejected: it would cut the enum's dispatch away from arbitrary
arms. Splitting by "config type vs impl" was rejected for the same reason -
each action's config and its `EventAction` impl are one unit. The chosen axis
is what the action touches, which is also how the tests already clustered.

Leaving the test modules whole in `mod.rs` (code split, tests not) was
rejected: a 1600-line test module in a 200-line `mod.rs` fails the same
one-file-one-context goal the epic is about.

**Difficulties.** The three test modules shared config-builder fixtures, so a
naive per-concern test split would have duplicated them; both `lint/` and
`loader/` grew a `#[cfg(test)] mod fixtures` instead. `actions/` needed none -
its tests shared no helpers. The mechanical comment rewrite (regex strip +
re-wrap of only the HUID-bearing blocks) needed a second pass for the dangling
`review R1.x` clauses left behind once the task ids were gone, and three sites
where the clause was load-bearing mid-sentence were fixed by hand.

**Evidence.** `cargo check --workspace --all-targets` and `cargo fmt --check`
clean. `cargo test -p nova_scenario --lib` 145 passed, `--test skybox_swap_e2e`
1 passed. Test-name parity proven by diffing the sorted `#[test]` names out of
the three pre-split files against the split tree: 90 names, identical.

**Reflection.** Extracting line ranges with `sed` and re-attaching headers beat
rewriting the files by hand - the moved code is provably byte-identical, so the
only thing left to review is the module headers and the visibility fixes the
compiler demanded. Worth reusing for the remaining epic children. The comment
rubric is the slower half: only the HUID grep can be automated, and deciding
narration-vs-constraint still needs reading every block.
