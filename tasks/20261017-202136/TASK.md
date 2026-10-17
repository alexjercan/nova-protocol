# Jobs: a done escort is dropped when the hauler dies after its mark

- STATUS: OPEN
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
