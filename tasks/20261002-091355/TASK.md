# Seed generated ships with cargo, credits, and optional industrial mining beams

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.15.0,world,ships,inventory,design

## Goal and sequence

Add a small PR on top of the regional world-ship stack after its rebase. Give generated free-mode ships meaningful starting goods and credits, let the player take credits from eligible wrecks/neutralized ships, and let some generated industrial hulls carry real Mining sections. This PR precedes encounter work in `20260925-190131`; review the complete stack before merging. Static-well gravity and trajectory work follow the stack. No implementation, commit, or PR is established by this task.

## Owner decisions from the starting-loadout grilling session

- Include both intact generated ships and generated derelicts. All four roles have nonzero items and credits even at minimum advancement. Use the civilization's existing seeded advancement value (which derives from 3D distance with variation), not each ship's raw distance or a second advancement curve. Keep selection deterministic by stable ship identity and effective pinned content, independent of visit order. No pickup-time reroll.
- Use existing ItemType IDs and role-specific mixes: industrial favors ore/parts; civilian favors rations/parts; scavenger/armored favor ammo/plates/parts. All three ammo types are eligible on armed ships even if their installed weapons cannot fire them; none is guaranteed when it cannot fit. Do not mint magazine contents as reserve or invent new item IDs.
- Target starting cargo as a seeded fraction of each ship's **actual** hold capacity: prototype intact target bands 5-15% at advancement 0, rising to 20-40% at advancement 1. Wreck targets are about half those bands. These are targets, not exact-fill promises: item granularity and actual pruned wreck capacity govern what fits. Preserve nonzero stock at low advancement, refuse capacity overflow, and inspect the resulting distributions before calling them balanced.
- Generate wreck loot independently from the same-identity intact loadout, against the pruned wreck's remaining hull capacity. Replace the existing guaranteed 1-8 HullPlate baseline with a mixed role-specific stock policy: a wreck may contain no HullPlate. Do not silently keep the old plate guarantee. Intact starting stock and wreck stock must both be validated before materialization.
- Credits use a separate seeded curve independent of hold size and the same bands for all four roles. Prototype intact balances range from 50-200 cr at advancement 0 to 500-2,000 cr at advancement 1; wreck balances are 10-25% of the corresponding intact role-independent budget, with a positive low-tier floor. These are prototype balance inputs, not final prices. Keep arithmetic bounded and account for credits when replacing a sector.
- Add a docked **Take credits** action for a derelict or a neutralized partner. One confirmation moves the partner's full balance atomically into the player ship and leaves the partner at zero; refuse overflow without partial transfer. Never permit theft from an intact, non-neutralized ship. Ordinary intact ships retain the existing trade path; item Take/Give and Buy/Sell remain separate. A neutralized ship's existing credits become available without rerolling its loadout.
- Mining equipment is optional and restricted to generated industrial roles. Prototype selection chance rises from 25% at minimum advancement to 75% at maximum. Selected lower-tier hulls can carry one real usable beam; higher-tier hulls may carry up to two, so mining output is real, not just a cosmetic mesh. Draw from valid eligible base and mod Mining sections under the existing canonical source/advancement rules. Require valid mounting and clear emitter lanes; a derelict retains any installed beam but its active systems remain disabled. Do not equip civilian, scavenger, or armored hulls with Mining sections.

## Current source findings (regional PR stack, before this change)

- `crates/nova_world_base/src/sector_ships.rs::plan_ship` draws a role and hull using civilization advancement; intact stock is empty and derelict stock uses `wreck_stock` with HullPlate. Its seed key already includes world seed, cluster node and hull slot.
- `crates/nova_world/src/generation.rs::SectorShip` carries stock but not a credit balance. `crates/nova_world/src/streaming.rs::spawn_sector_ship` passes stock and lootability to the scenario ship spawn, but does not explicitly author credits.
- `crates/nova_world_base/src/ship_parts.rs::ShipPartFamilyType::of` excludes `SectionKind::Mining`. `crates/nova_gameplay/src/inventory.rs` holds `ShipCredits`, ItemType masses and the existing item-transfer/trade plans. `crates/nova_interface/src/inventory/app.rs` currently chooses Take/Give for a neutralized/lootable partner and Buy/Sell for a trader; it has no credit-theft action.
- `20260925-190156` owns persistent ownership/claim-once. Until then, retiring and revisiting a sector can regenerate a pristine ship and its starting stock/credits. State this limit in player-facing claims; do not claim persistence or a one-time economy.

## Code-backed implementation gate (still required)

Before code edits, review the post-rebase exact paths/lines, existing and proposed types/fields/functions/signatures, before/after call graph, catalog and mod-validation ownership, UI and docs consumers, and the closest asserted/rendered proof. Resolve how optional Mining selection behaves when no eligible beam or legal lane exists, exact cargo sampling/rounding and positive-item floor against tiny holds, credit-range interpolation and integer rounding, and UI placement/refusal for Take credits. Fail loudly for malformed authored content; do not hide selected unbuildable layouts with a fallback. Get owner approval for any new type/function/test before implementing. Preserve the active regional rebase worker's files until handoff.

## Verification and done when

- Deterministic repeated seed/node/slot and visit-order tests show the same stock/credits; low/high advancement diagnostics report role mixes, item masses, credits, Mining frequency, and named exclusions across several seeds and hull sizes. Actual stock never exceeds resolved or pruned hold capacity; each eligible low-tier ship has nonzero goods and credits.
- Assert wreck mixed loot can have no plates; intact/non-neutralized partners cannot be robbed; docked neutralized/lootable partners can be robbed once for their full balance; repeated action or overflow refuses without duplication/loss. Confirm existing item Take/Give and trade remain intact, including exact-credit and exact-capacity boundaries.
- Prove one/two installed Mining sections under role/advancement/base+mod selection and legal lanes, absent on other roles, disabled on derelicts; inspect rendered low/high industrial ships and an actual mining action for a controllable sample. Update content lint/load contracts, docs and changelog with the code. Run affected checks, not workspace-wide tests by default.
- Deliver as a separately reviewed stacked PR before `20260925-190131`. Do not call this task closed from design approval or a published PR without source, focused proof, rendered proof, review and current-head checks.
