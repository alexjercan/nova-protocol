# Map shipped features and player-experience priorities for v0.16.0

- STATUS: OPEN
- PRIORITY: 90
- TAGS: v0.16.0,planning

## User facts
- Before scheduling polish or new systems, list all shipped game features and classify core player experience versus side features, with current maturity and friction. Focus v0.16.0 on making existing play enjoyable and reliable.
- Do not equate a compiled feature with a complete player journey.

## Delivery
- Inventory shipped features from code, content, in-game UI, docs, and current player-flow evidence. Include New Game/onboarding, travel/autopilot/world, combat, mining/canisters, inventory/trade/repair, scenarios/story, input/accessibility, mods, and menus. Distinguish available mechanics from complete journeys and non-shipped ideas.
- For each feature record: core/secondary/supporting role, player goal, entry/UI, current maturity (unavailable / prototype / usable with gaps / proven), specific evidence, top friction, nearest proof, and cost/risk of improvement. Mark untested claims unverified.
- Rank a short v0.16.0 polish shortlist by player impact and verified friction; exclude broad additions unless they unblock the core loop. Include old bug tasks only when still reproducible; the earlier docking issue was fixed and the owner cannot reproduce it now.

## Initial feature map (source audit, not a player-quality score)

Maturity here means **implemented**, **partially demonstrated**, or **needs player proof**. No category is rated enjoyable or complete by static code alone. Review content and rendered flows before final ranking.

| Shipped feature | Player role | Current evidence / maturity | Most useful next check |
| --- | --- | --- | --- |
| New Game seeded world, create modal | Core entry | Implemented: `crates/nova_menu/src/world_setup.rs:171-230`, `crates/nova_menu/src/menu_ui.rs:474-520`; boot seed identity proven in earlier bench smoke, not world continuation. | Show a first-session player can understand seed/create and later find the same world; there is no Load/Continue yet. |
| Flight, camera, input/rebinding | Core moment-to-moment | Implemented: `crates/nova_ship/src/flight/manual.rs`, `crates/nova_ship/src/input/`, `crates/nova_input/src/source.rs:20`; full hardware playability unverified. | Observe flight control and damage/rebinding recovery with a player. |
| Navigation: target locks, GOTO/STOP/ORBIT, Map prediction | Core travel | Implemented: `crates/nova_ship/src/input/targeting/state.rs:97-106`, `crates/nova_autopilot/src/autopilot.rs`, `crates/nova_interface/src/map/`; bounded trajectory tests/captures exist, broad long-trip reliability unverified. | From New Game, select a target, travel and recover from interruption. |
| Seeded sector streaming and generated ships/rocks | Core world | Implemented: `crates/nova_world/src/streaming.rs:154-286,870-893`; pristine revisit is explicit in `crates/nova_world_base/src/lib.rs:27-30`. | Measure player perception of discovery, repetition and streaming under actual travel; persistence is a separate delivery. |
| Mining and ore-canister production | Core economy | Implemented: `crates/nova_ship/src/sections/mining_section.rs:117-150`, `crates/nova_scenario/src/mining.rs:181-192`; a prior agent run proved carve pulses but not ore pickup. | Confirm emitter -> ore canister -> physical intake -> inventory delta in one recorded flow. |
| Cargo intake, jettison, inventory and ammo reserves | Core economy/logistics | Implemented: `crates/nova_ship/src/sections/cargo_intake_section.rs:129-147`, `crates/nova_gameplay/src/inventory.rs:191-336,575-641`; player-flow completion remains unverified. | Verify stock/capacity conservation and discoverability through pickup/jettison/reload. |
| Docked trade, give/take, credits | Core economy | Implemented: `crates/nova_gameplay/src/inventory.rs:488-575`, `crates/nova_interface/src/inventory/app.rs`; an earlier controlled demo is not proof of New Game sale. | Record dock -> transaction -> both stock/credit deltas; inspect quantity control accessibility. |
| Section damage and plate repair | Core recovery | Implemented: `crates/nova_gameplay/src/inventory.rs:770`, `crates/nova_interface/src/ship/app.rs:157-262`; UI and repair proof exist, whole-journey friction unmeasured. | Find damaged section and spend selected plates; judge preview/refusals. |
| Weapons, targeting, point defense and AI fights | Core challenge | Implemented: `crates/nova_ship/src/sections/`, `crates/nova_ship/src/input/ai/`, `crates/nova_ship/src/input/point_defense/`; AI-only fight data cannot establish player combat quality. | Measure whether players can read threats, weapon readiness, firing and salvage outcome. |
| Docking and docked helm | Supporting transition for trade/salvage | Implemented: `crates/nova_ship/src/sections/docking_section/`; reproduced joint error fixed in PR #80 and old owner report no longer recurs (`tasks/20260926-132243/TASK.md`). | Do not reopen absent new reproduction; inspect clarity of eligible docking when testing trade. |
| TAB Map/Ship/Inventory, flight HUD, tutorial lessons | Supporting comprehension | Implemented: `crates/nova_interface/src/lib.rs:98`, `crates/nova_hud/src/lib.rs:275`, `crates/nova_training/src/`; rendered assets exist, usability depends on real player observation. | Observe finding commands/status and recovery after a failed action. |
| Authored scenarios/campaigns and story | Secondary alternate play | Implemented: `crates/nova_scenario/src/lib.rs:65` and menu scenarios path; completion breadth is not inferred from loader code. | Review core onboarding and scenario handoff separately from open-world save scope. |
| Mod bundles, catalogs, lint and settings | Supporting creator/configuration | Implemented: `crates/nova_modding/src/lib.rs:442-522`, `crates/nova_assets/src/merge.rs:843-880`, `crates/nova_assets/src/persist.rs:56-61`; item identities are still closed `ItemType` (`crates/nova_gameplay/src/inventory.rs:41-150`). | Research RON item examples last; prove lint/merge and save mismatch before committing an identity design. |
| Audio, visual effects and presentation | Supporting feedback | Audio and rendering plugins exist (`crates/nova_gameplay/src/audio/mod.rs:308`, `crates/nova_hud/src/lib.rs:275`); no claim that moment-to-moment cues are clear or mixes are balanced. | Inspect live audio/video for legibility and action feedback during core flows. |
| Desktop/web builds, settings and keybinds | Supporting access | CI builds and settings persistence exist (`crates/nova_assets/src/persist.rs:56-61`, `crates/nova_input/src/lib.rs:43`); platform-specific player experience remains unmeasured. | Check startup, settings persistence and control readability on each supported platform. |
| Gamepad and mobile web controls | Supporting access | Input paths exist (`crates/nova_input/src/source.rs:20`), but hardware navigation/touch-pad tasks are still backlog. Do not label fully playable. | Later hardware/touch player proof, outside v0.16.0 sprint. |

**Provisional player-impact order:** make the New Game loop and its inventory outcomes observable; persist a chosen world and its player/sector mutations; research the biggest remaining comprehension/combat friction in recorded player flows. Item RON identity is last and research-only for now. This is a hypothesis until the polish spike supplies direct player evidence.

## Done when
- Owner can review the full feature matrix and evidence-backed priority order before implementation tasks are committed to a sprint. No gameplay implementation or release promise follows from this audit.
