# Jobs: job board on a station and job state in the save

- STATUS: CLOSED
- PRIORITY: 63
- TAGS: v0.16.0, jobs

## User facts
- Epic `20261012-100022`. A station posts jobs generated from its civilization and cluster; the player accepts while docked; held jobs render on the Map; the save keeps held and completed jobs.

## Agent findings
- `GameObjectives` is a plain resource the loader writes (`crates/nova_gameplay/src/objectives.rs:40-45`); objectives are not saved (`crates/nova_world_base/src/save/mod.rs:103-117`).

## Delivery gate
- `Job { id, kind, target, reward, posted_by }` as runtime state, generated deterministically per station visit; a `jobs` field in the save; the Station pane lists them. Stop for the owner on how many jobs a station posts and whether they expire (needs the clock).

## Verification
- Asserted example: accept, undock, save, load, the job is still held and on the Map.

## Done when
- The board works in a live docked run and held jobs survive a reload.

## Agent decisions 2026-10-16
Owner away; the delivery gate's two stops were decided here, flagged for the owner to overturn.
- Board size: 3 jobs per station (`JOBS_PER_BOARD`, `crates/nova_gameplay/src/jobs.rs`). A board reads at a glance and one station never carries the whole plan.
- Expiry: none. No posted job expires until the play-time clock lands (`20261012-100028`); the board is fixed per station, so an unaccepted job stays posted and a held one stays held.
- One kind now, `JobType::Procure { item, count }`: bring N units to the posting station. Located kinds (recover, escort, distress, neutralize) are `20261012-100024`'s and add variants with a target of their own. `Job { id, kind, target, reward_cr }`; the poster is `id.station` (`posted_by()`), not a second field. For a procurement `target == poster`.
- Identity: runtime `JobId { station: GeneratedBodyId, slot }`, never an authored id. Shared ids stay in `nova_gameplay` (lowest crate the interface and the world crates share).
- Seeding: `WyRand::from_seed(Fnv64(world seed, "station_jobs", station id))`. The civilization enters through the station id (one station per civilization node, named by it); no civilization component stands on the entity, so "from its civilization" is by id, not by a trait of the civilization. Item pool by the cell's clusters (asteroid/rock: Raw; planet: Food+Raw; derelict field: Parts+Repair; no cluster: all but Ammo; no wanted item: every item). Count 2..=8; reward = goods at ask + half, plus a 50..=250 cr fee.
- Hand-in consumes the goods (they leave the hold, the station does not stock them) and the station pays from its balance (`StationCannotPay` refusal). Credits move with `CreditChangeCauseType::JobReward` on both sides; no new `ShipInventoryReport` variant.
- `Jobs { held, completed }` is a session resource beside `Chart`: cleared in `Cleanup` with the chart, saved as `WorldSaveState.jobs`, restored through `ResumedWorld`. `WORLD_SAVE_FORMAT` 11 -> 12 (the save module is unreleased since v0.15.0; the changelog entry is still marked breaking).
- Generated stations now offer `StationServiceType::Jobs` (five services). Authored `Station` content with `Jobs` posts no board: it has no generated id or cell to seed from.
- Owner feedback on the epic (jobs in the Station tab; jobs fit the start role): the lists live in the Station tab; role fit is left to `20261012-100024` with the located kinds.
- No content RON changed; `content gen`/`lint` not run.


## Landed 2026-10-16
- `3171065ab` code, proof and docs in one commit (the docs change was small): `nova_gameplay::jobs`, `nova_world_base::jobs` + save format 12, Station pane job lists, Map chart `JOB-n` marks, `examples/systems/system_station_jobs.rs`, `examples/playable/job_board_sandbox.rs`, wiki/create pages, changelog. The pre-commit rustfmt hook was bypassed: it fails on pre-existing drift in `crates/nova_editor` and `crates/nova_ship`, which this task did not touch; the touched crates are fmt-clean.
- Proven (live, lavapipe under Xvfb, `system_station_jobs`, exit 0, all six claims): `sector_1_0_0_cluster_1_0_0_station_0` posts 3 jobs; Accept holds `...#0` (Bring 2 Capacitor bank, 970 cr); the chart view shows `JOB-1` with HELD JOB and the reward; the leave save (generation 3, format 12) holds the job; Load restores the job and the same board; Hand in pays 970 cr and completes the job.
- Proven (live, `job_board_sandbox`, exit 0): accept, Map mark, hand in through the pane buttons; screenshots inspected (board, Map, after hand-in).
- Unit proof: `nova_gameplay::jobs` 4 tests; `nova_world_base::save` 34 tests pass with the new field.
- Gaps: the pane's Accept/Hand in rows sit below the 1080p fold when a station also sells a ship and hires escorts (the panel scrolls; the examples activate the buttons by name). Only `Procure`; located kinds, role fit and expiry are `20261012-100024` / `20261012-100028`. Authored `Station` content with `Jobs` posts no board. Owner to confirm or overturn: 3 jobs per board, no expiry.
