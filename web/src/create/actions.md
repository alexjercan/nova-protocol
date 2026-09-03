# Actions

Everything a handler can DO. Actions run in authored order once every filter
passes; each is a newtype variant - `Name((field: value, ...))`, double
parens even for one field. Failures warn and continue (a missing target id
never panics a scenario). All 33 at a glance:

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
| [`MoveShipTo`](#moveshipto) | [ship state](#ship-state) | fly a scripted ship to a mark and report when it arrives |
| [`ForceAlign`](#forcealign) | [ship state](#ship-state) | turn a scripted ship's nose onto a point and hold it there |
| [`StopShip`](#stopship) | [ship state](#ship-state) | bring a scripted ship to rest with the real STOP burn |
| [`ClearShipOrder`](#clearshiporder) | [ship state](#ship-state) | cancel whatever helm order a scripted ship is under |
| [`ForceRailgunFire`](#forcerailgunfire) | [ship state](#ship-state) | fire one named railgun section |
| [`ForceTorpedoFire`](#forcetorpedofire) | [ship state](#ship-state) | launch one named torpedo bay at a named target |
| [`SetInfiniteAmmo`](#setinfiniteammo) | [ship state](#ship-state) | suspend or restore the finite magazine on every weapon of a ship |
| [`RefillAmmo`](#refillammo) | [ship state](#ship-state) | top a ship's magazines back up, or just one section's |
| [`VariableSet`](#variableset) | [variables](#variables-timers-debugging) | evaluate an expression and store the result in a variable |
| [`TimerStart`](#timerstart) | [variables](#variables-timers-debugging) | start (or restart) a keyed scenario timer |
| [`TimerCancel`](#timercancel) | [variables](#variables-timers-debugging) | cancel a running timer |
| [`DebugMessage`](#debugmessage) | [variables](#variables-timers-debugging) | log a line in debug builds |
| [`SetCamera`](#setcamera) | [camera](#camera-photo-mode) | pin the scenario camera at a pose |
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
    kind: Asteroid((radius: 50.0, texture: "dep://base/textures/asteroid.png", invulnerable: false)),
)),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `base` | object base | required | identity + pose (below) |
| `kind` | object kind | required | `Anchor((..))` / `Asteroid((..))` / `Spaceship((..))` / `Beacon((..))` / `SalvageCrate((..))` / `Light(..)` |

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
        kind: Asteroid((radius: 10.0, texture: "dep://base/textures/asteroid.png", invulnerable: false)),
    ),
    asteroid_radius: Some((10.0, 30.0)),
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
| `min_separation` | `Option` number | `None` | minimum centre-to-centre distance in meters against EVERY body scattered so far this scenario, earlier scatters included; 64 placement tries per copy, unplaceable copies are DROPPED, never overlapped |

`region` variants (struct variants - single parens, named fields):

| variant | fields | meaning |
|---|---|---|
| `Box(min: (..), max: (..))` | both required | uniform per axis in `[min, max]` |
| `Ring(center: (..), inner: .., outer: .., y_min: .., y_max: ..)` | `center` defaults to the origin | horizontal annulus: uniform angle, radius in `[inner, outer]`, y in `[y_min, y_max]` |

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
order, ~8 s hold each, at most three visible). One line per beat is the
style; the queue is the safety net.

```ron
StoryMessage((speaker: "Foreman Okono", text: "Strip it clean, Kestrel.", dwell: Some(12.0))),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `speaker` | string | required | the `SPEAKER >` prefix |
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
Outcome((outcome: Defeat, message: Some("The convoy is lost."))),
NextScenario((scenario_id: "lifeline", linger: true)),
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
NextScenario((scenario_id: "broadside_gunship", linger: true)),
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

</details>

### SetControllerVerb

Grant or withhold one flight verb on a scoped ship's controller - the
tutorial-progression primitive (the Shakedown Run starts with `Goto`
withheld and grants it at the beacon).

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

### Scripted ships

The six actions below drive a ship the scenario owns outright: one authored
with `controller: None` (see [Spaceship](../objects/#spaceship)). They refuse
a player-driven or AI-driven ship - a lint Error, and a runtime error if one
somehow reaches the engine - because taking the helm from either would lose. A
player's flight input drops any autopilot on the next frame, and the AI
rewrites the same seams every frame it runs.

The ship still needs to be a real ship: the helm orders fly it with its own
flight computer and thrusters, so a hull with no live controller section
cannot turn, and a battery with no drives can still shoot but never move.

`Move`, `Align` and `Stop` are ONE mutually exclusive family. A ship holds at
most one helm order; installing a second retires the first. Each is keyed, and
[`OnShipOrderComplete`](../events/#onshipordercomplete) with a
[`ShipOrder`](../filters/#shiporder) filter is how the next beat waits for it
- which is what lets a
[`Sequence`](#sequence) `until` gate hold a set piece together.

The two weapon actions are independent of the helm and of each other: a ship
can be aligned, firing its spinal gun and launching a bay in the same frame.

### MoveShipTo

Fly a scripted ship to a mark and report when it gets there. The ship's own
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
| `ship` | string | required | scoped `None`-controller ship root; a dangling or driven id is a lint Error |
| `position` | meters `(x, y, z)` | required | the mark to fly to, in world coordinates |
| `arrival_standoff` | `Option` meters | `None` | how far short of the mark to come to rest |

Completes when the ship's autopilot lets go - it is inside the standoff and
settled - and fires
[`OnShipOrderComplete`](../events/#onshipordercomplete) with `kind: Move`.

`arrival_standoff` exists because the default 500 m is far too coarse to
stage a shot with. It is installed for the life of the order and taken back
off when the order retires, so a cinematic's tight staging does not silently
retune every later GOTO the hull flies. `None` uses the ship's own standoff.

</details>

### ForceAlign

Turn a scripted ship's whole hull onto a point and HOLD it there. Rotation
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
| `ship` | string | required | scoped `None`-controller ship root |
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

Bring a scripted ship to rest with the real STOP maneuver - the same
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
| `ship` | string | required | scoped `None`-controller ship root |

Completes when the ship is at rest, with `kind: Stop`. A ship that already is
at rest completes almost immediately, which is the cheap way to make a beat
wait for "it definitely is not drifting any more".

</details>

### ClearShipOrder

Release a scripted ship's helm and let it drift. The counterpart to the three
orders above: whatever the ship was told, it is no longer being told it.

```ron
ClearShipOrder((ship: "warship")),
```

<details class="explain">
<summary>Show explanation</summary>

| field | type | default | meaning |
|---|---|---|---|
| `ship` | string | required | scoped ship root to release |

The hull KEEPS its velocity - this is space, and letting a ship coast out of
frame is usually the point. Emits NO completion event: a cancelled order did
not finish, so a beat gated on it correctly never runs. It also puts back the
ship's own arrival standoff if a
[`MoveShipTo`](#moveshipto) had displaced it.

</details>

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
