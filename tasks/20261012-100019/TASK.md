# Salvage: strip a docked lootable hull into recipe parts

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.16.0,salvage

## User facts
- Epic `20261012-100018`. Strip is a timed action on a docked lootable partner: the section detaches through the existing detach path and breaks down into its recipe parts in the player's hold, more of them the healthier it was; refuses on a non-lootable hull; a stripped wreck stays stripped.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- Generated derelicts spawn with every system disabled, `lootable: true`, Neutral, no controller; detached pieces exist (`FrozenTransient::DetachedPiece`, `crates/nova_world_base/src/save/transients.rs:61-81`). `SalvagedParts` is a required item role (`tasks/20260930-100831/TASK.md`).

## Delivery gate
- Strip uses the section recipe times `strip_yield` times health fraction; the lint rule (strip yield at bid never exceeds the recipe at ask) lives with the recipe. The detached piece despawns into parts, not a floating item.

## Verification
- Asserted example: strip once, retire and return, the wreck stays stripped, a second strip on the same section refuses; a strip on a non-lootable hull refuses.

## Done when
- Strip works in a live docked run and the lint rule is permanent.
