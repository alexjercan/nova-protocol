# Arena example: two wfc procedural ships fight

- STATUS: IN_PROGRESS
- PRIORITY: 63
- TAGS: v0.11.0,example,combat,wfc,skin

## Goal

Owner's words: "Create an Arena (with nice looking things in it like scattered
stuff - similar to the editor scene) example in which we can have 2 wfc
procedural ships fight each other."

## Why it works already in principle

`wfc_ship()` in `examples/screenshots/wfc_ships.rs` builds hulls out of REAL
prototype sections - turrets bolted on the skin, thrusters, bays with cleared
exit lanes (`erode_blocked_exits` protects exactly this). The example then
neuters them: `SpaceshipController::None`, `allegiance: None`. The arena flips
those two fields and lets the combat systems do the rest.

## Contents

- a new example: arena backdrop with scattered props, similar dressing to the
  editor scene
- two wfc hulls, opposing allegiances, AI-controlled (reuse the campaign
  raider controller)
- a watchable camera and a reroll key for a new matchup (new seeds both sides)
- ships are clad (`skin: true`) - this doubles as a combat-motion showcase of
  the skin

## Honest risk

Nobody has ever FLOWN a collapsed hull. Thrust against mass, turret arcs and
bay lanes on a random shape - this example is the first real test. That is its
value: it is the flyability bench for wfc ships, not just a spectacle.

## Definition of done

- one command runs the arena; two generated ships spawn, seek and engage
- the fight HAPPENS: a driven run logs shots fired and damage dealt on both
  sides (spawning two idle hulls does not pass)
- reroll produces a new matchup without restart
- props and lighting read as a place, not an empty void
