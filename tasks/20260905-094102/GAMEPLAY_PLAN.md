# Nova Protocol gameplay plan

This plan turns `STORY.md` into game features without turning Nova into an open
world. The campaign stays linear. Story logic stays in scenario variables where
possible. New engine vocabulary is added only when a Nova Protocol scene cannot
say its action clearly with what exists, and each addition is tagged with the
scenario that first needs it.

The plan has four parts: the scope decisions, what the engine has today, what
each scenario needs, and the consolidated inventory with a release cadence.

## Scope decisions

- v0.13.0 ships the story on the website first and keeps the current First
  Shift and Second Shift as playable proofs of concept.
- The game follows the final story over later releases. The current scenarios
  do not constrain it. Their dialogue and plot are provisional.
- Ship progression is scripted. Recovering a part installs it in its authored
  position, and the next scenario starts with the matching authored hull stage.
  No cargo grid, economy, crafting tree, or open-world fitting.
- Air, broadcast progress, code validity, alert state and similar story
  mechanics are scenario or campaign variables shown through data-driven HUD
  widgets. None of them becomes a simulation system.
- Radio reach and the Datum broadcast are scripted story state. There is no
  radio simulation.
- Docking is a later feature. Until it exists, an approach area plus a
  settled-speed condition stands in for a dock.
- A station is a static ship built from sections. A body mode or role marker
  keeps it from drifting. It reuses sections, damage, turrets and scripted
  actions.
- The midpoint choice is two places to fly to, not a menu. Handing over the
  module is one short bad ending.
- The comic and the game share the main events and not the pages. The three
  Roost missions and the two Datum scenarios are game content; every cutscene
  beat is also a comic page.
- Campaign authoring stays data-driven through the Rust builders and the event
  editor. Nova Protocol is the primary client; generic vocabulary is a
  consequence, not the first objective.

## What the engine has today

Verified against `crates/nova_scenario` on 2026-09-05.

| Kind | Today |
| --- | --- |
| Objects (7) | Anchor, Asteroid, Spaceship, Beacon, SalvageCrate, Light, Planet. |
| Events (24) | OnStart, OnDefeated, OnDestroyed, OnNeutralized, OnUpdate, OnTimerEnd, OnEnter, OnExit, OnGotoComplete, OnStopComplete, the five orbit events, travel and combat lock start and end, and the five ship-order lifecycle events. |
| Filters (5) | Entity, Conditional (Not, Or, And), Expression, Timer, ShipOrder. |
| Actions (42) | Variables and timers; Objective, ObjectiveComplete, marker attach and detach, hint emphasis; SpawnScenarioObject, ScatterObjects, DespawnScenarioObject, CreateScenarioArea; SetSpeedCap, SetInfiniteAmmo, RefillAmmo, SetControllerVerb, SetAllegiance; MoveShipTo, ForceAlign, StopShip, PatrolShip, OrbitShip, ClearShipOrder; SetAILeash, SetAIEngageRange, SetAIPointDefenseRange; ForceRailgunFire, ForceTorpedoFire; SetCamera, SetCameraAnchor, ReleaseCamera; SuspendPlayerControl, ResumePlayerControl; Screenshot, SetSkybox; NextScenario, Outcome; StoryMessage, HudReadout, Sequence, DebugMessage. |
| Queries | Scenario elapsed time. Entity speed. |
| HUD | Readouts (number, integer, time) in named slots, objective list with delayed posting, markers, ammo readouts, edge indicators, a Cinematic visibility level. |
| Narration | StoryMessage with speaker, text, dwell and portrait icon, rendered as comms cards, with the transcript in NOVA OS. |
| Direction | A `None`-controller ship takes helm orders and scripted railgun or torpedo shots. An AI-controller ship patrols or orbits, keeps a leash, changes engagement and point-defense ranges, stays a non-combatant, and interrupts a helm order on contact or damage. Allegiance is Player, Enemy or Neutral. |
| Proofs | First Shift proves tutorial pacing, crew comms, camera authority, scripted capital-ship movement and the Meridian strike. Second Shift proves wreck search, pickups, patrol orders, concealment, detection escalation, pursuit and scenario chaining. |

