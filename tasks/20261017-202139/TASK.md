# Jobs: a held job whose target or poster died can never be dropped

- STATUS: OPEN
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
