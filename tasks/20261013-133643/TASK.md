# Greebles: plating reshapes to fit the greeble placed on it

- STATUS: OPEN
- PRIORITY: 94
- TAGS: v0.16.0,skin

## User facts
- Owner feedback 2026-10-13: small square greebles (vents etc.) sometimes land on "pointy" plating. "would be nice if we can somehow detect that a greeble is being placed somewhere and we make the pointy thing adapt to the greeble that will be placed on top, so the plating will basically take into account what/where it is placed on (like in terms of sections it is placed on and where) and also what greeble goes on top of it, so it can actually accommodate for that shape to make sense".
- The greeble follows the plate's axis; an angled plate turns the greeble and the plate makes room for its size (see the placement research task).

## Agent findings
- Depends on the placement language research task (same strand). Not yet surveyed.

## Delivery gate
- After the research note is accepted: propose how a plate learns which greeble sits on it and reshapes (flattened pad, footprint), show before/after frames, then implement. Regenerate base content.

## Verification
- Before/after screenshots of hulls where a greeble sat on a spike; asserted check that every placed greeble lands on a plate face that fits its footprint.

## Done when
- No greeble floats on or clips a pointy plate on the reviewed hulls; the owner accepts the look.
