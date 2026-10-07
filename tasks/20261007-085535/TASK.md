# Plan v0.16.0 as a gameplay-polish sprint

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.16.0,planning

## User facts

- Focus v0.16.0 on making the existing game enjoyable and reliable to play, not adding a broad new feature set.
- Triage the open `backlog` tasks first. Identify what can be dropped, kept, or scoped to research; do not silently close tasks in this planning pass.
- Favor open-world save/load persistence and open-world modding, including items and other identities currently hardcoded as Rust enums.
- Delete the two ship-tuning tasks (including the spike) and the reputation/diplomacy task entirely. Crafting is not a goal; trading raw materials for processed goods can fill its role.
- Keep keybind and mobile virtual-pad work in backlog for later, without making them v0.16.0 commitments.
- Reassess `20260923-110247` (generated-world data structures) as a read-only scout to determine whether work is still needed.

## Decisions to make

- Which backlog tasks are stale or duplicated, and which are worth retaining? Present evidence and a proposed disposition before changing their status.
- What exact persistence boundary makes New Game resumable, and what breaks or is intentionally reset? Define this before implementation.
- Which currently fixed open-world identities and rules must become moddable first? Keep a bounded vertical slice instead of promising every world system.
- Which shipped features are core, supporting, or secondary, and how mature is each player journey? Inventory this before ranking polish work.
- Which concrete player-flow bugs, usability issues, and performance risks deserve priority over feature additions? Validate from code and existing evidence; propose focused player proof.

## Agent findings (read-only, not approved designs)

- New Game inserts `OpenWorldSession { seed }` in `crates/nova_menu/src/world_setup.rs:216-230`; `nova_world_base/src/lib.rs:27-30` promises only pristine same-build/seed generation and explicitly loses changed sectors. `crates/nova_world/src/streaming.rs:870-893` retires individual sector roots; separately, `crates/nova_scenario/src/loader/lifecycle.rs:63-130` sweeps scenario-scoped entities on scenario load/unload. Sector retirement and Retry are different paths, though both discard unsaved state.
- `crates/nova_world/src/streaming.rs:165-175,228-286` uses generated object IDs but excludes scenario-addressable markers. Persistence needs an explicit durable identity contract across sector regeneration, code/catalog changes, and mods; do not assert there are no generated IDs.
- Settings/mod preferences already use native atomic key-value storage and web string storage (`crates/nova_assets/src/storage.rs:194-249`), not a gameplay-save schema. A durable save must define player ship/inventory/credits, sector mutations, save points, write failures, and catalog mismatch before code.
- `ItemType` is a closed ten-variant enum with Rust-owned mass, prices, category, and label (`crates/nova_gameplay/src/inventory.rs:39-150`); the earlier M5 decision kept it only for the first economy slice (`tasks/20260926-174806/TASK.md:45-52`). A v0.16.0 redesign must replace the owning interface and update real inventory/canister/trade/UI consumers, not introduce an unused parallel registry.
- Player section repair already exists in `crates/nova_interface/src/ship/app.rs:157-262` and `crates/nova_interface/src/ship/sections.rs:399-476`; an earlier scout citing a pre-M0 task as evidence that repair is absent was stale. Do not list repair implementation as a missing v0.16.0 feature.
- `ShipRoleType` is also closed (`crates/nova_world/src/generation.rs:161-170`), but opening generated-ship roles reaches layout, parts, and civilization weights. `ShipDesignId` already resolves through a mod-mergeable catalog (`crates/nova_scenario/src/objects/ship_design.rs:46-65`); do not equate all world content with hardcoded enums.
- Cargo pickup requires physical intake overlap and free mass, not only proximity; the pilot manual names cargo delta as proof (`crates/nova_bench/src/pages/cargo.md:10-31`). The numeric transfer field lacks a named bench UI target. Both are candidate journey/UX checks, not reproduced gameplay defects.
- `tasks/20260926-132243/TASK.md:38-39` says one docking collision case was fixed, while the original intermittent run was not replayed. Reproduce the original case before changing docking.

## Proposed backlog disposition (no status changes)

