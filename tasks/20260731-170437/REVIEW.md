# Review: KISS: nova_ui, nova_os, nova_editor, nova_debug

- TASK: 20260731-170437
- BRANCH: refactor/kiss-ui-os-editor-debug

Branch has 1 commit, `11310690`.
Worktree: `/home/alex/.cache/sprouts/nova-protocol/refactor/kiss-ui-os-editor-debug`

## Round 1

- VERDICT: APPROVE
- REVIEWER: fresh `/flow 20260731-170437` session entering at REVIEWING, outside
the implementation context (skill default). No implementation state carried in;
every claim below re-derived from the diff and from commands run in the
worktree.

### DoD verification

| DoD | Result |
| --- | --- |
| 1. `cargo check --workspace --all-targets` | PASS - exit 0. Only warnings are 4 pre-existing `ambiguous import visibility` in `nova_gameplay` (out of scope). |
| 2. `cargo fmt --check` | PASS - clean, no output. |
| 3. HUID grep over the four crates | PASS - zero hits. Broader `grep -rnE '[0-9]{8}-[0-9]{6}'` over the same four crates (any comment form, plus `Cargo.toml`) also returns nothing, so no exception list is needed. |
| 4. No file over 1500 lines | PASS - largest is `nova_ui/src/widget/button.rs` at 863, then `nova_os/src/terminal/edit.rs` 697, `nova_debug/src/harness.rs` 650. |
| 5. Existing tests pass | PASS - `cargo test -p nova_ui -p nova_os -p nova_editor -p nova_debug --lib`: nova_debug 11, nova_editor 13, nova_os 20, nova_ui 21 = 65 passed, 0 failed. Matches the NOTES claim exactly. |
| 6. Owner skims the diff | PENDING - see below. |

### Re-derived claims

**"Public paths unchanged"** - re-derived two independent ways, not accepted
from NOTES:

- Set diff: the sorted `pub fn/struct/enum/const/trait/type/use` name sets of
  `master:widget.rs` vs `widget/*.rs` and `master:terminal.rs` vs
  `terminal/*.rs` differ only by the new `pub use <child>::*` lines themselves.
- Import probe: a throwaway `crates/nova_gameplay/tests/zz_path_probe.rs`
  importing all 56 top-level public items at their OLD paths
  (`nova_ui::widget::X`, `nova_os::terminal::X`) from an external crate
  compiles clean. This is the proof the sibling task 20260731-170340 had to
  add after its round 1; it holds here. Probe removed, worktree clean.

**"Moves, renames, deletions only - no behavior change"** - CONFIRMED
mechanically. Stripping comments and blank lines and diffing the sorted line
multiset of the pre-split file against the post-split folder:

- `terminal`: every added line is an import, a `pub(super)` visibility keyword,
  a `mod`/`pub use` line, or a rustfmt re-wrap. Zero logic lines removed.
- `widget`: same, with one deliberate deletion (below).

**"No test renamed or weakened"** - CONFIRMED. `#[test]` counts conserved
exactly (widget 12 -> 12, terminal 15 -> 15), and the sorted test-name lists
diff empty for both splits.

**Comment pass density** - the four crates now carry 342 non-marker comment
lines over 9147 source lines (~37/1k), at or below the already-landed siblings
(`nova_probe` ~40/1k, `nova_scenario` ~55/1k, `nova_menu` ~42/1k). Consistent
with the bar the epic has been accepting.

**Non-split crates** - the whole `nova_debug` + `nova_editor` diff is
comment-only; verified line by line. No code line changed.

### Findings

**LOW - NOTES.md omits the one deletion that is not a comment**
`tasks/20260731-170437/NOTES.md:76` says "no test was renamed or weakened", but
the widget split also deleted the test helper `fn only_button` and its
`let _ = only_button; // keep the helper referenced for future single-button
tests` keep-alive (`master:crates/nova_ui/src/widget.rs:1775` and `:2216`). The
deletion is right - it was dead, speculative, YAGNI-violating scaffolding - and
it is in scope ("deletions only"). The record just should say so, since it is
the sole non-comment deletion in the branch and a reader reconciling the line
multiset will trip over it. Change: add one line to the NOTES "Evidence"
section naming the deletion.

**LOW - two doc comments left ragged after the provenance clause was cut**
`crates/nova_ui/src/theme.rs:102` ("...changed nothing visually. Per-widget
tuned variants (the many slightly-") and `crates/nova_ui/src/units.rs:4` ("...
displays as / 10 m; distance...") keep the old wrap points, so a short line sits
mid-paragraph. Cosmetic only; `fmt` does not re-wrap doc prose. Change: re-wrap
those two paragraphs to the file's ~100 col.

Neither finding blocks. Both are record/cosmetic, not correctness.

### Verdict

APPROVE.

### Pending manual items

- DoD 6 - owner skims the diff and agrees no behavior changed. Supporting
  evidence: the non-comment line multiset of both splits differs from the base
  only by imports, `pub(super)` keywords and `mod`/`pub use` lines; all 56
  old public paths compile from an external crate; the 27 split-file test names
  are an identical set before and after, and all 65 tests in the four crates
  pass locally.

### Inspection commands

```
cd "$(sprout show refactor/kiss-ui-os-editor-debug)"
git diff master...HEAD --stat
git diff master...HEAD -- crates/nova_debug crates/nova_editor   # comment-only
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo fmt --check
nix develop --command cargo test -p nova_ui -p nova_os -p nova_editor -p nova_debug --lib
grep -rnE '[0-9]{8}-[0-9]{6}' crates/nova_ui/ crates/nova_os/ crates/nova_editor/ crates/nova_debug/
find crates/nova_ui crates/nova_os crates/nova_editor crates/nova_debug -name '*.rs' | xargs wc -l | sort -rn | head
```
