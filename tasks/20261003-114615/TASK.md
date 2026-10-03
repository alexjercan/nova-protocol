# Improve Ship, Inventory, and Map HUD clarity and controls

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.15.0, ui, bug

## User facts and approved direction

- Ship: Do not offer Rebind on sections with no actionable binding. Each Mining section must have its **own independently rebindable key**, like other sections; a shared mining key is explicitly rejected. Show repair pack cost and let the player choose a whole-number quantity from 1 through All, with All initially selected. Let ship orbit tilt below zero to inspect underside sections. Remove duplicate bottom-right selection text, move viewer buttons to the bottom right, and keep key hints in the middle. Present section HP, ammo and bindings as readable, labelled facts rather than an undifferentiated paragraph; allow useful additional section stats and explanatory tooltips.
- Inventory: When docked with a trader, own-side item selection should start on Sell rather than Give; retain Give when trade is unavailable. Give weight, capacity and credits clearer visual hierarchy. Reserve stable space for item description (about 3-4 lines), truncate visible overflow, expose its full text by hover and a keyboard-accessible route, and add restrained dividers. Avoid panel movement as selected items change. Improve reading and accessibility without relying on color alone.
- Map: Remove bottom-right duplicate selection text as in Ship; preserve ease of clicking contacts. Improve visual hierarchy and contrast. Consider coordinates or other useful map visual features, but decide coordinate frame and semantics at the implementation gate rather than inventing them now.
- Owner approved this plan and asked for this task to be created. Approval of this specification is not approval to edit code without the implementation gate.

## Agent findings (verify again before implementation)

- `crates/nova_interface/src/ship/app.rs:28-132` builds the ship panel and always spawns Repair/Rebind; `ship/scene.rs:949-1069` populates detail text and button state; `ship/sections.rs:353-357` collects per-section bindings for thrusters and three weapon types but no Mining; `ship/rebind.rs:24-115` writes the captured section binding and change message. Mining currently uses a ship-wide held flight action and `MiningHeld` (`crates/nova_ship/src/input/player/flight_rig.rs:68-79`), not independently bound emitters. This is a gameplay/input/content/save change, not only a button change. Trace the authored Mining section and every runtime-ID, serialization, flight-hint and action consumer before picking its new owning interface. The UI must not claim mining is rebindable until a new key controls only the selected emitter.
- `crates/nova_gameplay/src/inventory.rs:700-757` plans repair at 20 HP per whole HullPlate and spends the minimum of stock and plates needed. `crates/nova_interface/src/ship/sections.rs:405-489` shares repair availability/refusal with the handler. The new quantity must be checked against live health and stock at confirmation, not only at panel refresh; decide how the P shortcut uses the selected quantity.
- `crates/nova_interface/src/viewer.rs:27-31,116-134` clamps shared Ship/Map orbit elevation to positive angles. Allow below-plane viewing for Ship without unintentionally changing Map. Keep poles away from the camera up-vector singularity.
- `crates/nova_interface/src/pane.rs:534-742` builds both viewer footers with duplicate right-side summaries; `ship/scene.rs:989-1030` and `map/scene.rs:628-668` populate them. Keep selection identity available in each side panel.
- `crates/nova_interface/src/inventory/app.rs:800-831` opens an own-side docked item on Give; Sell is a switch in a trader draft. `inventory/app.rs:496-562` renders inspector description and facts; `inventory/app.rs:1612-1617` puts cargo and credits into one heading. The draft already has a quantity slider (`inventory/app.rs:567-665`); no existing interface tooltip facility was found. Ensure hover-only help is not the sole access path.
- Map already has range/bearing and metric rings in `crates/nova_interface/src/map/scene.rs`; coordinates need a chosen origin, units, precision and target (focus, selection or pointer). Existing projection and click targets must stay usable.

## Delivery sequence and blast radius

1. Reproduce and preserve evidence for mining rebinding failure, non-actionable Rebind, Sell default, orbit lower bound, duplicate footer and layout movement. Audit the flight action, section mapping, save/authoring and UI caller chain. Present exact paths/lines, types, fields, functions/signatures, before/after graph and proposed tests to the owner; get approval before adding types/functions/tests or settling remaining interface/default/error decisions.
2. Change the owning Mining input interface first so each installed Mining section has its own authored and live binding, activation and persistence. Update content builders, lint/load, player hints, editor and every consumer. Delete superseded ship-wide-only control paths rather than leaving misleading adapters. Fail loudly for invalid authored bindings.
3. Implement repair quantity/preview and atomic live validation; conditional rebind affordance; under-hull Ship orbit; structured, stable Ship inspector and footer layout.
4. Implement docked trader Sell default, inventory heading/facts, fixed-height short description with full accessible text, and restrained separators. Update Map visual treatment and footer, protecting selection/clicking. Coordinate overlay remains optional pending a specific owner decision at the gate.
5. Update affected docs and Unreleased changelog; run focused checks and inspect rendered Ship, Inventory and Map frames at varied viewport sizes and with keyboard navigation. Do not claim a visual improvement from headless test exit status alone.

## Verification and done when

- Two Mining sections can hold distinct keys; rebinding one changes only that emitter's flight behavior and survives the existing save/content path. Non-actionable sections offer no Rebind; actionable ones work. Both pointer and keyboard paths agree.
- Repair displays stock, requested plates and predicted HP; 1, intermediate and All requests consume exactly the selected affordable quantity, including stale-stock/health refusal without loss. The P shortcut has an explicit approved behavior.
- R/F and drag can show ship underside without a camera flip. Map controls stay as approved. Duplicate footer text is gone in both panes, buttons occupy the right zone, and center hints still track live bindings.
- Trader own-side selection opens Sell; non-trader/undocked cases keep valid existing behavior. Inventory numbers and section facts are labelled and readable. Switching between short/long descriptions and optional fields does not move action controls; full descriptions can be read without a mouse.
- Focused asserted behavior checks plus inspected rendered frames show legibility, stable geometry, usable click targets and accessible focus across the three panes. Record limits and skipped checks in this task before closure.
