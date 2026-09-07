# Add verified gameplay statistics to the wiki

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

Give the player wiki clear, exact gameplay statistics without maintaining a
second technical specification in lore. Use compact item tables and useful
variant comparisons, following the style of a detailed game wiki such as
Factorio's. Keep the current prose, widgets, and illustrations useful together.

## Reference

[Technical specification](TECHNICAL_SPEC.md) is the original story-planning
capability audit, preserved byte-for-byte as task reference. It is not a current
specification, world canon, or a document to publish as another reference page.

- Implementation baseline: `91f67d8d12aed96f04414240dda671ae7435ab8c`.
- SHA-256: `d8c5c61954e52d9e1cada836ad41a156318f48ca23ad3d602a50f539ba865d64`.
- Its paths, line numbers, catalog ordering, defaults, and derived figures must
  be checked against the implementation at the time of this task.
- Re-derive claims from runtime behavior and authored content. Do not merely
  copy the tables or verify that a named field still exists.

## Scope

1. Inventory the reference's six sections against existing wiki, creator, and
   developer articles. Record what is already covered, what needs updating, and
   what should be discarded as stale or irrelevant to players.
2. Extend existing wiki pages rather than add a monolithic stats specification.
   Put compact stats and variant tables with the item they describe. Use named,
   current base configurations, not the audit's anonymous catalog-row examples.
3. Separate configured values, derived estimates, and measured results. State
   assumptions for acceleration, firing duration, nominal projectile travel,
   damage output, and other comparisons. Do not call nominal travel distance a
   guaranteed engagement range, or a derived result a measurement.
4. Identify base/default values and explain which can vary with a build, damage,
   scenario, or mod. Use explicit units and player-facing lengths in meters.
   Do not invent kilograms, newtons, explosive yields, or fictional technology
   from internal simulation quantities.
5. Avoid duplicate maintained values. Inspect the existing widget and test
   sources, then choose a small shared data or validation approach so tables,
   calculators, and defaults cannot silently disagree. Keep source provenance
   with numerical claims, including repository paths and line references where
   required by the documentation conventions.
6. Add sparse, clearly labeled world-background links only where lore answers a
   different useful question. Keep controls, numerical gameplay facts, and
   simulation exceptions in the wiki, not in lore.

## Likely article owners

- `web/src/wiki/sections/{thruster,controller,hull,turret,torpedo-bay,railgun}.md`:
  item statistics, fit constraints, and variant comparisons. These already have
  tables and widgets; extend or correct them rather than create parallel copies.
- `web/src/wiki/{flight-autopilot,gravity-wells}.md`: observable flight behavior,
  gameplay limits, and the simulation's gravity rules.
- `web/src/wiki/{combat-weapons,targeting-radar,ships}.md`: damage, targeting,
  defeat, and the practical meaning of weapon and ship figures.
- `docs/{sections,architecture,scenario-system}.md`: unique implementation
  explanations, if still valid and not already documented. Do not move the
  complete historical snapshot into the developer book.
- `web/src/create/`: content fields and scenario-authoring contracts. Player
  tables should not become a second schema reference.
- Story-specific unsupported dependencies belong with season or episode plans.
  The shared season outline already lists its main campaign-development needs.

## Constraints

- Coordinate with the ongoing wiki image work. Preserve its pages, captures,
  captions, and widget explanations; do not regenerate unrelated media.
- Lore owns fictional people, places, technology, and consequences. The wiki
  owns the game and what the player can do. Some brief context may overlap, but
  neither surface should carry a second copy of the other's maintained details.
- No gameplay rebalance, engine changes, or new simulation features are implied
  by this documentation task. Record inconsistencies before proposing changes.
- Keep the coverage audit, data-source decisions, and verification evidence
  with this task. Follow the documentation skill and release-baseline rules.

## Acceptance

- [x] Preserve the technical audit as task-scoped reference, outside lore.
- [ ] Record a coverage audit and recheck retained claims against current sources.
- [ ] Put useful exact statistics and comparisons on the relevant wiki pages.
- [ ] Clearly label defaults, units, derivations, measurements, and assumptions.
- [ ] Verify repeated numerical values against their authoritative source.
- [ ] Preserve existing images and keep lore and gameplay documentation separate.
- [ ] Run affected web checks and inspect rendered tables and widgets at desktop
  and mobile widths, including deployed-prefix links.

## Intake proof

The reference was moved unchanged from `web/src/lore/TECHNICAL_SPEC.md`; its
tracked predecessor was `lore/TECHNICAL_SPEC.md`. Both match the SHA-256 above.
The lore authoring guide no longer points at a separate technical specification.
No wiki article or gameplay code was changed when creating this task.
