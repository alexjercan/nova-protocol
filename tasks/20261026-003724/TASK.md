# Make wreck rewards collectable within honest canister and hold limits

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.17.0

## User facts
- Defeated enemy can give no reward; improve capacity/stacking or fit clarity and physical pickup. Keep inventory constraints, no free loot. Specification only.
## Agent findings
- `strip_parts_at` floors fractional recipe yield (`crates/nova_gameplay/src/inventory.rs:1534-1551`); wreck drop is `crates/nova_scenario/src/wreck_cargo.rs:61-159`. Whole canister refused if insufficient hold (`crates/nova_ship/src/sections/cargo_intake_section.rs:511-646`), pickup readiness exists (`:178-185`). Existing cap 200 kg (`tasks/20261025-214714/TASK.md:39`; `tasks/20260926-174806/TASK.md:54`). This is a possible cause, not reproduced encounter.
## Decisions needed
- Reproduce empty drop and failed pickup first with artifacts. Choose whether carried stock, recipe floor, reward, canister cap, stacking/splitting or clear refusal changes; ownership, capacity, collision, overflow, persistence. No new size/guarantee/default approved.
## Delivery
- End: defeated ship -> identifiable reward -> fit and collect or understandable whole-fit refusal with goods preserved. Dies: only superseded cap/yield/UI after approval. May break: item mass/conservation, station stripping, trade, jettison, save/thaw and physics. Unknown item/invalid cap fails lint/load; full hold fails clearly.
- Owner/entry/state/order: `wreck_cargo` -> `inventory::strip_parts_at` -> `cargo_intake_section` -> pickup HUD/inventory. Audit item builders, generated content, runtime IDs, save, UI, wiki/changelog and all cap consumers.
## Verification / Done when
- Reproduced before/after encounter; asserted real drop/intake, conservation and whole-fit refusal across freeze/thaw; inspected pickup frames; `nova-bench` defeat -> salvage -> spend. Approve permanent test first.
