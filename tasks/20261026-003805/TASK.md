# Make existing torpedo engagements more engaging without free ammunition

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.17.0

## User facts
- More engaging torpedoes/weapons, meaningful consequences and finite ammo. Specification only; no unapproved weapon type or default.
## Agent findings
- Torpedoes already have proportional navigation, terminal weave, contact fuze, cold ignition and thrust (`crates/nova_ship/src/sections/torpedo_section/projectile.rs:12-155,230-582`); rendered effects exist (`crates/nova_ship/src/sections/torpedo_section/render.rs:70-169,504-806`). One ammo item per shot is consumed on reload, not free (`crates/nova_ship/src/sections/ammo.rs:212-259,421-473`). New homing/weave alone duplicates existing behavior. No Nova player verdict is proved by external screenshake analogy (`tasks/20261025-214714/TASK.md:60`).
## Decisions needed
- Reproduce and review a real engagement before choosing one player-observable deficiency (aim, counterplay, impact or feedback); select scope, tuning, AI parity, costs and failure when target/stock is lost. No new projectile algorithm, FX layer or unapproved test.
## Delivery
- End: approved change improves a named torpedo decision/feedback in a combat flow without minting ammo or obscuring damage. Dies: superseded torpedo behavior/copy only; preserve guidance already doing its job. May break: balance, point defense, visuals/audio, target locks, frozen torpedoes and deterministic replays. Invalid authored torpedo IDs/stats fail lint/load; missing ammo/target has explicit behavior.
- Owner/entry/state/order: player trigger (`crates/nova_ship/src/input/player/weapons.rs:271-351`) -> torpedo projectile/arming/guidance -> damage/effects/HUD. Audit section configs/builders, save/thaw, AI defenses, UI and wiki/changelog. Coordinate reward-economy task; do not include new income rule here.
## Verification / Done when
- Preserve a seeded before/after engagement with ammo, hit/damage, defender reaction and inspected rendered impact frames; `nova-probe` for deterministic rules, `nova-bench` for player experience. Name and approve any permanent assertion first.
