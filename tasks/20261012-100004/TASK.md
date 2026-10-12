# Stations: parts recipes on sections; rebuild and refit while docked

- STATUS: OPEN
- PRIORITY: 84
- TAGS: v0.16.0,stations

## User facts
- Epic `20261012-095958`. Decision 4 (2026-10-12): rebuild and refit only while docked at a station, paid in credits and parts; plate repair stays anywhere; sections are never items; each section config owns a parts recipe (part items such as turret barrel, turret body, section panels, plus credits) that the station assembles; lint refuses a recipe whose parts at bid are worth more than the recipe at ask.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Plate repair: 20 HP per plate, refusal order `NoIntegrity, Destroyed, Full, NoPlates, InsufficientPlates, ExcessPlates` (`crates/nova_gameplay/src/inventory.rs:885`, `:897-911`). Saved ship state keeps surviving sections by id and thaws the original design (`crates/nova_scenario/src/objects/spaceship.rs:583-615`). Items are `Content::Item` with `ITEM_ROLES` (`crates/nova_assets/src/items.rs:115-158`).

## Delivery gate
- Propose `build: BuildRecipe { credits, parts: {ItemDesignId: u32}, strip_yield: f32 }` on the section config; part items as base content in a `Parts` category; `Rebuild` respawns the design section at full health and consumes the recipe; `Refit` is the same-family swap of the wizard strand and is out of scope here. Stop for the owner on the part item list.

## Verification
- Unit tests for the recipe lint; an asserted example: lose a section, dock, rebuild, the section is back at full health with the old binding, parts and credits are gone; freeze and thaw keep it.

## Done when
- Every base section carries a recipe, lint enforces the value rule, and rebuild works in a live docked run.
