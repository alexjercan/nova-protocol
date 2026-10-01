# Build multi-faction reputation and diplomacy from player actions

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,world,relations,design

## Goal

After the v0.15.0 seeded civilization-to-player starting allegiances and encounter AI in `20260925-190131`, design a dynamic reputation system. Civilizations/factions may have different relationships to the player and to one another. Player interactions, including trades, may change those relationships and produce consequences for other factions (Mount & Blade-style). This is backlog planning, not implementation approval.

## User facts

- The first-release encounter task owns simple Player, Enemy, and Neutral combat sides; an attacked neutral ship retaliates. The civilization has a starting player-facing allegiance in that task.
- Later, multiple factions have relationships between one another as well as with the player; interactions with one can affect standing with others. Trade is an example input, not an approved numerical rule.

## Design gates

- Define faction versus civilization identity and membership, including whether one faction can span civilizations; keep display names separate from machine identity.
- Specify standing representation, defaults, persistence, thresholds, events and attribution for trade, combat, attacks on neutrals or allies, and repeated actions. Decide propagation through inter-faction relationships and how standing maps to ship targeting, assistance, station access, and UI. Do not treat the v0.15.0 `Allegiance` enum as a ready-made reputation API.
- Reconcile live encounter state, static generated ships, sector retirement/revisit, saves, and content/mod catalogs. Avoid retroactively changing the first-release neutral-retaliation rule without a reviewed contract.
- Require a code-backed interface map and explicit owner approval before adding types, functions, content fields, or tests.

## Done when

An owner-reviewed specification defines faction identity, relations, action effects and persistence; separately approved implementation proves deterministic consequences across factions and revisits without silently changing first-release encounters.
