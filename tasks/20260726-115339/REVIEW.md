# Review: NOVA OS ship computer 3D app + `ship` CLI verbs

- TASK: 20260726-115339
- BRANCH: feat/nova-os-ship-app

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

Verified by the reviewer: both DoD test groups green (nova_os 18, nova_os_ship
6); `cargo check -p nova_gameplay` and `cargo check --examples --features debug`
clean; the doc sweep (hud.md) is correct with no stale "read-only ship"; the
command-invocation pathway keeps `nova_os` ECS-free; the query-conflict
workaround is correct; section-code assignment is stable/collision-free under
despawn.

- [x] R1.1 (BLOCKER) crates/nova_gameplay/src/hud/nova_os_ship.rs (project_ship_blips
  + scene build) - blip projection used the section WORLD position
  (`view.world_pos = global.translation()`) while the scene root, blocks and
  orbit camera all live in ship-LOCAL space anchored at the origin. These
  coincide only when the ship root is at the world origin; in flight the root is
  at an arbitrary moving `GlobalTransform`, so blips project off-block and get
  culled by the on-screen bounds check - breaking click selection and the code
  labels. Fix: project the section's local/scene position.
  - Response: Fixed. Removed `world_pos` and the `GlobalTransform` column from
    `ShipSectionView`/`ShipSections` entirely; `project_ship_blips` now projects
    `view.local.translation` (the same space the blocks are placed in), so the
    projection can no longer use world space. Regression pin:
    `scene_blocks_use_local_space_when_ship_off_origin` builds the scene with the
    ship root at (500,-200,900) and asserts the block sits at the local offset.
  - Verified (in-session): re-read the diff - `world_pos` is gone crate-wide
    (`grep` clean), project reads `view.local.translation`; new test passes;
    all 8 nova_os_ship tests green.

- [x] R1.2 (MAJOR) crates/nova_gameplay/src/hud/nova_os_ship.rs:868
  `apply_ship_section_commands` (the in-app L/P -> `ShipSectionCommand` handler)
  had zero coverage though the DoD names the in-app action path; deleting it
  left every test green. Fix: add a test that writes the message and asserts the
  section's Health/SectionAmmo mutated and the note flashed.
  - Response: Added `ship_action_keys_mutate_through_message_handler`: writes a
    Repair then a Reload `ShipSectionCommand`, runs the handler, asserts hull HP
    -> max, turret ammo -> capacity, and `runtime.note` carries "repaired HULL-1".
  - Verified (in-session): test present and passing; it exercises target
    resolution + mutation + note, and fails if the handler is removed.

- [x] R1.3 (NIT) nova_os_ship.rs (sync_ship_arg_completions) - stale comment
  claiming an "immutable getter" read that the code does not do.
  - Response: Reworded to describe the `!=` gate.

- [ ] R1.4 (NIT) nova_os_ship.rs - `ShipAction`/`ShipSectionCommand` are `pub`
  but not in the module prelude.
  - Response: Left `pub` deliberately. They are the documented action seam the
    future queued/resource-costed model plugs into (NOTES + DECISION fork 4);
    keeping them `pub` (not `pub(crate)`) marks them as the intended extension
    API without exporting them through the prelude yet. Optional NIT, not taken.

## Round 2

- VERDICT: APPROVE
- REVIEWER: in-session (re-verify of the out-of-context Round 1 fixes)

Re-ran both DoD test groups on the updated branch: `cargo test -p nova_os --lib`
= 18 passed; `cargo test -p nova_gameplay --lib nova_os_ship` = 8 passed
(the two new tests included). `cargo check --examples --features debug` clean.
The BLOCKER is structurally closed (world-space projection is now impossible),
the MAJOR is pinned at its own boundary, R1.3 fixed. R1.4 is an accepted NIT.

Pending user check (open `manual:` DoD item, not resolved by APPROVE): owner
confirms the in-game look/feel of the 3D schematic + actions via
`NOVA_SHOT_DIR=target/reel BCS_AUTOPILOT=1 BCS_REEL=1 cargo run --example
screenshot_nova_os --features debug` (`nova-os-ship.png`).
