# Jobs: job-spawned raiders are offered as Neutralize targets on neighbouring boards

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.16.0,jobs,bug

## Evidence
- `job_field` (`crates/nova_world_base/src/jobs.rs:315-326`) takes every Enemy intact hull under the 27 roots.
- Escort raiders and the trap caller qualify (`jobs.rs:453`, `jobs.rs:573`). They are children of the poster's root (`jobs.rs:459`, `jobs.rs:578`).
- `taken` (`crates/nova_gameplay/src/jobs.rs:875`) excludes only `located_body()`, not `raider_ids()`.
- Renewal (`crates/nova_world_base/src/jobs.rs:212-218`) and hauler loss (`jobs.rs:744-749`) `try_despawn` them with no `onneutralized` or `ondestroyed` event. The neighbouring Neutralize is never done and cannot be dropped (see the held-job drop task).

## Fix direction
- Exclude every held job's `spawned_bodies()` from the field. Alternative: exclude any id with the `_job_` stem.

## Done when
- A neighbouring board never offers a job-spawned raider or trap caller as a Neutralize target.
- Permanent proof: a unit test in `crates/nova_world_base/src/jobs.rs` that holds an Escort with raiders and asserts the neighbouring field excludes them.

Found by the 2026-10-17 review of epic 20261012-100022.

## Fix 2026-10-17
- Claim: a neighbouring board never offers a held job's spawned bodies: an escort's raiders are not Neutralize targets and a genuine caller is not a wreck to recover.
- Evidence: `job_field` took every Enemy intact hull and every stocked derelict under the 27 roots, less the renewal's `retired` set only (`crates/nova_world_base/src/jobs.rs:280-333` before). Reproduced by `a_held_job_s_spawned_bodies_are_off_the_neighbouring_field` failing with four hostiles and two wrecks in the field.
- Change: `job_field` takes `held: &Jobs` and excludes every held job's `spawned_bodies()` with the retired set; `post_station_boards` passes the player's `Jobs`. `crates/nova_gameplay/src/jobs.rs` is unchanged.
- Verification: `cargo test -p nova_world_base --lib jobs::tests`: `a_held_job_s_spawned_bodies_are_off_the_neighbouring_field ... ok`.
