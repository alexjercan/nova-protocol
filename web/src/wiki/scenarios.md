# Scenarios

A scenario places a world and wires its objectives. It is the same machinery whether it is a five-minute tutorial or a combat sandbox: some objects, and a reactive events-filters-actions script over a set of variables.

## What a scenario places

A scenario spawns a handful of object kinds:

- **Asteroids** - rocks with a radar signature and an optional [gravity well](../gravity-wells/). A normal rock has no health: it is carved away by what hits it, and how big it is decides how long that takes (see [Shooting rock](../combat-weapons/#shooting-rock)). An invulnerable planetoid never wears at all.
- **Spaceships** - multi-section [builds](../sections/) under a player or AI controller (which can withhold or grant flight verbs).
- **Nav beacons** - lockable waypoints with authorable radar signatures and optional trigger areas.
- **Salvage crates** - small pickups collected by flying through them.
- **Lights** - the scene's own key/rim/fill lighting; a scenario that spawns
  none renders black.

Trigger areas - invisible volumes that fire on-enter / on-exit events - are
created by an action rather than spawned as objects.

## Objectives: events, filters, actions

Objectives are wired with a three-layer reactive system. An **event** fires under a named condition, a **filter** gates whether it applies, and an **action** runs when both match - all reading and writing typed **variables** (numbers, strings, booleans) with arithmetic and comparisons. This is the vocabulary [First Shift](../getting-started/) is built from - each beat is an event handler that grants the next verb, posts the next objective, and moves a beat counter forward.

What that buys you as a player is that a scenario reacts. Fly into a marked volume and something answers; neutralize the right ship and the objective completes on its own; a won chapter plays a short outro of comms beats over the live world before any victory screen appears, so the moment lands before the overlay does.

The full construct catalog - every event, filter and action by name, with its fields - is the authoring contract and lives in the [Create docs](../../create/scenarios/).

## The shipped scenarios

- **First Shift** - the New Game opening: a crewed cutter goes out of the carrier Meridian for an ordinary day on the rock plate, and it teaches one gesture per beat (burn, fine thrusters in open space, salvage, radar lock, GOTO, and an ORBIT the crew talks you into on the way to the last crate). Then, with the job finally finished, a warship comes out from behind a planetoid and destroys the carrier while you watch from abeam, unarmed. Winning continues straight into Second Shift.
- **Second Shift** - chapter two: the same belt an hour later, and the Meridian is a debris field. Recover three recorders out of the wreck while a five-ship cleanup group sweeps it for the same evidence. They fly real patrol lanes with short eyes, so rock between you and a lane is cover; being seen does not end the run, it costs you the quiet way home.
- **Menu backdrops** - the living scenes behind the menus, a rotating CAROUSEL: each scene plays its act and hands off to the next. **Torpedo Gauntlet** (a corvette's PDC turrets swat torpedoes streaming in from both flanks until its hard magazines run dry and the stand falls), **Asteroid Weave** (an AI ship threading a dense rock band on real patrol waypoints, hugging its nav beacons), **Duel Cycle** (two corvettes dogfight through the open center; a siege torpedo erases the winner), and **Waystation Traffic** (a hauler convoy circling a freight stop under amber dock lights). Menu entry starts the ring at a random scene. All of it is the real simulation, not a cutscene. Mods can ship their own by flagging a scenario `menu_backdrop`.

## Browsing and replaying scenarios

<figure class="figure">
    <!-- Capture: assets/wiki-scenarios-picker.png -->
    <div class="figure__placeholder">
        <span class="figure__placeholder-tag"
            >Screenshot needed</span
        >
        <span class="figure__placeholder-name"
            >assets/wiki-scenarios-picker.png</span
        >
        <span class="figure__placeholder-note"
            >The Scenarios tab with a campaign expanded: its
            chapters in play order under the collapsible
            header, standalone scenarios listed below.</span
        >
    </div>
    <figcaption class="figure__caption">Campaigns group their chapters; any chapter can be launched directly for a replay.</figcaption>
</figure>

The **Scenarios** tab groups a campaign under a collapsible header - click the
`[-]`/`[+]` header to expand or fold its chapters. A campaign lists its chapters
in play order, so you can launch any chapter directly for a replay without
flying the whole arc again. A campaign may also carry mid-story chapters that
the story reaches on its own; they are listed under the same header. Scenarios that belong to no campaign (standalone
mod scenarios) list on their own below the campaigns.

You can author your own scenarios and mods in RON - see the [Create docs](../../create/).
