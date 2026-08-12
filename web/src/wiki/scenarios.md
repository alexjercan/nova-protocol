# Scenarios

A scenario places a world and wires its objectives. It is the same machinery whether it is a five-minute tutorial or a combat sandbox: some objects, and a reactive events-filters-actions script over a set of variables.

## What a scenario places

A scenario spawns a handful of object kinds:

- **Asteroids** - rocks with health, a radar signature and an optional [gravity well](../gravity-wells/); destroyable debris or invulnerable planetoids.
- **Spaceships** - multi-section [builds](../sections/) under a player or AI controller (which can withhold or grant flight verbs).
- **Nav beacons** - lockable waypoints with authorable radar signatures and optional trigger areas.
- **Salvage crates** - small pickups collected by flying through them.
- **Lights** - the scene's own key/rim/fill lighting; a scenario that spawns
  none renders black.

Trigger areas - invisible volumes that fire on-enter / on-exit events - are
created by an action rather than spawned as objects.

## Objectives: events, filters, actions

Objectives are wired with a three-layer reactive system. An **event** fires under a named condition, a **filter** gates whether it applies, and an **action** runs when both match - all reading and writing typed **variables** (numbers, strings, booleans) with arithmetic and comparisons.

- **Events**: OnStart, OnUpdate, OnTimerEnd (a keyed scenario timer finished), OnDestroyed, OnNeutralized (a ship is combat-dead - weapons and thrusters gone, hull intact), OnEnter / OnExit (a trigger area), OnOrbitStart / OnOrbitStable / OnOrbitUnstable / OnOrbitEnd (orbit lifecycle edges), OnTravelLockStart / OnTravelLockEnd and OnCombatLockStart / OnCombatLockEnd ([lock](../targeting-radar/) lifecycle edges).
- **Filters**: match by object id or type (asteroid / beacon / salvage crate), combine with not / or / and, or test a variable expression.
- **Actions**: post or complete an objective, attach or detach the gold objective marker, emphasize a keybind dock chip, set a variable, spawn or despawn an object, scatter a seeded field of objects, drive a HUD readout, post a story message, swap the skybox, set a ship's allegiance, pose the camera, install or lift a speed cap, grant or withhold a flight verb, create a trigger area, queue the next scenario, or declare the outcome - a victory or defeat screen with Continue/Retry and Main Menu that pauses the game behind it.

This is the vocabulary the [Shakedown Run](../../tutorial/) is built from - each beat is an event handler that grants the next verb, posts the next objective, and moves a beat counter forward.

## The shipped scenarios

- **Shakedown Run** - the New Game starter: a guided tutorial that teaches one gesture per beat (burn, freelook, salvage, GOTO, ORBIT, radar lock, a live-fire rehearsal, and a scavenger fight). Winning it offers to continue straight into Broadside.
- **Broadside** - chapter two: the scavengers come back in force. Answer a neutral hauler's distress call across an asteroid cover field and break a two-corvette ambush; that win is a checkpoint, and the fight continues into the gang's gunship - screen its torpedoes with your PDC and take it apart section by section. Dying to the gunship retries the gunship, not the ambush, and hard boulders in the field now genuinely block incoming fire - use them.
- **Lifeline** - chapter three, part one: the gang hits back where it hurts. Screen a stalled two-hauler convoy against three telegraphed raider waves until the relief wing arrives - a live countdown on the HUD, a protect objective instead of kill-all, and the convoy genuinely draws fire (the haulers fly the player's flag). Winning the Broadside chapter continues here, and winning here continues to the finale.
- **Final Tally** - chapter three's finale, reached from Lifeline's victory: the trace ends at the gang's claim - a cracked megahauler anchorage deep in a planetoid's gravity well, ringed by a belt. Survey the anchorage with a travel lock, break the orbital picket riding the well, and finish the gang's flagship when it casts off with its escort. The campaign closes properly here.
- **Asteroid Field** - a combat and gravity sandbox: a dense field, a planetoid to orbit, a fully outfitted ship and an AI drone. (A tiny _Asteroid Field - Next_ loops it.)
- **Menu backdrops** - the living scenes behind the menus, picked at random on each menu entry: **Menu Ambience** (a planetoid with an AI ship flying a real ORBIT), **Waystation Traffic** (a hauler convoy circling a freight stop under amber dock lights), and **Scrapyard Drift** (a quiet salvage yard of drifting crates, two wrecks and a lone tug). No gameplay - just scale and motion. Mods can ship their own by flagging a scenario `menu_backdrop`.

## Browsing and replaying scenarios

The **Scenarios** tab groups a campaign under a collapsible header - click the
`[-]`/`[+]` header to expand or fold its chapters. A campaign lists its chapters
in play order, including mid-story chapters that the story reaches automatically
(the Broadside gunship phase, the Final Tally finale): they show under the
campaign header so you can launch any chapter directly for a replay without
flying the whole arc again. Scenarios that belong to no campaign (Asteroid Field,
standalone mod scenarios) list on their own below the campaigns.

You can author your own scenarios and mods in RON - see [Modding](../modding/).
