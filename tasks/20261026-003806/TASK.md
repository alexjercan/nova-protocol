# Make job rewards and ammo expenditure support another sortie

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.17.0

## User facts
- Quest ammunition can cost more than reward and exhaust progression; sustainable but not free ammo/reward loop with consequences. Specification only.
## Agent findings
- Initial roles vary 1000-2200 credits; four armed roles have 1000 PDC rounds, military has two torpedoes (`crates/nova_authoring/src/base_content/start_roles.rs:31-179`; `tasks/20261025-214714/TASK.md:37`). Idle reload draws finite inventory stock (`crates/nova_ship/src/sections/ammo.rs:212-259,421-473`); station jobs already offer/renew three slots (`crates/nova_gameplay/src/jobs.rs:89,1198-1409`). `crates/nova_authoring/src/base_content/market.rs:35-83` owns baseline restock and offer windows. No ammo-vs-payout encounter has been reproduced (`tasks/20261025-214714/TASK.md:43`).
## Decisions needed
- Reproduce real mission with credits, ammo before/after and alternatives. Choose eligible combat jobs, payout/pricing/supply/loot lever, finite repeatability and failure when out of ammo or unable to buy; do not invent a minimum reward, guaranteed ammo, rate or free refill. Coordinate event jobs and salvage tasks.
## Delivery
- End: an attainable next sortie after a reasonably executed combat job, with clear resource trade-offs; bad outcomes still matter. Dies: only approved obsolete price/reward rule or misleading UI. May break: economy, job progression, start role balance, market stock and saves. Invalid authored item/job/reward fails lint/load; shortage shown honestly, not silently subsidized.
- Owner/entry/state/order: authoring roles/market -> `nova_gameplay::jobs` reward/ledger -> station buy/Jobs UI (`crates/nova_interface/src/station/station_buy.rs:144-186`) -> weapon reserve/reload. Audit generated content, reward/stock ID consumers, save, wiki/changelog.
## Verification / Done when
- Record before/after `nova-bench` combat job flow with credits, each ammo item spent, purchase/loot and next job availability; assert conservation on approved stable rules. Do not grade exit zero alone; no permanent test until approved.
