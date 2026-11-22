# Docking: dock to spinning neutralized ships

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.19.0, docking, flight

## User facts
- Owner 2026-11-22: "we need a way to be able to dock 'spinning' things,
  because now if a ship with a docking station has spin on it it's hard to
  align I would personally make 'neutralized' things slow down until they
  are at zero velocity eventually but not sure, can you do some thinking
  here and come up with a plan?"
- The owner's idea (neutralized hulls settle to rest) is a proposal, not a
  decision.

## Agent findings
- Research is running: what a neutralized hull does with its spin today,
  how DOCK, MATCH and the capture check treat a rotating target, and a
  reproduction at spin rates measured from fights. Results and the design
  page go in this folder.
- Options under study: neutralized hulls settle (spin, maybe drift); DOCK
  flies the spinning port's frame; a stabilise action; relaxed capture for
  spin; a combination.

## Delivery
1. Map the slice, reproduce the failed dock, and write the design page with
   options and a recommendation.
2. Owner picks the design, then build it with a playable example.

## Verification
- An asserted example: dock to a neutralized ship with a docking port that
  spins at a rate seen in fights.

## Done when
- The player can dock to a spinning neutralized ship with a docking port,
  and the owner has played it.