Two rules follow from the table. First, most new mission intents are small:
the engine already moves, aligns, patrols, orbits and fires on order. Second,
three story mechanics need no new system at all: allegiance flips make allies
and neutrals, areas and timers make phases, and Outcome makes a bad ending.

## What each scenario needs

The beat numbers are `STORY.md`'s. R1 to R3 are the game-only Roost missions.
"Exists" means the current vocabulary carries it. "New" names the addition and
the inventory row below explains it. Cast lists the portraits the scenario
speaks through.

### 1. An ordinary shift (gameplay, First Shift today)

- Content: Cutter One, GOTO beacons, tag the hulls, pull the cache, Fleet
  challenge fragments on the guard channel, section nine as a crew joke.
- Exists: Beacon, SalvageCrate, areas for tagging, Meridian as a scripted ship,
  StoryMessage for comms, the tutorial hint emphasis.
- New: none required. `Sign` for hull markings and `OnInteraction` with a
  `tag` verb are polish, not blockers.
- Cast: the captain's crew, Demir, Brandt.
- Later: re-dialogue to the final story. Do this last, when the vocabulary
  the other chapters add is in place.

### 2. The strike (cutscene, First Shift today)

- Content: the clause read on the guard channel, Severance out of the shadow,
  the shots, Meridian dies, skiffs sweep, the cutter hides.
- Exists: Sequence, camera control, control suspension, scripted railgun and
  torpedo fire, spawn and despawn, concealment in the junk.
- New: the skip-safe `Cinematic` wrapper with `OnCinematicFinished` and
  `OnCinematicSkipped`, `PlaySound`, camera blend on pose changes, a
  `Comms` cue that can be fragmentary.
- Cast: Pell's voice unnamed, Demir.

### 3. Shelter in the junk, forty hours (gameplay)

- Content: find the tender hull, graft Cutter One onto it, salvage air, cells,
  two debris PDCs and the recorder module; Calloway's nine days; the recorder
  beacons when unseated.
- Exists: areas, timers, variables, Objective chain, StoryMessage with
  portrait, Outcome Defeat at zero air.
- New: `HudMeter` for air; `Prop` and `Pickup` with `OnCollected` for cells,
  air, PDCs and the module; the Meridian wreck as a static Spaceship without
  controller; `ReplaceShipHull` to move the player from Cutter One to Nova
  stage 0 mid-scenario; `InstallShipSection` for each PDC with a short
  installation cinematic; `SetInteractable` to gate the module until the wreck
  is reached; the campaign journal and `CampaignVariableSet` for
  `nova_hull_stage`, `pdc_count`, `has_module`; entity-exists query.
- Cast: the crew, Calloway.

### 4. The module (cutscene)

- Content: the archive plays: the Board's order, Calloway's acknowledgement,
  the no-rescue, Pell's refusal and boat-deck call, this morning's reading.
  Okoro's line. Halloran's silence.
- Exists: Sequence, camera on Nova.
- New: `NarrativeCue` presentation modes, above all `Archive` with timestamp,
  source and authentication state, and `Crew` for speech inside the ship;
  `OnNarrativeCueFinished` so the sequence paces on real display time; the
  Cinematic wrapper; optional voice clips.
- Cast: the Board as text, Calloway, Pell, the crew.

### 5. The run (gameplay)

- Content: the skiffs come to strip the carcass, read Nova as a company
  survivor with salvage and chase it for real; first PDC fight through the
  junk toward Datum.
- Exists: AI ships with Enemy allegiance pursue the player; scatter for junk;
  concealment; Beacon or area for the Datum approach; ammo readouts.
