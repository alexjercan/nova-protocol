# Own station: buy a station from a living civilization

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.16.0,wizard

## User facts
- Epic `20261012-100008`. Decision 13 (2026-10-12): the buy half of D10 lands right after Stage 2: buy a station from a living civilization; its stock targets and price rules become yours; it refuses nobody.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Delivery gate
- Ownership is a field on the frozen station body and in `SavedPlayer`; an owned station's admission skips the civilization rule. Stop for the owner on the price (a multiple of its stock value) and whether income accrues (needs the clock, economy strand).

## Verification
- Asserted example: buy, leave, return after a reload, the station is owned and admits an Enemy-flagged player ship.

## Done when
- Buy works in a live run and ownership survives retire, thaw and reload.
