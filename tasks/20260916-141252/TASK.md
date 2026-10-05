# Cycle menu backdrops in a non-repeating pseudorandom order

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.15.0,menu,backdrop

## User facts

- The main menu owns which backdrop plays next. A backdrop may signal that its
  act is done, but must not name or choose its successor.
- Keep a varied, non-repeating pseudorandom rotation. Preserve
  `NOVA_MENU_BACKDROP` pinning behavior.

## Agent findings

- `nova_menu/src/ambience.rs::load_menu_ambience` currently draws only the
  initial backdrop. The four base scenario builders each author a named
  `NextScenario` successor: gauntlet -> weave -> duel -> waystation -> gauntlet.
- Weave and waystation end on 150-second timers; gauntlet and duel also end
  after their acts. Keep those completion conditions, but replace authored
  successor selection with a completion signal owned by the menu.

## Delivery

- Move successor selection and rotation state into `nova_menu`; remove the
  base backdrops' hard-coded successor IDs and direct `NextScenario` hand-offs.
- A backdrop reports completion, including its watchdog/early-ending path.
  Decide the completion interface and what happens if an eligible backdrop
  disappears or becomes invalid before implementing it.
- Use a shuffled-cycle selector: visit each eligible backdrop once in a
  pseudorandom order, reshuffle, and avoid repeating at the boundary when two
  or more are eligible. Preserve the empty/single-backdrop fallback.

## Done when

- Each eligible backdrop appears exactly once per cycle; no adjacent repeat
  occurs with two or more eligible backdrops.
- The menu, not the active scenario, chooses the next backdrop after each
  completion, including a timeout or early ending.
- Zero- and one-backdrop catalogs degrade safely; pinning still works.
- Deterministic tests prove a full cycle, a boundary, completion hand-off, and
  zero-, one-, and two-backdrop cases.

## Scheduling

Pulled onto the v0.15.0 board at priority 40 on 2026-09-21 by owner decision,
in epic `20260921-231507`. It is self-contained and depends on nothing else
on the board.
