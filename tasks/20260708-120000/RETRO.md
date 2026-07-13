# Retro: Camera snaps to origin for one frame when switching camera modes

- TASK: 20260708-120000
- BRANCH: fix/camera-mode-origin-snap
- PR: #48 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260708-120000/TASK.md`. A one-frame visual glitch whose cause was a component re-
insert firing a side-effecting observer.

## What went well

- Followed the data across the crate boundary. The symptom (camera at origin for a frame) lived
  in nova, but the cause was a bcs observer. Reading `initialize_chase_camera` and spotting that
  it resets `ChaseCameraInput` unconditionally - while guarding `ChaseCameraState` behind
  `!has_state` - was the whole diagnosis. The asymmetry in that observer *was* the bug.
- Picked the fix that removes the trigger rather than masking it. Mutating `ChaseCamera` in place
  means the insert observer never fires, so there is nothing to reset - simpler and cheaper than
  re-inserting a component to change two fields. The correct fix was also the leaner one.
- Proved the regression test both ways. It passes on the fix and, when I restored the original
  code and re-ran it, failed with exactly `left: (0,0,0)` vs `right: (100,20,-50)` - the origin
  snap in numbers. A test that only passes on the fix is half a test; confirming it fails on the
  bug is what makes it a regression guard.
- Kept behaviour identical otherwise: same per-mode offsets, same smoothing (0.0, so instant
  switches), only the mechanism changed - so there is nothing new to re-tune.

## What went wrong

- The first attempt to temporarily revert-for-the-failing-test check was fiddly (the fix touches
  all three match arms and the param type, so a partial revert did not compile). Wasted one step
  before switching to the clean approach: `git checkout` the file, re-append just the test, run,
  then restore the fixed copy from a backup. Root cause: tried to hand-patch a revert instead of
  using git to get a known-good original.

## What to improve next time

- To prove a test fails on the pre-fix code, restore the original file wholesale (`git checkout`)
  and re-apply just the test, rather than hand-editing a partial revert - especially when the fix
  changes a function signature that the other call sites depend on.
- When a component is re-`insert`ed only to change a couple of fields, prefer `&mut` mutation:
  besides being cheaper, it avoids re-firing `On<Insert>`/`On<Add>` observers, which is a common
  source of one-frame state-reset glitches.

## Action items

- [ ] Possible cross-repo follow-up: guard bcs's `initialize_chase_camera` to only add
      `ChaseCameraInput` when absent (as it already does for `ChaseCameraState`), so re-inserting
      `ChaseCamera` elsewhere cannot resurrect this class of bug.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro).
