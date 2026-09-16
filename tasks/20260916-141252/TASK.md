# Cycle menu backdrops in a non-repeating pseudorandom order

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, menu, backdrop

## Goal

Make automatic main-menu backdrop selection feel varied without immediate
repeats.

Use a shuffled-cycle selector (shuffle bag): visit every eligible backdrop once
in a pseudorandom order, then reshuffle for the next cycle. Ensure the first
backdrop of a new cycle differs from the last backdrop of the previous cycle
when at least two backdrops are eligible.

Keep `NOVA_MENU_BACKDROP` pinning behavior unchanged.

## Acceptance criteria

- Each eligible backdrop appears exactly once per cycle.
- Selection order is pseudorandom within each cycle.
- Consecutive selections never repeat when at least two backdrops are eligible,
  including across cycle boundaries.
- Zero- and one-backdrop catalogs retain safe fallback behavior.
- Focused deterministic tests cover one full cycle, a cycle boundary, and the
  zero-, one-, and two-backdrop cases.
