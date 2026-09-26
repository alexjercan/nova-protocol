# Design station and inventory mechanics for the themed UI

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.15.0,stations,inventory,design

## User facts
- Continue iteration and research on stations, inventory and the mechanics offered by the new themed UI. The owner accepts PR #77's visual direction; its stock, credits, loot, trade and repair interactions are illustrative fixtures, not approved game mechanics.
- The themed UI becomes the player's normal TAB interface under the separate NOVA OS migration task. `:` remains the working advanced command entry. Do not reintroduce the terminal as the main station interface or preserve obsolete UI compatibility.

## Agent findings (recheck against live code)
- `20260824-125943/TASK.md` documents docking events and theme/UI seams but no station service, cargo/credits or persistent world mutation runtime. Free repair/reload and idle-refilling base ammunition currently conflict with costed station services. `20260925-190219` already researches a dock-salvage-to-physical-refit loop; `20260925-190156` owns the shared mutation/save contract. This task coordinates them, not duplicates or overrides their unapproved decisions.

## Research and decisions, not implementation approval
- Decide what a station is, how it gets a stable identity, who may dock, how visits and interruptions work, and which services should exist. Distinguish a real dockable station from a boarded ship/derelict, with partner-specific capabilities and refusal/rollback on undock, unloaded partner, broken port or missing stock.
- Specify inventory item/stack/content ownership, ship hold/capacity, source/sink and market stock, currency or no currency, buy/sell versus free salvage, repair/reload policy, and a physically observable ship-upgrade payoff. Contrast proposed loops with current free repair/reload and idle-batch ammo; choose what dies or stays before designing prices. Preserve simulation and content/lint boundaries.
- Define what basic map/ship/inventory views show while undocked and which real actions are dock-gated. Align station UI surfaces with the TAB migration without treating PR #77's example controls, numbers, assets or layout as production schemas. Coordinate persistence, consumed-body regeneration and service interruption with `20260925-190156`; defer implementation of saves until its owner-reviewed contract exists.
- Compare a minimum one-station/one-resource player loop with broader services; record prerequisites, risks, options, consequences and an owner-recommended first feature slice. Classify follow-on implementation slices rather than treating research as authorization.

## Verification to design
- Map each proposed transaction from trigger through ownership, atomic success/refusal and world mutation. Specify tests for wrong/absent dock partner, interrupted or broken-port transaction, insufficient stock/cargo/credits, duplicate salvage, retirement/revisit and a changed real ship. Save/restart claims depend on the separately approved persistence tier. Include an asserted player flow for the first loop, not UI-only counters.

## Done when
- An owner-reviewed mechanics specification chooses the minimum real station/inventory loop, its interface and content boundaries, error policy and dependencies; cross-task persistence and UI choices are linked; resulting implementation tasks have a code-backed gate. This task alone does not add runtime station, cargo or economy systems.

## Origin
- Split from `20260824-125943` after owner acceptance of PR #77's UI look on 2026-09-26. Coordinate the existing `20260925-190219` dock-salvage design and `20260925-190156` save contract instead of closing either by implication.
