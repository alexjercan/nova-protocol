# Arena example: two wfc procedural ships fight

- STATUS: CLOSED
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

## Closure

Landed as 3a7cfb8c (2026-08-16), lane wfc-arena. The generator moved to
examples/screenshots/shared/wfc.rs (wfc_ships shrank 2058 -> 592 lines and
keeps only the row, photography and keys). The arena drafts the seed stream
for the first two hulls with >= 4 turrets and >= 2 bays (armament variance is
wild: 0-22 turrets, 0-12 bays per roll, every skipped seed logged), arms them
AMBER=Player vs ONYX=Enemy under the campaign AI, spawns at 160u (inside the
180u PDC gate - from 280u the torpedo alpha strike ends it before guns bear),
and scoreboards shots and damage per side. Autopilot gates on BOTH sides
firing AND dealing damage; R rerolls, L cycles styles.

## Flyability findings (first honest flight of collapsed hulls)

1. TORPEDO SALVOS DOMINATE: 8-10 tubes volley on spawn; ~5 s flight, 8x750
   blast erases a ~180-section hull. Typical bout is mutual annihilation by
   t+9 s. Relevant to the owner's speed-cap thinking.
2. CLOSING SPEED IS SLOW: ~6.7 u/s from 280u - thrust-to-mass on solid hulls
   is sluggish, so a beyond-gun-range start means the salvo decides all.
3. The draft is what makes a mutual fight reliable at all; an unarmed roll
   can never satisfy it.
4. Under load the whole death cascade can land between two Update frames;
   kill credit for a vanished side goes to its rival.
