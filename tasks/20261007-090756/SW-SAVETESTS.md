# SW-SAVETESTS: torpedo-transient save/load proof

Task: 20261007-090756. Worktree: resumable-worlds. Scope: only
`crates/nova_world_base/src/save/tests.rs`.

## Claim

Five approved test items are in place and pass:

1. A torpedo that tracks a moving wreck links to the live wreck on the
   restore frame, with no extra tick.
2. A torpedo that tracks another saved torpedo (by index, not id) resumes
   tracking the respawned torpedo on the restore frame.
3. `a_resumed_shot_still_belongs_to_its_shooter` now covers a player
   torpedo and a dead-owner torpedo. The dead-owner torpedo resumes with
   `ProjectileOwner(Entity::PLACEHOLDER)`, and that placeholder reads as
   gone the same way a live dead launcher does.
4. `a_save_waits_out_a_live_blast_and_a_raking_slug` now covers a torpedo
   part at `Health.current = 0.0`: the save waits, the state file does
   not change, and despawning the torpedo lets the save finish.
5. The two id-check tests now cover 9 open-time refusal cases (including
   `Transient(self)` and `Transient(out of range)`), 2 open-time success
   cases, and one snapshot-time dangling-target case.

All 21 tests in `save::` pass. `cargo fmt -p nova_world_base` is clean.

## Evidence

- Item 1: `crates/nova_world_base/src/save/tests.rs:1029`
  (`a_moving_wreck_and_the_torpedo_tracking_it_come_back_together`).
  Assertions at line 1074 (saved record is `SavedTargetRef::Body` naming
  the minted wreck id) and line 1094 (`TorpedoTargetEntity` equals the
  live wreck after Load, same frame).
- Item 2: `crates/nova_world_base/src/save/tests.rs:1106`
  (`a_torpedo_tracking_a_torpedo_resumes_on_it`). Assertions at line 1142
  (A's saved record is `Transient(i)`) and line 1168 (A's
  `TorpedoTargetEntity` equals the respawned B after Load).
- Item 3: `crates/nova_world_base/src/save/tests.rs:927`
  (`a_resumed_shot_still_belongs_to_its_shooter`, extended). Assertions
  at line 976 (owners are player and `Entity::PLACEHOLDER`) and lines
  984-995 (the dead-owner torpedo's owner handle names no live entity,
  which is the same "launcher gone" read
  `nova_ship::sections::torpedo_section::projectile::update_torpedo_arming`
  uses, and `TorpedoArming::tick` treats a `None` launcher as cleared the
  same way; proven at the unit level by
  `nova_ship::sections::torpedo_section::mod::tests::a_launcher_that_died_leaves_nothing_for_its_salvo_to_clear`).
