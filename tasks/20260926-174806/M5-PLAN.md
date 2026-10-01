# M5 economy delivery plan

- STATUS: proposed implementation plan, not an approved item-ID/mining interface.
- Trade (owner-approved 2026-09-30): any docked live ship trades - neither neutralized nor lootable; no trader marker or station market. Buy and Sell use fixed per-item ask/bid against the partner's finite real inventory and `credits`. Every authored ship states `credits`; ordinary live ships, hostile ones included, hold a provisional 2000 and derelicts 0. A funded trader exists only in a controlled rendered demo fixture, never in New Game or `open_world`; that fixture needs its own gate.
- Stack: `item-loop-m5` branches from `item-loop-m3` at `16da6afbc`. PR target is `item-loop-m3`. Do not modify existing PRs or the separate regional-ship-prototype worktree.

## End state and boundaries

- The player mines ore with a short-range mining-only action, collects ore in physical cargo canisters, and buys or sells real goods for credits with a docked live ship's finite inventory and credits. Stock, credits and mined ore change atomically; refusals change nothing.
- Remove no established M0-M3 ownership or stock paths. Combat carving must never mint ore, mining must not work as a melee weapon, and thrust/RCS and food have no survival consumption.
- Defer the former M4 refit to `tasks/20260930-100908/TASK.md`, mod-added item identity to `tasks/20260930-100831/TASK.md`, and save/revisit guarantees to `tasks/20260925-190156/TASK.md`. State M5's session-only limit until a persistence tier is approved.

## Sequence and code-backed gate

1. Map the existing owner/callers before changes. Inventory and closed `ItemType` are in `crates/nova_gameplay/src/inventory.rs` (`ItemType:37`, `ShipInventory:182`, `CargoCanister:363`); queued physical drops are in `crates/nova_ship/src/sections/cargo_intake_section.rs:154`; carving is in `crates/nova_gameplay/src/integrity/carve.rs:314`, with dust effects in `integrity/spew.rs:741`. Inspect actual station, open-world rock, mining input, ship controls, UI and authoring paths rather than assuming a market/miner exists.
2. Show exact paths/lines, current types/functions, proposed signatures and fields, before/after call graphs, ordering and transaction ownership. Gate item IDs and per-unit mass, ore-body selection, range/yield/depletion, credits and price units, market identity/stock/replenishment, station authorization, transaction quantity/error policy and UI before introducing types/functions/tests. Use the simplest explicit first-slice values supported by content and render proof; fail at lint and load for invalid authored IDs or stocks.
3. Change the owning interface first, then update all authored Rust builders, runtime ID consumers, inventory/UI/commands and docs; generate, lint and run content. Keep the closed base `ItemType` until the separate mod-ID task.
4. Prove pure accounting and atomic refusal; prove ECS mining yields only from eligible ore and conserves depletion, a combat carve yields nothing, and a real trade exchanges exact goods and credits with capacity, stock, credit and derelict checks. Asserted gameplay states and a rendered buy/mine/pickup/sell loop in the controlled demo fixture are required. Do not use exit zero or UI counters alone as proof.
5. Review blast radius and affected tests, format, content lint/generation, rendered frames and PR CI. Only publish a stacked PR after independent review; report skipped checks and session-only caveats.

## What may break; what must fail loudly

- Adding goods affects inventory/canister mass accounting, capacity lint, inspector, jettison, reload mapping, cheat item IDs and every authored fixture. An unknown item, missing or unrepresentable `credits` or invalid stock must fail at lint, then load; a refused trade changes nothing; no invisible free purchase, ore yield or silent inventory fallback.
- Sector retirement may recreate ore, ship stock or credits before the separate persistence task lands. Do not claim claim-once mining or saved balances from the M5 PR.
