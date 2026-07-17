# Review: turret joint-tree core (merged 215742 + 215804 + 215835)

- VERDICT: APPROVE (after in-review fixes applied)
- REVIEWER: flow review pass on the working tree (uncommitted)

Scope: the recursive `TurretJoint` data model, the generic one-entity-per-joint
ECS, the Jacobi hinge-CCD aim solver, generic render, content migration, the
`04_turret_section` demo, and tests. Multi-muzzle (215857) and editor/lint
(215920) are follow-up commits, out of scope here.

## What was verified

- **Aim math (highest-risk).** Read `update_turret_target_joints_system` +
  `signed_angle_about` line by line. It is a correct Jacobi per-frame
  hinge-CCD: pre-rotation joint frame `F = parent_global * T(offset)`, muzzle
  pose + target transformed into joint-local, `d` and `(t-m)` projected onto the
  plane perpendicular to the hinge axis, signed angle about the axis added to the
  current controller output. Basis-independent (verified the derivation), the
  chain walk terminates cleanly at the section root, and `SmoothLookRotation`
  keeps rate-limiting + clamping. Behavioral parity is pinned by the convergence
  test, not by matching the old quirky per-axis theta - the right call.
- **base+rotator collapse.** Confirmed `SmoothLookRotation` (bcs v0.19.1) never
  reads `Transform`, so carrying the offset and the controller on one entity is
  sound; the golden-chain test pins the resulting shape.
- **Default parity.** `Default for TurretSectionConfig` rebuilds the exact
  base->yaw(Y)->pitch(X)->barrel->muzzle tree with the old offsets/speeds/limits.
- **Content migration.** `content_ron_parity` (builder == committed RON) passes;
  no straggler references to the removed markers/fields anywhere in the repo.
- **Tests.** Golden chain, aim convergence (3 dirs < 5 deg), 3-hinge reach
  (< 8 deg), RON round-trip - all meaningful, all green (46 turret tests pass).

## Findings

- **[minor, FIXED] `speed` serialized on every joint, fixed nodes included.**
  A bare `f32` with only `serde(default)` wrote `speed: 3.1415927` onto every
  node of the authored RON (base, barrel, muzzle - where a hinge speed is
  meaningless), noise a modder would copy. Added `skip_serializing_if =
  "is_default_joint_speed"`; regenerated content; parity re-passes. The tree RON
  is now clean (speed appears only on a hinge that wants a non-default rate).
- **[minor, FIXED] docs out of sync (docs-in-sync lesson).** The dev wiki
  section-authoring guide (`web/src/wiki/dev/guide-author-section.md`) documented
  the old flat turret schema, and there was no CHANGELOG line. Rewrote the wiki
  Turret section to the joint-tree schema (per-joint + section-wide fields) and
  added a **(breaking)** Modding & Mod Portal CHANGELOG entry. Player-facing
  behavior is unchanged; the break is purely the turret authoring schema.

## Deliberately deferred (not defects)

- Single-muzzle core: a tree with >1 muzzle uses the first and warns. This is by
  design - multi-muzzle firing is task 215857.
- Generic default render primitive replaces the bespoke placeholder art (ridged
  cylinders, layered barrel). Shipped turrets author GLBs, so the visible game is
  unchanged; noted in the CHANGELOG/commit. Acceptable.

## Verdict

APPROVE. The two minor findings were fixed in-review; the risky aim swap is
correct and behaviorally pinned. Pending the workspace/examples check going green
before landing.
