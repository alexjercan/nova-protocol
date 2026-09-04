# Actions

Everything a handler can DO. Actions run in authored order once every filter
passes; each is a newtype variant - `Name((field: value, ...))`, double
parens even for one field. Failures warn and continue (a missing target id
never panics a scenario). All 38 at a glance:

| action | group | what it does |
|---|---|---|
| [`SpawnScenarioObject`](#spawnscenarioobject) | [world](#spawning-the-world) | spawn one object: asteroid, ship, beacon, crate, or light |
| [`ScatterObjects`](#scatterobjects) | [world](#spawning-the-world) | spawn `count` copies of a template at deterministic random positions |
| [`DespawnScenarioObject`](#despawnscenarioobject) | [world](#spawning-the-world) | remove a scoped object and its whole child hierarchy |
| [`CreateScenarioArea`](#createscenarioarea) | [world](#spawning-the-world) | spawn an invisible spherical sensor zone for `OnEnter` / `OnExit` |
| [`Objective`](#objective) | [mission](#mission-story) | post (or update in place) a HUD objective |
| [`ObjectiveComplete`](#objectivecomplete) | [mission](#mission-story) | complete and remove the HUD objective with an id |
| [`ObjectiveMarkerAttach`](#objectivemarkerattach) | [mission](#mission-story) | pin the gold HUD marker chip on a scoped object |
| [`ObjectiveMarkerDetach`](#objectivemarkerdetach) | [mission](#mission-story) | remove that marker |
| [`StoryMessage`](#storymessage) | [mission](#mission-story) | queue a speaker-attributed comms line |
| [`HudReadout`](#hudreadout) | [mission](#mission-story) | bind a live HUD readout to a scenario variable |
| [`HintEmphasisSet`](#hintemphasisset) | [mission](#mission-story) | pulse one keybind-dock chip gold |
| [`HintEmphasisClear`](#hintemphasisclear) | [mission](#mission-story) | drop the gold emphasis on one chip |
| [`Sequence`](#sequence) | [pacing](#pacing) | run an ordered list of beats, each behind its own delay or gate |
| [`Outcome`](#outcome) | [flow](#flow-outcomes-transitions) | show the VICTORY / DEFEAT banner and freeze the sim behind it |
| [`NextScenario`](#nextscenario) | [flow](#flow-outcomes-transitions) | queue a switch to another scenario by id |
| [`SetSpeedCap`](#setspeedcap) | [ship state](#ship-state) | install, update or remove the soft manual-speed governor |
| [`SetControllerVerb`](#setcontrollerverb) | [ship state](#ship-state) | grant or withhold one flight verb on a ship's controller |
| [`SetAllegiance`](#setallegiance) | [ship state](#ship-state) | overwrite a ship's side at runtime |
| [`MoveShipTo`](#moveshipto) | [ship state](#ship-state) | fly an ordered ship to a mark and report when it arrives |
| [`ForceAlign`](#forcealign) | [ship state](#ship-state) | turn an ordered ship's nose onto a point and hold it there |
| [`StopShip`](#stopship) | [ship state](#ship-state) | bring an ordered ship to rest with the real STOP burn |
| [`PatrolShip`](#patrolship) | [ship state](#ship-state) | fly one loop of an authored waypoint route, back to where it started |
| [`OrbitShip`](#orbitship) | [ship state](#ship-state) | put a ship in a stable ring around a gravity well and hold it |
| [`ClearShipOrder`](#clearshiporder) | [ship state](#ship-state) | cancel whatever helm order a ship is under |
| [`ForceRailgunFire`](#forcerailgunfire) | [ship state](#ship-state) | fire one named railgun section |
| [`ForceTorpedoFire`](#forcetorpedofire) | [ship state](#ship-state) | launch one named torpedo bay at a named target |
| [`SetInfiniteAmmo`](#setinfiniteammo) | [ship state](#ship-state) | suspend or restore the finite magazine on every weapon of a ship |
| [`RefillAmmo`](#refillammo) | [ship state](#ship-state) | top a ship's magazines back up, or just one section's |
| [`SetAILeash`](#setaileash) | [ship state](#ship-state) | tether an AI ship's combat to a centre and radius, or release it |
| [`SetAIEngageRange`](#setaiengagerange) | [ship state](#ship-state) | change how far an AI ship looks for hostiles, or restore the default |
| [`SetAIPointDefenseRange`](#setaipointdefenserange) | [ship state](#ship-state) | change how close an inbound torpedo gets before the guns answer |
| [`VariableSet`](#variableset) | [variables](#variables-timers-debugging) | evaluate an expression and store the result in a variable |
| [`TimerStart`](#timerstart) | [variables](#variables-timers-debugging) | start (or restart) a keyed scenario timer |
| [`TimerCancel`](#timercancel) | [variables](#variables-timers-debugging) | cancel a running timer |
| [`DebugMessage`](#debugmessage) | [variables](#variables-timers-debugging) | log a line in debug builds |
| [`SetCamera`](#setcamera) | [camera](#camera-photo-mode) | pin the scenario camera at a pose |
| [`SetCameraAnchor`](#setcameraanchor) | [camera](#camera-photo-mode) | ride the camera on an object, framing what you name |
| [`ReleaseCamera`](#releasecamera) | [camera](#camera-photo-mode) | hand the camera back to the player's chase rig |
| [`SuspendPlayerControl`](#suspendplayercontrol) | [camera](#camera-photo-mode) | block human gameplay input and clear held intent |
| [`ResumePlayerControl`](#resumeplayercontrol) | [camera](#camera-photo-mode) | restore human gameplay input |
| [`Screenshot`](#screenshot) | [camera](#camera-photo-mode) | capture the primary window to a PNG |
| [`SetSkybox`](#setskybox) | [camera](#camera-photo-mode) | swap the scenario's skybox mid-scenario |

**Scoped targets.** Every by-id action resolves its id ONLY among
scenario-scoped entities (things this scenario spawned); the ship actions
additionally require the id to be a ship ROOT. This is deliberate: ship
SECTIONS carry per-ship ids like `"controller"` too, and an unscoped lookup
would hit every ship's section. Referencing an id before it is spawned warns
and does nothing - spawn first.

## Spawning & the world

### SpawnScenarioObject

Spawn one object. `base` is the shared identity block; `kind` picks the
object and carries its config - the six kinds are the
[Scenario objects reference](../objects/).

```ron
SpawnScenarioObject((
    base: (id: "rock_1", name: "Rock", position: (100.0, 0.0, -400.0), rotation: (0.0, 0.0, 0.0, 1.0)),
    kind: Asteroid((radius: 50.0, texture: "dep://base/textures/asteroid.png", material: "rock", invulnerable: false)),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `base` | object base | required | identity + pose (below) |
| `kind` | object kind | required | `Anchor((..))` / `Asteroid((..))` / `Planet((..))` / `Spaceship((..))` / `Beacon((..))` / `SalvageCrate((..))` / `Light(..)` |

The `base` block:

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the object's scenario id - the address every event, filter and by-id action uses |
| `name` | string | required | display name |
| `position` | 3-tuple | required | world position, meters |
| `rotation` | 4-tuple | required | world rotation quaternion `(x, y, z, w)` |

The same spawn id twice in ONE handler is a lint Error; the same id across
two handlers (an either-or fork) is a Warn.

</details>

### ScatterObjects

Spawn `count` copies of a template at deterministic random positions - the
declarative asteroid-field primitive. Copy `i` gets id `"{id_prefix}{i}"`.

```ron
ScatterObjects((
    id_prefix: "asteroid_",
    count: 20,
    seed: 433757350076153856,
    region: Box(min: (-1000.0, -200.0, -1000.0), max: (1000.0, 200.0, 1000.0)),
    template: (
        base: (id: "asteroid_", name: "Asteroid", position: (0.0, 0.0, 0.0), rotation: (0.0, 0.0, 0.0, 1.0)),
        kind: Asteroid((radius: 10.0, texture: "dep://base/textures/asteroid.png", material: "rock", invulnerable: false)),
    ),
    asteroid_radius: Some((10.0, 30.0)),
    asteroid_kinds: [("rock", 12), ("carbon", 4), ("ice", 3), ("metal", 1)],
    min_separation: Some(320.0),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id_prefix` | string | required | id prefix for the copies (a filter id starting with this prefix lints clean) |
| `count` | integer | required | copies; runtime cap 4096, absurd counts are a lint Error |
| `seed` | integer | required | RNG seed - the same seed gives the same layout every load. Asteroid templates without an authored `seed` also get deterministic per-rock silhouette seeds derived from it, so the field's shapes are stable too |
| `region` | region | required | sampling volume in meters (below) |
| `template` | object config | required | the object each copy clones (any kind; same `base`/`kind` shape as `SpawnScenarioObject`) |
| `asteroid_radius` | `Option` (lo, hi) | `None` | Asteroid templates only: randomize each rock's radius, in meters, in `[lo, hi)` |
| `asteroid_kinds` | list of (kind, weight) | required on an asteroid template | the field's [KINDS](../objects/#what-a-rock-is-made-of) and how common each one is. Weights are relative COUNTS, not percentages: `[("rock", 12), ("metal", 1)]` is one metal body in thirteen. Empty on any other template |
| `min_separation` | `Option` number | `None` | minimum centre-to-centre distance in meters against EVERY body scattered so far this scenario, earlier scatters included; 64 placement tries per copy, unplaceable copies are DROPPED, never overlapped |

`region` variants (struct variants - single parens, named fields):

| variant | fields | meaning |
|---|---|---|
| `Box(min: (..), max: (..))` | both required | uniform per axis in `[min, max]` |
| `Ring(center: (..), inner: .., outer: .., y_min: .., y_max: ..)` | `center` defaults to the origin | horizontal annulus: uniform angle, radius in `[inner, outer]`, y in `[y_min, y_max]` |

`asteroid_kinds` is drawn from the scatter's OWN seed, on a stream of its own:
adding a kind to a field that already ships moves no rock and changes no
radius. An asteroid template with no mix, or with every weight at zero, is a
lint error and spawns nothing - a belt has to say what it is made of.

Set `min_separation` on any field of SOLID bodies: uniform sampling WILL
nest rocks inside each other, and two overlapping dynamic bodies shove apart
violently on the first physics step. Size it as the two widest bodies side
by side - for asteroids that is NOT `radius`: the noise mesh reaches up to
6x the nominal radius. Scatter results are gameplay content and are never
thinned by graphics quality.

</details>

### DespawnScenarioObject

Remove the scoped object whose id matches (recursively, whole child
hierarchy). The classic pairing is a salvage crate on pickup.

```ron
DespawnScenarioObject((id: "crate_1")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped object to despawn; missing id warns |

A body despawned while inside a trigger area fires NO `OnExit` for itself.

</details>

### CreateScenarioArea

Spawn an invisible spherical SENSOR zone that drives
[`OnEnter`](../events/#onenter) / [`OnExit`](../events/#onexit) under its
id.

```ron
CreateScenarioArea((id: "safe_zone", name: "Safe Zone",
    position: (0.0, 0.0, -1000.0), rotation: (0.0, 0.0, 0.0, 1.0), radius: 100.0)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the id `OnEnter`/`OnExit` report as the area |
| `name` | string | required | display name |
| `position` | 3-tuple | required | sphere centre, meters |
| `rotation` | 4-tuple | required | rotation (cosmetic for a sphere) |
| `radius` | number | required | sensor radius, meters |

Works mid-scenario, and works even when created AROUND a body already
inside (the entry still fires).

Beacons with `area_radius` and salvage crates are their OWN areas - no
separate `CreateScenarioArea` needed for those.

</details>

## Mission & story

### Objective

Post a HUD objective. Objectives state goals; comms lines
([`StoryMessage`](#storymessage)) carry voice.

```ron
Objective((id: "destroy_asteroids", message: "Objective: Destroy 5 asteroids!")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | opaque handle `ObjectiveComplete` uses |
| `message` | string | required | the objective text on the HUD |

Re-posting the same id with new text updates the entry in place (the
"recovered N/3" tally trick). A duplicate post otherwise warns.

</details>

### ObjectiveComplete

Complete (remove) the HUD objective with this id.

```ron
ObjectiveComplete((id: "destroy_asteroids")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | the objective to complete; a never-posted id warns (and lints) |

</details>

### ObjectiveMarkerAttach

Pin the gold HUD marker chip (label + live distance) on a scoped object.

```ron
ObjectiveMarkerAttach((target_id: "beacon_1", label: "BEACON 1")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | string | required | scoped object to mark; spawn it first |
| `label` | string | required | short chip text ("BEACON 1") |

Re-attaching updates the label in place. A despawned target detaches
implicitly.

</details>

### ObjectiveMarkerDetach

Remove that marker.

```ron
ObjectiveMarkerDetach((target_id: "beacon_1")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `target_id` | string | required | scoped object to strip; a missing id is quietly fine (detach-after-despawn is legitimate) |

</details>

### StoryMessage

A speaker-attributed line for the HUD comms stack (bottom-left, arrival
order, ~8 s hold each, at most three visible). Pending lines wait without being
dropped. One line per beat is still the style; the queue is the safety net.

```ron
StoryMessage((speaker: "Foreman Okono", text: "Strip it clean, Kestrel.", dwell: Some(12.0))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `speaker` | string | required | the distinct uppercase speaker header |
| `text` | string | required | the line |
| `dwell` | `Option` number | `None` | per-line hold override in seconds, clamped to [3, 30] (lint warns outside); `Some(12.0)` |
| `icon` | `Option` asset ref | `None` | speaker portrait (`Some("self://icons/okono.png")`); omitted = the cockpit fallback tile |

Scenario-scoped: teardown clears the log.

Two story lines in one handler is a lint Warn (unreadable); a story line
beside an [`Outcome`](#outcome) in one handler is a Warn (frozen behind the
overlay). Let the outcome's own `message` carry the closing line.

</details>

### HudReadout

Bind a live HUD readout to a scenario variable - the DISPLAY half of the
variable vocabulary (a run clock, a score, a countdown).

```ron
HudReadout((slot: "run_timer", variable: "scenario_elapsed", format: Time, label: Some("TIME"))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `slot` | string | required | stable readout id; several slots run side by side; fire again to update or clear |
| `variable` | string | required | the variable shown (e.g. `"scenario_elapsed"`); undefined or non-numeric reads 0.0 |
| `format` | format | `Number` | `Number` (one decimal), `Integer` (rounded), `Time` (`mm:ss.s`) |
| `label` | `Option` string | `None` | caption before the value (`Some("TIME")`) |
| `visible` | bool | `true` | `true` shows/updates; `false` clears exactly this slot |

One fire is enough; the readout tracks the variable's current value every
frame thereafter. It freezes under pause and behind the outcome overlay
because the variable does - a time-trial's final time simply holds through
the banner. Cleared at teardown.

</details>

### HintEmphasisSet

Pulse one keybind-dock chip gold - how a tutorial points at a key before
granting it.

```ron
HintEmphasisSet((verb: "RADAR")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `verb` | string | required | one of `"STOP"`, `"GOTO"`, `"ORBIT"`, `"CANCEL"`, `"RADAR"`, `"COMPONENT"`, `"RCS"`; unknown verbs warn and do nothing |

The dock normally hides verbs the player cannot use yet, so emphasizing an
unavailable verb REVEALS its chip dimmed and pulses it. Emphasis never
grants the verb ([`SetControllerVerb`](#setcontrollerverb) does).

</details>

### HintEmphasisClear

Drop the gold emphasis on one chip (teardown clears all emphasis anyway).

```ron
HintEmphasisClear((verb: "RADAR")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `verb` | string | required | the chip to un-pulse |

</details>

## Pacing

### Sequence

Run an ordered list of BEATS. The ENGINE holds the cursor, so a paced chain
costs one action and no scenario variable.

```ron
Sequence((
    key: "opening",
    steps: [
        (
            after: Some(2.0),
            actions: [
                StoryMessage((
                    speaker: "Capt. Halloran",
                    text: "Kestrel, you are cleared to burn.",
                )),
            ],
        ),
        (
            after: Some(8.4),
            actions: [
                Objective((id: "b1_burn", message: "Fly to BEACON 1.")),
            ],
        ),
    ],
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `key` | string | required | scenario-local chain key; the engine files the cursor under it |
| `steps` | list | required | the beats, in order; at least one |

A step:

| field | type | default | meaning |
|---|---|---|---|
| `after` | seconds | none | scenario time to wait, from when this step became current |
| `until` | event + filters | none | an event that must arrive, qualified by its filters |
| `deadline` | seconds | none | how long the step may wait before the chain is called stuck; REQUIRED with `until` |
| `actions` | list | `[]` | what the beat does |

Both waits may sit on one step, and they run TOGETHER: a gate that opens early
still owes the delay, and a delay that elapses first still owes the gate. A
step with neither runs the moment it becomes current - which is how a hand-off
beat rides the end of a chain.

```ron
(
    after: Some(6.0),
    until: Some((
        name: OnUpdate,
        filters: [Expression((Equal(
            Term(Factor(Name("surveyed"))),
            Term(Factor(Literal(Number(1.0)))),
        )))],
    )),
    deadline: Some(600.0),
    actions: [ /* the beat */ ],
),
```

The semantics are WAIT, never SKIP: a step whose gate stays shut blocks the
beats behind it. That is why a gated step must carry a `deadline` - when it
expires the chain STOPS and logs an error, so a soft-lock is loud instead of
silent. `content lint` refuses a gated step without one.

Delays ride the pause-frozen scenario clock, so a gap measures play time, not
wall time.

One cursor per key. Starting a key whose chain is still running is refused and
logged; a chain that has finished frees its key. Several handlers MAY start the
same key when only one of them can ever fire - every win variant of a scenario
starting one shared outro chain is the idiom.

A step's actions are a FRAME of their own, landing seconds after the handler
that queued them, and everything else applies inside a step unchanged: an
objective still must not share a beat with a comms line, and a beat may start
a chain of its own.

**What it replaces.** A paced chain used to be sibling `OnUpdate` handlers
strung together by hand: a counter variable seeded in `OnStart`, a
`VariableSet` per beat to advance it, and a filter per beat reading both the
counter and `scenario_elapsed` against a stamped deadline. All of that is
about the machine. Reach for a chain whenever the only thing a beat is waiting
for is "later"; keep a handler where the beat must still ASK something when it
lands - which of two lines to speak, whether the fight is still live - because
a step runs when its wait ends and only a handler can re-check.

</details>

## Flow: outcomes & transitions

### Outcome

Declare the scenario's win or lose: the gold VICTORY / red DEFEAT banner,
an optional message, and buttons - and it freezes the simulation behind it
exactly like the pause menu until it clears.

```ron
Outcome((outcome: Defeat, message: Some("The cutter is lost."))),
NextScenario((scenario_id: "second_shift", linger: true)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `outcome` | kind | required | `Victory` or `Defeat` |
| `message` | `Option` string | `None` | the line under the banner; strict RON `Some("...")` |
| `auto_advance_secs` | `Option` number | `None` | timed banner: advance the queued LINGERING switch after this many REAL seconds (the player can still click sooner); cap 300 s; meaningless without a lingering switch queued |

PRESENTATION ONLY: compose the consequence beside it. Queue a lingering
[`NextScenario`](#nextscenario) and the overlay offers Continue (Victory) or
Retry (Defeat); queue nothing and it offers only Main Menu.

The paired switch must LINGER: an instant switch tears the scenario down
the same frame and SWALLOWS the banner (lint warns on that composition).

</details>

### NextScenario

Queue a switch to another scenario by id - a hard cut, a delayed cut, or a
modal hold behind the outcome overlay.

```ron
NextScenario((scenario_id: "second_shift", linger: true)),
```

<details class="explain">
<summary>Show explanation</summary>

Three gears:

- **Hard cut** - `linger: false`, no delay: switches on the next sync.
  Menu-scene plumbing.
- **Delayed cut** - `linger: false, delay: Some(4.0)`: the world keeps
  playing for the delay (a story line can land), then cuts. Ticks on
  pause-frozen time; Enter skips the wait.
- **Modal hold** - `linger: true`: waits for the scenario-advance input
  (Enter / DPadDown) or the outcome overlay's Continue/Retry.

| field | type | default | meaning |
|---|---|---|---|
| `scenario_id` | string | required | target scenario; a dangling id is a lint Error |
| `linger` | bool | required | `true` defers until released; `false` switches now (or after `delay`) |
| `delay` | `Option` number | `None` | delayed non-lingering cut, seconds; cap 300 s, lint warns above 60; meaningless with `linger: true` |

The last request wins wholesale - a fresh `NextScenario` replaces a queued
one and resets its clock.

</details>

## Ship state

### SetSpeedCap

Install, update or remove the soft manual-speed governor on a scoped ship.

```ron
SetSpeedCap((id: "player_spaceship", cap: Some(250.0))),
SetSpeedCap((id: "player_spaceship"))              // release the governor
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root |
| `cap` | `Option` number | `None` | `Some(250.0)` installs/updates the cap, in m/s; `None` or omitted REMOVES it |

The governor limits the ship's TOTAL speed, not the speed along whatever
heading it points: a pilot who turns and burns again spends the same one
allowance. It is soft - the manual burn tapers off over the last stretch below
the cap - and it never blocks a burn that slows the ship, so a ship carried
past the cap by a well or a maneuver can always brake back inside it. Only the
MANUAL burn reads it; the autopilot plans its own deceleration.

</details>

### SetControllerVerb

Grant or withhold one flight verb on a scoped ship's controller - the
tutorial-progression primitive (First Shift starts with `Goto` withheld
and grants it when the player first locks the planetoid).

```ron
SetControllerVerb((id: "player_spaceship", verb: Goto, enabled: true)),
// The battery answers a salvo by itself - until this takes it away.
SetControllerVerb((id: "player_spaceship", verb: PointDefense, enabled: false)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root |
| `verb` | verb | required | `Stop` / `Goto` / `Orbit` / `Lock` / `Rcs` / `PointDefense` (bare enum, no quotes). Convention: never withhold `Stop` - an engaged autopilot should always be cancelable |
| `enabled` | bool | required | `true` grants, `false` withholds |

The spawn-time twin is the `DisableVerb` section modification (see
[Spaceship](../objects/#spaceship)); this action is its runtime mirror.

</details>

### SetAllegiance

Overwrite a scoped ship's side at runtime - the neutral-until-provoked
primitive: spawn a bystander `Neutral`, flip it `Enemy` on a trigger.

```ron
SetAllegiance((id: "magpie", allegiance: Enemy)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root; a dangling id is a lint Error |
| `allegiance` | side | required | `Player` / `Enemy` / `Neutral` (bare enum) |

</details>

### Helm orders

The six actions below take the helm of a ship the scenario owns: one authored
with `controller: None` OR one authored with `controller: AI` (see
[Spaceship](../objects/#spaceship)). They refuse the PLAYER's ship - a lint
Error, and a runtime error if one somehow reaches the engine - because a
player's flight input drops any autopilot on the next frame, so the order
would fight the stick and lose.

An order on an AI ship OUTRANKS the bot. While the order runs, the AI stops
writing to the helm; it keeps looking around and keeps shooting. This is the
seam that lets a mission tell an ordinary patrol craft to go somewhere
specific and then hand it back, instead of authoring a second inert hull.

The ship still needs to be a real ship: the helm orders fly it with its own
flight computer and thrusters, so a hull with no live controller section
cannot turn, and a battery with no drives can still shoot but never move.

`Move`, `Align`, `Stop`, `Patrol` and `Orbit` are ONE mutually exclusive
family. A ship holds at most one helm order; installing a second cancels the
first. Each is keyed, and the
[order events](../events/#onshipordercomplete) with a
[`ShipOrder`](../filters/#shiporder) filter are how the next beat waits for it
- which is what lets a
[`Sequence`](#sequence) `until` gate hold a set piece together.

Some orders end and let go; others end and KEEP HOLDING. `Move`, `Stop` and
`Patrol` release the helm when they report, so an AI ship goes straight back
to its own routine. `Align` and `Orbit` report the condition and then hold it
until something else takes the helm - the hold is the point of both.

### MoveShipTo

Fly an ordered ship to a mark and report when it gets there. The ship's own
GOTO maneuver, so it accelerates, coasts and arrives on its authored thrust -
this is the cinematic approach, not a teleport.

```ron
MoveShipTo((
    order: "close_the_gap",
    ship: "warship",
    position: (0.0, 0.0, -1200.0),
    arrival_standoff: Some(60.0),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `order` | string | required | the key this order's completion is reported under; an empty key is a lint Error |
| `ship` | string | required | scoped `None`- or AI-controller ship root; a dangling or player-driven id is a lint Error |
| `position` | meters `(x, y, z)` | required | the mark to fly to, in world coordinates |
| `arrival_standoff` | `Option` meters | `None` | the margin to leave between the ship's own hull and the mark; `Some(0.0)` parks the hull's face on it |

Completes when the ship's autopilot lets go - it is inside the standoff and
settled - and fires
[`OnShipOrderComplete`](../events/#onshipordercomplete) with `kind: Move`.

`arrival_standoff` exists because the default 500 m is far too coarse to
stage a shot with. It is a MARGIN, not a centre distance: the ship comes to
rest with that much clear water between its own hull and the mark, so a
warship and a shuttle sent to the same mark both stop where you staged them.
It is installed for the life of the order and taken back off when the order
retires - on completion as well as on a cancel or an interruption - so a
cinematic's tight staging does not silently retune every later GOTO the hull
flies. `None` uses the ship's own standoff.

A margin is a PARKING rule, not a route guarantee. The leg flies a straight
line and nothing is dodged on the way in; stage the approach yourself if
something is in it.

</details>

### ForceAlign

Turn an ordered ship's whole hull onto a point and HOLD it there. Rotation
only - no autopilot, so no drive ever burns for translation and the ship keeps
whatever velocity it had.

```ron
ForceAlign((
    order: "bore_on_target",
    ship: "warship",
    look_at: (0.0, 0.0, 0.0),
    tolerance_degrees: 1.5,
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `order` | string | required | the key this order's completion is reported under |
| `ship` | string | required | scoped `None`- or AI-controller ship root |
| `look_at` | meters `(x, y, z)` | required | the world position to put under the bore |
| `tolerance_degrees` | number | required | how close the aim must come before the order completes; must be finite and not negative, or the order is refused |

The hold is the point. A [railgun](../sections/#railgun) does not traverse, so
the shot leaves down whatever line the hull is holding: align, wait for the
completion, then fire, and the slug goes where you aimed. The facing is held
until another helm order replaces it, which is what lets several guns run
their charges on one bearing.

The tolerance is also what "settled" is measured against - a tight tolerance
asks for a genuinely steady hull, a coarse one accepts a drift - so an
alignment order on a tumbling wreck may take a while, and one with an
impossible tolerance is refused outright rather than left never completing.

</details>

### StopShip

Bring an ordered ship to rest with the real STOP maneuver - the same
flip-retrograde-and-burn the player's X key runs, so it costs fuel and time,
visibly.

```ron
StopShip((order: "hold_here", ship: "warship")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `order` | string | required | the key this order's completion is reported under |
| `ship` | string | required | scoped `None`- or AI-controller ship root |

Completes when the ship is at rest, with `kind: Stop`. A ship that already is
at rest completes almost immediately, which is the cheap way to make a beat
wait for "it definitely is not drifting any more".

</details>

### PatrolShip

Fly ONE loop of an authored waypoint route: the ship visits each mark in
order, returns to the first one, and reports. This is the sweep-and-come-back
beat - not a standing assignment.

```ron
PatrolShip((
    order: "sweep_the_belt",
    ship: "picket",
    waypoints: [
        (0.0, 0.0, -800.0),
        (900.0, 0.0, -800.0),
        (900.0, 0.0, 0.0),
    ],
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `order` | string | required | the key this order's completion is reported under |
| `ship` | string | required | scoped `None`- or AI-controller ship root |
| `waypoints` | list of meters `(x, y, z)` | required | the marks to visit, in order; an EMPTY list is a lint Error |

ONE loop, then `kind: Patrol` and the helm goes back. A route of one point is
one leg out and one leg home; a route with the same point twice in a row still
flies both legs, and the second lands immediately. Repeat the action to run
another loop, or hand the ship a
[`ClearShipOrder`](#clearshiporder) mid-route to abandon it.

For a STANDING patrol that never ends, author the route on the ship instead -
an AI ship's own `patrol` field (see [Spaceship](../objects/#spaceship)) is
the routine it returns to when nothing is happening.

</details>

### OrbitShip

Put a ship in a stable ring around a gravity well and hold it there. The
ship's own ORBIT maneuver, so it aligns, burns into the band, and stays.

```ron
OrbitShip((order: "take_the_ring", ship: "surveyor", well: "planetoid")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `order` | string | required | the key this order's completion is reported under |
| `ship` | string | required | scoped `None`- or AI-controller ship root |
| `well` | string | required | scoped id of the gravity-well object to orbit; a dangling id is a lint Error |

Completes with `kind: Orbit` the moment the ring is ESTABLISHED - the ship is
in the band and holding - and then keeps holding it, the same way
[`ForceAlign`](#forcealign) keeps its bearing. A completion here means "it is
in orbit now", not "it has stopped orbiting".

A well too small, too far, or with no stable band for this hull fails the
order rather than leaving the beat waiting: see
[`OnShipOrderFailed`](../events/#onshiporderfailed).

</details>

### ClearShipOrder

Release a ship's helm. The counterpart to the five orders above: whatever the
ship was told, it is no longer being told it.

```ron
ClearShipOrder((ship: "warship")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped ship root to release |

A `None`-controller hull KEEPS its velocity and coasts - this is space, and
letting a ship drift out of frame is usually the point. An AI ship goes back
to its own routine on the next frame.

Emits [`OnShipOrderCanceled`](../events/#onshipordercanceled), NOT a
completion: an order that was called off did not finish, so a beat gated on
the completion correctly never runs. It also puts back the ship's own arrival
standoff if a [`MoveShipTo`](#moveshipto) had displaced it. Clearing a ship
that is under no order does nothing and says nothing.

</details>

### Forced fire

The two weapon actions below are independent of the helm and of each other: a
ship can be aligned, firing its spinal gun and launching a bay in the same
frame.

Unlike the helm orders, these still refuse an AI-driven ship as well as the
player's. A bot picks its own targets and times its own shots, and a scripted
trigger pull cutting across that would fight the same weapon seams every
frame.

### ForceRailgunFire

Fire one named [railgun](../sections/#railgun) section of a scripted ship.

```ron
ForceRailgunFire((ship: "warship", section: "spinal")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped `None`-controller ship root that fires |
| `section` | string | required | the authored section id of the railgun; a section the hull does not carry, or one that is not a railgun, is a lint Error |

No target and no steering: a railgun does not traverse, so the shot leaves
down whatever line the hull holds when the charge completes. Putting that line
on something is [`ForceAlign`](#forcealign)'s job. Everything else is the
gun's own behavior - the authored charge time, the magazine, the reload, the
recoil through the hull, the slug, the sound and the flash.

The order is ONE SHOT: it holds the trigger until the gun actually fires
(through a reload, if the magazine was empty) and then retires itself. Fire a
second shell with a second action.

</details>

### ForceTorpedoFire

Launch one named [torpedo bay](../sections/#torpedo) of a scripted ship at one
named target.

```ron
ForceTorpedoFire((ship: "battery_west", section: "bay", target: "patrol_ship")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped `None`-controller ship root that launches |
| `section` | string | required | the authored section id of the bay; a missing id or a non-bay is a lint Error |
| `target` | string | required | scoped ship root the ordnance homes on; a dangling id is a lint Error |

ONE bay, addressed by its section id. The bay's own cooldown and ammo still
time the launch, and the ordnance is committed to the target the same way an
AI or player launch is - which is also what makes the torpedo visible to
hostile point defense. The AI's launch gates (range envelope, hull-forward
cone, line of sight, the AI cadence) do NOT apply: the script is the decision.

A missing target skips the launch entirely, so no dud goes out while a target
is mid-respawn. The order is one shot, like the railgun's.

</details>

### SetInfiniteAmmo

Suspend (or restore) the finite magazine on every weapon of a scoped ship, so
a training range or a set-piece is not decided by a reload.

```ron
SetInfiniteAmmo((id: "player_spaceship", enabled: true)),
SetInfiniteAmmo((id: "player_spaceship", enabled: false)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root; a dangling id is a lint Error |
| `enabled` | bool | required | `true` suspends the magazines, `false` restores them |

`true` puts each weapon's finite magazine aside and lets the section fire on
its cooldown alone. `false` gives the magazine back at the section's own
AUTHORED capacity, full, with the reload re-seeded from the section
prototype - not at whatever count was left when it was suspended. A count
from before the suspension is not a count the run earned either, and a full
magazine is the one state a player and an author can both predict.

Turning it on twice is idempotent, and turning it off on a ship that never had
it on does nothing. A weapon authored with no magazine at all was already
unlimited and is untouched in both directions.

</details>

### RefillAmmo

Top a scoped ship's finite magazines back up without changing how they work.

```ron
RefillAmmo((id: "player_spaceship")),                          // every weapon
RefillAmmo((id: "player_spaceship", section: Some("turret_dorsal"))), // just one
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `id` | string | required | scoped ship root; a dangling id is a lint Error |
| `section` | `Option` string | `None` | one section's authored id, written `Some("...")`; omitted refills every weapon on the ship |

Each magazine returns to its authored capacity and any reload in flight is
cleared. Weapons with no finite magazine - and any whose magazine is
currently suspended by [`SetInfiniteAmmo`](#setinfiniteammo) - are skipped.

</details>

### AI constraints

The three actions below retune an AI ship's JUDGEMENT at runtime: how far it
will chase, how far it looks, how close it lets a torpedo come. They accept
ONLY an AI-controller ship - a hull nobody is thinking for has nothing to
retune, so a `None` or player id is a lint Error.

They are independent of each other and of the helm. Each takes ONE optional
payload: set it to install or update the constraint, omit it to put the ship
back on the engine default. They are also outranked by a helm order - while
one runs, the AI is not flying, so a constraint change has no visible effect
until the order lets go.

The spawn-time twins are the AI controller's own `leash`, `engage_range` and
`pd_range` fields (see [Spaceship](../objects/#spaceship)); these are their
runtime mirrors.

### SetAILeash

Tether an AI ship's combat to a centre and a radius, or release it. Past the
radius the ship breaks off and comes home.

```ron
SetAILeash((ship: "picket", leash: Some((center: (0.0, 0.0, -800.0), radius: 2000.0)))),
SetAILeash((ship: "picket")),   // let it chase freely
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped AI-controller ship root |
| `leash` | `Option` leash | `None` | `Some((center: ..., radius: ...))` installs or updates the tether; omitted RELEASES it |
| `leash.center` | meters `(x, y, z)` | required | the anchor the radius is measured from, in world coordinates |
| `leash.radius` | meters | required | how far from the anchor combat may go; must be positive |

Widening a leash mid-scenario is how a garrison is let off its post; releasing
it entirely turns a picket into a pursuer. The anchor is an authored point,
not the ship - moving the ship does not move the tether.

</details>

### SetAIEngageRange

Change how far an AI ship looks for hostiles, or restore the engine's 4 km
default.

```ron
SetAIEngageRange((ship: "watchtower", range: Some(16000.0))),
SetAIEngageRange((ship: "watchtower")),   // back to the default
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped AI-controller ship root |
| `range` | `Option` meters | `None` | `Some(16000.0)` installs or updates the range; omitted RESTORES the default; must be positive |

Wide is a long-watch emplacement that wakes for targets parked outside
everyone else's detection. Short is a ship that ignores a brawl next door.
Narrowing it mid-fight does NOT drop a target it has already acquired.

</details>

### SetAIPointDefenseRange

Change how close an inbound torpedo gets before an AI ship's guns answer it,
or restore the engine's 1.5 km default.

```ron
SetAIPointDefenseRange((ship: "escort", range: Some(600.0))),
SetAIPointDefenseRange((ship: "escort")),   // back to the default
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped AI-controller ship root |
| `range` | `Option` meters | `None` | `Some(600.0)` installs or updates the range; omitted RESTORES the default; must be positive |

Short stages the intercept close in, where the player can see it. Past the
turret's own ~1.8 km reach it only wastes the opening shots, so authoring
wider than the guns can shoot buys nothing.

</details>

## Variables, timers & debugging

### TimerStart

Start a keyed scenario timer. Starting an existing key restarts its deadline.

```ron
TimerStart((
    key: "orbit_hold",
    seconds: Term(Factor(Literal(Number(8.0)))),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `key` | string | required | scenario-local timer key |
| `seconds` | numeric expression | required | live, unpaused seconds until `OnTimerEnd` |

The duration is a numeric expression and must evaluate to a positive finite
number. Invalid values log an error and leave an existing timer unchanged.

Timers freeze under pause and clear on retry or teardown. Use an
[`OnTimerEnd`](../events/#ontimerend) handler with a
[`Timer`](../filters/#timer) filter to react once.

A timer earns its handler when the beat must still ASK something when it
lands - is the wing still alive, which line fits. A run of beats that only
waits for later is a [`Sequence`](#sequence) instead.

</details>

### TimerCancel

Cancel a running timer. A missing key is a no-op.

```ron
TimerCancel((key: "orbit_hold")),
```

### VariableSet

Evaluate an expression against the CURRENT variables and store the result -
the write half of the whole
[variable vocabulary](../expressions/). Re-evaluated per event, so
`n = n + 1` accumulates.

```ron
VariableSet((
    key: "asteroids_destroyed",
    expression: Add(Factor(Name("asteroids_destroyed")), Term(Factor(Literal(Number(1.0))))),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `key` | string | required | mutable variable to write (overwrites); writing a watched variable is a lint ERROR |
| `expression` | expression node | required | see the [expression grammar](../expressions/); an evaluation error skips the write |

In the in-game editor the expression is not a text field: the action opens a
`Value` page under its `Key`, a row per node, exactly as an expression filter
opens its condition. See
[the typed form](../expressions/#the-typed-form-in-the-editor).

</details>

### DebugMessage

Log a line (debug builds; run with `--features dev` while iterating). No
game effect - sprinkle these to watch handlers fire.

```ron
DebugMessage((message: "gate 3 armed")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `message` | string | required | the log line |

</details>

## Camera & photo mode

### SetCamera

Pin the scenario camera at a pose (drops free-fly control; the pose is
re-enforced every frame).

```ron
SetCamera((position: (0.0, 300.0, 800.0), look_at: (0.0, 0.0, 0.0))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `position` | 3-tuple | required | world camera position, meters |
| `look_at` | 3-tuple | required | world point to face, meters (up is +Y) |

Part of the screenshot/photo surface; no scenario camera present is a warn
no-op.

</details>

### SetCameraAnchor

Ride the camera on a scenario object at a fixed offset, facing what you name.
The shot follows the object every frame, so a cinematic can hold the player's
own ship in frame while they are still flying it.

```ron
// Over the player's shoulder, watching the carrier come apart.
SetCameraAnchor((anchor: "cutter", offset: (140.0, 55.0, -195.0), frame: World, look_at: Point((-1000.0, 0.0, 2500.0)))),
// Riding the hull, watching whatever is coming.
SetCameraAnchor((anchor: "cutter", offset: (-165.0, 30.0, 70.0), look_at: Object("warship"))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `anchor` | object id | required | the scoped object the camera rides |
| `offset` | 3-tuple | required | where the camera sits relative to it, meters |
| `frame` | `Local` \| `World` | `Local` | whether `offset` turns with the anchor's hull or stays on world axes |
| `look_at` | `Anchor` \| `Point((x,y,z))` \| `Object("id")` | `Anchor` | what the shot faces |

`Local` composes the same shot whatever heading the anchor is on - an
over-the-shoulder chase. `World` composes the same shot whatever the anchor is
DOING, which is what you want while the player is still flying: their heading
is not yours to choose.

`look_at: Object` follows a live object and falls back to the anchor if that
object dies mid-shot; `look_at: Point` is fixed world space and survives
whatever was standing there. Frame something that is about to be destroyed with
`Point`, not `Object`.

Camera authority only. It never steers or stops the ship it rides and does not
change player input authority. Like `SetCamera` it drops free-fly control and
re-enforces the pose every frame; unlike `SetCamera` there is something to hand
back to, so pair it with [`ReleaseCamera`](#releasecamera). Pair a staged shot
that must own gameplay input with [`SuspendPlayerControl`](#suspendplayercontrol)
and explicitly resume when the shot returns control. An anchor id that names
nothing warns and leaves the camera alone.

</details>

### ReleaseCamera

Drop every scripted camera override and hand the view back to the player's
normal chase rig.

```ron
ReleaseCamera(()),
```

<details class="explain">
<summary>Show explanation</summary>

No fields. Takes off both [`SetCamera`](#setcamera)'s fixed pose and
[`SetCameraAnchor`](#setcameraanchor)'s ride, whichever is on. The chase rig
never stopped solving underneath, so the shot ends and the player is looking
out of their own ship again - there is no restore pose to author.

Harmless when nothing is pinned. This is camera-only: it does not implicitly
resume player control.

</details>

### SuspendPlayerControl

Block human flight, look, targeting, combat-stance, and weapon input while the
simulation and scripted scene continue.

```ron
SuspendPlayerControl(()),
```

<details class="explain">
<summary>Show explanation</summary>

No fields. The action immediately clears held burn, RCS, rotation, radar,
combat stance, thruster, turret, torpedo, and railgun intent so an input pressed
before the cut cannot remain latched. Pause, menu, and an explicitly always-live
cinematic-skip binding remain available. Physics, timers, AI, scripted ships,
weapons already in flight, and camera actions continue.

Repeated suspension is harmless. Scenario teardown always restores control,
but authored content should still pair each interval with
[`ResumePlayerControl`](#resumeplayercontrol).

</details>

### ResumePlayerControl

Restore human gameplay input after an explicit suspension.

```ron
ResumePlayerControl(()),
```

<details class="explain">
<summary>Show explanation</summary>

No fields. Repeated resume is harmless. This is input-only: pair it with
[`ReleaseCamera`](#releasecamera) when returning both view and control.

</details>

### Screenshot

Capture the primary window to a PNG. A dev tool - pair `SetCamera` + settle
frames + `Screenshot` to script a framed shot.

```ron
Screenshot((path: "shots/my_scene.png")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `path` | string | required | output PNG path |

Relative paths land under the `NOVA_CAPTURE_DIR` env var when set; parent
directories are created.

</details>

### SetSkybox

Swap the scenario's skybox mid-scenario.

```ron
SetSkybox((cubemap: "self://textures/nebula.png", brightness: Some(700.0))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `cubemap` | asset ref | required | the new cubemap path (`"self://textures/nebula.png"`) |
| `brightness` | `Option` number | `None` | multiplier; `None` keeps the current brightness (initial scenario default 1000) |

The install is deferred until the new image has loaded; a failed load leaves
the sky unchanged (warned).

</details>
