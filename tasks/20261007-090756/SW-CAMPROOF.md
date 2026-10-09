# Camera Resume Proof

## Claim and change

The four approved proofs now cover the resumed first frame, capture source, view validation, and save waiting. No production files were changed by this subtask. Changes are tests only:
- `crates/nova_ship/src/camera/resume/tests.rs:132` adds the three camera proofs.
- `crates/nova_world_base/src/save/tests.rs:502` adds the save-wait proof.

Blast radius: test code and imports only. No production types, functions, interfaces, or docs changed.

## Assertions

1. `a_resumed_camera_renders_its_saved_pose_first_then_eases_onto_the_chase_pose` checks the first propagated camera pose relative to a rotated ship, resumed steer, zoom 3, and consumed resource. It moves and turns the ship, runs 40 fixed frames, checks blend removal, and compares camera Transform to a recomputed chase-sync pose from `ChaseCameraState.anchor_pos`. A controller respawn has no blend and an identity Normal rig.
2. `a_captured_view_is_the_rendered_pose_without_shake_relative_to_a_rotated_ship` checks Normal capture against propagated GlobalTransforms, FreeLook and Turret solved poses, live shake and scripted Override separation from the solve, Normal steer relative to physics rotation, Autopilot identity steer, and `None` for missing solve, Normal rig, or controller.
3. `a_saved_view_refuses_a_non_finite_or_non_unit_view_or_an_out_of_range_zoom` checks NaN/infinite position, non-unit/NaN view and steer rotations, invalid zooms, and accepted zoom endpoints.
4. `a_save_waits_visibly_until_the_player_camera_is_ready` removes the fixture camera, checks Waiting with `player camera` and only `world.lock`, restores the camera, waits for Saved, then compares the opened view to the solved camera Transform relative to the fixture ship.

## Green checks

- `nix develop --command cargo test -j 8 -p nova_ship --lib camera::resume`
  `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1095 filtered out; finished in 0.03s`
- `nix develop --command cargo test -j 8 -p nova_world_base --lib save::`
  `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.04s`
- Both focused checks passed again after all mutations were restored. `rustfmt --check` passed for the two test files.

## Mutation checks

Each mutation was temporary, tested with its named test, restored using `cp` from `/tmp/nova-camprove.ccE8jZ`, and followed by a green rerun.

- M1, `chase.rs`: added `blend.elapsed = HANDBACK_BLEND_SECONDS;` inside the `blend.elapsed == 0.0` block. The resumed-pose test failed: `position Vec3(-31.449497, 16.4896, -30.21625) differs from saved Vec3(0.0, 5.0, 10.0)`. Restored; named test: 1 passed.
- M2, `resume.rs`: changed the capture source from `query_filtered::<&ChaseCameraState, With<SpaceshipCameraController>>().single(world).ok()?.solved?` to `query_filtered::<&GlobalTransform, With<SpaceshipCameraController>>().single(world).ok()?.compute_transform()`. Capture test failed: `position Vec3(7.4847064, 6.1326494, 8.664995) differs from saved Vec3(7.1470256, 5.600296, 6.2872167)`. Restored; named test: 1 passed.
- M3, `resume.rs`: kept the `state.solved?` readiness check, but temporarily derived position as `to_ship * (anchor_pos + steer * Vec3::new(offset.x, offset.y, -offset.z) - ship_translation)` and rotation as `to_ship * steer`, using the chase offset. Capture test failed: `position Vec3(11.907374, 7.044959, 15.283406) differs from saved Vec3(23.814753, 14.089924, 30.566809)`. Restored; named test: 1 passed.
- M4, `session.rs`: replaced the `None => SectorSnapshotError::Unsettled { label: "the player camera", ... }` path with `drop_wanted("the player camera is not ready"); return;`. Save test failed: `a missing camera must wait, got Unsaved`. Restored; named test: 1 passed.

After each restore, `diff -u` against its backup was empty for `chase.rs`, `resume.rs`, or `session.rs`, as applicable. Final checks repeated all three exact diffs; all were empty.

## Unverified

No workspace tests, Clippy, player render, or cross-process Load probe ran; they were outside this test task. One cargo rerun reported a build-directory lock wait, then completed. No process was stopped.
