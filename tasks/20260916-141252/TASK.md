# Cycle menu backdrops in a non-repeating pseudorandom order

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.15.0,menu,backdrop

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

## Scheduling

Pulled onto the v0.15.0 board at priority 40 on 2026-09-21 by owner decision,
in epic `20260921-231507`. It is self-contained and depends on nothing else
on the board.