`tatr ls --filter ':status eq OPEN and :tags contains backlog'` lists **nine** tasks on this checkout; these are the entire currently OPEN backlog-tagged set, not all OPEN tasks:

| Task | Proposed v0.16.0 disposition | Basis |
| --- | --- | --- |
| `20260925-190156` | Keep: first persistence design gate | Seed regeneration drops modifications; choose durable state and mismatch/failure policy. |
| `20260930-100831` | Keep: second item-identity design gate | Closed `ItemType`; coordinate catalog identity with saves before migrating real consumers. |
| `20260923-110247` | Scout only when persistence/modding touches generated records | Its task already defers standalone refactoring; current separate descriptions/payloads warrant inspection, not speculative cleanup. |
| Ship-tuning spike and implementation, reputation/diplomacy (three tasks) | Delete as the owner requested | No longer candidate work, including the ship-tuning spike; preserve historical decisions in completed research artifacts without live task links. |
| `20260714-001140` | Defer | Hardware gamepad navigation and L2 conflict remain open; no v0.16.0 commitment. |
| `20260831-145917` | Defer | Mobile pad depends on unresolved gamepad layout. |
| `20260908-161328` | Keep unscheduled backlog | Season-one story is intentionally backburnered, unrelated to this gameplay sprint. |

The nine-task listing was taken before the owner's deletion decision. After deleting those three tasks and moving persistence/item research to v0.16.0, four OPEN backlog-tagged tasks remain. Do not close any other task based only on release-scope exclusion; older-release bug/CI tasks were not all reviewed. The docking report `20260926-132243` is already CLOSED; the owner now reports no recurrence and cannot reproduce the old issue.

## Proposed sprint sequence (owner review required)

1. **Feature and maturity inventory first** (`20261007-090751`). Classify shipped mechanics by core/side role, player-visible maturity, friction and evidence. Use it to select journeys for a bounded polish research task (`20261007-090722`). The old docking report is resolved for now; do not make a speculative docking fix.
2. **Real world persistence** (`20260925-190156` research, then `20261007-090756` implementation). New Game must resume a selected world after both sector retirement and game restart, with improved Create World and Load/Continue UI. Decide save points, Retry, ownership, player ship/cargo/credits and sector deltas, catalog/missing-mod handling, and visible write/load failures before designing a schema. Existing generated IDs are not yet a cross-build save contract; scope to New Game, not every scenario.
3. **Measured polish.** Preserve audits and rendered before/after evidence for important feature flows and prioritize reproduced friction. The inventory quantity control's missing named target (`crates/nova_bench/src/pages/cargo.md:22-31`) is a research lead, not yet a gameplay bug.
4. **RON-authored items last, research only for now** (`20260930-100831`). The owner has not selected item schema, migration, and mod behavior. Collect examples and spikes before deciding whether/when to replace `ItemType` through real inventory/canister/trade consumers. Defer implementation; keep generated ship-role/civilization opening separate. Gamepad/mobile and standalone generated-world cleanup are not sprint commitments. Ship tuning/reputation/crafting are removed from this sprint.

Nothing above authorizes code or backlog-status edits. The specific persistent-state tier, UX/save failures, identity contract, and proof budgets need the owning implementation gates before work starts.

## Delivery

1. Inventory OPEN `backlog` tasks, read candidate task bodies, and prepare a drop / defer / scout / keep shortlist with reasons. Leave v0.14.0 Steam/store tasks out of this sprint.
2. Map the current New Game/open-world boot, world and inventory ownership, mod loading/identity boundaries, and save/load state. Identify code-backed gaps and risks.
3. Recommend a bounded v0.16.0 sprint with a sequenced persistence and modding plan plus small polish/bug slices. State exclusions, dependencies, and verification for each; seek owner choices before changing existing tasks or implementing features.

## Done when

- The owner can review a code-backed backlog triage and a prioritized v0.16.0 plan, with explicit scope cuts and proof proposals.
- Follow-up work is tracked in the linked tasks. The three named task deletions are explicitly authorized; no other gameplay changes, backlog closures, release date, or tag are authorized here.
