# Fix dead-code warning: ShipBlock.section read only in tests after legibility refactor

- STATUS: CLOSED
- PRIORITY: 32
- TAGS: v0.9.0,bug,ui,hud
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

The ship-legibility change (`20260728-115435`) moved the selected-outline tint
onto `ShipBlockOutline`, so `ShipBlock.section` is now read only by test code.
Under `cargo test` (cfg(test)) the field looks used, so the miss hid; a plain
`cargo check` warns `field 'section' is never read` (dead_code). Fix the
ownership so the field is read in production, and verify with a non-test build.

## Approach

`ShipBlock` (the section's visual proxy) owns the section identity; the outline
is a child decoration and should DERIVE its section from its parent rather than
duplicating it. Make `update_ship_blocks` look the section up on the parent
`ShipBlock` via `ChildOf`, and drop the now-redundant `ShipBlockOutline.section`
field (it becomes a unit marker used only as a query filter). This makes
`ShipBlock.section` a production read and removes the duplicate.

## Steps

- [x] In `crates/nova_gameplay/src/hud/nova_os_ship.rs`, remove the `section`
      field from `ShipBlockOutline` (make it a unit marker) and drop it from the
      outline spawn in `manage_ship_scene`.
- [x] Rewrite `update_ship_blocks` to query
      `(&ChildOf, &mut MeshMaterial3d), With<ShipBlockOutline>` plus a
      `Query<&ShipBlock>`, resolve the parent block's `section`, and tint the
      outline amber when it is the selected section. This reads
      `ShipBlock.section` in production.
- [x] Verify with a NON-test build so the dead_code lint is active.

## Definition of Done

- `cargo check -p nova_gameplay` reports no `dead_code` warning for
  `ShipBlock.section` (cmd: `cargo check -p nova_gameplay 2>&1 | grep -c "never read"` prints `0`).
- The selected section's outline still tints amber and the ship tests still pass
  (test: `cargo test -p nova_gameplay --lib nova_os_ship`).

## Close-out

`ShipBlockOutline` is now a unit marker; `update_ship_blocks` resolves the
section from the parent `ShipBlock` via `ChildOf`, so `ShipBlock.section` is read
in production and the outline no longer duplicates the identity. Verified:
`cargo check -p nova_gameplay` (non-test build, dead_code lint active) reports 0
`never read`; all 12 `nova_os_ship` tests pass; `cargo fmt` clean. The selected
outline still tints amber (the `runtime.selected == block.section` branch is
unchanged, just sourced from the parent).

Self-reflection: the original miss was verifying the parent task with `cargo
test` only - a field read solely by `cfg(test)` code looks live under the test
build, so the dead_code lint never fired. A plain `cargo check` (or
`--all-targets` in a way that also builds the non-test cfg) is what surfaces it.
Recorded as a lesson.

## Notes

- Follow-up defect from `20260728-115435` (found by the post-land dead-code
  diagnostic). Mechanical fix, single file.
- Root-cause lesson: `cargo test` hides a `dead_code` field that only test code
  reads; a plain `cargo check`/`--all-targets` build is needed to surface it.
