# REVIEW: recenter the ship-app orbit camera on the selected section

Out-of-context reviewer, round 1, against `feat/ship-orbit-recenter` vs `master`.

## Round 1

- MINOR - `nova_os_ship.rs` `spawn_offset_ship` fixture: the ship root carried a
  Y rotation but each section's `GlobalTransform` was hand-set to a pure
  `local + offset` translation that ignored it, so the "world frame" was not a
  truly composed world transform - dead dressing that could mislead a reader.
  The local-vs-world distinction the test relies on was still real (target lands
  on the local `(9,2,-1)`, not the world value), so it was a clarity issue, not
  a correctness one. FIXED: sections now use `root_world * local` for
  `GlobalTransform`, so the world pose is genuinely composed and consistent.
- NIT - `ship_input` computes `dt`/takes `Res<Time>` used only by the pre-existing
  note countdown + turn rate; the new reconcile correctly does not use `dt` (the
  ease lives in `drive_ship_camera`). No action.

Everything else checked out: reconcile only retargets on selection change; `T`
retargets home and consumes the selection so the reframe sticks; the exponential
ease is frame-rate independent with a `1/240` dt floor; init opens framed on the
whole ship; blip projection reads the same local frame (unregressed); the two
new live-tree tests fail if the ease no-ops or the reconcile is reverted.

- VERDICT: APPROVE