- New: `Group` filter and group alive or neutralized count query for the
  skiff pack; `OnShipDamaged` for the crew's first-time-under-fire lines.
  `AttackShip` is not needed: allegiance and engagement range already point
  the pack at the player.
- Cast: the crew, Calloway on the channel.

### 6. Recovery (gameplay)

- Content: Parallax early on the approach, module first and crew second,
  rotation offered by name; refuse and dive into the junk, or hand over and
  fly the GOTO to a Defeat; Parallax fires to disable; Rotation follows,
  holds fire, gives the Kestrel word and a heading.
- Exists: MoveShipTo and ForceAlign for Parallax's intercept, two areas as
  the choice, Outcome Defeat with a message for the bad ending, SetAllegiance
  Neutral to make Rotation hold fire, MoveShipTo to bring Rotation alongside,
  SetSpeedCap to sell a hit drive.
- New: distance-between-objects query for "out of Parallax's range" and for
  Rotation's approach; ship structural health fraction query; `OnShipDamaged`
  to cue the disabling shots. `SetCombatPreference { disable_only }` only if
  scripted shots and a speed cap cannot sell the disable.
- Campaign: `calloway_refused`, `rotation_heading_received`.
- Cast: the crew, Calloway, Rotation's pilot.

### 7. The Roost (cutscene)

- Content: the village, Severance at the dock, Pell alive, Okoro's reunion,
  the module played in the village, Halloran's homecoming.
- Exists: Sequence, camera, static ships.
- New: `Installation` for the Roost; camera rail between poses; `Archive` and
  `Crew` modes reused; portraits for Pell and Solberg.
- Cast: Pell, Solberg, the crew.

### R1. The refit at the Nest (gameplay, game only)

- Content: Rotation leads Nova to a stripped Kestrel station; strip a drive
  and a sensor mast; learn how the Kites pick a wreck; install the parts.
- Exists: Rotation leads by a MoveShipTo chain with OnGotoComplete; areas;
  Pickups once they exist; the installation cinematic from beat 3.
- New: a second `Installation`; `InstallShipSection` for the drive and mast;
  `Sign` for Kestrel markings; a hazard, either a picket patrol with existing
  AI or drifting debris. `FollowShip` is optional and only if the waypoint
  chain plays badly.
- Campaign: `nova_hull_stage` 2, `pdc_count` 3, `sensor_mast`. The mast is a
  larger contact range on the stage-2 hull, not a new system.
- Cast: Rotation's pilot, Okoro.

### R2. The Azimuth shadow (gameplay, game only)

- Content: shadow Azimuth on its approach lane with Rotation, engines cold,
  and listen to four hundred voices make the renewal joke.
- Exists: Azimuth as a large ship on a MoveShipTo lane; escorting pickets as
  AI ships whose contact detection is the failure condition, the way Second
  Shift's detection works; timers for how long to hold; SetSpeedCap for cold
  engines.
- New: distance query to gate the comms by range from a moving ship;
  `OnDetected` and `OnContactLost` if the order-interrupt trick cannot carry a
  stealth scene cleanly; `HudMeter` reused for detection.
- Campaign: `azimuth_heard`.
- Cast: Azimuth's crew as unnamed voices, Halloran, Rotation's pilot.

### 8. The argument (cutscene)

- Content: Halloran to the sixty, Okoro's question, Solberg's answer, Pell
  takes Datum, the captain decides. No fork.
- Exists: Sequence, camera.
- New: nothing beyond the Cinematic wrapper and the `Crew` mode.
- Cast: Pell, Solberg, the crew.

### R3. The Roost defense (gameplay, game only)

- Content: Parallax's pickets follow Rotation's track home; hold the village
  with the skiffs; Solberg on the radio; the clock starts.
- Exists: pickets as Enemy AI, skiffs as Player-allegiance AI with a leash
  around the station, waves on timers, OnDestroyed for the station core if
  the station is a ship body.
