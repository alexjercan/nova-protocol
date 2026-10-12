# Upgrades: add a section at a free attachment point

- STATUS: OPEN
- PRIORITY: 75
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 6, second rung: add a section at a free attachment point of the hull, station only, validated by post-build checks; an occupied point refuses and nothing changes.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Attachment points are implicit in generated layouts (`crates/nova_world_base/src/ship_layout.rs`); authored hulls have none declared. Bindings come from the hull's sections.

## Delivery gate
- Propose explicit attachment points on a hull design (content), lint for overlap, and the bind rule for a new section. Stop for the owner on whether base hulls get authored points or only generated ones.

## Verification
- Asserted example: add at a free point, post-build checks pass, binding exists; add at an occupied point refuses with nothing changed.

## Done when
- Add works in a live docked run on one base hull and one generated hull.
