# Review: HUD indicator when torpedo is fired (reticle sizing + aim-assist)

- TASK: 20260525-133022
- BRANCH: torpedo-hud-133022

## Round 1

- VERDICT: REQUEST_CHANGES

Both scoped items are delivered and the check suite is green (46 lib tests pass,
clippy clean, fmt clean, `cargo check --workspace` clean). The angular targeting
is correct and well-tested. One correctness/feel gap in the reticle sizing is
worth fixing before merge, plus a missing test.

- [x] R1.1 (MAJOR) hud/torpedo_target.rs:195 - `project_aabb_screen_size` returns
  `None` as soon as ANY of the 8 AABB corners fails to project (behind the camera
  near-plane), and the caller then falls back to `MIN_RETICLE_PX`. This collapses
  the reticle to the minimum size precisely when the target is large and close -
  the exact case item 1 exists to handle - because a near target is the one whose
  rear corners cross behind the camera. The headline feature silently no-ops in
  its most visible scenario. Suggest sizing from the target's projected bounding
  radius instead: take the world bounding-sphere radius from the AABB
  (`aabb.size().length() * 0.5`), project `center` (already done) and
  `center_world + camera_right * radius`, and set the reticle to
  `2 * pixel_distance` (clamped to the min). That only needs the center to be in
  front, which is guaranteed whenever the target is locked and on-screen, so it is
  robust for close targets and drops the fragile 8-corner projection.
  - Response: Fixed. Replaced the 8-corner projection with the suggested
    bounding-radius method (size = `2 * pixel distance from the projected centre
    to a point one world radius to the camera's right`). Only the centre needs to
    project, so it no longer collapses to min for close targets.
    `project_aabb_screen_size` deleted.
- [x] R1.2 (MINOR) hud/torpedo_target.rs:171 - `target_world_aabb` (the subtree
  walk + merge that item 1 relies on) has no test. Add a small `App`-based test
  that spawns a parent with two children carrying `ColliderAabb`s and asserts the
  merged min/max spans both. This is cheap (no physics needed - insert
  `ColliderAabb` directly) and locks in the merge behavior.
  - Response: Added two tests (`target_world_aabb_unions_child_collider_aabbs`,
    `target_world_aabb_is_none_without_colliders`) using `SystemState` to build the
    queries. 48 lib tests pass.
- [x] R1.3 (NIT) input/player.rs:121 - `TARGETING_MAX_RANGE = 2000.0` may exceed a
  torpedo's actual reach (`max_speed * projectile_lifetime`), so a lock can be
  acquired on something no torpedo can reach. Not blocking (locking != firing, and
  the old raycast had unbounded range), but consider tying the range to the
  torpedo's reach or documenting 2000 as an intentional generous cap.
  - Response: Accepted as-is. Kept 2000 as a deliberate generous cap: locking is
    decoupled from firing (a lock only matters at launch), and the previous raycast
    had unbounded range, so this is not a regression. The const doc explains the
    intent. Tying it to torpedo reach is left for the weapons-HUD spike.
- [x] R1.4 (NIT) targeting behavior changed materially (single raycast -> angular
  aim-assist cone that can lock occluded targets). The `/compound` retro will
  capture it, but a line in `docs/sections.md` (HUD/targeting) would help future
  readers. Take-it-or-leave-it.
  - Response: Done. Added a HUD/targeting paragraph to `docs/sections.md`
    describing the aim-assist cone, the exclusions, and the reticle sizing.

## Round 2

- VERDICT: APPROVE

All Round 1 findings resolved and verified against the diff. R1.1 refit to the
bounding-radius sizing (verified: `project_aabb_screen_size` gone, close-target
collapse fixed); R1.2 tests added and passing (48 lib tests); R1.3 accepted with
reasoning; R1.4 documented. Re-ran the suite on the updated branch: 48 lib tests
pass, clippy clean, fmt clean. Both scoped items are delivered.