- New: `OnSectionDestroyed` for station parts; `ObjectiveFail` when a named
  part or too many skiffs are lost; group alive count; `DefendObject` only if
  leash plus allegiance does not hold the allies where the fight is.
- Campaign: `roost_defended`, always true on completion. No fork.
- Cast: Solberg, Rotation's pilot, the crew.

### 9. The plan (cutscene)

- Content: Datum, the relay, Meridian's open roster and code, Nova first,
  Severance second, Pell's knowledge of Vigilant.
- Exists: Sequence, camera.
- New: `HudBanner` for a mission title card; nothing else.
- Cast: Pell, Solberg, the crew.

### 10a. Datum approach (gameplay)

- Content: in under Meridian's code through the turret envelope; kill control
  nodes and sensor masts before the code stops matching; pickets launch when
  it does; dock at the relay; signal Severance.
- Exists: Datum as Neutral until a timer or trigger flips it to Enemy, which
  is the code stopping to match; spawn and AI for pickets; OnEnter plus the
  speed query as the docking stand-in; MoveShipTo for Severance's arrival.
- New: `Installation` with `TurretBattery` built from existing weapon
  sections; control nodes and masts as destructible sections with
  `OnSectionDestroyed` and the `Section` filter; `SetSectionEnabled` so a
  dead control node silences its turrets; section active and health queries;
  `HudMeter` for code validity; `DockingPort` with `OnDocked` and `OnUndocked`
  when docking ships, replacing the stand-in.
- Cast: Datum control as System notices, Calloway, the crew.

### 10b. Hold the dock (gameplay)

- Content: the captain holds the dock while the module plays; Parallax
  returns; the duel; the archive on every channel; Calloway turns on the
  relay tower; Okoro's boat-deck call; Pell's ram; boats away; cover the boats
  into the junk; Vigilant decelerating; undock and run.
- Exists: OnExit to fail the mission if Nova leaves the dock area; timers and
  a variable for broadcast segments; Severance and Parallax with opposite
  allegiance fight through existing AI; ForceRailgunFire and ForceAlign for
  Calloway's shots at the tower; MoveShipTo for the ram, the boats' run and
  Vigilant's arrival; OnGotoComplete for boats reaching the junk mark.
- New: `SetObjectInvulnerable` to hold the duel until its cue;
  `DestroyScenarioObject` to end both ships through the real destruction path
  on cue; `SpawnScenarioObject` at a moving object's position for the boats;
  `AttackShip` so the duel targets by name rather than by nearest hostile;
  `ObjectiveProgress` for segments played and boats saved; group alive count
  for the boats; `HudMeter` for the broadcast; `Archive` cues marked as live
  on all channels; `ObjectiveFail` if the dock falls before the module ends.
- Campaign: `severance_survivors` for the ending text.
- Cast: Calloway, Pell, Okoro, Halloran, Solberg, Marsh.

### 11. Minute zero (cutscene)

- Content: Vigilant takes the victory, the official account, the praise and
  the rotation home to a channel nobody answers, every recorder at Saturn,
  Azimuth holding station, the Roost.
- Exists: Sequence, camera, NextScenario to the menu.
- New: a `System` mode card for the press release; the final camera at the
  Roost. Credits are a menu concern.
- Cast: Marsh, EWI as text, the crew in silence.

## Consolidated inventory

Each row names the first scenario that needs it. Build a row with its
lifecycle events, filters, lint, editor reflection, documentation and tests,
and not before its scenario.

### Events

| Event | Shape | First needed |
| --- | --- | --- |
| OnCinematicFinished, OnCinematicSkipped | `{ key }` | 2 |
| OnNarrativeCueFinished | `{ key }` | 4 |
| OnCollected | `{ object, collector }` | 3 |
| OnInteraction | `{ object, actor, verb }` | 3 for connect, 1 for tag as polish |
| OnShipDamaged | `{ ship, source }` | 5 |
| OnSectionDestroyed | `{ ship, section }` | R3 |
| OnDetected, OnContactLost | `{ observer, target }` | R2, only if order interrupts fall short |
| OnObjectiveFailed | `{ id }` | R3 |
| OnDocked, OnUndocked | `{ ship, installation, port }` | 10a, with docking |
| OnInstallComplete | `{ ship, section }` | 3, if the install cinematic needs a hook |

