# Review: editor/lint + joint-tree well-formedness (task 215920)

- VERDICT: APPROVE (pending shipped-content lint pass)
- REVIEWER: flow review pass on the working tree (uncommitted, atop e882e2cf)

## What the task asked, and what shipped

1. **Confirm placement + adjacency lint still pass (regression).** Confirmed by
   inspection: `nova_editor/src/placement.rs` matches `SectionKind::Turret(turret)`
   and passes `turret_section(turret.clone())` - it clones the whole config and
   never reads the internal joint tree, so it places the section entity and the
   Add observer builds the tree. Tree-blind, unchanged. `check_mount_adjacency`
   uses `rotation * -Y` (base face) + grid position - also tree-blind; its test
   (`mount_base_at_an_empty_cell_is_an_error`) still passes.
2. **Joint-tree well-formedness lint.** Added `lint_section_config` in
   nova_scenario, walking a turret's `root` tree (one DFS) and flagging: a hinge
   with a degenerate (zero/non-finite) axis, a non-positive/non-finite hinge
   speed, `min > max` (all errors), rotation limits on a fixed node (warning),
   and a tree with NO muzzle (error - the spawn observer rejects it at runtime).
   Wired into BOTH the scenario per-ship loop (inline turret sections) and
   `lint_bundle` (every base + mod section catalog), so a malformed turret fails
   the build even when no scenario inlines it. New test
   `turret_joint_tree_wellformedness_is_linted` covers each case (pass).
3. **Runtime backstop (clamp-trio).** `spawn_turret_joint` now validates the
   hinge axis: a degenerate axis degrades the joint to FIXED (marker axis None,
   no controller) with a warn, instead of NaN-ing through `axis.normalize()`.
   The lint is the author-time gate; this is the runtime cap for a code-built or
   lint-bypassing turret. (finite-check + runtime-cap + lint-range, per
   `authored-durations-clamp-trio`.)
4. **Per-arm footprint open question - RESOLVED.** Every shipped/authored turret
   is a single-base mount occupying one grid cell; the joint tree is internal to
   that cell. No mount needs arms abutting different neighbor cells, so the
   single-base adjacency lint stays correct and needs no per-arm generalization.
   If such a mount is ever authored, the adjacency lint would need per-arm
   support - noted here and in the spike, not built (YAGNI).

## Findings

None blocking. The lint runs on the catalog via `lint_bundle`, so shipped base +
mod turrets are validated (must stay clean - see verification).

## Verdict

APPROVE, pending the `content lint` CLI confirming base + mod content passes the
new check with no new issues, and the turret gameplay tests staying green under
the axis-guard change.
