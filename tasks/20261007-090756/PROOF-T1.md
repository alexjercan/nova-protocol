# PROOF-T1: focused verification run

Worktree: /home/alex/.cache/sprouts/nova-protocol/resumable-worlds (branch resumable-worlds)
Command form: `CARGO_TARGET_DIR=./target nix develop --command cargo test -j 8 ...`
No files edited, staged, or committed. No workspace tests, no Clippy run.

## 1. cargo check -p nova_core --tests

Result: PASS (check, no test run). Wall time ~3.3s (incremental, nova_core/nova_menu/nova_console/nova_world_base rebuilt).
Warnings: `the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1` (pre-existing dependency future-incompat notice, not from this change). No errors, no project-code warnings.

## 2. cargo test -p nova_world_base --lib save (run 1 of 3, see also item 11)

`test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.06s`
Wall time: 1m 58s (cold build of nova_gameplay/nova_ship/nova_training/nova_hud/nova_scenario/nova_modding/nova_assets/nova_world/nova_world_base chain) + 0.06s test run.
Tests (all ok): a_missing_root_lists_nothing_and_an_unreadable_root_is_an_error, create_refuses_an_empty_long_or_unsafe_name_and_makes_nothing, create_refuses_a_name_whose_folder_exists, a_world_another_game_holds_open_is_refused, opening_a_world_removes_the_files_its_header_does_not_name, a_failed_write_leaves_the_last_good_save, a_written_world_opens_as_it_was_saved, with_no_player_a_crossing_writes_nothing_and_a_leave_fails, the_list_shows_why_each_refused_world_cannot_load, a_save_waits_visibly_until_the_player_camera_is_ready, a_load_holds_the_world_until_every_saved_sector_is_live, the_first_frame_and_a_leave_each_write_a_save, a_save_waits_out_a_live_blast_and_a_raking_slug, a_resumed_shot_still_belongs_to_its_shooter, a_world_that_disarms_and_rearms_never_overwrites_its_save, a_load_saves_nothing_until_its_transients_are_back, a_leave_save_that_never_settles_fails_at_the_bound.
No warnings beyond the proc-macro-error2 future-incompat notice.

## 3. cargo test -p nova_gameplay --lib --features serde rounds

`test result: ok. 37 passed; 0 failed; 1 ignored; 0 measured; 339 filtered out; finished in 0.41s`
Wall time: 14.52s build + 0.41s run.
Ignored: `rounds::tests::exact::known_collider_cost_against_the_spatial_reference` with reason `ignored, wall-clock comparison; run manually, never a timing assertion` -- expected, matches AGENTS.md "never assert timing" policy; not run here as it needs manual wall-clock comparison against a named reference, which is out of scope for this check pass.
No failures. No warnings beyond proc-macro-error2.

## 4. cargo test -p nova_ship --lib --features serde frozen_rounds

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1106 filtered out; finished in 0.02s`
Test: `sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok`
serde feature required and used as instructed; no build needed (cached).

## 5. cargo test -p nova_ship --lib turret_section (no serde feature)

`test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 1016 filtered out; finished in 0.39s`
Filter matched tests under aim, arc, config, firing, render, setup, stow, and the top-level turret_section module; all 83 ok, none skipped. serde feature was NOT needed -- filter matched without it, so no rerun required.
Build: 48.24s (nova_ship rebuilt without serde feature, separate artifact from steps 4/6/7's serde build).
No warnings beyond proc-macro-error2.

## 6. cargo test -p nova_ship --lib torpedo_section (no serde feature)

`test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 1029 filtered out; finished in 1.00s`
Filter matched tests under bay, projectile (incl. point_defense_cost_tests and scripted submodules), render, and the top-level torpedo_section module; all 70 ok. serde feature NOT needed.
No warnings beyond proc-macro-error2.

## 7. cargo test -p nova_ship --lib ship_audio (no serde feature)

`test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 1045 filtered out; finished in 0.03s`
Filter matched tests under combat, cues, levels, loops, machinery, and routing submodules; all 54 ok. serde feature NOT needed.
No warnings beyond proc-macro-error2.

## 8. cargo test -p nova_menu --lib tests::load_screen

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.02s`
Test: `tests::load_screen::load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one ... ok`
Build: 22.84s (nova_world_base + nova_menu rebuilt).
No warnings beyond proc-macro-error2.

## 9. cargo test -p nova_menu --lib tests::leave (run 1 of 3, see also item 11)

`test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 196 filtered out; finished in 0.04s`
Tests (all ok): load_last_save_reopens_the_saved_world_without_writing, a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent, leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused, window_close_with_a_saved_world_waits_for_the_leave_save.
No warnings beyond proc-macro-error2.

## 10. cargo test -p nova_core --test world_resume_refusal

`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s`
Test: `a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks ... ok`
No warnings beyond proc-macro-error2.

## 11. Flake check: repeat items 2 and 9 two more times each

### nova_world_base --lib save

- Run 2/3: `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.04s`
- Run 3/3: `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.04s`

Combined with run 1: 3/3 runs at 17 passed, 0 failed. No flake observed in this environment (8 jobs, warm cache). The historical race on a missing WorldSaveSession did not reproduce here; this does not prove the race is fixed, only that it did not trigger in 3 runs on this machine under this load.

### nova_menu --lib tests::leave

- Run 2/3: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 196 filtered out; finished in 0.05s`
- Run 3/3: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 196 filtered out; finished in 0.05s`

Combined with run 1: 3/3 runs at 4 passed, 0 failed. Test print order varied slightly between runs (consistent with multi-threaded test execution), but pass/fail outcome was stable across all 3 runs. No flake observed.

## Summary

| # | Command | Result | Pass/Fail/Ignored |
|---|---|---|---|
| 1 | check -p nova_core --tests | PASS (compiles) | n/a |
| 2 | test nova_world_base --lib save | PASS | 17/0/0 |
| 3 | test nova_gameplay --lib --features serde rounds | PASS | 37/0/1 ignored |
| 4 | test nova_ship --lib --features serde frozen_rounds | PASS | 1/0/0 |
| 5 | test nova_ship --lib turret_section | PASS | 83/0/0 |
| 6 | test nova_ship --lib torpedo_section | PASS | 70/0/0 |
| 7 | test nova_ship --lib ship_audio | PASS | 54/0/0 |
| 8 | test nova_menu --lib tests::load_screen | PASS | 1/0/0 |
| 9 | test nova_menu --lib tests::leave | PASS | 4/0/0 |
| 10 | test nova_core --test world_resume_refusal | PASS | 1/0/0 |
| 11 | repeats of #2 and #9 (x2 each) | PASS all 4 reruns | #2: 17/0/0 x2 more; #9: 4/0/0 x2 more |

No test failures anywhere. No filter matched 0 tests. The only recurring warning across all runs is the pre-existing `proc-macro-error2 v2.0.1` future-incompat notice from a dependency, unrelated to this change. nova_ship steps 5-7 did not need `--features serde` (only steps 4 and 3's crate did, as instructed). No project-code warnings surfaced in any step.

## Not covered by this run

- No visual/GPU verification was performed (no frames captured, no probe or bench run), per the exact command list given.
- Headless/CI-style cargo test output does not prove in-game appearance; this report covers only the listed unit/integration test commands.
- The historical "raced on a missing WorldSaveSession" flake was not observed in 3 repeats here, but 3 passing runs on one quiet machine is not proof of absence; it is evidence against reproduction under this specific load, nothing stronger.