### Filters

| Filter | Shape | First needed |
| --- | --- | --- |
| Group | `{ group, other_group }` over authored groups such as `skiffs`, `boats`, `control_nodes` | 5 |
| Section | `{ ship, section, kind }` | 10a |
| Interaction | `{ object, actor, verb }` | 3 |
| Campaign variables | import into ordinary variables before OnStart and keep one Expression filter | 3 |

### Actions

| Action | Purpose | First needed |
| --- | --- | --- |
| StartCinematic, CancelCinematic | skip-safe scene lifetime, see below | 2 |
| PlaySound | authored cue | 2 |
| NarrativeCue | StoryMessage with a presentation mode: Comms, Crew, Archive, System, Cinematic | 4 |
| NarrativeClear | drop a queued or shown cue | 4 |
| HudMeter | bound meter with thresholds | 3 |
| HudBanner | mission or location title | 9 |
| ReplaceShipHull | swap the player onto an authored hull stage, preserving transform, velocity, controller, allegiance | 3 |
| InstallShipSection, RemoveShipSection | a recovered part moves into its authored mount | 3 |
| SetInteractable | gate a prop's verbs | 3 |
| CampaignVariableSet, CampaignVariableClear | explicit campaign-scoped writes | 3 |
| ObjectiveFail | failed state distinct from Defeat | R3 |
| ObjectiveProgress | `{ id, current, total }` | 10b |
| SetSectionEnabled, RepairSection | silence a turret group, restore a part | 10a |
| SetObjectInvulnerable | hold a set piece until its cue | 10b |
| DestroyScenarioObject | run the real destruction path on cue | 10b |
| Spawn at object | `SpawnScenarioObject` gains an `at: object` origin | 10b |
| AttackShip | fight a named target | 10b |
| FollowShip, DefendObject | optional mission intents | R1, R3, only if staging fails |
| SetCombatPreference | `{ target_sections, disable_only }` | 6, only if staging fails |
| ForceTurretFire | fully scripted PDC shot | none yet |
| StartMusic, SetMusic, StopMusic | when a soundtrack exists | none yet |

`FleeTo` is `MoveShipTo` to a far mark. `EscortShip` is a leash plus
allegiance. Neither needs a new action.

### Queries

| Query | First needed |
| --- | --- |
| entity exists | 3 |
| distance between two named objects | 6 |
| ship structural health fraction | 6 |
| group alive count, group neutralized count | 5 |
| named section active, named section health fraction | 10a |
| docked state and dock partner | 10a, with docking |
| installed-section presence | 3, if a scenario reads the hull instead of the journal |
| current helm order and order state | none yet; the ship-order events carry it |

### Scenario objects

| Object | Purpose | First needed |
| --- | --- | --- |
| Prop | authored mesh, transform, optional collider, material, highlight | 3 |
| Pickup | a collected object with OnCollected; SalvageCrate is its first look | 3 |
| Wreck | a static Spaceship with no controller and no thrust; Meridian's carcass | 3 |
| Area as an object | static areas declared like other objects; CreateScenarioArea stays for runtime areas | 3 |
| Sign | authored text or image: WE ARE EXPANDING, Kestrel markings, station warnings | R1 |
| Installation | static ship body with a station role: the Roost, the Nest, Datum | 7 |
| TurretBattery | an installation part built from existing weapon sections | 10a |
| DockingPort | a capability on an installation | 10a, with docking |

Content that is not a new object kind: skiffs, pickets, boats and every named
ship are catalog ships; the Roost, the Nest and Datum are installations
assembled from sections; radio, archive records and Fleet's triangulation are
variables and cues.

