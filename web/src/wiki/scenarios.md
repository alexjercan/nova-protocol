# Scenarios

A scenario places a world and wires its objectives. It is the same machinery whether it is a five-minute tutorial or a combat sandbox: some objects, and a reactive events-filters-actions script over a set of variables.

## What a scenario places

A scenario spawns a handful of object kinds:

- **Asteroids** - rocks with health, a radar signature and an optional [gravity well](../gravity-wells/); destroyable debris or invulnerable planetoids.
- **Spaceships** - multi-section [builds](../sections/) under a player or AI controller (which can withhold or grant flight verbs).
- **Nav beacons** - lockable waypoints with authorable radar signatures and optional trigger areas.
- **Salvage crates** - small pickups collected by flying through them.
- **Trigger areas** - invisible volumes that fire on-enter / on-exit events.

## Objectives: events, filters, actions

Objectives are wired with a three-layer reactive system. An **event** fires under a named condition, a **filter** gates whether it applies, and an **action** runs when both match - all reading and writing typed **variables** (numbers, strings, booleans) with arithmetic and comparisons.

- **Events**: OnStart, OnUpdate, OnDestroyed, OnEnter / OnExit (a trigger area), OnOrbit (an orbit held a few seconds), OnTravelLock / OnCombatLock (a [lock](../targeting-radar/) landed on an object).
- **Filters**: match by object id or type (asteroid / beacon / salvage crate), combine with not / or / and, or test a variable expression.
- **Actions**: post or complete an objective, attach or detach the gold objective marker, emphasize a keybind hint, set a variable, spawn or despawn an object, install or lift a speed cap, grant or withhold a flight verb, create a trigger area, queue the next scenario, or declare the outcome - a victory or defeat screen with Continue/Retry and Main Menu.

This is the vocabulary the [Shakedown Run](../../tutorial/) is built from - each beat is an event handler that grants the next verb, posts the next objective, and moves a beat counter forward.

## The shipped scenarios

- **Shakedown Run** - the New Game starter: a guided tutorial that teaches one gesture per beat (burn, freelook, salvage, GOTO, ORBIT, radar lock, a live-fire rehearsal, and a scavenger fight). Winning it offers to continue straight into Broadside.
- **Broadside** - chapter two: the scavengers come back in force. Answer a neutral hauler's distress call across an asteroid cover field, break a two-corvette ambush, then screen the gang gunship's torpedoes with your PDC and take it apart section by section - under a real victory/defeat screen.
- **Asteroid Field** - a combat and gravity sandbox: a dense field, a planetoid to orbit, a fully outfitted ship and an AI drone. (A tiny _Asteroid Field - Next_ loops it.)
- **Menu Ambience** - the living backdrop behind the main menu: a planetoid with an AI ship flying a real ORBIT, no gameplay - just scale and motion.

You can author your own scenarios and mods in RON - see [Modding](../modding/).
