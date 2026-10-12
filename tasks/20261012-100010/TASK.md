# Wizard: seed then start role; six roles as content

- STATUS: OPEN
- PRIORITY: 78
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decisions 5 and 5b (2026-10-12): the New World modal becomes a wizard: seed, then a start role; six roles, each resolving to a small base hull plus a kit (credits, plates, ammo, goods) at inner-ring advancement; relations start the same for every role; the Line Warship is no longer the start.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- The modal pre-fills a seed and a name (`crates/nova_menu/src/menu_ui.rs:111-115`, `crates/nova_menu/src/world_setup.rs:53`, `:98-103`, `:324-370`); the bootstrap scenario spawns the Line Warship with `credits: 2000` (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:38-100`). Base hulls at `crates/nova_authoring/src/base_content/ships/mod.rs:31-56`.

## Delivery gate
- Propose `StartRoleDesignId`, `StartRole { id, name, about, hull: ShipDesignId, kit, lore }` as content; the harness spawns the role's hull; the save header records the role. Stop for the owner on the six hull and kit assignments.

## Verification
- Each role start spawns, passes post-build checks and survives a save round trip (asserted example per role); lint refuses a role naming an unknown hull.

## Done when
- The wizard ships with six roles as base content and the example mod can add one.