### Catalog content

| Content | Needed for |
| --- | --- |
| Nova Protocol hull stages 0, 1, 2 | 3, 3, R1 |
| Skiff, with Rotation's markings | 2 |
| Picket | R2 |
| Escape boat, a skiff variant | 10b |
| Parallax | 6 |
| Severance | 2 |
| Azimuth | R2 |
| Vigilant | 10b |
| The Roost, the Nest, Datum with relay tower, control nodes, sensor masts, turret batteries | 7, R1, 10a |
| Portraits: Pell, Solberg, Rotation's pilot, Calloway, Marsh, Azimuth voice, Archive and System cards | 4, 6, 7, 10b, 11 |

### HUD and menus

- Narrative presentation: one focused current line, one or two previous lines
  reduced, speaker and channel in a stable place, faction treatment (EWI
  green, Fleet blue, Kites amber), archived material visibly recorded, a
  portrait, an optional voice clip, an authored dwell, a completion event, and
  an explicit interruption policy: queue, replace a repeated warning, or
  interrupt and resume. Objectives stay separate from dialogue. First needed
  by 4; a first pass may keep StoryMessage serialization and replace only the
  HUD presentation.
- HudMeter (3), HudBanner (9), ObjectiveProgress display (10b), failed
  objective state (R3).
- HudChoice is deferred. The midpoint choice is two destinations.
- Campaign menu: New Campaign, Continue, Restart Chapter, and the scenario
  picker as replay that never writes campaign progress. First needed by 3.
- The transcript stays in NOVA OS.

### Cinematic contract

A small wrapper around a Sequence, not a timeline editor:

```text
Cinematic {
    key,
    skippable,
    steps,
    on_finish,
    on_skip,
}
```

- `steps` use the sequence-step and action vocabulary.
- `on_finish` restores the intended post-scene state.
- `on_skip` authors the landing frame: actors spawned, destroyed or moved,
  variables latched, objectives posted, camera and control released.
- Skip cancels the cursor. It does not run every intermediate action at once.
- Teardown cancels the cinematic and restores input.
- Only the current skippable cinematic consumes the skip binding, and the
  binding is held briefly to prevent accidents.

Camera additions in order of need: blend duration and easing (2), a rail
between two poses (7), authored field of view and letterbox (7).

### Campaign continuity

A Wesnoth-style boundary save. Never serialize Bevy's world, live entities,
pending physics or event handlers.

```text
CampaignProgress {
    campaign_id,
    content_revision,
    next_scenario,
    completed_scenarios,
    campaign_variables,
}
```

- New Campaign creates the journal. A successful NextScenario commits the
  completed chapter and the next one. Continue loads `next_scenario`. Restart
  Chapter reloads the current scenario from its initial state.
- Campaign variables are imported into the event world before OnStart. The
  saved facts cause the next scenario's authored OnStart handlers to build the
  correct world: `nova_hull_stage` picks the hull, `pdc_count` and
  `sensor_mast` are already on it.
- Variables the story writes: `has_module`, `nova_hull_stage`, `pdc_count`,
  `sensor_mast`, `calloway_refused`, `rotation_heading_received`,
  `azimuth_heard`, `roost_defended`, `severance_survivors`.
- Mid-scenario checkpoints come later and need an authored checkpoint that
  declares its reconstruction state.

### Scripted ship progression

Authored hull stages, not free fitting. `InstallShipSection` when the player
should see a part move into place, `ReplaceShipHull` between scenarios to load
the validated stage.

1. Enter the pickup area, or complete the interaction.
2. OnCollected identifies the part.
3. Suspend control and start a short installation cinematic.
4. Install the section at its authored mount.
5. Write the campaign variable at mission completion.
6. The next scenario spawns the stage that already includes the part.

## Cadence

One scenario and one enabling feature slice at a time. Version numbers after
v0.14.0 are not fixed here.

### v0.13.0: prove and publish

Website:

