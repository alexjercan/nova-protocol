# Design one dock-salvage-physical-refit loop

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,stations,design

## User facts
- The owner wants approachable themed click-based station, inventory, map and ship views; NOVA OS stays for advanced and cheat commands. Mining/salvage, supplies, trading, repair, ammo and *visible physical* ship improvement are candidate mechanics; no skill tree or crew simulator.

## Agent findings
- Docking and OnDocked exist, but no station service, cargo, credits or ship mutation save exists (`tasks/20260824-125943/TASK.md:45-93`). Ship-app Repair and Reload are free/instant anywhere (`crates/nova_os_ui/src/ship/sections.rs:542-592`); every base catalog magazine idle-refills (`crates/nova_authoring/src/base_content/sections/mod.rs:302-325`). Editor-built ships do not provide live refit. `tasks/20260824-125943/artifacts/` illustrates options, not shipped UI.

## Proposed design work, not implementation approval
- Compare one salvage claim to dock to ship-visible benefit with alternatives: whole-design swap versus one validated section refit; free repair retained with a different station reward versus replacing it with costed dock repair. Explicitly decide which paths die. Ammo-as-budget and propellant/supplies require separate combat/drive decisions; do not silently add an economy.
- Specify station identity and dock-partner authorization, minimum item/hold ownership, source/sink/refusal and persistence; name mod content/lint surfaces. Use one station and one resource choice before any general market. Prototype map/ship/station UI boundary against `NovaOsAppRuntime`, with terminal advanced-only.

## Verification to design
- The station mockup illustrates undocked, broken-port auto-release, unloaded station and insufficient stock; the corrected browser click/state checks pass, but no game run or true mobile viewport proof exists. Future implementation proofs must also refuse a wrong dock partner and duplicate rewards, roll back or refuse an in-progress transaction when `OnUndocked` fires, then show changed *actual ship* behavior. Follow with a player flow and save/return proof only after the shared persistence decision. Never use a UI-only counter as physical-upgrade proof.

## Done when
- Owner selects a first loop, service/repair/ammo policy, UI ownership and persistent identity, with exact approved follow-up feature slices; design alone edits no game code.
