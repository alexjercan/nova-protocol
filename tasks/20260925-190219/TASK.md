# Design one dock-salvage-physical-refit loop

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,stations,design

## User facts
- The owner accepts PR #77's themed UI appearance. TAB will open the new UI, not the old NOVA OS screen; `:` keeps the existing command capabilities. This breaking migration is tracked in `20260926-174836` and supersedes the older TAB-versus-dedicated-key decision in `20260824-125943`. Mining/salvage, supplies, trading, repair, ammo and *visible physical* ship improvement are candidate mechanics; no skill tree or crew simulator.

## Agent findings
- Docking and OnDocked exist, but no station service, cargo, credits or ship mutation save exists (`tasks/20260824-125943/TASK.md:53-101`). Ship-app Repair and Reload are free/instant anywhere (`crates/nova_os_ui/src/ship/sections.rs:542-592`); every base catalog magazine idle-refills (`crates/nova_authoring/src/base_content/sections/mod.rs:302-325`). Editor-built ships do not provide live refit.

## Proposed design work, not implementation approval
- Compare one salvage claim to dock to ship-visible benefit with alternatives: whole-design swap versus one validated section refit; free repair retained with a different station reward versus replacing it with costed dock repair. Explicitly decide which paths die. Ammo-as-budget and propellant/supplies require separate combat/drive decisions; do not silently add an economy.
- Specify station identity and dock-partner authorization, minimum item/hold ownership, source/sink/refusal and persistence; name mod content/lint surfaces. Use one station and one resource choice before any general market. Coordinate the station/ship UI boundary with `20260926-174806` and the breaking `NovaOsAppRuntime` migration in `20260926-174836`; `:` retains advanced commands and transfers/trades require the appropriate dock partner.

## Verification to design
- Future implementation proofs must refuse service when undocked, after broken-port auto-release, with an unloaded station record and with insufficient stock. They must also refuse a wrong dock partner and duplicate rewards, roll back or refuse an in-progress transaction when `OnUndocked` fires, then show changed *actual ship* behavior. Follow with a player flow and save/return proof only after the shared persistence decision. Never use a UI-only counter as physical-upgrade proof.

## Done when
- Owner selects a first loop, service/repair/ammo policy, UI ownership and persistent identity, with exact approved follow-up feature slices; design alone edits no game code.