1. `STORY.md` is the accepted story, with `REVIEW01.md` folded in.
2. Write and publish the Shelter prologue comic.
3. Write the main comic or, if the art pass is larger than this release,
   publish the complete chapter text and the marked comic plan without
   presenting unfinished pages as final.
4. The website story is the source later game chapters are designed from.

Game: ship and verify First Shift and Second Shift as proofs of concept. Do
not rewrite Second Shift to the final story. Release checks:

- New Game starts First Shift, and First Shift reaches Second Shift through
  Continue.
- Both scenarios complete and retry. The strike cannot return control during
  its cinematic. Second Shift completes unseen and detected.
- Scenario picker replay works. A full gamepad playthrough works.
- The website story is linked and readable on desktop and mobile.

Optional: one short uncampaigned Combat Drill with existing ships and
vocabulary, one armed gunship, one PDC target, one torpedo threat, finite
ammunition, a retry. Cut it if it threatens the story, gamepad or release work.

### v0.14.0: stabilize and package

Bug fixes, balance, polish and store packaging only. Watch where players lose
narrative lines, misread objectives or never find combat, and keep the
evidence for the next slice.

### Increment A: Forty Hours (beat 3)

Content: the tender, the graft, air, cells, two PDCs, the module, Calloway's
nine days. Features: campaign journal and Continue, campaign variables before
OnStart, HudMeter, Prop and Pickup with OnCollected, the Wreck body,
ReplaceShipHull and InstallShipSection, SetInteractable, entity-exists query.

### Increment B: The Archive (beat 4, and the strike's cinematic)

Content: the module plays; the strike in First Shift moves onto the Cinematic
wrapper. Features: NarrativeCue modes and the focused-line HUD,
OnNarrativeCueFinished, Cinematic with its two events, PlaySound, camera
blend, voice clip support, the campaign menu with Restart Chapter.

### Increment C: The Run and Recovery (beats 5 and 6)

Content: the skiffs chase for real, Parallax asks for the module, refusal and
the dive, the bad ending, Rotation's word. Features: Group filter and group
counts, OnShipDamaged, distance and health queries, two-area choice, Rotation
by allegiance flip. SetCombatPreference only if staging cannot sell the
disable. Docking is still not required.

### Increment D: The Roost (beats 7, R1, R2, 8, R3, 9)

Content: Pell alive, the module in the village, the refit, the shadow, the
argument, the defense, the plan. Features: Installation body and role, Sign,
Interaction verbs, InstallShipSection for drive and mast, hull stage 2,
OnSectionDestroyed with the Section filter, ObjectiveFail, HudBanner, camera
rail, Pell and Solberg portraits, OnDetected only if needed.

### Increment E: Datum (beats 10a, 10b, 11)

Content: the approach under Meridian's code, the dock, the broadcast, the
duel, Calloway's turn, the boats, extraction, minute zero. Features:
TurretBattery, SetSectionEnabled, section queries, docking or its stand-in,
spawn at object, SetObjectInvulnerable, DestroyScenarioObject, AttackShip,
ObjectiveProgress, group counts for the boats, Vigilant, System cards.

### Last: the opening

Re-dialogue First Shift to beats 1 and 2 of the final story, on the vocabulary
the other increments added. Retire Second Shift's plot; its systems live on in
Increment A.

## Explicit non-goals

- Open-world Saturn, a simulated economy, free-form cargo and crafting, a
  skill tree.
- Custom factions before a mission needs more than Player, Enemy and Neutral.
- General radio propagation.
- On-foot station interiors. Datum's inside is unseen; the captain never
  leaves the helm.
- A custom campaign or cutscene editor. A large dialogue tree.
- Saving raw ECS worlds or physics state.

The target is a linear authored campaign whose systems are clear enough to
reuse. Every feature must make one Nova Protocol action more playable:
survive, recover, install, refuse, escape, persuade, shadow, defend, dock,
transmit, or rescue.
