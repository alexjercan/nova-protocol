# Rename collision_damage to collision_impact

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.3.1, refactor

Naming clarity. Legacy #119.

## Resolution (CLOSED - already resolved)

No `collision_damage` symbol exists anywhere in the workspace. The collision damage
handlers are already named for impact: `on_impact_collision_deal_damage` and
`on_blast_collision_deal_damage` (crates/nova_gameplay/src/integrity/plugin.rs), with
the impact path explicitly distinguished from blast. The naming clarity this task asked
for is already in place, so there is nothing to rename. Closed as already-resolved.
