# Review: Turret lead/intercept pip (HUD)

- TASK: 20260708-165701
- BRANCH: weapons-hud (implementation commit 21f95c2)

## Round 1

- VERDICT: APPROVE

Verified independently: `cargo fmt --check` clean, `cargo check --workspace`
green, all 29 hud tests pass, and the extended 12_hud_range scripted run
passes with the pip stages (pip on the projected `TurretSectionAimPoint` at
0.0 px drift, hidden after `SectionInactiveMarker`). The reconcile-system
deviation from the planned observers is recorded in TASK.md with its
rationale and covers orderings the observer design would have missed
(sections attached after the player marker); the plan step was updated to
match reality as the work skill requires. The driver's explicit
inactive-turret handling is justified against `update_turret_aim_point`,
which really does keep computing aim points for disabled turrets.

Out-of-diff observation, filed as follow-up rather than a finding: nothing
in game code writes `TurretSectionTargetVelocity`, so the "lead" pip
currently sits on the commanded aim point (the crosshair ray) rather than a
true intercept - pre-existing gameplay gap, now tatr 20260709-173700. The
pip renders exactly what the task scoped (the already-computed
`TurretSectionAimPoint`), so this does not block.

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/turret_lead.rs:143 -
  `drive_pip_anchors`' turret query is unconstrained
  (`Query<(&TurretSectionAimPoint, Has<SectionInactiveMarker>)>`); add
  `With<TurretSectionMarker>` so the query states its intent even though
  pips only ever link turret entities.
  - Response: fixed in 1c46abe - With<TurretSectionMarker> added to the driver query.

## Round 2

- VERDICT: APPROVE

R1.1 verified in the fix commit; 29 hud tests green, fmt clean. No new
findings.
