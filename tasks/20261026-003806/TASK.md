# Make job rewards and ammo expenditure support another sortie

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.17.0

## User facts
- Quest ammunition can cost more than reward and exhaust progression; sustainable but not free ammo/reward loop with consequences.
- The owner authorized a balance investigation on master with Claude workers: run source-backed tests/experiments, create focused examples if useful, and deliver an HTML report with inspected isolated-display screenshots and references to the examples. The report proposes ammo prices/rewards from evidence; it does not authorize changing shipped balance values yet.
## Agent findings
- Initial roles vary 1000-2200 credits; four armed roles have 1000 PDC rounds, military has two torpedoes (`crates/nova_authoring/src/base_content/start_roles.rs:31-179`; `tasks/20261025-214714/TASK.md:37`). Idle reload draws finite inventory stock (`crates/nova_ship/src/sections/ammo.rs:212-259,421-473`); station jobs already offer/renew three slots (`crates/nova_gameplay/src/jobs.rs:89,1198-1409`). `crates/nova_authoring/src/base_content/market.rs:35-83` owns baseline restock and offer windows. The earlier review had no reproduced ammo-vs-payout encounter (`tasks/20261025-214714/TASK.md:43`); the measured escort experiment below supersedes that gap. Initial read-only audit: `PdcRound` ask 4 cr, `RailSlug` 40 cr and `Torpedo` 400 cr (`crates/nova_authoring/src/base_content/items.rs:45-85`); Neutralize pays 400 cr plus twice a 50-250 cr fee (`crates/nova_gameplay/src/jobs.rs:1256-1337`). Escort pays half 300-2400 cr cargo value plus one fee (200-1450 cr), not two fees. These are source calculations, not observed expenses; Buy prices vary by station stock and role (`crates/nova_gameplay/src/market.rs:367-412`).
## Experiment: seed-2 Escort, 2026-10-30
- Evidence and offline HTML report: [`REPORT.html`](REPORT.html), `evidence/run{1,2,3}/` (timelines, logs, hashes, three inspected isolated-X screenshots). Measurement example: `examples/systems/system_job_ammo_economics.rs`; catalog: `Cargo.toml`; invariant roster: `crates/nova_probe_cli/tests/catalog_drift.rs`. The example and report are balance research, not shipped economy changes.
- Matched capture-off runs 2/3 paid 390 cr, consumed 1057/136 PDC rounds, and quoted 4 cr per round. Full replacement would cost 4228/544 cr; the real Buy paid 756/544 cr for 189/136 rounds. Run 2 was stock-capped with 868 rounds not replaced. Run 1 used captures, a different renderer, and an incompletely recorded launch environment: 568 rounds, 390 cr reward, 189 bought for 756 cr. Do not average these runs or assert a cause for their spread.
- The job completed with the raider surviving in every run. The starting military hull carries 2000 usable PDC rounds, so one measured run does not demonstrate that another sortie is impossible. Berthing uses fixture moves; flight, weapon fire, hand-in and Buy use live systems. This does not cover other jobs, human aim, torpedo spend, repairs, other station prices or future restock.
- Focused example check and `catalog_drift` 2/2 passed; each timeline ends in successful probe markers. The recorded run-2 process exit code is missing. No gameplay price or reward change is authorized by this experiment.
## Decisions needed
- Reproduce real mission with credits, ammo before/after and alternatives. Choose eligible combat jobs, payout/pricing/supply/loot lever, finite repeatability and failure when out of ammo or unable to buy; do not invent a minimum reward, guaranteed ammo, rate or free refill. Coordinate event jobs and salvage tasks.
## Delivery
- End: an attainable next sortie after a reasonably executed combat job, with clear resource trade-offs; bad outcomes still matter. Dies: only approved obsolete price/reward rule or misleading UI. May break: economy, job progression, start role balance, market stock and saves. Invalid authored item/job/reward fails lint/load; shortage shown honestly, not silently subsidized.
- Owner/entry/state/order: authoring roles/market -> `nova_gameplay::jobs` reward/ledger -> station buy/Jobs UI (`crates/nova_interface/src/station/station_buy.rs:144-186`) -> weapon reserve/reload. Audit generated content, reward/stock ID consumers, save, wiki/changelog.
## Verification / Done when
- Record before/after `nova-bench` combat job flow with credits, each ammo item spent, purchase/loot and next job availability; assert conservation on approved stable rules. Do not grade exit zero alone; no permanent test until approved.
