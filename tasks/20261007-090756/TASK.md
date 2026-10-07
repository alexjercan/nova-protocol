# Deliver resumable New Game worlds with sector and player persistence

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.16.0,world,persistence

## User facts
- New Game must persist changed world state across sector retirement and application restart; returning to a world must not regenerate everything pristine.
- Improve Create World UI and offer Load/Continue with world selection. This is a real save/load feature, not just re-entering a seed.

## Dependencies and design gate
- Complete the lifetime/identity/error-policy decision in the existing persistence research task before adding a save schema. Map ownership, entry points, inputs, state, outputs, ordering, IDs, UI, docs and callers; present exact APIs and proof before code changes.
- Decide save slots/world naming and selection, save timing, Retry versus Continue, missing/corrupt/older saves, partial writes, mod/catalog mismatch, native and web storage constraints, and whether any loaded game can overwrite a save. Fail visibly; do not silently replace stock or reward a depleted sector twice.
- Include player ship pose, damage, inventory, credits and selected sector mutations; define the boundary of loose/queued cargo, market ships, and mined/destroyed sources. Keep other authored scenarios out unless approved.

## Proposed delivery slices (not approved interfaces)
1. Review the feature inventory and approve a bounded persistence contract with the existing research task.
2. Establish stable ownership/identity for mutated streamed objects and atomic save/load with validation.
3. Integrate named Create World and Load/Continue selection with New Game, Retry, exit, and sector retire/revisit. Remove seed-only-as-continuation affordances if replaced; retain explicit seed creation if useful.
4. Validate deterministic sector revisit, restart, player stock/credits/hull and load refusal plus rendered menu/player flows. Preserve before/after artifacts; no exit-zero-only proof.

## Done when
- An owner-approved, visibly recoverable New Game world persists the agreed player/sector changes across sector reload and game restart, with proven failure handling and selected-world UI; unpromised state remains explicitly documented.
