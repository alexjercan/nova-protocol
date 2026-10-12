# Economy: restock and the robbery refusal window

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.16.0,economy

## User facts
- Epic `20261012-100025`. Restock refills a station's stock toward its targets by elapsed play time at thaw. Decision 10 (2026-10-12): a station of a civilization whose hull the player neutralized refuses service for a play-time window; when the stance graph lands this becomes an edge flip.

## Delivery gate
- Restock rate and the refusal window are content constants with a reason; the refusal is a visible HUD line at admission. Record the neutralized-hull civilization on the player save, not on the station.

## Verification
- Asserted examples: leave with empty stock, return after the window, stock is back; neutralize a hull, its civilization's station refuses, the window passes, it admits.

## Done when
- Both proofs are permanent and the design page's D8 pitfall line matches the code.
