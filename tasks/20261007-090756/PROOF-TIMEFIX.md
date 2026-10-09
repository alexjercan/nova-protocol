# Proof: Time fix in arm_save_fixture (re-check)

Worktree: /home/alex/.cache/sprouts/nova-protocol/resumable-worlds (branch resumable-worlds)

Change under test:
- crates/nova_world_base/src/test_support.rs:81-82: arm_save_fixture now also
  calls `world.init_resource::<Time>()` (comment: the chase sync reads the
  generic clock, which TimePlugin would add).
- crates/nova_world_base/src/save/tests.rs:314-327: armed_session no longer
  calls init_resource::<Time>() itself.
- Confirmed Res<Time> is read at crates/nova_ship/src/camera/chase.rs:220,276.

## 1. cargo test -j 8 -p nova_menu --lib load_screen

Command:
  nix develop --command cargo test -j 8 -p nova_menu --lib load_screen

Result: FAILED (reproduced twice, identical panic both runs).

  test tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one ... FAILED
  thread '...' panicked at crates/nova_menu/src/tests/load_screen.rs:61:5:
  the fixture world did not finish saving in time
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.05-0.07s

Context: load_screen.rs:48-61 (save_until_idle) runs up to 200 app.update()
frames waiting for WorldSaveSession to reach Saved; it panics at line 61 when
that bound is exhausted (not a Failed status, which is asserted separately
at line 55-59).

Evidence this is a regression, not a flake: tasks/20261007-090756/SW-LOAD.md
records this exact test passing (`... ok`, 1 test) under the production
save pipeline before this re-check. The test file is otherwise unmodified
(it is untracked/new in this worktree, same as recorded in SW-LOAD.md).

The save gate that is not settling is CameraView::capture
(crates/nova_ship/src/camera/resume.rs:49-53): it requires
`ChaseCameraState.solved` to be `Some` on the player's camera controller
entity; session.rs:336-345 (snapshot_world) treats a `None` there as
"the player camera ... it is not ready" and holds the save request.
`app()` in crates/nova_menu/src/tests/support.rs builds a full NovaMenuPlugin
stack (MinimalPlugins, which already includes bevy's TimePlugin), so
`Res<Time>` exists before arm_save_fixture runs; the fixture's new
`init_resource::<Time>()` call is a no-op there (Bevy's init_resource only
inserts when absent). This points to the camera never reaching `solved`
within 200 frames in this app wiring, not to the Time resource itself -
but this was not isolated further (no source edits were made per the task
scope); flagged as unresolved.

## 2. cargo test -j 8 -p nova_menu --lib leave

Command:
  nix develop --command cargo test -j 8 -p nova_menu --lib leave

Result: PASSED.

  running 9 tests
  ... (all 9 ok, including tests::leave::load_last_save_reopens_the_saved_world_without_writing,
       tests::leave::a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent,
       tests::leave::leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused,
       tests::leave::window_close_with_a_saved_world_waits_for_the_leave_save)
  test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 191 filtered out; finished in 0.04s

## 3. cargo test -j 8 -p nova_world_base --lib save:: --features test-support

Command as given (the feature flag is not wrong for lib tests: test_support
is gated `#[cfg(all(not(target_arch = "wasm32"), any(test, feature =
"test-support")))]` in crates/nova_world_base/src/lib.rs:55-56, so `cfg(test)`
alone already exposes it to --lib tests; `--features test-support` is
additive and harmless, no alternate path needed):

  nix develop --command cargo test -j 8 -p nova_world_base --lib save:: --features test-support

Result: PASSED.

  running 13 tests
  save::tests::a_missing_root_lists_nothing_and_an_unreadable_root_is_an_error ... ok
  save::tests::create_refuses_an_empty_long_or_unsafe_name_and_makes_nothing ... ok
  save::tests::create_refuses_a_name_whose_folder_exists ... ok
  save::tests::a_world_another_game_holds_open_is_refused ... ok
  save::tests::opening_a_world_removes_the_files_its_header_does_not_name ... ok
  save::tests::a_failed_write_leaves_the_last_good_save ... ok
  save::tests::with_no_player_a_crossing_writes_nothing_and_a_leave_fails ... ok
  save::tests::a_written_world_opens_as_it_was_saved ... ok
  save::tests::the_list_shows_why_each_refused_world_cannot_load ... ok
  save::tests::a_save_waits_visibly_until_the_player_camera_is_ready ... ok
  save::tests::the_first_frame_and_a_leave_each_write_a_save ... ok
  save::tests::a_world_that_disarms_and_rearms_never_overwrites_its_save ... ok
  save::tests::a_leave_save_that_never_settles_fails_at_the_bound ... ok
  test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.05s

This suite exercises armed_session (save/tests.rs:314-327), the function the
Time-init call moved out of, using the test_support::WorldSaveTestPlugin app
(not the full nova_menu stack) - including the camera-readiness test
`a_save_waits_visibly_until_the_player_camera_is_ready` and the never-settles
bound test `a_leave_save_that_never_settles_fails_at_the_bound`. Both pass,
so the Time-init relocation is sound for this app wiring.

## Summary

- Check 2 (nova_menu --lib leave): PASS.
- Check 3 (nova_world_base --lib save::, --features test-support): PASS,
  the relocated Time init is proven correct for the WorldSaveTestPlugin app.
- Check 1 (nova_menu --lib load_screen): FAIL, deterministic across two runs.
  This is a regression against the passing run recorded in SW-LOAD.md. The
  panic is the 200-frame "did not finish saving in time" bound, gated on
  CameraView::capture's ChaseCameraState.solved never becoming Some in the
  full nova_menu app. Root cause not isolated (no source edits made per task
  scope - this task is verification-only). Needs follow-up before this
  change is considered proven end to end.

No files were edited or staged for this check.
