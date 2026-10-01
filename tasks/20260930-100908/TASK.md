# Bring player ship tuning and section editing into the game

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,ship,editor,design

## User facts
- Former item-loop M4 station refit is a separate, later task. Bring a ship editor into the game so the player can tune a real ship. Do not gate the existing plate repair behind a station.
- Coordinate with `tasks/20260925-190219/TASK.md` (existing ship-tuning research) rather than replacing its undecided interfaces or claiming a delivered refit.

## Design gate
- Read real ship-design and runtime-section owners, docking/station state, UI entry points and part catalogs. Decide where editing is authorized and whether it requires a station, which components/materials/credits it consumes, what installed changes are allowed, and how invalid placement, missing stock, interrupted service and section damage refuse atomically.
- Define a stable owner for section identity, hull capacity after edits, flight validity, mod parts and save/migration semantics with `tasks/20260925-190156/TASK.md`. Do not silently infer that current player ships can be rebuilt in place.
- Show exact owning paths, proposed interfaces/call graph, content/docs/UI changes and proof before implementing. Keep M5 trade/mining free of ship-editor dependencies unless approved.

## Verification
- An accepted edit changes the actual player ship, consumes exactly its approved cost and preserves connectivity, controls, weapons and clearance. An invalid edit leaves ship and stock unchanged. Inspect a rendered before/after ship; state explicitly what survives sector retirement and restart.

## Done when
- An owner-approved in-game editor/refit contract and its separately verified implementation produce observable player-ship tuning without changing M0 repair-anywhere behavior.
