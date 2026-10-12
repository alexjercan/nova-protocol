# Jobs: job board on a station and job state in the save

- STATUS: OPEN
- PRIORITY: 63
- TAGS: v0.16.0,jobs

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
