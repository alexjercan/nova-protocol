# Devlog #3: zones, torpedoes and blast damage

v0.2.0 gave the game objectives and a data-driven system to describe them. v0.3.0 is about making that system say more interesting things, and about giving the player a weapon that is not just a turret. So this update lands on three fronts: the event system grows lifecycle and location awareness, the first area-of-effect weapon arrives as the torpedo bay, and the health model underneath it all gets reworked to make section-by-section damage clean.

## Events that know where and when

The v0.2.0 modding backbone could react to things happening (an object destroyed, a variable crossing a threshold). What it could not do was react to _lifecycle_ or to _place_. v0.3.0 adds both.

- **OnEnter and OnExit events** fire when a scenario state begins and ends, so a mission can run setup on entry (spawn a wave, seed variables, show a briefing) and teardown on exit without hand-wiring it into game code.
- **A zone-entry trigger** fires an event when a ship moves into a defined region of space. That single primitive unlocks a whole class of objectives - "reach the nav point", "leave the debris field", "hold this area" - all expressed as data through the same filters-and-actions pipeline from the last devlog.

None of this is special-cased gameplay code. It is more vocabulary for the same event language, which is exactly the payoff I was hoping for when I built it data-driven: new scenario shapes without new engine code. This is the machinery the current scenario system still runs on.

## The torpedo bay: the first area weapon

Until now the only way to hurt something was the turret - a hitscan round that damages exactly what it touches. v0.3.0 adds the **torpedo bay section**, and with it the game's first **area-of-effect damage**.

A torpedo does not need a pixel-perfect hit. It detonates and deals **blast damage** to everything inside its radius, which changes the tactical texture completely: turret fire is precise and pointed, torpedoes are about zoning and catching clustered or fragile targets. Because a torpedo is a section like any other, you bolt it onto a build the same way you would a thruster or a turret, and it inherits the same physics and mounting rules for free.

## A health system built for sections

Blast damage only means something if there is a consistent thing for it to damage, so this release reworks health into a proper **per-section health system**. Every section of a ship carries its own health, and both flavours of damage feed into it the same way: a turret round hits one section, a torpedo blast hits every section within its radius. The section-local model that made destruction feel right in the earlier devlogs is now the single, shared path all damage flows through.

The nice property carries forward: damage stays local. A torpedo that goes off next to a ship can shred the sections on that side while the far side survives, so where a hit lands matters, not just whether it landed.

## Shader polish

Two visual upgrades round the release out. The **directional shader** was reworked to make a ship's forward direction far more legible at a glance - a small thing that matters a lot when you are trying to read orientation in the middle of a fight. And the **thruster shaders** were extended to support more complex shapes and animations, so exhaust plumes can be more expressive than the single cone they started as.

## Where v0.3.0 lands

So v0.3.0 deepens the two systems that make Nova Protocol a game rather than a flight toy: scenarios that can react to lifecycle and location, and combat that now has both precise and area weapons feeding a unified health model - all wrapped in clearer, prettier shaders. Next up: pushing combat and the flight model further. Or you can just [go fly something](../../play/).
