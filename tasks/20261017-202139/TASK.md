# Jobs: a held job whose target or poster died can never be dropped

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.16.0,jobs,bug

## Evidence
- Only Escort has a drop path (`crates/nova_world_base/src/jobs.rs:651-701,762-793`).
- A held Recover whose wreck is destroyed, a held genuine Distress whose caller is destroyed, or any held job whose poster turns derelict is refused `NotDone` or `WrongStation` forever (`crates/nova_gameplay/src/jobs.rs:1164-1170`). Hand-in needs a living Jobs station (`crates/nova_interface/src/station/jobs.rs:171-181`).
- A held job pins its slot (`crates/nova_world_base/src/jobs.rs:204`). The poster never renews or expires that slot.

## Fix direction
- Option A: add drop paths on target loss and on poster death, like the Escort drop.
- Option B: add a player-side Drop action that releases the slot.

## Done when
- A held job whose target or poster is gone leaves the held set and its slot renews. The player is never stuck with a job that cannot be handed in.
- Permanent proof: unit tests in `crates/nova_world_base/src/jobs.rs` for a destroyed Recover wreck, a destroyed Distress caller and a derelict poster, each asserting the job is dropped and the slot is free.

Found by the 2026-10-17 review of epic 20261012-100022.

## Fix 2026-10-17
- Claim: option A. A held recovery whose wreck, a held genuine distress call whose caller, and every held job whose poster is neutralized or destroyed is dropped off the held set; its spawned bodies are taken out; the slot is unpinned and renews at its expiry like any unheld offer.
- Evidence: only Escort had a drop path (`crates/nova_world_base/src/jobs.rs:745-758` before). Reproduced by `a_held_recovery_is_dropped_when_its_wreck_is_destroyed_unless_it_is_done`, `a_held_genuine_distress_call_is_dropped_when_its_caller_is_destroyed_and_a_trap_is_done` and `every_held_job_is_dropped_when_its_poster_dies_done_or_not` failing before the change.
- Change: new observer `drop_on_loss` in `crates/nova_world_base/src/jobs.rs` over `onneutralized` and `ondestroyed`: a poster match drops every job of that poster, done or not; a hauler, wreck or genuine caller match drops an undone job. A done job keeps its hand-in. Registration in `lib.rs` is left to the lane that owns that file. No `Jobs` field was added, so the save format is unchanged; a dropped slot renews on its expiry (`post_station_boards`), not on the drop frame.
- Verification: `cargo test -p nova_world_base --lib jobs::tests`: the three tests above `... ok`.