- Item 4: `crates/nova_world_base/src/save/tests.rs:827`
  (`a_save_waits_out_a_live_blast_and_a_raking_slug`, extended).
  Assertions at line 876 (`Waiting` reason contains "a torpedo part is
  being destroyed"), lines 877-886 (no file change while waiting), and
  line 890 (`Saved` after the torpedo despawns).
- Item 5, snapshot case: `crates/nova_world_base/src/save/tests.rs:1208`
  (`a_save_with_a_duplicate_id_fails_and_keeps_the_last_save`, extended).
  Assertion at line 1259 (a torpedo tracking a live, unsaved-id entity
  fails with "transient 0: its target 'stray' is not saved", file
  unchanged).
- Item 5, open-time cases: `crates/nova_world_base/src/save/tests.rs:1278`
  (`a_world_with_a_duplicate_id_is_refused_on_open`, extended). 9
  refusal cases at lines 1302-1388 (duplicate body id, duplicate id
  across sectors, malformed wreck-shaped id, dangling owner, dangling
  Body target, dangling bay section, dangling Canister, `Transient(self)`,
  `Transient` out of range). 2 open-time success cases at lines
  1409-1438 (`Frozen` target skips the id check; a `Body` target naming
  a saved ledger id resolves). Original wreck-of-wreck success case at
  lines 1440-1452.

## Test output

```
nix develop --command cargo test -p nova_world_base --lib save::
```

```
test save::tests::a_missing_root_lists_nothing_and_an_unreadable_root_is_an_error ... ok
test save::tests::create_refuses_an_empty_long_or_unsafe_name_and_makes_nothing ... ok
test save::tests::with_no_player_a_crossing_writes_nothing_and_a_leave_fails ... ok
test save::tests::create_refuses_a_name_whose_folder_exists ... ok
test save::tests::a_world_another_game_holds_open_is_refused ... ok
test save::tests::opening_a_world_removes_the_files_its_header_does_not_name ... ok
test save::tests::a_failed_write_leaves_the_last_good_save ... ok
test save::tests::a_save_waits_visibly_until_the_player_camera_is_ready ... ok
test save::tests::a_load_holds_the_world_until_every_saved_sector_is_live ... ok
test save::tests::a_leave_save_that_never_settles_fails_at_the_bound ... ok
test save::tests::a_written_world_opens_as_it_was_saved ... ok
test save::tests::the_list_shows_why_each_refused_world_cannot_load ... ok
test save::tests::a_load_saves_nothing_until_its_transients_are_back ... ok
test save::tests::a_save_with_a_duplicate_id_fails_and_keeps_the_last_save ... ok
test save::tests::a_save_waits_out_a_live_blast_and_a_raking_slug ... ok
test save::tests::the_first_frame_and_a_leave_each_write_a_save ... ok
test save::tests::a_resumed_shot_still_belongs_to_its_shooter ... ok
test save::tests::a_world_that_disarms_and_rearms_never_overwrites_its_save ... ok
test save::tests::a_moving_wreck_and_the_torpedo_tracking_it_come_back_together ... ok
test save::tests::a_torpedo_tracking_a_torpedo_resumes_on_it ... ok
test save::tests::a_world_with_a_duplicate_id_is_refused_on_open ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out; finished in 0.31s
```

`nix develop --command cargo fmt -p nova_world_base` ran clean (reformatted
only this file's own style, no content change).

## Named mutations

Each mutation is in production code. It is named here, not applied. The
check column names the assertion in my new/extended tests that would
break if the mutation were present.

1. Phase 2 of `spawn_resumed` (in `transients.rs`) skips inserting
   `TorpedoTargetEntity` for a `Transient` or `Body` link.
   Breaks: `a_moving_wreck_and_the_torpedo_tracking_it_come_back_together`
   line 1094 and `a_torpedo_tracking_a_torpedo_resumes_on_it` line 1168,
   both of which require `TorpedoTargetEntity` to be set on the restore
   frame.
2. Phase 2 runs in a later, separate command buffer flush than phase 1,
   so a tick runs between spawn and link and a system sees the torpedo
   with no target for one frame.
   Breaks: both new tests' claim of "no extra tick" (line 1097 and line
   1171 comments and the single `frame(&mut world)` call before each
   assertion; a leaked intermediate frame would still pass today's
   assertions by luck only if no system reacts to a targetless torpedo
   in one frame, but it breaks the stated contract this suite is meant
   to pin; the sharper break is mutation 1, which this suite catches
   directly).
3. `freeze_transients` maps a `Transient` target to `SavedTorpedoTarget`'s
   dangling/`NoDurableId` case instead of `SavedTargetRef::Transient(i)`.
   Breaks: `a_torpedo_tracking_a_torpedo_resumes_on_it` line 1142 (the
   `find_map` would find no `Transient` case and panic "A's record must
   track B by index").
4. `check_saved_ids` skips the `Transient` range/self check (removes the
   branch that rejects `Transient(self)` and out-of-range `Transient`).
   Breaks: `a_world_with_a_duplicate_id_is_refused_on_open`, the two
   cases at lines 1363-1387 (`Transient(0)` self-reference and
   `Transient(5)` out of range), which would then open instead of being
   refused with "transient 0: its target transient N is not another of
   the 1 saved".

## Unverified

- Item 1's wreck is a hand-spawned stand-in: an `EntityId` of the minted
  `<ship>/wreck/<section>` shape, a `RigidBody`, and a `LinearVelocity`.
  It is not a real severed wreck from `nova_ship`'s integrity sever path,
  which this crate has no public entry point to call. The test proves
  the save/load link, not the real sever-to-wreck-id minting path.
- Mutation 2 (phase ordering within one command buffer) has no test in
  this suite that fails strictly because of an extra frame, only because
  of the missing link afterward (mutation 1's effect). I did not find an
  assertion that depends only on frame-count and not on the final link
  value. Flagging this as a gap rather than asserting a stronger claim.
- I did not re-run the suite under each named mutation; per scope, I am
  not permitted to make production changes, even transiently. The
  breakage claims above are by code-reading against `transients.rs` and
  `save/mod.rs`, not by observed failing output.
- One test-code defect was found and fixed, inside my own file: three
  `open_world` calls in `a_world_with_a_duplicate_id_is_refused_on_open`
  bound their lock guard to a named variable (`_lock`), which in Rust is
  not dropped at the point of shadowing. The second and third calls saw
  the world still locked and failed with `Locked`. Fixed by scoping the
  first two calls in their own blocks so each lock drops before the next
  call opens the same world name. This is a test-file-only fix, within
  my granted scope.
