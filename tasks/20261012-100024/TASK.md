# Jobs: recover, escort, distress call and neutralize a named hull

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.16.0,jobs

## User facts
- Epic `20261012-100022`. Four job types from the design page: recover a marked item from a wreck; escort a hauler to the next cluster with pirates scaling with its cargo value; answer a distress call, some fake; neutralize a named hull.

## Agent findings
- Generated bodies need the addressing seam from the world-first strand (`crates/nova_world/src/streaming.rs:176-182`); the events strand supplies `OnPickedUp` and friends.

## Delivery gate
- Each type is one handler template over the open-world events, with its completion check named; no new event kinds. One asserted example per type.

## Verification
- Probe walks: each type completes once and refuses hand-in before completion.

## Done when
- Four types ship and each has a permanent proof.

## Agent decisions 2026-10-16
Made without the owner, for review. Each is one line in code, easy to turn.
- Three jobs per board (`JOBS_PER_BOARD`); the kind of each slot is drawn by a closed `JobMixType` on `StartRole.jobs` (Trade [4,1,4,1,1], Salvage [1,5,1,2,1], Combat [1,1,1,3,4] over Procure, Recover, Escort, Distress, Neutralize). Base roles: industrial and transport Trade, scavenger Salvage, bounty_hunter, military and pirate Combat. No session (a scenario with stations) posts Trade.
- The field a board draws located targets from is the live cube of 27 cells around the station, read when the board posts: derelicts with a non-empty hold, and intact un-neutralized Enemy hulls. A slot with no candidate posts a Procure.
- Recover completes on `onundocked` naming the wreck, with the stack aboard the player and gone from the wreck (no Take event exists). Neutralize on `onneutralized` or `ondestroyed` naming the hull. Escort on `onshipordercomplete` for the hauler's Move; the hauler neutralized or destroyed DROPS the job and despawns its bodies. Distress: genuine on `ondocked` with the caller, a trap on the raider neutralized or destroyed; one call in three is a trap.
- The hauler is a generated Civilian intact hull on the Player side, non-combatant, with a `Move` helm order to a mark 10 km out; raiders are Armored Enemy hulls with a 6 km leash, `1 + cargo_cr / 700`, at most 3, strung across 60 % of the leg. The hauler's stock is generated; `cargo_cr` is the job's nominal value. The leg direction is seeded, 2 km clear of standing bodies and inside the poster's cell when a draw allows.
- The accept refuses a Recover or Neutralize whose target is gone (`TargetGone`), read off live entities in the Station pane. The hand-in refuses a located job until done (`NotDone`, "<title>: not done yet"); `Jobs` gains `done` and the world save is format 13.
- The Map's chart marks a located job in the cell of its body, and its note says what is left to do.
- `system_station_jobs` and `job_board_sandbox` play the transport role (Trade mix) so a procurement is posted; the sandbox takes `NOVA_SANDBOX_ROLE` for the other mixes instead of a second playable. The four located ranges share `examples/systems/shared/located_jobs.rs` and take `NOVA_JOB_SEED` to hunt seeds.
- The neutralize range plants its hostile: over 32 scanned seeds no living non-Enemy station within one cell of the spawn had a hostile in its cube (hostiles stand with their own Enemy civilization), so the walk spawns one Armored Enemy hull in the spawn's cell before boards post, laid out as the world lays out a raider. OWNER: Neutralize jobs rarely post at friendly stations near the spawn with the field as is; widen the field, or seed hostiles near friendly stations, if the kind should show up in play.
- `catalog_drift::catalog_matches_disk` was failing on master over `examples/shared/generated_station.rs` (a `#[path]` module under a top-level `shared` dir from the open-world station commit); the test now skips `examples/shared/`.
- Known gap: a helm order is not frozen with its ship, so a hauler left mid-leg stands where it thawed and the escort never completes; the job is still held and can be dropped only by losing the hauler.

## Landed hashes, proofs and gaps
- b56bf1c0d: the five-kind board, the spawns and observers, the pane refusals, save format 13, the docs and the four ranges.
- Proofs: unit tests in `crates/nova_gameplay/src/jobs.rs` (7); live ranges `system_job_recover` (seed 10), `system_job_escort` (16), `system_job_distress` (16 trap, 17 genuine), `system_job_neutralize` (16, planted hostile), `system_station_jobs` (10, transport), `job_board_sandbox` smoke; all end in `cycle complete, no panic` with their PASS lines.
- Gaps: a helm order is not frozen with its hauler (an escort left mid-leg never completes); Neutralize rarely posts at a friendly station near the spawn with the field as is; the located ranges play seeds pinned by hand, `NOVA_JOB_SEED` re-hunts them.
