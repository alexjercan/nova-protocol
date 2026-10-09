# PROOF-LOADFLAKE

## Target
Worktree: /home/alex/.cache/sprouts/nova-protocol/resumable-worlds
Test: `cargo test -p nova_menu --lib load_screen`
(crates/nova_menu/src/tests/load_screen.rs,
load_lists_a_saved_world_and_a_refused_one_and_loads_only_the_saved_one)

Reported failure (1 in 7 runs, prior session): panic at
crates/nova_menu/src/tests/load_screen.rs:124, WorldSaveSession missing
after press(&mut app, "Load World Button"). Means on_load_world
(crates/nova_menu/src/load_screen.rs:118-150) got Err(refusal) from
open_world.

## Setup
- ./target as CARGO_TARGET_DIR (no cross-worktree sharing), -j 8,
  nix develop --command, as instructed.
- Backed up the test file to /tmp/load_screen.rs.bak.20261008 before any
  edit.

## Instrumentation (test file only, no production code touched)
Before the WorldSaveSession assert, added: if WorldSaveSession is absent,
read crate::load_screen::WorldListings (pub(crate) resource), find the
row for slug "good-world", and panic with its row.header Err (a
WorldRefusal, Debug) instead of the generic missing-resource panic.
This surfaces the refusal open_world returned, which the row list never
shows (list_worlds only reads headers; open_world additionally re-reads
the state file and runs CameraView::validate per
crates/nova_world_base/src/save/mod.rs:264-299 and
crates/nova_ship/src/camera/resume.rs:88-101).

Verified compile: `nix develop --command cargo test -j 8 -p nova_menu
--lib load_screen` built and the test passed once before the loop.

## Runs attempted
1. 60x `nix develop --command <built test binary> load_screen
   --test-threads=1` directly (exit code checked) - 0 failures.
2. 60x `nix develop --command cargo test -j 8 -p nova_menu --lib
   load_screen` (exit code checked; the first pass used a `grep -qi
   FAILED` heuristic that false-matched the harness's own "0 failed"
   summary line - corrected to check the process exit code) - 0
   failures.
3. 40x the same cargo test command while 20 `yes >/dev/null` CPU
   stressors ran concurrently on a 24-core box (load average rose from
   ~1-2 to ~9.7), to pressure-test the dt-variance hypothesis - 0
   failures. Stressors were started and later killed by their own
   recorded PIDs (no broad process matching); `pgrep -a yes` confirmed
   none remained.

Total: 160 attempts, 0 reproductions.

## Failures seen
None. The instrumentation never fired; WorldSaveSession was present and
named "Good World" on every run.

## Refusal text
Not observed - no run reached the Err(refusal) branch.

## Diagnosis status
The code-path hypothesis stands on static evidence alone and is
unconfirmed by execution:
- open_world (crates/nova_world_base/src/save/mod.rs:284-288) validates
  state.player.view via CameraView::validate
  (crates/nova_ship/src/camera/resume.rs:88-101) and turns a fault into
  WorldRefusal::Unreadable, independent of list_worlds' header-only read.
  This is the only place a Load click can fail after the row shows
  unrefused (load_screen.rs:132,141-147), so it remains the only
  candidate for the missing-WorldSaveSession panic.
- chase_camera_sync_transform_system
  (crates/nova_ship/src/camera/chase.rs:243-265) calls
  `transform.look_at(focus, up)`. If `focus` coincides with
  `state.anchor_pos` (zero look direction), Bevy's look_at divides a
  zero-length vector during normalization, producing a NaN rotation;
  `Quat::is_normalized` is false for NaN, so CameraView::validate would
  reject it as NonUnitRotation. This is a real path to the fault
  described in the hypothesis, but I could not force it: 160 runs,
  including 40 under heavy CPU contention meant to perturb per-frame
  `Time<Real>` deltas, never produced a zero-length focus vector or a
  non-finite pose.
- chase_camera_update_state_system's lerp (nova_gameplay/src/math.rs:77-99)
  does not divide by dt; `t.powf(dt)` at dt=0 yields 1.0, so a zero-delta
  frame leaves state.anchor_pos unchanged rather than producing NaN. This
  specific "divide by zero dt" mechanism in the hypothesis is not
  supported by the code as read.

No exact line is named as the cause because the failure did not
reproduce under this budget; the fault, if it exists, is rarer than 1 in
160 runs under the conditions tried here, or needs a different trigger
(e.g. a real multi-frame wall-clock gap from GPU/process scheduling
noise that `yes` CPU contention does not reproduce, since this is a
headless CPU-only test with no GPU frame to stall on).

## Restore
Restored crates/nova_menu/src/tests/load_screen.rs from
/tmp/load_screen.rs.bak.20261008 with cp.
`diff /tmp/load_screen.rs.bak.20261008 crates/nova_menu/src/tests/load_screen.rs`
exit code 0 (no difference).
`git status --porcelain` shows the file as untracked (it was untracked
before this session too - part of this worktree's pre-existing
uncommitted work, not new from this instrumentation). Nothing staged.

## Remaining uncertainty
- The 1-in-7 failure was observed once in a prior session under unknown
  machine conditions; it was not reproduced here in 160 runs including a
  CPU-stressed set.
- The look_at zero-direction path is a plausible but unconfirmed
  mechanism; it was not exercised.
- No production code was changed. The instrumentation that would catch a
  recurrence (reading WorldListings' stored refusal on a missing
  WorldSaveSession) was not kept; a future repro attempt should re-add it
  from /tmp/load_screen.rs.bak.20261008's diff or this report's patch
  description.

## Main-worker resolution

Cause: a concurrent mutation, not a race in the test.
- SW-P6.md "Mutation test": sw-p6 replaced the `resume_world` call in
  `on_load_world` with a no-op and left the `Playing` transition in place.
  With no `resume_world`, no `WorldSaveSession` is inserted. That is exactly
  the panic at `tests/load_screen.rs:124`.
- Timeline (stat):
  - 19:53:02 PROOF-TIMEFIX.md was written, and proof-loadorder started.
  - 19:53:42-44 `nova_menu` was compiled (`target/debug/deps/nova_menu-e57acb8cf3d2dade.*.rcgu.dwo`),
    which was proof-loadorder's first run.
  - 19:54:24 `crates/nova_menu/src/load_screen.rs` was restored by sw-p6.
  - All later runs (6 in proof-loadorder, 160 here) passed.
- Process fix: run no proof while a mutation is open on a shared file.
