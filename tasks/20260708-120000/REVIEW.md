# Review: Camera snaps to origin for one frame when switching camera modes

- TASK: 20260708-120000
- BRANCH: fix/camera-mode-origin-snap

## Round 1

- VERDICT: APPROVE

Diff changes `sync_spaceship_control_mode` to mutate the existing `ChaseCamera` in place instead
of re-inserting it, plus one regression test. Single file.

Verified:

- Root cause is correct and confirmed in the bcs source. `initialize_chase_camera`
  (`On<Insert, ChaseCamera>`) inserts `ChaseCameraInput::default()` unconditionally, while it
  guards `ChaseCameraState` behind `!has_state`. So a `ChaseCamera` re-insert resets the anchor
  (but not the smoothing state) - exactly a one-frame origin snap given the default `smoothing:
  0.0`. Mutating `offset`/`focus_offset` via `&mut` does not fire the insert observer, so the
  anchor is preserved.
- The regression test is meaningful, not a tautology. It puts the ship at `(100,20,-50)`, sets
  the anchor, switches to FreeLook, and asserts the anchor is unchanged and the offset became
  FreeLook's. Confirmed it FAILS on the pre-fix code (`left: (0,0,0)` vs `right: (100,20,-50)`)
  and passes on the fix - so it pins the exact bug.
- Behaviour preserved otherwise: the offsets/focus_offsets per mode are identical to before; only
  the mechanism (mutate vs re-insert) changed. `smoothing` stays at the camera's spawned value
  (`ChaseCamera::default()` = 0.0), same as the old `..default()`, so mode switches remain
  instant. The input-marker/`PointRotation` command logic is untouched.
- `commands` is still used (the input-marker inserts/removes), so no unused-param fallout; clippy
  is clean.
- Full suite green: `cargo clippy --workspace --all-targets` (only the pre-existing
  `hull_section.rs` warning), `cargo test --workspace` (59 nova_gameplay incl. the new test,
  examples_smoke under Xvfb).

Honest note in TASK.md: the deeper trigger is a bcs observer resetting `ChaseCameraInput` on every
insert; the nova-side in-place mutation is the correct and leaner fix, with a possible bcs guard
noted as out-of-scope cross-repo work.

No BLOCKER/MAJOR/MINOR findings.
