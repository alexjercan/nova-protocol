# Jobs: a done escort is dropped when the hauler dies after its mark

- STATUS: CLOSED
- PRIORITY: 61
- TAGS: v0.16.0,jobs,bug

## Evidence
- `escort_on_arrival` (`crates/nova_world_base/src/jobs.rs:736-749`) drops the job in the hauler-loss branch. The branch is not gated on `!jobs.is_done`.
- Raiders wait at 60% of the leg inside the 6 km leash. They follow the hauler to the mark and kill it after `mark_done`.
- Result: the reward is lost and the board re-offers the job.
- A kill on the Hand in frame panics `apply_station_job_commands` (`crates/nova_interface/src/station/jobs.rs:484-486`).

## Fix direction
- Gate the drop on the done state: `else if !jobs.is_done(&job.id) && is_taken_out(&event, hauler)`.

## Done when
- A hauler that dies after its mark keeps the job done and the hand-in pays. The board does not re-offer it. A kill on the Hand in frame does not panic.
- Permanent proof: a unit test in `crates/nova_world_base/src/jobs.rs` that marks the escort done, takes the hauler out, and asserts the job stays done.

Found by the 2026-10-17 review of epic 20261012-100022.

## Fix 2026-10-17
- Claim: a hauler taken out after its mark no longer drops the done escort; the hand-in pays and the board does not re-offer it.
- Evidence: the drop branch sat in `escort_on_arrival` ungated (`crates/nova_world_base/src/jobs.rs:745-758` before). Reproduced by `done_escort_stays_done_when_the_hauler_dies_after_its_mark` failing at "a hauler killed at its mark does not lose the job".
- Change: `escort_on_arrival` marks done only. Every drop moved to the new observer `drop_on_loss` (`crates/nova_world_base/src/jobs.rs`), which skips a done job for a target loss. Registration in `crates/nova_world_base/src/lib.rs` (`app.add_observer(jobs::drop_on_loss)`) is left to the lane that owns that file.
- Verification: `cargo test -p nova_world_base --lib jobs::tests`: `done_escort_stays_done_when_the_hauler_dies_after_its_mark ... ok`, `an_escort_is_dropped_and_its_bodies_taken_out_when_the_hauler_dies_before_its_mark ... ok`. The interface panic is not reproduced here; with the job kept done the hand-in reads a held job.
