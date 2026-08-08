# Turret joint tree: editor/lint updates + joint-tree well-formedness lint

- STATUS: CLOSED
- PRIORITY: 10
- TAGS: v0.7.0, spike, refactor, weapons, turret, editor

## Goal

The spike found that `nova_editor/src/placement.rs` and the mount-base adjacency
lint in `nova_scenario/src/lint.rs` operate on the section's placement on the
ship grid and are blind to the internal joint chain, so they should keep working
unchanged once the section spawns the new tree from its Add-observer. This task:

- confirm placement + adjacency lint still pass on the migrated + new multi-arm
  content (regression), fixing anything that turns out to inspect the old chain;
- add a cheap joint-tree well-formedness lint: rotation axes non-zero (or
  explicitly fixed), `min <= max` limits, at least one reachable muzzle;
- resolve the spike open question: confirm no intended mount needs a per-arm
  grid footprint (arms abutting different neighbor cells). If one does, extend
  the adjacency lint for per-arm support instead of assuming a single base face.

## Notes

Spike: tasks/20260717-214834/SPIKE.md (editor+lint option A; per-arm footprint
open question). Depends on 20260717-215742 (data model). Direction-level; run
/plan for steps.

## Resolution

- Placement + mount-base adjacency lint confirmed tree-blind (placement clones
  the whole config into `turret_section()`; adjacency uses `rotation * -Y` + grid
  position) - no change needed, existing tests stay green.
- Added `lint_section_config` (nova_scenario) walking the turret joint tree:
  degenerate hinge axis / non-positive speed / min>max / no-muzzle are errors,
  limits-without-axis a warning. Wired into the scenario per-ship loop (inline
  sections) AND `lint_bundle` (base + mod catalogs). Runtime backstop in
  `spawn_turret_joint` degrades a degenerate axis to a fixed joint (no NaN).
- Open question (per-arm grid footprint) resolved: every shipped/authored turret
  is a single-base mount in one cell; no per-arm adjacency needed. If ever
  authored, the adjacency lint would extend - noted, not built (YAGNI).
