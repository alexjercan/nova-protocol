# PROOF-LOADORDER

Worktree: /home/alex/.cache/sprouts/nova-protocol/resumable-worlds
Change under test: crates/nova_menu/src/tests/load_screen.rs:73-74 adds
WorldSaveTestPlugin before arm_save_fixture (chase-camera observer order fix).
Build: ./target (no CARGO_TARGET_DIR), -j 8, nix develop --command.

## 1. cargo test -j 8 -p nova_menu --lib load_screen

Run 1 (first attempt, default harness):
  FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 199 filtered out
  panic at crates/nova_menu/src/tests/load_screen.rs:124:21
  "Requested resource <Enable the debug feature to see the name> does not exist"
  (WorldSaveSession missing after press(&mut app, "Load World Button"))

Runs 2-7 (rerun same binary, no source changes, with and without --nocapture):
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out
  (6 consecutive passes, including one run with --nocapture, no warn!/error
  output emitted)

Diagnosis: the fixture-ordering change under review is confirmed present
(load_screen.rs:73-74) and is not the cause of the one observed failure -
the failure happened only on the very first run and could not be reproduced
in 6 subsequent runs of the identical binary/command on an otherwise idle
host. save_until_idle() (load_screen.rs:48-62) already documents the save
pipeline as off-thread and multi-frame; the single failure is consistent
with a one-off scheduling stall on that off-thread path, not a defect
introduced by the plugin-order change. Flagging as flaky rather than
confirmed-fixed: evidence does not rule out an intermittent race.

## 2. cargo test -j 8 -p nova_menu --lib leave

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 191 filtered out

## 3. cargo test -j 8 -p nova_world_base --lib save::

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 62 filtered out

## Skipped

No workspace-wide test or Clippy run (not requested). No GPU/game measurement
run (not applicable to this check). Only one test command ran at a time.

## Remaining uncertainty

load_screen's single first-run failure is unexplained beyond "off-thread save
pipeline, did not reproduce in 6 reruns." Exit-zero/pass-rate alone is not
proof of a timing fix; no repeated-measurement harness (e.g. nova-probe) was
used here, matching the no-timing-assertion rule. If this test is seen to fail
again, capture RUST_LOG and the frame count at panic before changing the fix.
