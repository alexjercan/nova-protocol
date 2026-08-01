# Review: KISS: nova_gameplay sections and integrity

- TASK: 20260731-170351
- BRANCH: refactor/kiss-gameplay-sections-integrity

## Round 1

- REVIEWER: in-session (session directive forbids spawning subagents; the diff
  is moves + comment deletions only, and every load-bearing claim is
  re-derivable from mechanical conservation checks the reviewer ran itself)
- VERDICT: APPROVE

- [x] R1.1 (MINOR) tasks/20260731-170351/TASK.md:80 - three counts in the
  close-out are wrong: `NOTE:` promotions are 10, not 7; items widened to
  `pub(super)` are 17, not 15; ID-only section separators deleted are 5, not 4.
  Each number was correct when first measured and none was re-measured after
  the last edit - the exact shape of the ledger's
  `re-measure-records-after-the-last-edit`. Replace the three numbers and put
  the producing command next to each.
  - Response: fixed - all three re-measured after the last edit, and each now
    carries the command that produced it.

### What the reviewer verified

- `cargo check --workspace --all-targets` and `cargo fmt --check`: clean.
- `cargo test -p nova_gameplay --lib sections::` 100 passed / 0 failed;
  `--lib integrity::` 21 passed / 0 failed.
- Conservation, re-derived independently of the implementer's per-file counts:
  `#[test]` over the WHOLE scope is 126 before and 126 after.
- Comment audit: extracted every comment line from the base scope and the new
  scope and diffed the multisets. All 201 removed lines are either a re-wrapped
  continuation present in the new text or a deliberate deletion. No comment
  that guards a value was lost - each such site (`sections/mod.rs` reload
  ordering, `thruster_section.rs` raw-pose burn, `torpedo_section` shot-down
  kill, `turret_section/render.rs` `with_emit_on_start`) reappears as a
  `NOTE:`. Base carried zero `TODO:`/`FIXME:`/`BUG:`, so none was dropped.
- Visibility: all 17 items widened to `pub(super)` have a reference outside
  their defining file (12 turret + 5 torpedo systems named by the compiler's
  own E0603/E0425 list, `default_joint_speed` used by three sibling test
  modules, `muzzle_entity` and `joint_entities` by two each). Nothing was
  widened past `pub(super)` except `update_turret_aim_point`, which
  `hud/turret_lead.rs` already reached at `pub(crate)`. Closes the upper bound
  that `visibility-sweep-narrows-back` says no check can see.
- Doc integrity, the `doc-comment-rewrap-changes-the-render` lesson, checked
  two ways: a scan for odd-backtick blocks and for a block construct newly
  following prose inside a doc block (4 hits, all four identical at base), and
  `cargo doc -p nova_gameplay --no-deps`, which emits ZERO warnings under
  `sections/` or `integrity/` - so no intra-doc link broke when items moved
  between modules.
- Public paths: each `prelude` block is byte-identical to base, and the
  workspace `--all-targets` build (examples included) resolves every path.

### Pending user checks

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed.
